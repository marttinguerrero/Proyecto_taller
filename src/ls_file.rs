use crate::git_errors::errors::ErrorType;
use crate::index::Index;
use crate::ls_tree::LsTree;
use crate::repo_paths::RepoPaths;
use std::fs;
use std::path::{Path, PathBuf};

const OPTION_OTHER: &str = "--others";

pub struct LsFile {
    others: bool,
}

impl LsFile {
    pub fn show_file(args: Vec<String>, repo_paths: &RepoPaths) -> Result<String, ErrorType> {
        let file = Self::process_arguments(args, repo_paths)?;
        file.output_info(repo_paths)
    }

    fn process_arguments(args: Vec<String>, _repo_paths: &RepoPaths) -> Result<Self, ErrorType> {
        Ok(LsFile {
            others: args.contains(&OPTION_OTHER.to_string()),
        })
    }

    fn output_info(&self, repo_paths: &RepoPaths) -> Result<String, ErrorType> {
        // let (_untracked, not_staged, staged) = self.get_status_files(repo_paths)?;
        let index = Index::open(&repo_paths.get_index())?;
        // status = vec [untracked, not_staged, staged]
        let status = index.status(&repo_paths.get_home(), &repo_paths.get_ignore())?;
        let not_staged = self.convert_vec_pathbuf_to_vec_string(status[1].clone());
        let staged = self.convert_vec_pathbuf_to_vec_string(status[2].clone());
        match self.others {
            true => self.get_untrackers(
                not_staged,
                staged,
                LsTree::show_tree(
                    vec!["HEAD".to_string(), "--format=%(path)".to_string()],
                    repo_paths,
                )?,
            ),
            false => Ok(self.get_trackers(
                not_staged,
                staged,
                LsTree::show_tree(
                    vec!["HEAD".to_string(), "--format=%(path)".to_string()],
                    repo_paths,
                )?,
            )),
        }
    }

    fn get_trackers(
        &self,
        not_staged: Vec<String>,
        staged: Vec<String>,
        show_file: String,
    ) -> String {
        let mut result = staged.clone();
        for file in show_file.split('\n') {
            if not_staged.contains(&file.to_string()) || staged.contains(&file.to_string()) {
                continue;
            }
            result.push(file.to_string());
        }
        for name in not_staged {
            if result.contains(&name) {
                continue;
            }
            result.push(name);
        }
        result.join("\n")
    }

    fn get_untrackers(
        &self,
        not_staged: Vec<String>,
        staged: Vec<String>,
        show_file: String,
    ) -> Result<String, ErrorType> {
        let result = Self::get_files_in_dir(None)?;
        let filter_not_staged = Self::filter_no_repeat(result, not_staged);
        let filter_staged = Self::filter_no_repeat(filter_not_staged, staged);
        let files: Vec<String> = show_file
            .split(&"\n".to_string())
            .map(|x| x.to_string())
            .collect();
        let filter_files = Self::filter_no_repeat(filter_staged, files);
        Ok(filter_files.join("\n"))
    }

    fn convert_vec_pathbuf_to_vec_string(&self, vec_pathbuf: Vec<PathBuf>) -> Vec<String> {
        let mut result = Vec::new();
        for path_buf in vec_pathbuf {
            match path_buf.to_str() {
                None => continue,
                Some(a) => result.push(a.to_string()),
            }
        }
        result
    }

    fn get_files_in_dir(path_initial: Option<&Path>) -> Result<Vec<String>, ErrorType> {
        let mut files = Vec::new();
        let path = match path_initial {
            None => Path::new("."),
            Some(p) => p,
        };
        for dir in fs::read_dir(path)? {
            let entry = dir?;
            let path_dir = entry.path();
            if path_dir.is_file() {
                let file_name = path_dir.display().to_string();
                let (_, file_name) = file_name.split_at(2);
                files.push(String::from(file_name));
            } else if path_dir.is_dir() {
                let sub_element = Self::get_files_in_dir(Some(&path_dir))?;
                for element in sub_element {
                    files.push(element);
                }
            }
        }
        Ok(files)
    }

    /// given two vectors,
    /// returns only the values of the first vector that do not appear in the second vector
    fn filter_no_repeat(vec_base: Vec<String>, vec_repeat: Vec<String>) -> Vec<String> {
        let repeat_values: std::collections::HashSet<String> = vec_repeat.into_iter().collect();
        let result: Vec<String> = vec_base
            .into_iter()
            .filter(|elem| !repeat_values.contains(elem))
            .collect();
        result
    }
}
