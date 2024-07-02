use crate::blob::Blob;
use crate::commit::Commit;
use crate::compressor::Compressor;
use crate::files::object_type::ObjectType;
use crate::git_errors::{command_error::CommandError::InvalidHash, errors::ErrorType};
use crate::hash::GitHash;
use crate::tree::Tree;
use std::{fs, fs::File, io::Write, path::Path};

#[derive(Clone, Debug, PartialEq)]
pub struct GitObject;

impl GitObject {
    pub fn read_commit(hash: &GitHash, path_objects: &Path) -> Result<Commit, ErrorType> {
        let (obj_type, _, content) = Self::parse_object(hash, path_objects)?;
        let content: String = String::from_utf8(content)
            .ok()
            .ok_or(ErrorType::FormatError(format!(
                "invalid_content in commit object {hash}"
            )))?;
        match obj_type {
            ObjectType::Commit => Ok(Commit::from_object(hash, content, path_objects)?),
            _ => Err(ErrorType::ObjectType(
                "commit".to_string(),
                obj_type.to_string(),
            )),
        }
    }

    pub(crate) fn read_tree(hash: &GitHash, path_objects: &Path) -> Result<Tree, ErrorType> {
        let (obj_type, _, content) = Self::parse_object(hash, path_objects)?;
        match obj_type {
            ObjectType::Tree => Ok(Tree::from_object(hash, content, path_objects)?),
            _ => Err(ErrorType::ObjectType(
                "tree".to_string(),
                obj_type.to_string(),
            )),
        }
    }

    pub(crate) fn read_blob(hash: &GitHash, path_objects: &Path) -> Result<Blob, ErrorType> {
        let (obj_type, _, content) = Self::parse_object(hash, path_objects)?;
        let content: String = String::from_utf8(content)
            .ok()
            .ok_or(ErrorType::FormatError(format!(
                "invalid_content in blob object {hash}"
            )))?;
        match obj_type {
            ObjectType::Blob => Ok(Blob::new(content)),
            _ => Err(ErrorType::ObjectType(
                "blob".to_string(),
                obj_type.to_string(),
            )),
        }
    }

    // separa el contenido del object en (type, size y content)
    fn parse_object(
        hash: &GitHash,
        path_objects: &Path,
    ) -> Result<(ObjectType, usize, Vec<u8>), ErrorType> {
        let file = Self::open_object(hash, path_objects)?;
        let file_content = Compressor::uncompress(file)?;

        if !file_content.contains(&b' ') || !file_content.contains(&b'\0') {
            return Err(ErrorType::FormatError(format!(
                "object '{hash}' has invalid format (should be [{}])",
                "<type> <size>\0<content>"
            )));
        }

        match Self::parse_object_content(file_content) {
            Ok(v) => Ok(v),
            Err(ErrorType::Parse(_)) => Err(ErrorType::FormatError(format!(
                "Invalid size in object {hash}"
            ))),
            Err(ErrorType::ObjectType(_, g)) => Err(ErrorType::FormatError(format!(
                "invalid object type '{g}' in object {hash}"
            ))),
            Err(e) => Err(e),
        }
    }

    pub fn parse_object_content(
        content: Vec<u8>,
    ) -> Result<(ObjectType, usize, Vec<u8>), ErrorType> {
        let mut iter = content.iter();

        let obj_type: String = iter
            .by_ref()
            .take_while(|&&byte| byte != b' ')
            .map(|&byte| byte as char)
            .collect();

        let obj_type = match obj_type.as_str() {
            "tree" => ObjectType::Tree,
            "blob" => ObjectType::Blob,
            "commit" => ObjectType::Commit,
            _ => {
                return Err(ErrorType::ObjectType(
                    "tree, blob or commit".to_string(),
                    obj_type.to_string(),
                ))
            }
        };

        // let mut size = Vec::new();
        // while let Some(&byte) = iter.next(){
        //     if byte == b'\0' {
        //         break;
        //     }
        //     size.push(byte);
        // }
        // let mut cursor = Cursor::new(size);
        // let mut size_string = String::new();
        // cursor.read_to_string(&mut size_string)?;
        // let size :usize = size_string.parse::<usize>().map_err(|_| ErrorType::FormatError(format!("Invalid object type in object {hash}")))?;

        let size: usize = iter
            .by_ref()
            .take_while(|&&byte| byte != b'\0')
            .map(|&byte| byte as char)
            .collect::<String>()
            .parse()?;
        // .map_err(|_| ErrorType::FormatError(format!("Invalid size in object {:?}", hash)))?;

        // let mut content = Vec::new();

        // while let Some(&byte) = iter.next() {
        //     content.push(byte);
        // }
        let content: Vec<u8> = iter.cloned().collect();

        Ok((obj_type, size, content))
    }

    pub fn open_object(hash_object: &GitHash, path_objects: &Path) -> Result<File, ErrorType> {
        let (dir, file) = hash_object.split_at_2();
        let path = path_objects.join(dir).join(file);
        if !Path::new(&path).exists() {
            return Err(ErrorType::CommandError(InvalidHash(format!(
                "{hash_object} not in objects"
            ))));
        }
        Ok(File::open(path)?)
    }

    pub(crate) fn save_commit(content: Vec<u8>, path_objects: &Path) -> Result<(), ErrorType> {
        Self::save_object(content, ObjectType::Commit, path_objects)
    }

    pub fn delete_object(hash: GitHash, path_objects: &Path) -> Result<(), ErrorType> {
        // println!("Borrar {}.", hash);
        let (dir, file) = hash.split_at_2();
        let path = path_objects.join(format!("{dir}/{file}"));
        if path.exists() {
            // println!("Borrado iniciado.");
            fs::remove_file(path)?;
            // println!("Borrado exitoso.");
        };
        Ok(())
    }

    pub(crate) fn save_tree(content: Vec<u8>, path_objects: &Path) -> Result<(), ErrorType> {
        Self::save_object(content, ObjectType::Tree, path_objects)
    }

    pub(crate) fn save_blob(content: Vec<u8>, path_objects: &Path) -> Result<(), ErrorType> {
        Self::save_object(content, ObjectType::Blob, path_objects)
    }

    pub fn save_object(
        content: Vec<u8>,
        obj_type: ObjectType,
        path_objects: &Path,
    ) -> Result<(), ErrorType> {
        let header_content = ObjectType::add_header(&content, &obj_type);

        let hash = GitHash::hash_sha1(&header_content);
        let (dir, file) = hash.split_at_2();

        let path = path_objects.join(format!("{dir}/{file}"));

        if let Some(parent_dir) = path.parent() {
            std::fs::create_dir_all(parent_dir)?;
        }
        let mut file = File::create(path)?;

        let compressed_content = Compressor::compress(header_content)?;

        file.write_all(compressed_content.as_slice())?;
        Ok(())
    }
}
