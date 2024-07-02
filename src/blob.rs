use crate::{git_errors::errors::ErrorType, hash::GitHash};
use std::{fs::File, io::Write, path::PathBuf};

pub struct Blob {
    // path : PathBuf,
    content: String,
}

impl Blob {
    pub(crate) fn new(content: String) -> Self {
        Self { content }
        // Self{ path, content }
    }

    pub(crate) fn write_to_working_directory(&self, path: &PathBuf) -> Result<(), ErrorType> {
        //todo verificar si esta bien que lo cree
        let mut file = File::create(path)?;
        file.write_all(self.content.as_bytes())?;
        Ok(())
    }

    pub(crate) fn get_content(&self) -> String {
        self.content.clone()
    }

    pub(crate) fn get_hash(&self) -> GitHash {
        GitHash::hash_blob(&self.content.as_bytes().to_vec())
    }
}
