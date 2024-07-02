use std::io::{BufReader, Write};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::Read,
};

use crate::git_errors::command_error::CommandError::{self, IncorrectAmount, InvalidBranch};
use crate::protocol::pack_file::{read_packfile, send_packfile};
use crate::protocol::pkt_line::read_pkt_line;
use crate::{
    branch::Branch, files::object_type::ObjectType, git_errors::errors::ErrorType,
    git_object::GitObject, hash::GitHash, index::Index, merge::Merge,
    protocol::pkt_line::create_pkt_line, refs::BranchRef, remote::Remote, repo_paths::RepoPaths,
};

const HASH_ZERO: [u8; 20] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const DEFAULT_REMOTE: &str = "origin";

////////////////////////////////////////////////////////////////////////////////////////////////////////
///                                         CLONE                                                    ///
////////////////////////////////////////////////////////////////////////////////////////////////////////
pub fn clone_command(repo_paths: RepoPaths, args: Vec<String>) -> Result<(), ErrorType> {
    let _ = crate::init::git_init(repo_paths.clone())?;

    if args.len() != 1 {
        return Err(ErrorType::CommandError(IncorrectAmount(
            "1".to_string(),
            args.len(),
        )));
    }

    // todo remote hardcodeado
    Remote::add(
        repo_paths.get_remote(),
        DEFAULT_REMOTE.to_string(),
        args[0].to_string(),
    )?;

    fetch_command(repo_paths.clone())?;

    let remote_head_path = repo_paths.get_remote_head();
    if !remote_head_path.exists() {
        println!("Remote repository succesfully cloned");
        return Ok(());
    }

    let head_commit = fs::read_to_string(remote_head_path)?;
    let head_commit_hash = GitHash::new(&head_commit)?;
    let remote_refs = Branch::list_branches(&repo_paths.get_refs_remote())?;

    let mut head_branch = None;

    for (branch_name, hash) in remote_refs {
        let branch = Branch::new(&branch_name, &repo_paths.get_refs_heads(), hash.clone())?;
        if hash == head_commit_hash {
            head_branch = Some(branch);
        }
        Remote::set_upstream(
            repo_paths.get_remote(),
            branch_name.clone(),
            DEFAULT_REMOTE.to_string(),
            branch_name,
        )?;
    }

    let head_branch = match head_branch {
        Some(h) => h,
        None => {
            let branch = match Branch::open(&repo_paths.get_refs_heads(), "master"){
                Ok(b) => b,
                Err(ErrorType::CommandError(InvalidBranch(_))) => return Err(ErrorType::ProtocolError(format!("couldn't clone because the remote HEAD commit ({head_commit_hash}) doesn't match with a valid remote branch and remote doesn't have a default 'master' branch"))),
                Err(e) => return Err(e)
            };
            println!("remote HEAD doesn't match with a valid remote ref. local HEAD set to default 'master' branch");
            branch
        }
    };

    let index = Index::open(&repo_paths.get_index())?;
    let mut head = BranchRef::open(repo_paths.get_head(), &repo_paths.get_refs_heads())?;
    head.checkout_to(
        head_branch,
        index,
        &repo_paths.get_home(),
        &repo_paths.get_objects(),
        &repo_paths.get_index(),
    )?;
    head.save()?;

    println!("Remote repository succesfully cloned");
    Ok(())
}

////////////////////////////////////////////////////////////////////////////////////////////////////////
//                                          FETCH                                                    ///
////////////////////////////////////////////////////////////////////////////////////////////////////////

