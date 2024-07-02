use crate::branch::Branch;
use crate::commit::Commit;
use crate::git_errors::errors::ErrorType;
use crate::http::http_error::HTTPError::{BadRequest, MethodNotAllowed, NotFound};
use crate::merge::Merge;
use crate::repo_paths::RepoPaths;
use crate::server_paths::ServerPaths;
use crate::user::User;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::string::String;

// SERVER
// |-<nombre de un repo>
//    |-repo (contiene el repositorio NO CONTIENE INFO DEL PR)
//    |-pull_request (contiene datos del pull request)

// todo reemplazar por serverpaths

const OPEN_STATE: &str = "open";
const CLOSED_STATE: &str = "closed";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PullRequest {
    number: usize,
    title: String,
    state: String,
    branch_base: String,
    branch_target: String,
    hash_creation_commit: String, // commit donde se creo el PR, deberia ser el hash del commit
}

impl PullRequest {
    // json body should include fields branch_base and branch_target
    pub fn create_pull_request(
        server_repo_path: PathBuf,
        base_name: String,
        target_name: String,
        title: Option<String>,
    ) -> Result<Self, ErrorType> {
        // todo ver que existan las branches
        let server_paths = ServerPaths::new(&server_repo_path);
        let pull_requests_path = server_paths.get_pull_requests_path();
        let repo_paths = server_paths.get_repo_paths()?;
        let path_objects = repo_paths.get_objects();

        let base = match Branch::open(&repo_paths.get_refs_heads(), &base_name) {
            Ok(b) => b,
            Err(_) => {
                return Err(ErrorType::HTTPError(NotFound(format!(
                    "Branch {} not found",
                    base_name
                ))))
            }
        };
        let target = match Branch::open(&repo_paths.get_refs_heads(), &target_name) {
            Ok(b) => b,
            Err(_) => {
                return Err(ErrorType::HTTPError(NotFound(format!(
                    "Branch {} not found",
                    target_name
                ))))
            }
        };

        let base_commit = base.get_last_commit(&path_objects)?;
        let target_commmit = target.get_last_commit(&path_objects)?;

        let last_common_ancestor =
            target_commmit.last_common_ancestor(&base_commit, &path_objects)?;

        if last_common_ancestor.is_none() {
            return Err(ErrorType::HTTPError(BadRequest(
                "Can't create PullRequest for branches with no common ancestor".to_string(),
            )));
        }
        let hash = target.get_last_commit_hash();

        let number = get_highest_pr_number(&pull_requests_path)? + 1;

        let title = title.unwrap_or(format!("Branch {target_name} into {base_name}"));

        let pr = PullRequest {
            number,
            title,
            state: OPEN_STATE.to_string(),
            branch_base: base_name.to_string(),
            branch_target: target_name.to_string(),
            hash_creation_commit: hash.to_string(),
        };

        pr.save(&pull_requests_path)?;
        Ok(pr)
    }

    pub fn save(&self, server_pulls_path: &Path) -> Result<(), ErrorType> {
        let file_path = server_pulls_path.join(format!("{}.json", self.number));

        let mut file = File::create(file_path)?;

        let content = match serde_json::to_string(self) {
            Ok(c) => c,
            Err(e) => {
                return Err(ErrorType::RepositoryError(format!(
                    "Error serializing pull request into JSON: {}",
                    e
                )))
            }
        };
        file.write_all(content.as_bytes())?;

        Ok(())
    }

    // ===========================================================================================

    pub fn get_pull_request(repo_path: &Path, number: &str) -> Result<PullRequest, ErrorType> {
        Self::read(repo_path, number)
    }

    fn read(server_repo_path: &Path, number: &str) -> Result<Self, ErrorType> {
        let pull_requests_path = ServerPaths::new(server_repo_path).get_pull_requests_path();
        let pr_path = pull_requests_path.join(format!("{}.json", number));
        if !pr_path.exists() {
            return Err(ErrorType::HTTPError(NotFound(format!(
                "Pull Request {} not found",
                number
            ))));
        }
        Self::from_json_file(&pr_path)
    }

