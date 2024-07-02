use core::fmt;

const HEADER_BLOB: &str = "blob";
const HEADER_COMMIT: &str = "commit";
const HEADER_TREE: &str = "tree";

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum ObjectType {
    Commit,
    Blob,
    Tree,
}
impl ObjectType {
    pub fn add_header(content: &Vec<u8>, obj_type: &ObjectType) -> Vec<u8> {
        let size = content.len().to_string();
        let header = match obj_type {
            ObjectType::Commit => HEADER_COMMIT,
            ObjectType::Blob => HEADER_BLOB,
            ObjectType::Tree => HEADER_TREE,
        };
        let mut header_content = format!("{} {}\0", header, &size).as_bytes().to_vec();
        header_content.extend(content);
        header_content
    }
}

impl fmt::Display for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ObjectType::Commit => write!(f, "commit"),
            ObjectType::Blob => write!(f, "blob"),
            ObjectType::Tree => write!(f, "tree"),
        }
    }
}
