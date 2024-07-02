use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    fs::File,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local, TimeZone};
use serde::{ser::SerializeStruct, Serialize};

use crate::{
    config::RepoConfig,
    files::object_type::ObjectType,
    git_errors::{command_error::CommandError, errors::ErrorType},
    git_object::GitObject,
    hash::GitHash,
    index::Index,
    refs::BranchRef,
    repo_paths::RepoPaths,
    tree::Tree,
    user::User,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Commit {
    hash: GitHash,
    tree: Tree,
    parent_hash: Option<GitHash>,
    second_parent_hash: Option<GitHash>,
    author: User,
    author_date: DateTime<Local>,
    committer: User,
    committer_date: DateTime<Local>,
    message: String,
}

// todo commit refactor cambiar user por author
impl Commit {
    // todo esta debería ya recibir los parametros inicializados y solo instanciar el struct
    // hay que hacer una build() o create()
    pub fn new(
        tree: Tree,
        parent_hash: Option<GitHash>,
        second_parent_hash: Option<GitHash>,
        message: &str,
        user: User,
    ) -> Result<Self, ErrorType> {
        let date = Local::now();

        let content = Self::generate_content(
            &tree.get_hash()?,
            parent_hash.clone(),
            second_parent_hash.clone(),
            (&user, &date),
            (&user, &date),
            message,
        )?;

        let commit_hash = GitHash::hash_object(&content, ObjectType::Commit);

        // verificar que esto se haga afuera en todos los llamados a new
        // index.reset_previous_blob_hash();
        // index.save(&mut File::create(index_path)?)?;

        Ok(Commit {
            hash: commit_hash,
            tree,
            parent_hash,
            second_parent_hash,
            author: user.clone(),
            author_date: date,
            committer: user,
            committer_date: date,
            message: message.to_string(),
        })
        // todo : meter el save aca adentro???
    }

    pub fn get_hash(&self) -> GitHash {
        self.hash.clone()
    }

    pub fn save(&self, path_objetcts: &Path) -> Result<(), ErrorType> {
        self.tree.save(path_objetcts)?;
        let content = self.get_content()?;
        GitObject::save_commit(content, path_objetcts)?;
        Ok(())
    }

    // que se guarde como Option<> para no calcular dos veces
    pub fn get_content(&self) -> Result<Vec<u8>, ErrorType> {
        Self::generate_content(
            &self.tree.get_hash()?,
            self.parent_hash.clone(),
            self.second_parent_hash.clone(),
            (&self.author, &self.author_date),
            (&self.committer, &self.committer_date),
            &self.message,
        )
    }

    fn generate_content(
        hash_tree: &GitHash,
        parent_hash: Option<GitHash>,
        second_parent_hash: Option<GitHash>,
        author: (&User, &chrono::DateTime<Local>),
        committer: (&User, &chrono::DateTime<Local>),
        message: &str,
    ) -> Result<Vec<u8>, ErrorType> {
        let mut result = String::new();
        writeln!(result, "tree {hash_tree}")?;
        if let Some(parent_hash) = parent_hash {
            writeln!(result, "parent {}", parent_hash.as_str())?;
        }
        if let Some(second_parent_hash) = second_parent_hash {
            writeln!(result, "parent {}", second_parent_hash.as_str())?;
        }

        let offset: String = author.1.offset().to_string().split(':').collect();
        writeln!(
            result,
            "author {} <{}> {} {}",
            author.0.get_name(),
            author.0.get_mail(),
            author.1.timestamp(),
            offset
        )?;

        let offset: String = committer.1.offset().to_string().split(':').collect();
        writeln!(
            result,
            "committer {} <{}> {} {}",
            committer.0.get_name(),
            committer.0.get_mail(),
            committer.1.timestamp(),
            offset
        )?;

        writeln!(result)?;

        writeln!(result, "{}", message)?;
        Ok(result.as_bytes().to_vec())
    }

    // todo meter casi todo en new(), aca que se parsee args y se imprima por consola #
    // esto es para que no esté repetido en otros lados como gui o merge
    pub fn commit_command(repo_paths: &RepoPaths, args: Vec<String>) -> Result<(), ErrorType> {
        let path_index = repo_paths.get_index();

        let mut index = Index::open(&path_index)?;

        let mut head = BranchRef::open(repo_paths.get_head(), &repo_paths.get_refs_heads())?;

        let parent_hash = head.get_last_commit_hash();

        let merge_head = match repo_paths.get_head_merge().exists() {
            true => Some(BranchRef::open(
                repo_paths.get_head_merge(),
                &repo_paths.get_refs_heads(),
            )?),
            false => None,
        };

        let second_parent_hash = match &merge_head {
            Some(r) => r.get_last_commit_hash(),
            None => None,
        };

        let config = RepoConfig::open(repo_paths.get_config())?;

        let user = config.get_user().ok_or(ErrorType::ConfigError("User name and mail should be set before commiting (use 'git-rustico config --user-name <name> --user-mail <mail>')".to_string()))?;

        let message = Self::parse_args(args)?;

        let index_tree = Tree::from_index(index.as_files_vector())?;
        let commit = Self::new(index_tree, parent_hash, second_parent_hash, &message, user)?;

        commit.save(&repo_paths.get_objects())?;

        index.reset_previous_blob_hash();

        index.save(&mut File::create(path_index)?)?;

        println!("Commit succesfull: {}", commit.get_hash());

        head.set_last_commit(commit.get_hash(), &repo_paths.get_refs_heads())?;

        if let Some(merge_head) = merge_head {
            merge_head.delete()?;
        }

        Ok(())
    }

    pub fn parse_args(args: Vec<String>) -> Result<String, ErrorType> {
        if args.len() != 1 {
            return Err(ErrorType::CommandError(CommandError::IncorrectAmount(
                "1".to_string(),
                0,
            )));
        }
        let message = args[0].clone();
        Ok(message)
    }

    pub fn get_commits_history(&self, path_objects: &PathBuf) -> Result<Vec<Commit>, ErrorType> {
        let mut commits = HashSet::new();
        let mut final_history = Vec::new();
        let history = self.get_commits_history_rec(path_objects)?;
        for commit in history {
            if commits.contains(&commit.get_hash()) {
                continue;
            }
            commits.insert(commit.get_hash());
            final_history.push(commit);
        }
        Ok(final_history)
    }

    // todo : cuando uno tiene 2 padres solo tomar los "distintos" del segundo padre,
    // es decir hasta que se vuelven a juntar
    // funcion recursiva que devuelve el historial de commits accesibles desde un commit
    pub fn get_commits_history_rec(
        &self,
        path_objects: &PathBuf,
    ) -> Result<Vec<Commit>, ErrorType> {
        let mut history = Vec::new();
        history.push((*self).clone());

        if let Some(parent1_hash) = &self.parent_hash {
            let parent1_commit = GitObject::read_commit(parent1_hash, path_objects)?;
            let mut sub_history = parent1_commit.get_commits_history(path_objects)?;

            if let Some(parent2_hash) = &self.second_parent_hash {
                let parent2_commit = GitObject::read_commit(parent2_hash, path_objects)?;
                let parent2_sub_history = parent2_commit.get_commits_history(path_objects)?;
                sub_history = Self::commits_ordered_by_date(sub_history, parent2_sub_history)?;
            }
            history.append(&mut sub_history);
        }

        Ok(history)
    }

    fn commits_ordered_by_date(
        branch1_history: Vec<Commit>,
        branch2_history: Vec<Commit>,
    ) -> Result<Vec<Commit>, ErrorType> {
        let mut commits1: HashSet<GitHash> = HashSet::new();
        for commit in &branch1_history {
            commits1.insert(commit.get_hash());
        }
        let mut last_common_ancestor = None;
        for commit in &branch2_history {
            if commits1.contains(&commit.get_hash()) {
                last_common_ancestor = Some(commit.get_hash());
            }
        }

        let mut interleaved_commits: Vec<Commit> = [branch1_history, branch2_history].concat();
        interleaved_commits.sort_by(|a, b| b.author_date.cmp(&a.author_date));

        let mut i = 0;
        while i < interleaved_commits.len() {
            if let Some(ancestor_hash) = &last_common_ancestor {
                if interleaved_commits[0].get_hash() == *ancestor_hash {
                    break;
                }
            }
            i += 1;
        }

        Ok(interleaved_commits[0..i - 1].to_vec())
    }

    // imprime el display de un commit
    pub(crate) fn log_display(&self, refs: Vec<String>) {
        let mut commit_line = format!("commit: {}", self.hash);
        if !refs.is_empty() {
            let refs = refs.join(", ");
            commit_line += &format!(" ({})", refs);
        }

        let author_line = format!(
            "Author: {} <{}>",
            self.author.get_name(),
            self.author.get_mail()
        );
        let date_line = format!("Date: {}", self.author_date);
        let message_line = format!("\n\t{}\n", self.message);

        let content = [commit_line, author_line, date_line, message_line].join("\n");
        println!("{}", content);
    }

    pub(crate) fn get_files_vec(&self) -> Vec<(PathBuf, GitHash)> {
        self.tree.get_files_vec()
    }

    // parsea el contenido de un commit object y devuelve la instancia
    pub fn from_object(
        hash: &GitHash,
        content: String,
        path_objects: &Path,
    ) -> Result<Self, ErrorType> {
        let mut parent_hash = None;
        let mut second_parent_hash = None;

        let mut found = HashMap::new();

        let mut lines = content.lines().peekable();
        while let Some(line) = lines.next() {
            if line.is_empty() {
                //message
                let mut mssg = String::new();
                for line in lines.by_ref() {
                    mssg.push_str(line);
                    mssg.push('\n');
                }
                mssg.pop();
                found.insert("message".to_string(), mssg);
                break;
            }
            if let Some((category, data)) = line.split_once(' ') {
                if category == "parent" {
                    if parent_hash.is_none() {
                        parent_hash = Some(GitHash::new(data)?)
                    } else {
                        second_parent_hash = Some(GitHash::new(data)?)
                    }
                    continue;
                }
                found.insert(category.to_string(), data.to_string());
            }
        }

        let tree_hash = found
            .get("tree")
            .ok_or(Self::missing_field(hash, "tree"))
            .and_then(|data| GitHash::new(data))?;
        let tree = GitObject::read_tree(&tree_hash, path_objects)?;

        let (author, author_date) = found
            .get("author")
            .ok_or(Self::missing_field(hash, "author"))
            .and_then(|d| Self::parse_user_line(d))?;

        let (committer, committer_date) = found
            .get("committer")
            .ok_or(Self::missing_field(hash, "committer"))
            .and_then(|d| Self::parse_user_line(d))?;

        let message = found
            .get("message")
            .ok_or(Self::missing_field(hash, "message"))?
            .to_owned();

        Ok(Self {
            hash: hash.clone(),
            tree,
            parent_hash,
            second_parent_hash,
            author,
            author_date,
            committer,
            committer_date,
            message,
        })
    }

    fn missing_field(commit_hash: &GitHash, field: &str) -> ErrorType {
        ErrorType::FormatError(format!(
            "Commit object '{commit_hash}' is missing a field '{field}'"
        ))
    }

    fn parse_user_line(fields: &str) -> Result<(User, DateTime<Local>), ErrorType> {
        let fields: Vec<&str> = fields.split(' ').collect();
        if fields.len() != 4 {
            return Err(ErrorType::FormatError(
                "Invalid user format in commit file".to_string(),
            ));
        }

        let mail = fields[1]
            .strip_prefix('<')
            .and_then(|m| m.strip_suffix('>'))
            .ok_or(ErrorType::FormatError(
                "User mail in commit must be between '<' '>'".to_string(),
            ))?;
        let user = User::new(fields[0], mail);
        let seconds: i64 = fields[2].parse::<i64>()?;

        let time = match Local.timestamp_opt(seconds, 0) {
            chrono::LocalResult::None => {
                return Err(ErrorType::FormatError(
                    "invalid date time in commit".to_string(),
                ))
            }
            chrono::LocalResult::Single(dt) => dt,
            chrono::LocalResult::Ambiguous(dt, _) => dt,
        };

        Ok((user, time))
    }

    pub(crate) fn last_common_ancestor(
        &self,
        other: &Commit,
        path_objects: &PathBuf,
    ) -> Result<Option<Commit>, ErrorType> {
        let history1 = self.get_commits_history(path_objects)?;
        let history2 = other.get_commits_history(path_objects)?;

        let mut hist1_set = HashSet::new();

        for commit in history1 {
            hist1_set.insert(commit.get_hash());
        }

        for commit in history2 {
            if hist1_set.contains(&commit.get_hash()) {
                return Ok(Some(commit));
            }
        }
        Ok(None)
    }

    // pub(crate) fn get_blob(
    //     &self,
    //     file_path: &PathBuf,
    //     path_objects: &Path,
    // ) -> Result<Blob, ErrorType> {
    //     let files = self.get_files_vec();
    //     for (file, hash) in files {
    //         if file == *file_path {
    //             return GitObject::read_blob(&hash, path_objects);
    //         }
    //     }
    //     Err(ErrorType::RepositoryError(format!(
    //         "file {} does not belong to commit {}",
    //         file_path.display(),
    //         self.get_hash()
    //     )))
    // }

    pub fn get_user(&self) -> User {
        self.author.clone()
    }

    pub fn get_date(&self) -> DateTime<Local> {
        self.author_date
    }

    pub(crate) fn get_tree(&self) -> &Tree {
        &self.tree
    }

    pub fn set_parent_hash(&mut self, new_hash: Option<GitHash>) -> Result<(), ErrorType> {
        self.parent_hash = new_hash;
        self.hash = GitHash::hash_object(&self.get_content()?, ObjectType::Commit);
        Ok(())
    }

    pub fn delete(&self, path_objetcts: &Path) -> Result<(), ErrorType> {
        // println!("orden de borrado {:?}.", path_objetcts);
        GitObject::delete_object(self.hash.clone(), path_objetcts)
    }

    pub fn get_message(&self) -> String {
        self.message.clone()
    }
}

impl Serialize for Commit {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut state = serializer.serialize_struct("Commit", 5)?;
        state.serialize_field("hash", &self.hash.as_str())?;
        state.serialize_field(
            "parent_hash",
            &self.parent_hash.as_ref().map(|h| h.as_str()),
        )?;
        state.serialize_field(
            "second_parent_hash",
            &self.second_parent_hash.as_ref().map(|h| h.as_str()),
        )?;
        state.serialize_field("author", &self.author)?;
        state.serialize_field("author_date", &self.author_date.to_string())?;
        state.serialize_field("committer", &self.committer)?;
        state.serialize_field("committer_date", &self.committer_date.to_string())?;
        state.serialize_field("message", &self.message)?;
        state.end()
    }
}
// #[cfg(test)]
// mod tests_commit {

