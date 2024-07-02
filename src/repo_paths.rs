use std::path::PathBuf;

use crate::git_errors::errors::ErrorType;

// const DEFAULT_HOME_PATH: &str = ".git-rustico/";
const DEFAULT_OBJECTS_PATH: &str = ".git-rustico/objects/";
const DEFAULT_INDEX_PATH: &str = ".git-rustico/index";
const DEFAULT_REF_HEADS_PATH: &str = ".git-rustico/refs/heads/";
const DEFAULT_HEAD_PATH: &str = ".git-rustico/HEAD";
const DEFAULT_CONFIG_PATH: &str = ".git-rustico/config";
const DEFAULT_HEAD_MERGE_PATH: &str = ".git-rustico/HEAD_MERGE";
const DEFAULT_REMOTE_HEAD: &str = ".git-rustico/HEAD_REMOTE";
const DEFAULT_REMOTE_PATH: &str = ".git-rustico/remote";
const DEFAULT_REFS_REMOTE: &str = ".git-rustico/refs/remote/";
const DEFAULT_LOG_FILE: &str = ".git-rustico/LOG";
const DEFAULT_LOG_SERVER_FILE: &str = ".LOG_SERVER";
const DEFAULT_REFS_TAGS: &str = ".git-rustico/refs/";
const IGNORE_FILE: &str = ".gitignore";

#[derive(Clone)]
pub struct RepoPaths {
    home: PathBuf,
    // objects: PathBuf,
    // index: PathBuf,
    // refs: PathBuf,
    // head: PathBuf,
    // head_merge: PathBuf,
    // config: PathBuf,
    // remote: PathBuf,
}

impl RepoPaths {
    // todo pasarle un Option<path> a un path_config con paths alternativos y que se los guarde (para testear)
    pub fn new(home: PathBuf) -> Result<Self, ErrorType> {
        if !home.exists() {
            return Err(ErrorType::InvalidPath(home.display().to_string()));
        }

        Ok(Self { home })
    }

    pub fn get_index(&self) -> PathBuf {
        self.home.join(DEFAULT_INDEX_PATH)
    }

    pub fn get_refs_heads(&self) -> PathBuf {
        self.home.join(DEFAULT_REF_HEADS_PATH)
    }

    pub fn get_home(&self) -> PathBuf {
        self.home.clone()
    }

    pub fn get_objects(&self) -> PathBuf {
        self.home.join(DEFAULT_OBJECTS_PATH)
    }

    pub fn get_head(&self) -> PathBuf {
        self.home.join(DEFAULT_HEAD_PATH)
    }

    pub fn get_config(&self) -> PathBuf {
        self.home.join(DEFAULT_CONFIG_PATH)
    }

    pub fn get_head_merge(&self) -> PathBuf {
        self.home.join(DEFAULT_HEAD_MERGE_PATH)
    }
    pub fn get_remote(&self) -> PathBuf {
        self.home.join(DEFAULT_REMOTE_PATH)
    }

    pub(crate) fn get_refs_remote(&self) -> PathBuf {
        self.home.join(DEFAULT_REFS_REMOTE)
    }

    pub(crate) fn get_remote_head(&self) -> PathBuf {
        self.home.join(DEFAULT_REMOTE_HEAD)
    }

    pub fn get_log_file_path(&self) -> PathBuf {
        self.home.join(DEFAULT_LOG_FILE)
    }

    pub fn get_log_server_file_path(&self) -> PathBuf {
        self.home.join(DEFAULT_LOG_SERVER_FILE)
    }

    pub(crate) fn get_ignore(&self) -> std::path::PathBuf {
        self.home.join(IGNORE_FILE)
    }

    pub fn get_refs_tags(&self) -> PathBuf {
        self.home.join(DEFAULT_REFS_TAGS)
    }
}
