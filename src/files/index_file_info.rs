use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use crate::{git_errors::errors::ErrorType, git_object::GitObject, hash::GitHash, utils};

// todo rename IndexFileInfo
/// Struct que guarda la información que tiene cada linea del index
#[derive(Debug, PartialEq, Clone)]
pub struct IndexFileInfo {
    path: PathBuf,
    mod_date: String,
    current_blob_hash: GitHash,
    previous_blob_hash: Option<GitHash>, // Useful to implement restore --stashed and to verify changes since commit
}

impl IndexFileInfo {
    pub(crate) fn new(local_path: PathBuf, home_path: &Path) -> Result<Self, ErrorType> {
        let global_path = home_path.join(&local_path);
        utils::verify_path_exists(&global_path)?;
        let mod_date = Self::date_modified_as_string(&global_path)?;
        let current_blob_hash = Self::current_blob_hash(&global_path)?;

        // this is initialized as Some(hash) so it can be recognized by status command as a new file
        // it will change to None only when a commit containing this file is created
        // see "status()" in status.rs and "FileInfo::added_since_commit(&self)"
        let previous_blob_hash = Some(current_blob_hash.clone());

        Ok(IndexFileInfo {
            path: local_path,
            mod_date,
            current_blob_hash,
            previous_blob_hash,
        })
    }

    pub(crate) fn get_path(&self) -> PathBuf {
        self.path.clone()
    }

    pub(crate) fn get_hash(&self) -> GitHash {
        self.current_blob_hash.clone()
    }

    /// Verifica si hubo un cambio en el working directory desde el ultimo add o commit
    pub(crate) fn verify_change(
        &mut self,
        path_objects: &Path,
        home_path: &Path,
    ) -> Result<(), ErrorType> {
        let current_date = Self::date_modified_as_string(&home_path.join(&self.path))?;
        if current_date != self.mod_date {
            self.mod_date = current_date;

            let new_content = std::fs::read(home_path.join(&self.path))?;
            let current_blob_hash = GitHash::hash_blob(&new_content);

            if current_blob_hash != self.current_blob_hash {
                if let Some(prev_hash) = &self.previous_blob_hash {
                    if prev_hash == &current_blob_hash {
                        // caso en que se des-modificó: vuelve al estado del ultimo commit
                        self.previous_blob_hash = None;
                        self.current_blob_hash = current_blob_hash;
                        return Ok(());
                    }
                }
                self.previous_blob_hash = Some(self.current_blob_hash.clone());
                self.current_blob_hash = current_blob_hash;

                // todo esto no deberia estar aca
                GitObject::save_blob(new_content, path_objects)?;
            }
        }
        Ok(())
    }

    // todo delegar a GitTimeDate
    /// Devuelve la fecha de modificacion del archivo en el working directory
    fn date_modified_as_string(path: &PathBuf) -> Result<String, ErrorType> {
        // utils::verify_path_exists(path)?;
        let file_metadata = std::fs::metadata(path)?;

        let system_time = file_metadata.modified()?;
        let modified_time: DateTime<Utc> = system_time.into();
        let format = "%Y/%m/%d-%H:%M:%S";
        Ok(modified_time.format(format).to_string())
    }

    /// Devuelve el hash del archivo en el current working directory
    fn current_blob_hash(path: &PathBuf) -> Result<GitHash, ErrorType> {
        utils::verify_path_exists(path)?;
        let content = std::fs::read(path)?;
        Ok(GitHash::hash_blob(&content))
    }

    /// Convierte una instancia en una linea con el formato del archivo index.
    /// formato : [path fecha_modificacion current_blob_hash previous_blob_hash]
    pub(crate) fn to_index_line(&self) -> String {
        let previous = match &self.previous_blob_hash {
            Some(h) => h.as_str(),
            None => "None",
        };
        format!(
            "{} {} {} {}\n",
            self.path.display(),
            self.mod_date,
            self.current_blob_hash,
            previous
        )
    }

