use std::{
    collections::HashMap,
    fmt::Debug,
    path::{Path, PathBuf},
};

use crate::{
    files::index_file_info::IndexFileInfo, git_errors::errors::ErrorType, git_object::GitObject,
    hash::GitHash,
};

const FILE_MODE: &str = "100644";
const DIR_MODE: &str = "040000";

enum Type {
    File,
    Dir,
}

/// Estructura recursiva del objeto Tree. Puede instanciarse desde un object o desde index
#[derive(Debug, Clone, PartialEq)]
pub struct Tree {
    trees: HashMap<String, Tree>,
    files: HashMap<String, GitHash>,
}

// formato de la linea en el object: [modo path\0hash_en_binario]
impl Tree {
    pub(crate) fn new() -> Self {
        Self {
            trees: HashMap::new(),
            files: HashMap::new(),
        }
    }

    pub fn from_index(index_vec: Vec<IndexFileInfo>) -> Result<Self, ErrorType> {
        let mut tree = Self::new();

        for file in index_vec {
            let path = file.get_path();
            let path_str = path.to_str().ok_or(ErrorType::InvalidPath(format!(
                "{} in index file",
                file.get_path().display()
            )))?;
            tree.add(path_str, file.get_hash())
        }

        Ok(tree)
    }

    // format is [mode path\0hash]
    pub(crate) fn from_object(
        hash: &GitHash,
        content: Vec<u8>,
        path_objects: &Path,
    ) -> Result<Self, ErrorType> {
        let mut trees = HashMap::new();
        let mut files = HashMap::new();

        let mut iter = content.into_iter().peekable();

        while iter.peek().is_some() {
            let mode: String = iter
                .by_ref()
                .take_while(|&byte| byte != b' ')
                .map(|byte| byte as char)
                .collect::<String>();

            let path: String = iter
                .by_ref()
                .take_while(|&byte| byte != b'\0')
                .map(|byte| byte as char)
                .collect::<String>();
            // let path = PathBuf::from(path);

            let hash_bytes: Vec<u8> = iter.by_ref().take(20).collect();
            let hash_entry = GitHash::from_hex(&hash_bytes)?;

            match mode.as_str() {
                "100644" => _ = files.insert(path, hash_entry),
                "040000" => {
                    _ = trees.insert(path, GitObject::read_tree(&hash_entry, path_objects)?)
                }
                _ => {
                    return Err(ErrorType::FormatError(format!(
                        "invalid UNIX filesystem mode ('{mode}') in tree '{hash}'"
                    )))
                }
            };
        }

        let mut tree = Self::new();
        tree.set_trees(trees);
        tree.set_files(files);
        Ok(tree)
    }

    pub(crate) fn get_hash(&self) -> Result<GitHash, ErrorType> {
        // self.hash.clone()
        let content = self.generate_content()?;
        Ok(GitHash::hash_tree(&content))
    }

    /// Dado una instancia de Tree convierte sus datos en texto con el formato del objeto.
    /// El formato de cada linea es el siguiente : [modo path_elemento\0hash_elemento_en_bin]
    /// siendo elemento un tree o un blob
    pub fn generate_content(&self) -> Result<Vec<u8>, ErrorType> {
        let entries: Result<Vec<(Type, String, GitHash)>, ErrorType> = self
            .trees
            .iter()
            .map(|(key, tree)| Ok((Type::Dir, key.clone(), tree.get_hash()?)))
            .collect();
        let mut entries = entries?;

        let files: Vec<(Type, String, GitHash)> = self
            .files
            .iter()
            .map(|f| (Type::File, f.0.clone(), f.1.clone()))
            .collect();
        entries.extend(files);

        entries.sort_by_key(|e| e.1.clone());

        let mut content: Vec<u8> = Vec::new();

        for (tipe, name, hash) in entries {
            let mode = match tipe {
                Type::File => FILE_MODE,
                Type::Dir => DIR_MODE,
            };

            let mode_path = format!("{mode} {name}\0");
            content.extend(mode_path.as_bytes());

            let hash_hex = hash.to_hex()?;
            content.extend(hash_hex);
        }

        Ok(content)
    }

    /// Dado una instancia de Tree escribe su contenido en objects al igual que el de sus
    /// sub-trees recursivamente
    pub(crate) fn save(&self, path_objects: &Path) -> Result<(), ErrorType> {
        for tree in self.trees.values() {
            tree.save(path_objects)?;
        }
        let content = self.generate_content()?;

        GitObject::save_tree(content, path_objects)?;
        Ok(())
    }

