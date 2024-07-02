use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
};

use crate::{
    files::index_file_info::IndexFileInfo, git_errors::errors::ErrorType, hash::GitHash,
    ignore::Ignore, refs::BranchRef, repo_paths::RepoPaths,
};
use std::io::BufRead;

// formato de la linea [path modification_date current_blob_hash previous_blob_hash]

/// Represents the index of a git-rustico repository.
///
/// The index, also known as the staging area, is a data structure that tracks the changes made to files in the repository.
/// It contains a hash map that maps file paths to `IndexFileInfo` objects, which store information about each file in the index.
///
/// The `Index` struct provides methods for opening, creating, and manipulating the index, such as adding and removing files,
/// saving the index to a file, and checking the status of files in the index.
pub struct Index {
    hash_map: HashMap<PathBuf, IndexFileInfo>,
    // path_index : PathBuf,
}

impl Index {
    /// Opens an index file at the specified path and returns a new `Index` instance.
    ///
    /// # Arguments
    ///
    /// * `path_index` - The path to the index file.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Index` instance if successful, or an `ErrorType` if an error
    /// occurred.
    pub fn open(path_index: &PathBuf) -> Result<Self, ErrorType> {
        let file = File::open(path_index)?;
        Self::new(file)
    }

    /// Gets the Git hash associated with the specified file path in the index.
    ///
    /// # Arguments
    ///
    /// * `branch_file_path` - The path of the file in the branch.
    ///
    /// # Returns
    ///
    /// An `Option` containing the Git hash if the file is found in the index, or `None` if the
    /// file is not in the index.
    pub fn new<R: Read>(index_file: R) -> Result<Self, ErrorType> {
        let mut hash_map = HashMap::new();
        let reader = BufReader::new(index_file);

        for line in reader.lines() {
            let current_line: &str = &line?;
            let info = IndexFileInfo::try_from(current_line)?;
            hash_map.insert(info.get_path(), info);
        }
        Ok(Self { hash_map })
    }

    /// Gets the Git hash associated with the specified file path in the index.
    ///
    /// # Arguments
    ///
    /// * `branch_file_path` - The path of the file in the branch.
    ///
    /// # Returns
    ///
    /// An `Option` containing the Git hash if the file is found in the index, or `None` if the
    /// file is not in the index.
    // pub(crate) fn get_hash(&self, branch_file_path: &PathBuf) -> Option<GitHash> {
    //     if let Some(file_info) = self.hash_map.get(branch_file_path) {
    //         return Some(file_info.get_hash());
    //     }
    //     None
    // }

    /// Returns a vector of `IndexFileInfo` representing all the files in the index.
    pub fn as_files_vector(&self) -> Vec<IndexFileInfo> {
        self.hash_map.values().cloned().collect()
    }

    /// Resets the previous blob hash to `None` for all files in the index. This is necessary to "reset" the index
    /// whenever a commit is created. Any files that were added to the index before the commit will have their
    /// previous blob hash set to `None` so that they will not appear as modified in the next commit.
    pub fn reset_previous_blob_hash(&mut self) {
        for file in self.hash_map.values_mut() {
            file.reset_previous_blob_hash();
        }
    }

    /// Saves the index to the specified index file.
    ///
    /// # Arguments
    ///
    /// * `index_file` - The index file to write to.
    ///
    /// # Returns
    ///
    /// A `Result` indicating whether the operation was successful or an `ErrorType` if an error
    /// occurred.
    pub fn save<W: Write>(&self, index_file: &mut W) -> Result<(), ErrorType> {
        let mut text: String = String::new();
        for file in self.hash_map.values() {
            text += &file.to_index_line();
        }
        index_file.write_all(text.as_bytes())?;

        Ok(())
    }

    /// Adds a file to the index.
    ///
    /// # Arguments
    ///
    /// * `file_path` - The path of the file to add.
    /// * `home_path` - The path of the repository's home directory.
    /// * `path_objects` - The path of the objects directory.
    ///
    /// # Returns
    ///
    /// A `Result` indicating whether the operation was successful or an `ErrorType` if an error
    /// occurred.
    pub(crate) fn add(
        &mut self,
        file_path: PathBuf,
        home_path: &Path,
        path_objects: &Path,
    ) -> Result<(), ErrorType> {
        if let Some(file_info) = self.hash_map.get_mut(&file_path) {
            // the file was already tracked in index
            file_info.verify_change(path_objects, home_path)?;
        } else {
            // this is a new file, it wasn't previously in index
            let file_info = IndexFileInfo::new(file_path, home_path)?;
            // todo devolver el fileinfo y hacer save afuera ?
            file_info.save(path_objects, home_path)?;
            self.hash_map.insert(file_info.get_path(), file_info);
        }
        Ok(())
    }

