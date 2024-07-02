use crate::git_errors::command_error::CommandError::{FormatError, InexistentPath};
use crate::git_object::GitObject;
use crate::repo_paths::RepoPaths;
use crate::{files::object_type::ObjectType, git_errors::errors::ErrorType};
use sha1::{Digest, Sha1};
use std::fmt::Write;
use std::{fmt::Display, fs, path::PathBuf};

const FLAG_T: &str = "-t";
const FLAG_W: &str = "-w";
// const FLAG_STDIN: &str = "--stdin";

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct GitHash {
    hash: String,
}

impl GitHash {
    pub fn new(hash: &str) -> Result<Self, ErrorType> {
        if hash.len() != 40 {
            return Err(ErrorType::InvalidHash(hash.to_string()));
        }
        Ok(Self {
            hash: hash.to_string(),
        })
    }

    pub(crate) fn split_at_2(&self) -> (&str, &str) {
        self.hash.split_at(2)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.hash
    }

    pub fn hash_commit(content: &Vec<u8>) -> Self {
        Self::hash_object(content, ObjectType::Commit)
    }

    pub(crate) fn hash_blob(content: &Vec<u8>) -> GitHash {
        Self::hash_object(content, ObjectType::Blob)
    }

    pub(crate) fn hash_tree(content: &Vec<u8>) -> GitHash {
        Self::hash_object(content, ObjectType::Tree)
    }

    pub fn hash_object(content: &Vec<u8>, obj_type: ObjectType) -> Self {
        let header_text = ObjectType::add_header(content, &obj_type);

        Self::hash_sha1(&header_text)
    }

    pub fn hash_sha1(content: &Vec<u8>) -> Self {
        let mut hasher = Sha1::new();
        hasher.update(content);

        let result = hasher.finalize();

        let hash = format!("{:x}", result);

        Self { hash }
    }

    // todo --stdin (nice to have)
    pub fn hash_object_command(args: Vec<String>, repo_paths: RepoPaths) -> Result<(), ErrorType> {
        let (path, obj_type, write) = Self::parse_args(args)?;

        if !path.exists() {
            return Err(ErrorType::CommandError(InexistentPath(path)));
        }

        let content = fs::read(path)?;

        let obj_type = match obj_type {
            Some(t) => t,
            None => ObjectType::Blob,
        };

        let hash = Self::hash_object(&content, obj_type);

        if write {
            match obj_type {
                ObjectType::Commit => GitObject::save_commit(content, &repo_paths.get_objects())?,
                ObjectType::Blob => GitObject::save_blob(content, &repo_paths.get_objects())?,
                ObjectType::Tree => GitObject::save_tree(content, &repo_paths.get_objects())?,
            };
        }

        println!("{hash}");
        Ok(())
    }

    fn parse_args(args: Vec<String>) -> Result<(PathBuf, Option<ObjectType>, bool), ErrorType> {
        let mut found_t = false;
        let mut found_w = false;
        let mut path = None;
        let mut object_type = None;
        for arg in args {
            if !found_t && arg == FLAG_T {
                found_t = true;
            } else if !found_w && arg == FLAG_W {
                found_w = true
            } else if !found_t {
                path = Some(PathBuf::from(arg));
            } else if found_t {
                object_type = match arg.as_str() {
                    "blob" => Some(ObjectType::Blob),
                    "commit" => Some(ObjectType::Commit),
                    "tree" => Some(ObjectType::Tree),
                    _ => {
                        return Err(ErrorType::CommandError(FormatError(format!(
                            "invalid object type '{arg}' in hash-object command"
                        ))))
                    }
                };
            }
        }
        match path {
            Some(p) => Ok((p, object_type, found_w)),
            None => Err(ErrorType::CommandError(FormatError(
                "path not found for hash-object command".to_string(),
            ))),
        }
    }

    pub(crate) fn to_hex(&self) -> Result<Vec<u8>, ErrorType> {
        let mut bytes = Vec::new();

        for i in (0..40).step_by(2) {
            if let Some(byte_str) = self.hash.get(i..i + 2) {
                let byte = u8::from_str_radix(byte_str, 16)?;
                bytes.push(byte);
            }
        }

        Ok(bytes)
    }

    pub(crate) fn from_hex(bytes: &[u8]) -> Result<Self, ErrorType> {
        let mut hash = String::new();

        for &byte in bytes.iter() {
            write!(hash, "{:02x}", byte)?;
        }

        Ok(Self { hash })
    }
}

impl Display for GitHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.hash)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_hash_sha1_empty_string() {
//         assert_eq!(_hash_sha1(""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
//     }

//     #[test]
//     fn test_hash_sha1_hello_world() {
//         assert_eq!(
//             _hash_sha1("Hello, world"),
//             "e02aa1b106d5c7c6a98def2b13005d5b84fd8dc8"
//         );
//     }

//     #[test]
//     fn test_hash_sha1_different_input() {
//         assert_eq!(
//             _hash_sha1("abc"),
//             "a9993e364706816aba3e25717850c26c9cd0d89d"
//         );
//         assert_eq!(
//             _hash_sha1("12345"),
//             "8cb2237d0679ca88db6464eac60da96345513964"
//         );
//     }
// }
