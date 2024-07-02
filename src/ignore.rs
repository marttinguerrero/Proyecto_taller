use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use crate::{git_errors::errors::ErrorType, repo_paths::RepoPaths};

pub struct Ignore;

impl Ignore {
    pub fn get_ignored_files(path_ignore: &Path) -> Result<HashSet<PathBuf>, ErrorType> {
        let mut ignored = HashSet::new();

        if !path_ignore.exists() {
            return Ok(ignored);
        }

        let ignore_file = fs::read_to_string(path_ignore)?;

        for file in ignore_file.lines() {
            ignored.insert(PathBuf::from(file));
        }
        Ok(ignored)
    }

    pub fn check_ignore_command(
        args: Vec<String>,
        repo_paths: &RepoPaths,
    ) -> Result<(), ErrorType> {
        let ignored = Self::get_ignored_files(&repo_paths.get_ignore())?;
        for path in args {
            if ignored.contains(&PathBuf::from(&path)) {
                println!("{}", path);
            }
        }
        Ok(())
    }
}
