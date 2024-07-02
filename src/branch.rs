use std::collections::HashMap;
use std::{
    fs::{self, File},
    io::Write,
    // path::{self, Path, PathBuf},
    path::{Path, PathBuf},
};

use crate::remote::Remote;
use crate::{
    commit::Commit,
    git_errors::{
        command_error::CommandError::{self, IncorrectAmount, InvalidBranch},
        errors::ErrorType,
    },
    git_object::GitObject,
    hash::GitHash,
    refs::BranchRef,
    repo_paths::RepoPaths,
};

#[derive(Clone)]
pub struct Branch {
    path_branches: PathBuf,
    name: String,
    last_commit_hash: GitHash, // refactor objects convertir esto en hash
}
// pub struct Branch;

impl Branch {
    pub fn open(
        path_branches: &Path,
        branch_name: &str,
        // path_objects: &Path,
    ) -> Result<Self, ErrorType> {
        if !path_branches.join(branch_name).exists() {
            return Err(ErrorType::CommandError(CommandError::InvalidBranch(
                branch_name.to_string(),
            )));
        }
        let last_commit_hash = GitHash::new(&fs::read_to_string(path_branches.join(branch_name))?)?;

        Ok(Self {
            path_branches: path_branches.to_path_buf(),
            name: branch_name.to_string(),
            last_commit_hash,
        })
    }

    pub fn branch_command(repo_paths: &RepoPaths, args: Vec<String>) -> Result<String, ErrorType> {
        let head = BranchRef::open(repo_paths.get_head(), &repo_paths.get_refs_heads())?;
        let path_branches = repo_paths.get_refs_heads();
        match args.len() {
            0 => {
                let head_branch_name = fs::read_to_string(repo_paths.get_head())?;
                Self::display_branches(head_branch_name.clone(), path_branches)?;
                Ok(format!("Display branch {}.", head_branch_name))
            }
            1 => {
                // create branch
                let name = &args[0];
                let head_commit_hash = match head.get_last_commit_hash() {
                    Some(h) => h,
                    None => {
                        return Err(ErrorType::RepositoryError(
                            "Cannot branch if there are no commits".to_string(),
                        ))
                    }
                };

                Self::new(name, &repo_paths.get_refs_heads(), head_commit_hash)?;
                let result = format!("Branch {name} successfully created.");
                println!("{}", result.clone());
                Ok(result)
            }
            _ => {
                let option: &str = &args[0];

                if option.starts_with("--set-upstream-to=") {
                    // --set-upstream-to=origin/rama-remota rama-local
                    if args.len() != 2 {
                        return Err(ErrorType::CommandError(IncorrectAmount(
                            "2".to_string(),
                            args.len(),
                        )));
                    }
                    let remote_branch = option.split('=').collect::<Vec<&str>>()[1];
                    Self::set_upstream(remote_branch, args[1].clone(), repo_paths)?;
                    Ok(format!(
                        "Set branch '{}' upstream to {}.",
                        args[1], remote_branch
                    ))
                } else if option == "-u" {
                    if args.len() != 3 {
                        return Err(ErrorType::CommandError(IncorrectAmount(
                            "3".to_string(),
                            args.len(),
                        )));
                    }
                    Self::set_upstream(args[1].as_str(), args[2].clone(), repo_paths)?;
                    return Ok(format!("Set branch '{}' upstream to {}.", args[2], args[1]));
                } else if (option == "-d") | (option == "delete") {
                    Self::delete_branch(
                        args[1].as_str(),
                        repo_paths.get_refs_heads(),
                        repo_paths.get_head(),
                    )?;
                    return Ok(format!("Delete branch {}.", option));
                } else {
                    return Err(ErrorType::CommandError(CommandError::InvalidArgument(
                        option.to_string(),
                    )));
                }
            }
        }
    }

    fn display_branches(head_branch_name: String, path_refs: PathBuf) -> Result<(), ErrorType> {
        let branches = Self::list_branches(&path_refs)?;

        for (mut name, _) in branches {
            if name == head_branch_name {
                name += " <--HEAD";
            }
            println!("{name}");
        }
        Ok(())
    }