    // todo quedo viejo, cambiar por el refactor de objects
    pub(crate) fn save(&self, path_objects: &Path, home_path: &Path) -> Result<(), ErrorType> {
        let content = std::fs::read(home_path.join(&self.path))?;
        GitObject::save_blob(content, path_objects)
    }

    pub(crate) fn has_changed(&self, home_path: &Path) -> Result<bool, ErrorType> {
        let current_hash = Self::current_blob_hash(&home_path.join(&self.path))?;
        Ok(current_hash != self.current_blob_hash)
    }

    pub(crate) fn added_since_commit(&self) -> bool {
        self.previous_blob_hash.is_some()
    }

    pub(crate) fn reset_previous_blob_hash(&mut self) {
        self.previous_blob_hash = None
    }
}

impl TryFrom<&str> for IndexFileInfo {
    type Error = ErrorType;
    // recibe una linea del index y devuelve un file info si el formato estaba bien, sino error
    fn try_from(line: &str) -> Result<IndexFileInfo, ErrorType> {
        let vec: Vec<&str> = line.split(' ').collect();
        if vec.len() != 4 {
            return Err(ErrorType::FormatError("the format for file index is:'path modification_date current_blob_hash previous_blob_hash' separated by spaces.Previous blob hash may be None".into()));
        }
        let path = PathBuf::from(vec[0]);
        let mod_date = vec[1].to_string();
        let current_blob_hash = GitHash::new(vec[2])?;
        let previous_blob_hash = match vec[3] {
            "None" => None,
            _ => Some(GitHash::new(vec[3])?),
        };

        Ok(Self {
            path,
            mod_date,
            current_blob_hash,
            previous_blob_hash,
        })
    }
}

#[cfg(test)]
mod tests {

    mod tests_current_blob_hash {
        use std::path::PathBuf;

        use crate::{
            files::index_file_info::IndexFileInfo, git_errors::errors::ErrorType, hash::GitHash,
        };

        #[ignore]
        #[test]
        fn current_blob_hash_ok() -> Result<(), ErrorType> {
            let path = "tests/data/empty_file";

            let hash = IndexFileInfo::current_blob_hash(&PathBuf::from(path))?;

            assert_eq!(
                hash,
                GitHash::new("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391")?
            );
            Ok(())
        }

        #[ignore]
        #[test]
        fn current_blob_hash_err() {
            let path = "tests/data/does_not_exist.txt";

            match IndexFileInfo::current_blob_hash(&PathBuf::from(path)) {
                Ok(_) => panic!(),
                Err(e) => assert_eq!(e.to_string(), "ERROR:[The given path does not match any files: tests/data/does_not_exist.txt]"),

            }
        }
    }

    mod tests_to_line {
        // use std::path::PathBuf;

        // use crate::{files::index_file_info::IndexFileInfo, git_errors::errors::ErrorType};

        // #[test]
        // fn test_to_line() -> Result<(), ErrorType> {
        //     let path = "tests/data/file_info/do_not_modify.txt";
        //     let file_info = IndexFileInfo::new(PathBuf::from(path))?;

        //     let date = IndexFileInfo::date_modified_as_string(&PathBuf::from(path))?;
        //     let hash = IndexFileInfo::current_blob_hash(&PathBuf::from(path))?;

        //     let line = file_info.to_index_line();
        //     assert_eq!(
        //         line,
        //         format!("{path} {date} {hash} 8a2616884b887ded5518f2242423c847bf98fd15\n")
        //     );

        //     Ok(())
        // }
    }

    mod tests_try_from {
        use std::path::PathBuf;

        use crate::{files::index_file_info::IndexFileInfo, git_errors::errors::ErrorType};

        #[ignore]
        #[test]
        fn try_from_ok() -> Result<(), ErrorType> {
            let path = "tests/data/file_info/do_not_modify.txt";
            let date = IndexFileInfo::date_modified_as_string(&PathBuf::from(path))?;
            let hash = IndexFileInfo::current_blob_hash(&PathBuf::from(path))?;
            let line: &str = &format!("{path} {date} {hash} None");

            let file_info = IndexFileInfo::try_from(line)?;

            assert_eq!(file_info.to_index_line().trim_end(), line);

            Ok(())
        }
    }
}