//     mod new_commit {
// use crate::{
// commit::Commit, git_errors::errors::ErrorType, hash::GitHash, repo_paths::RepoPaths, index::Index,
// };

// use std::io::{Read, BufReader, Cursor};

// #[test]
// fn new_ok() -> Result<(), ErrorType> {

//     let index_file = String::from("file1.txt 17/10/2023-18:15:00 da39a3ee5e6b4b0d3255bfef95601890afd80709 None");
//     let index_file = Cursor::new(index_file);

//     let index = Index::new(index_file)?;

//     let parent_hash

//     let commmit = Commit::new(&index, , second_parent_hash, message, user)?;

//     assert_eq!(
//         GitHash::new("1e6a731d76b0ab5081a483a4b36bb85acc2fa4fc")?,
//         commmit.get_hash()
//     );
//     Ok(())
// }

// #[test]
// fn new_from_different_order_index() -> Result<(), ErrorType> {
//     let mut repo_paths = RepoPaths::new()?;
//     let current_dir = std::env::current_dir()?;

//     let index_path_a = current_dir.join("tests/commit/index_1_a");
//     repo_paths.set_index(index_path_a)?;

//     let head = current_dir.join("tests/commit/HEAD");
//     repo_paths._set_head(head)?;

//     let refs = current_dir.join("tests/commit/refs/heads/");
//     repo_paths._set_refs(refs)?;