    pub(crate) fn get_files_vec(&self) -> Vec<(PathBuf, GitHash)> {
        self.get_files_vec_rec(PathBuf::from(""))
    }

    fn get_files_vec_rec(&self, current_dir: PathBuf) -> Vec<(PathBuf, GitHash)> {
        let mut vec = Vec::new();

        for (name, hash) in &self.files {
            let path = current_dir.join(name);
            vec.push((path, hash.clone()));
        }
        for (dir, tree) in &self.trees {
            let sub_vec = tree.get_files_vec_rec(current_dir.join(dir));
            vec.extend(sub_vec);
        }
        vec
    }

    fn set_trees(&mut self, trees: HashMap<String, Tree>) {
        self.trees = trees;
    }
    fn set_files(&mut self, files: HashMap<String, GitHash>) {
        self.files = files;
    }

    pub(crate) fn add(&mut self, path: &str, hash: GitHash) {
        // match path.split_once('/') {
        //     Some((dir, sub_dir)) => match self.trees.get_mut(dir) {
        //         Some(tree) => tree.add(sub_dir, hash),
        //         None => {
        //             let mut sub_tree = Self::new();
        //             sub_tree.add(sub_dir, hash);
        //             self.trees.insert(dir.to_string(), sub_tree);
        //         }
        //     },
        //     // Here path is just file name
        //     None => self.files.push((path.to_string(), hash)),
        // }
        if let Some((dir, sub_dir)) = path.split_once('/') {
            self.trees
                .entry(dir.to_string())
                .or_insert_with(Self::new)
                .add(sub_dir, hash);
        } else {
            self.files.insert(path.to_string(), hash);
        }
    }
}

// impl Debug for Tree {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "tree name: {}\n\tfiles:\n", self.name)?;

//         for (name, hash) in &self.files {
//             writeln!(f, "\t\tfile:{name}   {hash}")?;
//         }
//         writeln!(f, "\ttrees:")?;
//         for name in self.trees.keys() {
//             writeln!(f, "\t\t{}", name)?;
//         }
//         for tree in self.trees.values() {
//             writeln!(f, "{:?}", tree)?;
//         }
//         Ok(())
//     }
// }

// #[cfg(test)]
// mod tests_tree {
//     use crate::{files::hashing::hash_tree, hash::GitHash, git_errors::errors::ErrorType};

//     use super::Tree;

//     // test to show how Debug displays trees
//     // #[test]
//     // fn add() {
//     //     let mut tree = Tree::new("");
//     //     tree.add("dir1/dir2/dir3/file3", "hash_file3");
//     //     tree.add("dir1/dir2/file2", "hash_file2");
//     //     tree.add("dir1/file1", "hash_file1");
//     //     tree.add("dir1_b/file1_b", "hash_file1_b");
//     //     println!("{:?}", tree);
//     // }

//     #[test]
//     fn one_file_no_dir() -> Result<(), ErrorType>{
//         let mut tree = Tree::new("repo_home");
//         tree.add("file1", "file_1_hash");

//         let text = "file1 blob file_1_hash";

//         let hash_tree_home = crate::files::hashing::hash_tree(text);

//         assert_eq!(tree.get_hash(), GitHash::new(&hash_tree_home)?);
//         Ok(())
//     }

//     #[test]
//     fn one_file_one_dir() -> Result<(), ErrorType>{
//         let mut tree = Tree::new("repo_home");
//         tree.add("dir1/file1", "file_1_hash");

//         let text_dir1 = "file1 blob file_1_hash";
//         let hash_dir1 = hash_tree(text_dir1);

//         let text_tree_home = format!("dir1 tree {hash_dir1}");
//         let hash_tree_home = hash_tree(&text_tree_home);

//         assert_eq!(tree.get_hash(), GitHash::new(&hash_tree_home)?);
//         Ok(())
//     }

//     #[test]
//     fn two_files_two_dirs() -> Result<(), ErrorType>{
//         let mut tree = Tree::new("repo_home");
//         tree.add("dir1/file1", "file_1_hash");
//         tree.add("dir2/file2", "file_2_hash");

//         let text_dir1 = "file1 blob file_1_hash";
//         let hash_dir1 = hash_tree(text_dir1);

//         let text_dir2 = "file2 blob file_2_hash";
//         let hash_dir2 = hash_tree(text_dir2);