    /// Removes a file from the index.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the file to remove.
    ///
    /// # Returns
    ///
    /// A `Result` indicating whether the operation was successful or an `ErrorType` if an error
    /// occurred.
    pub fn remove(&mut self, path: PathBuf) -> Result<(), ErrorType> {
        if self.hash_map.remove(&path).is_none() {
            return Err(ErrorType::FileNotInIndex(format!(
                "ERROR {} not in the index.",
                path.display()
            )));
        }
        Ok(())
        // no se si debería hacer save adentro
        // self.save()
    }

    pub fn add_command(file_paths: Vec<String>, repo_paths: &RepoPaths) -> Result<(), ErrorType> {
        let path_home = repo_paths.get_home();
        let path_index = repo_paths.get_index();

        for path in file_paths.iter() {
            let path = path_home.join(path);
            crate::utils::verify_path_exists(&path)?;
        }

        // todo preguntar si esta es la forma correcta
        let mut index = Self::new(File::open(&path_index)?)?;

        for path in file_paths {
            index.add(PathBuf::from(path), &path_home, &repo_paths.get_objects())?;
        }

        // todo preguntar si esta es la forma correcta
        let _ = index.save(&mut File::create(path_index)?);

        Ok(())
    }

    pub fn rm_command(file_paths: Vec<String>, index_path: PathBuf) -> Result<(), ErrorType> {
        let mut index = Self::new(File::open(index_path.clone())?)?;

        for file_path in file_paths {
            index.remove(PathBuf::from(file_path))?;
        }

        let mut index_file = File::create(index_path)?;
        index.save(&mut index_file)?;

        Ok(())
    }

    /// Returns the status of files in the index.
    ///
    /// # Arguments
    ///
    /// * `home_path` - The path of the repository's home directory.
    /// * `path_ignore` - The path of the ignore file.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of vectors of `PathBuf` representing the untracked files,
    /// not staged files, and staged files, respectively, if successful, or an `ErrorType` if an
    /// error occurred.
    pub fn status(
        &self,
        home_path: &Path,
        path_ignore: &Path,
    ) -> Result<Vec<Vec<PathBuf>>, ErrorType> {
        let file_paths = Self::list_dir_file_paths(home_path.to_path_buf())?;

        let ignored = Ignore::get_ignored_files(path_ignore)?;

        let mut untracked: Vec<PathBuf> = Vec::new();
        let mut not_staged: Vec<PathBuf> = Vec::new();
        let mut staged: Vec<PathBuf> = Vec::new();

        for file_path in file_paths.iter() {
            if ignored.contains(file_path) {
                continue;
            }
            match self.hash_map.get(file_path) {
                Some(file_info) => {
                    if file_info.has_changed(home_path)? {
                        not_staged.push(file_path.clone());
                    } else {
                        if !file_info.added_since_commit() {
                            continue;
                        }
                        staged.push(file_path.clone());
                    }
                }
                None => {
                    untracked.push(file_path.clone());
                }
            }
        }

        Ok(vec![untracked, not_staged, staged])
    }

    fn list_dir_file_paths(repo_home_path: PathBuf) -> Result<Vec<PathBuf>, ErrorType> {
        Self::list_dir_file_paths_rec(repo_home_path.clone(), repo_home_path)
    }

    fn list_dir_file_paths_rec(
        path_dir: PathBuf,
        repo_home_path: PathBuf,
    ) -> Result<Vec<PathBuf>, ErrorType> {
        let mut files: Vec<PathBuf> = Vec::new();

        let entries = fs::read_dir(path_dir)?;

        for entry in entries {
            let entry = entry?;
            let path_buf = entry.path();

            if path_buf.is_file() {
                if path_buf.ends_with(".gitignore") {
                    continue;
                }
                let path = match path_buf.strip_prefix(&format!("{}/", repo_home_path.display())) {
                    Ok(p) => PathBuf::from(p),
                    Err(_) => path_buf,
                };
                files.push(path);
            } else if path_buf.is_dir() {
                if path_buf.ends_with(".git-rustico") {
                    continue;
                }
                let paths_subdirectory =
                    Self::list_dir_file_paths_rec(path_buf, repo_home_path.clone())?;
                files.extend(paths_subdirectory);
            }
        }
        Ok(files)
    }