    // ===========================================================================================

    pub fn list_pull_requests(server_repo_path: &Path) -> Result<Vec<PullRequest>, ErrorType> {
        let mut pull_requests = Vec::new();

        let repo_pr = ServerPaths::new(server_repo_path).get_pull_requests_path();
        let dir_entries = fs::read_dir(repo_pr)?;

        for entry in dir_entries {
            let entry = entry?;
            let file_path = entry.path();

            if file_path.is_file() {
                let pr = Self::from_json_file(&file_path)?;
                pull_requests.push(pr);
            }
        }
        Ok(pull_requests)
    }

    // ===========================================================================================

    /// Returns a list of commits from the creation of the Pull Request
    pub fn list_commits_pull_request(
        server_repo_path: PathBuf,
        number: &str,
    ) -> Result<Vec<Commit>, ErrorType> {
        let pr = Self::read(&server_repo_path, number)?;

        let server_repo_paths = ServerPaths::new(&server_repo_path);
        let repo_paths = server_repo_paths.get_repo_paths()?;

        let target = pr.open_branch_target(&repo_paths.get_refs_heads())?;

        let commit = target.get_last_commit(&repo_paths.get_objects())?;
        let vec = commit.get_commits_history(&repo_paths.get_objects())?;
        let mut list_commits = Vec::new();
        for commit in vec {
            let hash = commit.get_hash().to_string();
            list_commits.push(commit);
            if hash == pr.hash_creation_commit {
                break;
            }
        }
        Ok(list_commits)
    }

    // ===========================================================================================

    pub fn merge_pull_request(
        repo_path: PathBuf,
        number: &str,
        json_body: &str,
    ) -> Result<String, ErrorType> {
        let mut pr = Self::read(&repo_path, number)?;
        if pr.is_closed() {
            return Err(ErrorType::HTTPError(MethodNotAllowed(
                "Can't merge PullRequest, it is already closed".to_string(),
            )));
        }

        let server_repo_paths = ServerPaths::new(&repo_path);
        let repo_paths = server_repo_paths.get_repo_paths()?;
        let path_branches = &repo_paths.get_refs_heads();

        let mut base = pr.open_branch_base(path_branches)?;
        let target = pr.open_branch_target(path_branches)?;

        Self::verify_no_conflicts(&base, &target, &repo_paths)?;

        let (user, message) = Self::parse_json_merge_body(json_body)?;

        let (modified_files, conflict_files) = Merge::merge(
            &mut base,
            target,
            repo_paths.clone(),
            Some(message),
            Some(user),
        )?;
        if !conflict_files.is_empty() {
            return Err(ErrorType::RepositoryError("Error merging pull request, an unexpected conflict happened while merging. This server repo ended in a corrupt state".to_string()));
        };

        pr.close();
        pr.save(&server_repo_paths.get_pull_requests_path())?;

        let base = pr.open_branch_base(path_branches)?;
        let commit = base.get_last_commit(&repo_paths.get_objects())?;
        let message = match modified_files.is_empty() {
            true => "Fast-forward merge completed succesfully".to_string(),
            false => format!(
                "Three-way merge completed succesfuly. New merge commit created: {}: {}",
                commit.get_hash().as_str(),
                commit.get_message()
            ),
        };
        Ok(message)
    }