//         let text_tree_home = format!(
//             "dir1 tree {hash_dir1}\n\
//                                               dir2 tree {hash_dir2}"
//         );
//         let hash_tree_home = hash_tree(&text_tree_home);

//         assert_eq!(tree.get_hash(), GitHash::new(&hash_tree_home)?);

//         //swap order of add call:
//         let mut tree = Tree::new("repo_home");
//         tree.add("dir2/file2", "file_2_hash");
//         tree.add("dir1/file1", "file_1_hash");

//         let text_dir1 = "file1 blob file_1_hash";
//         let hash_dir1 = hash_tree(text_dir1);

//         let text_dir2 = "file2 blob file_2_hash";
//         let hash_dir2 = hash_tree(text_dir2);

//         let text_tree_home = format!(
//             "dir1 tree {hash_dir1}\n\
//                                               dir2 tree {hash_dir2}"
//         );
//         let hash_tree_home = hash_tree(&text_tree_home);

//         assert_eq!(tree.get_hash(), GitHash::new(&hash_tree_home)?);
//         Ok(())
//     }

//     #[test]
//     fn three_files_two_dirs() -> Result<(), ErrorType>{
//         let mut tree = Tree::new("repo_home");
//         tree.add("dir1/file1", "file_1_hash");
//         tree.add("dir2/file2", "file_2_hash");
//         tree.add("dir2/file2b", "file_2b_hash");

//         let text_dir1 = "file1 blob file_1_hash";
//         let hash_dir1 = hash_tree(text_dir1);

//         let text_dir2 = "file2 blob file_2_hash\n\
//                                file2b blob file_2b_hash";
//         let hash_dir2 = hash_tree(text_dir2);

//         let text_tree_home = format!(
//             "dir1 tree {hash_dir1}\n\
//                                               dir2 tree {hash_dir2}"
//         );
//         let hash_tree_home = hash_tree(&text_tree_home);

//         assert_eq!(tree.get_hash(), GitHash::new(&hash_tree_home)?);
//         Ok(())
//     }

//     #[test]
//     fn sub_dirs() -> Result<(), ErrorType>{
//         let mut tree = Tree::new("repo_home");
//         tree.add("dir1/file1", "file_1_hash");
//         tree.add("dir1/dir1b/file1b", "file_1b_hash");
//         tree.add("dir2/file2", "file_2_hash");
//         tree.add("dir2/file2b", "file_2b_hash");

//         let text_dir1b = "file1b blob file_1b_hash";
//         let hash_dir_1b = hash_tree(text_dir1b);

//         let text_dir1 = format!(
//             "dir1b tree {hash_dir_1b}\n\
//                                file1 blob file_1_hash"
//         );
//         let hash_dir1 = hash_tree(&text_dir1);

//         let text_dir2 = "file2 blob file_2_hash\n\
//                                file2b blob file_2b_hash";
//         let hash_dir2 = hash_tree(text_dir2);

//         let text_tree_home = format!(
//             "dir1 tree {hash_dir1}\n\
//                                               dir2 tree {hash_dir2}"
//         );
//         let hash_tree_home = hash_tree(&text_tree_home);

//         assert_eq!(tree.get_hash(), GitHash::new(&hash_tree_home)?);

//         //swap add() calls order
//         let mut tree = Tree::new("repo_home");
//         tree.add("dir2/file2b", "file_2b_hash");
//         tree.add("dir1/file1", "file_1_hash");
//         tree.add("dir2/file2", "file_2_hash");
//         tree.add("dir1/dir1b/file1b", "file_1b_hash");

//         let text_dir1b = "file1b blob file_1b_hash";
//         let hash_dir_1b = hash_tree(text_dir1b);

//         let text_dir1 = format!(
//             "dir1b tree {hash_dir_1b}\n\
//                                file1 blob file_1_hash"
//         );
//         let hash_dir1 = hash_tree(&text_dir1);

//         let text_dir2 = "file2 blob file_2_hash\n\
//                                file2b blob file_2b_hash";
//         let hash_dir2 = hash_tree(text_dir2);

//         let text_tree_home = format!(
//             "dir1 tree {hash_dir1}\n\
//                                               dir2 tree {hash_dir2}"
//         );
//         let hash_tree_home = hash_tree(&text_tree_home);

//         assert_eq!(tree.get_hash(), GitHash::new(&hash_tree_home)?);
//         Ok(())
//     }
// }