//     let commmit_a = Commit::new(
//         &repo_paths,
//         None,
//         None,
//         "message",
//         "theo <tmiguel@fi.uba.ar>",
//         "2023/10/30-12:00:00",
//     )?;

//     let index_path_b = current_dir.join("tests/commit/index_1_b");
//     repo_paths.set_index(index_path_b)?;

//     let commmit_b = Commit::new(
//         &repo_paths,
//         None,
//         None,
//         "message",
//         "theo <tmiguel@fi.uba.ar>",
//         "2023/10/30-12:00:00",
//     )?;

//     assert_eq!(commmit_a.get_hash(), commmit_b.get_hash());
//     Ok(())
// }
// }

//     mod generate_content {
//         use chrono::{Local, TimeZone};

//         use crate::{commit::Commit, git_errors::errors::ErrorType, hash::GitHash, user::User};

//         NO PASA EN GITHUB PQ ES LOCAL Y TIENE OTRO TIMEZONE
//         #[test]
//         fn generate_content_ok() -> Result<(), ErrorType> {
//             let hash_tree = GitHash::new("8cb2237d0679ca88db6464eac60da96345513964")?;

//             let parent_hash = GitHash::new("42194fedb79b970d60b4f7f646ba7419eb674d24")?;

//             let user = User::new("user", "mail");
//             let message = "commit message";

//             let date = Local.timestamp_opt(0, 0).unwrap();
//             let content = Commit::generate_content(
//                 &hash_tree,
//                 Some(parent_hash),
//                 None,
//                 &user,
//                 message,
//                 &date,
//             )?;

//             assert_eq!(
//                 content,
//                 "tree 8cb2237d0679ca88db6464eac60da96345513964\n\
//                 parent 42194fedb79b970d60b4f7f646ba7419eb674d24\n\
//                 author user <mail> 0 -0300\n\
//                 \n\
//                 commit message\n"
//             );

//             assert_eq!(
//                 GitHash::hash_commit(&content),
//                 GitHash::new("d9af3aae2a14c7901e1d0757e9b83bc675e03d40")?
//             );
//             Ok(())
//         }
//     }
// }