/// Download objects and refs from another repository
pub fn fetch_command(repo_paths: RepoPaths) -> Result<usize, ErrorType> {
    // todo refactor remote
    let mut stream = Remote::connect_upload_pack(repo_paths.get_remote(), "origin".to_string())?;

    // references discovery
    let path_remote_branches = repo_paths.get_refs_remote();
    if !path_remote_branches.exists() {
        fs::create_dir_all(&path_remote_branches)?;
    }

    let (mut remote_refs, _) = read_server_refs(&mut stream)?;
    let local_remote_refs = Branch::list_branches(&path_remote_branches)?;

    if let Some(remote_head_hash) = remote_refs.remove("HEAD") {
        let mut file = File::create(repo_paths.get_remote_head())?;
        file.write_all(remote_head_hash.as_str().as_bytes())?;
    }

    if remote_refs == local_remote_refs {
        // up to date, flush
        stream.write_all(b"0000")?;
        stream.flush()?;
        return Ok(0);
    }

    let mut want_hashes = Vec::new();

    for (branch_name, remote_hash) in remote_refs {
        if let Some(local_hash) = local_remote_refs.get(&branch_name) {
            if local_hash == &remote_hash {
                continue;
            }
        }
        want_hashes.push(remote_hash.clone());
        Branch::new(&branch_name, &path_remote_branches, remote_hash)?;
    }

    // packfile negotiation
    for hash in want_hashes {
        let want_pkt_line = create_pkt_line(&format!("want {}", hash))?;
        stream.write_all(want_pkt_line.as_bytes())?;
    }

    stream.write_all("0000".as_bytes())?; // flush
    let done_pkt = create_pkt_line("done")?; //done
    stream.write_all(done_pkt.as_bytes())?;

    let pkt_line = read_pkt_line(&mut stream)?.unwrap_or("flush line (0000)".to_string());
    if pkt_line != "NAK" {
        return Err(ErrorType::ProtocolError(format!(
            "expected NAK line got {pkt_line}"
        )));
    }
    // read packfile

    let mut reader = BufReader::new(stream);
    let packfile_objects = read_packfile(&mut reader)?;
    println!("{} objects fetched from remote", packfile_objects.len());

    for (object_type, content) in packfile_objects.iter().cloned() {
        GitObject::save_object(content, object_type, &repo_paths.get_objects())?;
    }

    Ok(packfile_objects.len())
}

////////////////////////////////////////////////////////////////////////////////////////////////////////
///                                          PULL                                                    ///
////////////////////////////////////////////////////////////////////////////////////////////////////////

// "pull" -> pull from upstream
// "pull [remote-branch]"
pub fn pull_command(repo_paths: RepoPaths, args: Vec<String>) -> Result<(), ErrorType> {
    let index = Index::open(&repo_paths.get_index())?;
    index.check_for_changes(&repo_paths.get_home(), &repo_paths.get_ignore())?;
    let object_ammount = fetch_command(repo_paths.clone())?;
    if object_ammount == 0 {
        println!("Already up to date");
        return Ok(());
    }

    let mut head = BranchRef::open(repo_paths.get_head(), &repo_paths.get_refs_heads())?;
    let mut head_branch = match head.get_branch() {
        Some(b) => b,
        None => {
            // HEAD is uninitialized, creates new branch same as the remote one received and checks out to it
            if args.len() != 1 {
                return Err(ErrorType::RepositoryError("you must provide a remote branch name to pull from because HEAD isn't pointing to any branch".to_string()));
            }
            let remote_branch_name = args[0].clone();
            let remote_branch = Branch::open(&repo_paths.get_refs_remote(), &remote_branch_name)?;
            let branch = Branch::new(
                &remote_branch_name,
                &repo_paths.get_refs_heads(),
                remote_branch.get_last_commit_hash(),
            )?;
            let index = Index::open(&repo_paths.get_index())?;
            head.checkout_to(
                branch,
                index,
                &repo_paths.get_home(),
                &repo_paths.get_objects(),
                &repo_paths.get_index(),
            )?;
            head.save()?;
            println!(
                "local branch created from remote branch '{}'",
                remote_branch_name
            );
            return Ok(());
        }
    };
    let branch_name = parse_pull_branch_name(repo_paths.clone(), args, head_branch.get_name())?;
    let remote_branch = Branch::open(&repo_paths.get_refs_remote(), &branch_name)?;

    let (modified_files, conflic_files) = Merge::merge(
        &mut head_branch,
        remote_branch,
        repo_paths.clone(),
        None,
        None,
    )?;

    if !conflic_files.is_empty() {
        println!(
            "Your local changes conflict with remote changes in the following files :{}",
            conflic_files
                .into_iter()
                .map(|f| f.0.display().to_string())
                .collect::<Vec<String>>()
                .join(", ")
        );
        println!("Review them and commit the merge manually.");
        return Ok(());
    }

    head.checkout_to(
        head_branch,
        index,
        &repo_paths.get_home(),
        &repo_paths.get_objects(),
        &repo_paths.get_index(),
    )?;

    let method = if modified_files.is_empty() {
        "fast-forward"
    } else {
        println!("Your branch and the remote branch have diverged.");
        "three-way"
    };

    println!("Pull completed succesfully with {method} merge");

    Ok(())
}