    fn verify_no_conflicts(
        base: &Branch,
        target: &Branch,
        repo_paths: &RepoPaths,
    ) -> Result<(), ErrorType> {
        let path_objects = repo_paths.get_objects();

        let base_files = base.get_last_commit(&path_objects)?.get_files_vec();
        let target_files = target.get_last_commit(&path_objects)?.get_files_vec();

        let last_common_ancestor = match target
            .get_last_commit(&repo_paths.get_objects())?
            .last_common_ancestor(
                &base.get_last_commit(&repo_paths.get_objects())?,
                &repo_paths.get_objects(),
            )? {
            Some(lca) => lca,
            None => return Err(ErrorType::HTTPError(BadRequest(
                "Can't merge PullRequest, branches base and target have no common commit ancestor"
                    .to_string(),
            ))),
        };
        let lca_files = last_common_ancestor.get_files_vec();

        let (_, conflict_files) =
            Merge::compare_files(base_files, target_files, lca_files, repo_paths)?;

        if !conflict_files.is_empty() {
            let files = conflict_files
                .iter()
                .map(|f| f.0.display().to_string())
                .collect::<Vec<String>>()
                .join(", ");
            return Err(ErrorType::HTTPError(MethodNotAllowed(format!(
                "Can't merge PullRequest, some conflicts need to be solved in files: {}",
                files
            ))));
        }
        Ok(())
    }

    fn parse_json_merge_body(json_body: &str) -> Result<(User, String), ErrorType> {
        let body: Value = match serde_json::from_str(json_body) {
            Ok(b) => b,
            Err(_) => {
                return Err(ErrorType::RepositoryError(
                    "Error deserializing http request body".to_string(),
                ))
            }
        };

        let user: User = match body["user"].as_object() {
            Some(b) => match serde_json::from_value(Value::Object(b.clone())) {
                Ok(u) => u,
                Err(_) => {
                    return Err(ErrorType::RepositoryError(
                        "Error deserializing user from http request body".to_string(),
                    ))
                }
            },
            None => {
                return Err(ErrorType::RepositoryError(
                    "Error reading user from http request body".to_string(),
                ))
            }
        };

        let message = match body["message"].as_str() {
            Some(b) => b,
            None => {
                return Err(ErrorType::RepositoryError(
                    "Error reading message from http request body".to_string(),
                ))
            }
        };

        Ok((user, message.to_string()))
    }

    // ===========================================================================================

    fn open_branch_base(&self, path_branches: &Path) -> Result<Branch, ErrorType> {
        Self::open_branch(path_branches, &self.branch_base)
    }

    fn open_branch_target(&self, path_branches: &Path) -> Result<Branch, ErrorType> {
        Self::open_branch(path_branches, &self.branch_target)
    }

    fn open_branch(path_branches: &Path, branch_name: &str) -> Result<Branch, ErrorType> {
        match Branch::open(path_branches, branch_name) {
            Ok(b) => Ok(b),
            Err(_) => {
                // todo esta no es la unica razon por la que puede fallar
                Err(ErrorType::RepositoryError(format!(
                    "Branch {} not found, must have been deleted",
                    branch_name
                )))
            }
        }
    }

    pub(crate) fn to_json_string(&self) -> Result<String, ErrorType> {
        match serde_json::to_string(self) {
            Ok(s) => Ok(s),
            Err(_) => Err(ErrorType::RepositoryError(format!(
                // todo esta no es la unica razon por la que puede fallar
                "Error serializing pull request {} into JSON",
                self.number
            ))),
        }
    }

    pub(crate) fn to_json_array(list: &[PullRequest]) -> Result<String, ErrorType> {
        match serde_json::to_string(list) {
            Ok(s) => Ok(s),
            Err(_) => Err(ErrorType::RepositoryError(
                "Error serializing pull request list into JSON".to_string(),
            )),
        }
    }

    // fn delete_pull_request(directory: PathBuf, pull_request: PullRequest) -> Result<(), ErrorType> {
    //     let binding = Self::pull_request_format(pull_request);
    //     let line_to_delete = match binding.split('\n').next() {
    //         Some(l) => l,
    //         None => return Ok(()),
    //     };
    //     let mut file = Self::get_read_file_pull_request(directory.clone())?;
    //     let mut old_lines = String::new();
    //     file.read_to_string(&mut old_lines)?;
    //     let mut new_lines = Vec::new();
    //     for line in old_lines.split('\n') {
    //         if line_to_delete != line {
    //             new_lines.push(line);
    //         }
    //     }
    //     let mut file = Self::get_write_file_pull_request(directory)?;
    //     file.write_all(new_lines.join("\n").as_bytes())?;
    //     Ok(())
    // }