    pub fn list_branches(path_branches: &Path) -> Result<HashMap<String, GitHash>, ErrorType> {
        let mut branches = HashMap::new();
        if !path_branches.exists() {
            return Ok(branches);
        }

        let entries = fs::read_dir(path_branches)?;
        let branch_names: Vec<String> = entries
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                if path.is_file() {
                    path.file_name()?.to_str().map(|s| s.to_owned())
                } else {
                    None
                }
            })
            .collect();
        for branch_name in branch_names {
            let path = path_branches.join(&branch_name);
            let hash = fs::read_to_string(path)?;
            let hash = GitHash::new(&hash)?;
            branches.insert(branch_name, hash);
        }
        Ok(branches)
    }

    fn set_upstream(
        remote_branch: &str,
        local_branch_name: String,
        repo_paths: &RepoPaths,
    ) -> Result<(), ErrorType> {
        let binding = remote_branch.split('/').collect::<Vec<&str>>();
        let (remote_name, remote_branch_name) = match binding.as_slice() {
            [remote_name, remote_branch_name] => {
                (remote_name.to_string(), remote_branch_name.to_string())
            }
            _ => {
                return Err(ErrorType::CommandError(CommandError::InvalidArgument(
                    format!("invalid remote repo '{}'", remote_branch),
                )))
            }
        };
        let remotes = Remote::get_remotes(repo_paths.get_remote())?;
        if !remotes.contains_key(&remote_name) {
            return Err(ErrorType::CommandError(CommandError::InvalidArgument(
                format!("no such remote '{}'", remote_name),
            )));
        }

        let remote_refs = Branch::list_branches(&repo_paths.get_refs_remote())?;

        if !remote_refs.contains_key(&remote_branch_name) {
            return Err(ErrorType::CommandError(CommandError::InvalidArgument(
                format!("no such remote branch '{}'", remote_branch_name),
            )));
        }
        let local_branches = Branch::list_branches(&repo_paths.get_refs_heads())?;

        if !local_branches.contains_key(&local_branch_name) {
            return Err(ErrorType::CommandError(CommandError::InvalidArgument(
                format!("no such local branch '{}'", local_branch_name),
            )));
        }
        Remote::set_upstream(
            repo_paths.get_remote(),
            local_branch_name,
            remote_name,
            remote_branch_name,
        )?;
        Ok(())
    }

    pub fn new(name: &str, path_branches: &Path, hash: GitHash) -> Result<Self, ErrorType> {
        let path_branch = path_branches.join(name);
        // if !path_branch.exists() {
        //     fs::create_dir_all(path_branches)?;
        // }
        let mut file = File::create(path_branch)?;
        file.write_all(hash.as_str().as_bytes())?;

        Ok(Self {
            path_branches: path_branches.to_path_buf(),
            name: name.to_string(),
            last_commit_hash: hash,
        })
    }

    pub fn delete_branch(
        name: &str,
        path_refs: PathBuf,
        path_head: PathBuf,
    ) -> Result<(), ErrorType> {
        let head_branch_name = fs::read_to_string(path_head)?;
        if head_branch_name == name {
            return Err(ErrorType::CommandError(CommandError::InvalidBranch(
                "Can't delete branch that head is pointing to".to_string(),
            )));
        }
        let path_branch = path_refs.join(name);
        if !path_branch.exists() {
            return Err(ErrorType::CommandError(InvalidBranch(format!(
                "Branch doesn't exist: {:?}",
                path_branch.file_name().unwrap_or_default()
            ))));
        }
        fs::remove_file(path_branch)?;
        Ok(())
    }

    pub fn set_last_commit_hash(&mut self, new_hash: GitHash) {
        self.last_commit_hash = new_hash;
    }

    pub(crate) fn get_last_commit_hash(&self) -> GitHash {
        self.last_commit_hash.clone()
    }

    pub(crate) fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn save(&self) -> Result<(), ErrorType> {
        let mut file = File::create(self.path_branches.join(&self.name))?;
        file.write_all(self.last_commit_hash.as_str().as_bytes())?;
        Ok(())
    }

    pub(crate) fn get_last_commit(&self, path_objects: &Path) -> Result<Commit, ErrorType> {
        let commit = GitObject::read_commit(&self.last_commit_hash, path_objects)?;
        Ok(commit)
    }
}