fn parse_pull_branch_name(
    repo_paths: RepoPaths,
    args: Vec<String>,
    local_branch_name: String,
) -> Result<String, ErrorType> {
    if args.is_empty() {
        // pull from upstream
        let (_remote_name, remote_branch_name) =
            Remote::get_upstream(repo_paths.get_remote(), local_branch_name.clone())?.ok_or(
                ErrorType::ConfigError(format!(
                    "current branch '{}' doesn't have an upstream branch",
                    local_branch_name
                )),
            )?;
        if !repo_paths
            .get_refs_remote()
            .join(&remote_branch_name)
            .exists()
        {
            return Err(ErrorType::RepositoryError(format!(
                        "the current branch '{local_branch_name}' has an upstream which does not exist: '{remote_branch_name}'"
                    )));
        }
        Ok(remote_branch_name)
    } else {
        // pull from specified remote branch
        if args.len() != 1 {
            return Err(ErrorType::CommandError(IncorrectAmount(
                "1".to_string(),
                args.len(),
            )));
        }
        let remote_branch_name = args[0].clone();
        if !repo_paths
            .get_refs_remote()
            .join(&remote_branch_name)
            .exists()
        {
            return Err(ErrorType::CommandError(CommandError::InvalidArgument(format!(
                        "the given remote branch name '{remote_branch_name}' does not exist as a valid remote ref"
                    ))));
        }
        Ok(remote_branch_name)
        // todo refactor remote : que el comando sea pull <remote> <branch>
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////
///                                          PUSH                                                    ///
////////////////////////////////////////////////////////////////////////////////////////////////////////

// notes:
//  doesnt support tags. if remote sends one it will produce unwanted behaviour
//  it discards remote ref HEAD if it was sent
pub fn push_command(repo_paths: RepoPaths, args: Vec<String>) -> Result<(), ErrorType> {
    let index = Index::open(&repo_paths.get_index())?;
    index.check_for_changes(&repo_paths.get_home(), &repo_paths.get_ignore())?;

    //todo refactor remote
    let mut stream = Remote::connect_receive_pack(repo_paths.get_remote(), "origin".to_string())?;

    let local_refs = parse_refs_to_push(repo_paths.clone(), args)?;

    // server sends a list of all the references it has and the commit they are pointing to
    let (mut remote_refs, _) = read_server_refs(&mut stream)?;

    let mut head_remote_hash = match remote_refs.remove("HEAD") {
        Some(h) => h,
        None => GitHash::from_hex(&HASH_ZERO)?,
    };

    let (commands, commits_to_update) = references_to_update(local_refs, &remote_refs)?;

    let mut first = true;
    let head_local = BranchRef::open(repo_paths.get_head(), &repo_paths.get_refs_heads())?;
    if let Some(hash) = remote_refs.get(&head_local.get_branch_name().ok_or(
        ErrorType::RepositoryError("cant push if you havent commited yet".to_string()),
    )?) {
        head_remote_hash = hash.clone();
    }
    let head_local_commit_hash =
        head_local
            .get_last_commit_hash()
            .ok_or(ErrorType::RepositoryError(
                "cant push if you havent commited yet".to_string(),
            ))?;
    if head_remote_hash != head_local_commit_hash {
        first = false;
        let line = format!("{} {} HEAD\0", head_remote_hash, head_local_commit_hash);
        let pkt_line = create_pkt_line(&line)?;
        stream.write_all(pkt_line.as_bytes())?;
    }

    // client sends a list of commands on refs (update, create or delete)
    send_commands(&mut stream, commands, first)?;
    stream.write_all(b"0000")?; //flush

    // packfile construction:

    let repeated_hashes_remote: HashSet<GitHash> = remote_refs.values().cloned().collect();

    let packfile_objects = get_packfile_objects(
        commits_to_update,
        repeated_hashes_remote,
        &repo_paths.get_objects(),
    )?;

    send_packfile(&mut stream, packfile_objects)?;

    let mut report = Vec::new();

    stream.read_to_end(&mut report)?;

    println!("succesfully pushed to remote");
    Ok(())
}

fn parse_refs_to_push(
    repo_paths: RepoPaths,
    args: Vec<String>,
) -> Result<HashMap<String, GitHash>, ErrorType> {
    match args.as_slice() {
        [] => {
            // pushes just current branch
            let head = BranchRef::open(repo_paths.get_head(), &repo_paths.get_refs_heads())?;
            let branch = head.get_branch().ok_or(ErrorType::RepositoryError(
                "can't push if HEAD is not pointing to a valid branch".to_string(),
            ))?;
            let mut local_refs = HashMap::new();
            local_refs.insert(branch.get_name(), branch.get_last_commit_hash());
            Ok(local_refs)
        }
        [branch_name] => {
            if branch_name == "--all" {
                //pushes all
                Ok(Branch::list_branches(&repo_paths.get_refs_heads())?)
            } else {
                // pushes just specified branch
                let branch = Branch::open(&repo_paths.get_refs_heads(), branch_name)?;
                let mut local_refs = HashMap::new();
                local_refs.insert(branch_name.to_string(), branch.get_last_commit_hash());
                Ok(local_refs)
            }
        }
        // [remote_name, branch_name] => { // todo refactor remote

        // }
        _ => Err(ErrorType::CommandError(IncorrectAmount(
            "1".to_string(),
            args.len(),
        ))),
    }
}

// given the local references and the ones sent by remote return which ones will be updated (update, delete, create)
// and all the objects reachable by each new referenced commit which will then be included in the packfile
type ReferencesVec = Vec<(GitHash, GitHash, String)>;
fn references_to_update(
    local_refs: HashMap<String, GitHash>,
    remote_refs: &HashMap<String, GitHash>,
) -> Result<(ReferencesVec, Vec<GitHash>), ErrorType> {
    let mut commands = Vec::new();
    let mut commits_to_update: Vec<GitHash> = Vec::new();

    for (local_ref, local_hash) in &local_refs {
        if let Some(remote_hash) = remote_refs.get(local_ref) {
            if local_hash == remote_hash {
                continue;
            }
            // update command : {old-id new-id refname}
            commands.push((remote_hash.clone(), local_hash.clone(), local_ref.clone()));
        } else {
            // create command :  {zero-id new-id refname}
            commands.push((
                GitHash::from_hex(&HASH_ZERO)?,
                local_hash.clone(),
                local_ref.clone(),
            ))
        }
        commits_to_update.push(local_hash.to_owned());
    }

    for (remote_ref, remote_hash) in remote_refs {
        if !local_refs.contains_key(remote_ref) {
            // delete command : {old-id zero-id refname}
            commands.push((
                remote_hash.clone(),
                GitHash::from_hex(&HASH_ZERO)?,
                remote_ref.clone(),
            ))
        }
    }

    Ok((commands, commits_to_update))
}

fn send_commands(
    stream: &mut std::net::TcpStream,
    commands: Vec<(GitHash, GitHash, String)>,
    mut first: bool,
) -> Result<(), ErrorType> {
    for command in commands {
        let line: String;
        if first {
            first = false;
            line = format!("{} {} refs/heads/{}\0", command.0, command.1, command.2);
        } else {
            line = format!("{} {} refs/heads/{}", command.0, command.1, command.2);
        }

        let pkt_line = create_pkt_line(&line)?;
        stream.write_all(pkt_line.as_bytes())?;
    }
    Ok(())
}

pub fn get_packfile_objects(
    commits_to_update: Vec<GitHash>,
    mut repeated_hashes_remote: HashSet<GitHash>,
    path_objects: &std::path::PathBuf,
) -> Result<Vec<(ObjectType, GitHash, Vec<u8>)>, ErrorType> {
    // finding which objects to send:
    // for each ref that was created or updated locally:
    //     client sends all the objects reachable from the commit pointed by the ref (commits, trees and blobs)
    //     until a parent commit was already on the server

    let mut packfile_objects: Vec<(ObjectType, GitHash, Vec<u8>)> = Vec::new();

    // todo convertir en commit.get_reachable_objects()
    for commit_hash in commits_to_update {
        let commit = GitObject::read_commit(&commit_hash, path_objects)?;
        let commit_history = commit.get_commits_history_rec(path_objects)?;
        for commit in commit_history {
            if !repeated_hashes_remote.insert(commit.get_hash()) {
                // break; // from here on remote should allready have all the objects
            }
            packfile_objects.push((ObjectType::Commit, commit.get_hash(), commit.get_content()?));
            let tree = commit.get_tree();
            let tree_hash = tree.get_hash()?;
            if !repeated_hashes_remote.insert(tree_hash.clone()) {
                continue;
            }
            packfile_objects.push((ObjectType::Tree, tree_hash, tree.generate_content()?));
            let tree_files = tree.get_files_vec();
            for (_, blob_hash) in tree_files {
                if !repeated_hashes_remote.insert(blob_hash.clone()) {
                    continue;
                }
                let blob = GitObject::read_blob(&blob_hash, path_objects)?;
                packfile_objects.push((
                    ObjectType::Blob,
                    blob_hash,
                    blob.get_content().as_bytes().to_vec(),
                ));
            }
        }
    }
    Ok(packfile_objects)
}

/////////////   UTILS   /////////////

type ReferencesHashMap = HashMap<String, GitHash>;
/// expects to receive hashes and references names, returns two hashmaps (ref: hash) one for tags other for heads
pub fn read_server_refs<R: Read>(
    stream: &mut R,
) -> Result<(ReferencesHashMap, ReferencesHashMap), ErrorType> {
    let mut first_line = true;
    let mut heads: HashMap<String, GitHash> = HashMap::new();
    let mut tags = HashMap::new();

    while let Some(mut ref_line) = read_pkt_line(stream)? {
        if first_line {
            if ref_line.len() == 9 && ref_line.contains("version") {
                continue;
            }
            if ref_line.starts_with("0000000000000000000000000000000000000000") {
                // no refs
                return Ok((heads, tags));
            }
            first_line = false;
            let (temp_ref_line, _) = match ref_line.split_once('\0') {
                // "_" = CAPABILITIES ARE IGNORED IN THIS FIRST VERSION
                Some((r, c)) => (r.to_string(), Some(c)),
                None => (ref_line, None),
            };
            ref_line = temp_ref_line;
        }
        let (ref_type, ref_name, hash) = parse_ref_line(ref_line)?;
        match ref_type.as_str() {
            "heads" => heads.insert(ref_name, hash),
            "tags" => tags.insert(ref_name, hash),
            _ => {
                return Err(ErrorType::ProtocolError(format!(
                    "invalid ref type '{}'",
                    ref_type
                )))
            }
        };
    }
    Ok((heads, tags))
}

pub fn parse_ref_line(ref_line: String) -> Result<(String, String, GitHash), ErrorType> {
    if let Some((hash, ref_name)) = ref_line.split_once(' ') {
        // todo : Treating all refs as heads, possibly discarding tags
        let ref_parts: Vec<&str> = ref_name.trim().split('/').collect();
        let ref_name;
        let ref_type;

        if ref_parts.len() == 1 && ref_parts[0] == "HEAD" {
            ref_name = "HEAD";
            ref_type = "heads";
        } else if ref_parts.len() != 3 || ref_parts.first() != Some(&"refs") {
            return Err(ErrorType::ProtocolError(format!(
                "invalid ref format '{}'",
                ref_parts.join("/")
            )));
        } else {
            ref_type = ref_parts[1];
            ref_name = ref_parts[2];
        }

        let hash = GitHash::new(hash)?;
        return Ok((ref_type.to_string(), ref_name.to_string(), hash));
    }
    Err(ErrorType::ProtocolError(format!(
        "invalid ref line '{}'",
        ref_line
    )))
}