    fn from_json_file(path: &PathBuf) -> Result<Self, ErrorType> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        match serde_json::from_reader(reader) {
            Ok(p) => Ok(p),
            Err(_) => Err(ErrorType::RepositoryError(
                "Error deserializing pull request from JSON".to_string(),
            )),
        }
    }

    fn close(&mut self) {
        self.state = CLOSED_STATE.to_string();
    }

    pub(crate) fn is_open(&self) -> bool {
        self.state == OPEN_STATE
    }

    pub(crate) fn get_base(&self) -> String {
        self.branch_base.clone()
    }

    pub(crate) fn set_title(&mut self, new_title: &str) {
        self.title = new_title.to_string();
    }

    pub(crate) fn set_state(&mut self, new_state: &str) -> Result<(), ErrorType> {
        match new_state {
            "open" | "closed" => {
                self.state = new_state.to_string();
                Ok(())
            }
            _ => Err(ErrorType::HTTPError(BadRequest(format!(
                "Invalid state passed in body: {new_state}"
            )))),
        }
    }

    pub(crate) fn set_base(
        &mut self,
        new_base: &str,
        path_branches: &Path,
    ) -> Result<(), ErrorType> {
        match Branch::open(path_branches, new_base) {
            Ok(b) => {
                self.branch_base = b.get_name();
                Ok(())
            }
            Err(e) => match e {
                ErrorType::CommandError(_) => Err(ErrorType::HTTPError(NotFound(format!(
                    "New base branch '{new_base}' passed in body not found"
                )))),
                _ => Err(e),
            },
        }
    }

    fn is_closed(&self) -> bool {
        self.state == CLOSED_STATE
    }
}

fn get_highest_pr_number(pull_requests_path: &PathBuf) -> Result<usize, ErrorType> {
    let mut highest = 0;

    if let Ok(entries) = fs::read_dir(pull_requests_path) {
        for entry in entries.flatten() {
            // maybe return error if any entry is a dir
            if let Some(name) = entry.file_name().to_str() {
                let number = match name.strip_suffix(".json").unwrap_or(name).parse::<usize>() {
                    Ok(n) => n,
                    Err(_) => {
                        return Err(ErrorType::HTTPError(BadRequest(format!(
                            "Invalid PullRequest name: {}",
                            name
                        ))))
                    }
                };
                if number > highest {
                    highest = number;
                }
            }
        }
    } else {
        return Err(ErrorType::RepositoryError(format!(
            "Error reading pull request directory: {}",
            pull_requests_path.display()
        )));
    }

    Ok(highest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_json_string_success() {
        // Arrange
        let pull_request = PullRequest {
            title: "title".to_string(),
            number: 1,
            state: "open".to_string(),
            branch_base: "base".to_string(),
            branch_target: "target".to_string(),
            hash_creation_commit: "0000".to_string(),
        };

        // Act
        let result = pull_request.to_json_string();

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "{\"number\":1,\"title\":\"title\",\"state\":\"open\",\"branch_base\":\"base\",\"branch_target\":\"target\",\"hash_creation_commit\":\"0000\"}");
    }

    // como prodria hacerla fallar?
    // #[test]
    // fn test_to_json_string_error() {
    //     // Arrange
    //     let pull_request = PullRequest {
    //         number: 1,
    //         branch_base: "base".to_string(),
    //         branch_target: "target".to_string(),
    //         hash_creation_commit: "0000".to_string(),
    //     };

    //     // Act
    //     let result = pull_request.to_json_string();

    //     // Assert
    //     assert!(result.is_err());
    //     assert_eq!(
    //         result.unwrap_err(),
    //         ErrorType::RepositoryError("Error serializing pull request into JSON".to_string())
    //     );
    // }
}