    // todo agregar la branch actual
    // todo que te diga la branch y si estas en un merge abierto
    // todo trakear los borrados tambien
    // todo puede estar en changes to be commited (add) pero tambien en changes not staged for commmit (cambio despues del add) (nice to have)
    pub fn status_command(repo_paths: &RepoPaths) -> Result<String, ErrorType> {
        // archivo puede estar:
        // en repo pero no en index -> untracked
        // en repo y en index pero modificado -> not staged for commit
        // en repo e index pero previous_blob_hash != None -> changes to be commited
        // en repo e index pero igual al ultimo commit (previous_blob_hash = None) -> no se imprime

        let index = Index::new(File::open(repo_paths.get_index())?)?;
        let head = BranchRef::open(repo_paths.get_head(), &repo_paths.get_refs_heads())?;
        let results = index.status(&repo_paths.get_home(), &repo_paths.get_ignore())?;
        Ok(Self::print_status(
            head,
            results[0].clone(),
            results[1].clone(),
            results[2].clone(),
        ))
    }

    pub fn print_status(
        head: BranchRef,
        untracked: Vec<PathBuf>,
        not_staged: Vec<PathBuf>,
        staged: Vec<PathBuf>,
    ) -> String {
        if let Some(branch_name) = head.get_branch_name() {
            println!("On branch {}", branch_name);
        } else {
            println!("Not currently on any branch.");
        }
        if untracked.is_empty() && not_staged.is_empty() && staged.is_empty() {
            println!("Up to date. Nothing to commit.");
            return "Up to date. Nothing to commit.".to_string();
        }
        let mut changes = vec!["Exist files:".to_string()];
        if !staged.is_empty() {
            println!("Changes to be commited:");
            for file in &staged {
                println!("\t{}", file.display());
            }
            println!();
            changes.push("To be commited.".to_string())
        }
        if !not_staged.is_empty() {
            println!("Changes not staged for commit:\n    (Use 'git-rustico add <file>...' to update what will be commited)");
            for file in not_staged {
                println!("\t{}", file.display());
            }
            println!();
            changes.push("Not staged for commit.".to_string())
        }
        if !untracked.is_empty() {
            println!(
                "Untracked files:\n    (Use 'git-rustico add <file>' to include in what will be commited)"
            );
            for file in untracked {
                println!("\t{}", file.display());
            }
            println!();
            changes.push("Untracked.".to_string())
        }
        if staged.is_empty() {
            println!("nothing added to commit (use 'git-rustico add <file>')");
        }
        changes.join(" ")
    }

    /// Updates the index to reflect the working directory.
    ///
    /// # Arguments
    ///
    /// * `working_dir_files` - A vector of tuples containing the local file paths and their Git
    ///                         hashes in the working directory.
    /// * `home_path` - The path of the repository's home directory.
    ///
    /// # Returns
    ///
    /// A `Result` indicating whether the operation was successful or an `ErrorType` if an error
    /// occurred.
    pub(crate) fn update_to_working_dir(
        &mut self,
        working_dir_files: Vec<(PathBuf, GitHash)>,
        home_path: &Path,
    ) -> Result<(), ErrorType> {
        let files = self.as_files_vector();
        for file in files {
            if !home_path.join(file.get_path()).exists() {
                self.remove(file.get_path())?;
            }
        }

        for (local_path, _) in working_dir_files {
            let file_info = IndexFileInfo::new(local_path.clone(), home_path)?;
            self.hash_map.insert(local_path, file_info);
        }
        Ok(())
    }

