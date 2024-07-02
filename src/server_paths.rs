use std::path::{Path, PathBuf};

use crate::{git_errors::errors::ErrorType, repo_paths::RepoPaths};

const SERVER_REPO_DIRECTORY: &str = "repo";
const PULL_REQUEST_DIRECTORY_PATH: &str = "pull_requests";

pub struct ServerPaths {
    server_repo_path: PathBuf,
}

impl ServerPaths {
    pub fn new(server_repo_path: &Path) -> Self {
        //todo checkear si existe?
        Self {
            server_repo_path: server_repo_path.to_path_buf(),
        }
    }

    pub fn get_pull_requests_path(&self) -> PathBuf {
        self.server_repo_path.join(PULL_REQUEST_DIRECTORY_PATH)
    }

    pub fn get_repo_path(&self) -> PathBuf {
        self.server_repo_path.join(SERVER_REPO_DIRECTORY)
    }

    pub fn get_repo_paths(&self) -> Result<RepoPaths, ErrorType> {
        RepoPaths::new(self.get_repo_path())
    }
}