    /// Checks if there are any uncommitted changes in the index.
    ///
    /// # Arguments
    ///
    /// * `path_home` - The path of the repository's home directory.
    /// * `path_ignore` - The path of the ignore file.
    ///
    /// # Returns
    ///
    /// A `Result` indicating whether there are any uncommitted changes or an `ErrorType` if an
    /// error occurred.
    pub fn check_for_changes(&self, path_home: &Path, path_ignore: &Path) -> Result<(), ErrorType> {
        let results = self.status(path_home, path_ignore)?;
        for result in results {
            if !result.is_empty() {
                return Err(ErrorType::RepositoryError("There's uncommited changes either tracked in index or not.\nAdd and commit or delete them before continuing".to_string()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests_status {

    // mod tests_save_index {
    //     use std::io::Cursor;

    //     use super::super::save_index;
    //     use crate::{git_errors::errors::ErrorType, utils::new_index_hashmap};

    //     #[test]
    //     fn writes_ok() -> Result<(), ErrorType> {
    //         let content =
    //             "file1.txt 17/10/2023-18:15:00 da39a3ee5e6b4b0d3255bfef95601890afd80709 None\n";
    //         let readable = content.as_bytes();
    //         let index_map = new_index_hashmap(readable)?;

    //         let mut index_file = Cursor::new(Vec::new());
    //         save_index(index_map, &mut index_file)?;

    //         let written_data = String::from_utf8(index_file.into_inner()).unwrap();

    //         assert_eq!(
    //             written_data,
    //             "file1.txt 17/10/2023-18:15:00 da39a3ee5e6b4b0d3255bfef95601890afd80709 None\n"
    //         );

    //         Ok(())
    //     }
    // }

    // mod tests_new_index_hashmap {
    //     use super::super::new_index_hashmap;
    //     use crate::{files::file_info::FileInfo, git_errors::errors::ErrorType};

    //     #[test]
    //     fn single_file_ok() -> Result<(), ErrorType> {
    //         let file =
    //             "file.txt 17/10/2023-18:15:00 da39a3ee5e6b4b0d3255bfef95601890afd80709 None\n"
    //                 .as_bytes();

    //         let hash_map = new_index_hashmap(file)?;

    //         let file_info1 = FileInfo::try_from(
    //             "file.txt 17/10/2023-18:15:00 da39a3ee5e6b4b0d3255bfef95601890afd80709 None",
    //         )?;
    //         assert_eq!(hash_map["file.txt"], file_info1);
    //         Ok(())
    //     }

    //     #[test]
    //     fn two_files_ok() -> Result<(), ErrorType> {
    //         let file = "file1.txt 17/10/2023-18:15:00 da39a3ee5e6b4b0d3255bfef95601890afd80709 None\n\
    //                            file2.txt 17/10/2023-18:31:00 e02aa1b106d5c7c6a98def2b13005d5b84fd8dc8 a9993e364706816aba3e25717850c26c9cd0d89d\n".as_bytes();

    //         let hash_map = new_index_hashmap(file)?;

    //         let file_info1 = FileInfo::try_from(
    //             "file1.txt 17/10/2023-18:15:00 da39a3ee5e6b4b0d3255bfef95601890afd80709 None",
    //         )?;
    //         assert_eq!(hash_map["file1.txt"], file_info1);

    //         let file_info2 = FileInfo::try_from("file2.txt 17/10/2023-18:31:00 e02aa1b106d5c7c6a98def2b13005d5b84fd8dc8 a9993e364706816aba3e25717850c26c9cd0d89d")?;
    //         assert_eq!(hash_map["file2.txt"], file_info2);
    //         Ok(())
    //     }

    //     #[test]
    //     fn single_file_err() {
    //         let file = "file.txt 17/10/2023-18:15:00 da39a3ee5e6b4b0d3255bfef95601890afd80709\n"
    //             .as_bytes();

    //         match new_index_hashmap(file){
    //             Ok(_) => panic!(),
    //             Err(e) => assert_eq!(e.to_string(), "ERROR:[Error in file format: the format for file index is:'path modification_date current_blob_hash previous_blob_hash' separated by spaces.Previous blob hash may be None]"),
    //         }

    //         let file = "file.txt 17/10/2023-18:15:00 da39a3ee5e6b4 None\n".as_bytes();
    //         match new_index_hashmap(file) {
    //             Ok(_) => panic!(),
    //             Err(e) => assert_eq!(
    //                 e.to_string(),
    //                 "ERROR:[Invalid hash 'da39a3ee5e6b4'. Hashes must be 40 characters long]"
    //             ),
    //         }
    //     }
    // }

    // mod status_tests {

    //     #[test]
    //     fn status_ok() {
    //         let test_repo_path = std::env::current_dir().unwrap().display().to_string();
    //         let status_vecs = status(
    //             &(test_repo_path.clone() + "/tests/data/status/.git-rustico/index"),
    //             &(test_repo_path + "/tests/data/status"),
    //         )
    //         .unwrap();
    //         let untracked = &status_vecs[0];
    //         let not_staged = &status_vecs[1];
    //         let staged = &status_vecs[2];

    //         assert!(untracked.contains(&"untracked".to_string()));
    //         assert!(untracked.contains(&"dir/dir_file".to_string()));
    //         assert!(untracked.contains(&"dir/subdir/subdir_file".to_string()));

    //         assert!(not_staged.contains(&"not_staged".to_string()));

    //         assert!(staged.contains(&"staged".to_string()));
    //     }
    // }
}
