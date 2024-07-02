use crate::branch::Branch;
use crate::cat_file::cat_file;
use crate::git_errors::errors::ErrorType;
use crate::git_object::GitObject;
use crate::hash::GitHash;
use crate::refs::BranchRef;
use crate::repo_paths::RepoPaths;
use crate::tag::Tag;
use crate::tree::Tree;

const OPTION_FORMAT: &str = "--format=";
const FORMAT_PATH: &str = "%(path)";
const FORMAT_MODE: &str = "%(objectmode)";
const FORMAT_TYPE: &str = "%(objecttype)";
const FORMAT_HASH: &str = "%(objectname)";
const FORMAT_SIZE: &str = "%(objectsize)";
const OPTION_SIZE: &str = "-l";
const REF_HEAD: &str = "HEAD";
// Posibles "modos":
// - 040000: Indica un directorio (tree). Este número representa el modo y los permisos de un directorio en Git.
// - 100644: Indica un archivo normal (blob) con permisos de lectura y escritura para el propietario y solo lectura para otros.
// - 100755: Indica un archivo ejecutable (blob) con permisos de ejecución para el propietario y lectura y ejecución para otros.
// - 120000: Indica un enlace simbólico (blob) en Git.
// - 160000: Indica un enlace a un commit de submodule en Git.
const MODE_TREE: &str = "040000";
const MODE_BLOB: &str = "100644";
// const MODE_BLOB_EXEC: &str = "100755";
// const MODE_BLOB_SYMBOL: &str = "120000";
const MODE_LINK_COMMIT: &str = "160000";

pub struct LsTree {
    tree: Tree,
    ref_path: String,
    option_size: bool,
    format: String,
}

impl LsTree {
    // https://linuxhint.com/git-ls-tree-documentation/
    // debe tener nombre de rama/HEAD/hash del commit/etiqueta del Tag
    /// output format (default) -> <mode> SP <type> SP <object> TAB <file>
    /// where:
    ///     mode = The mode of the object.
    ///     type = The type of the object (commit, blob or tree).
    ///     object = The name of the object.
    ///     file = path
    pub fn show_tree(args: Vec<String>, repo_paths: &RepoPaths) -> Result<String, ErrorType> {
        let ls_tree = Self::process_arguments(args, repo_paths)?;
        ls_tree.output_info()
    }

    /// parses the arguments to parse them to the struct
    fn process_arguments(args: Vec<String>, repo_paths: &RepoPaths) -> Result<Self, ErrorType> {
        let mut option_size = false;
        let mut ref_commit_or_branch = String::new();
        let mut ref_path = String::new();
        let mut format = String::new();
        for arg in args {
            if arg.contains(OPTION_FORMAT) {
                format = arg.clone();
            } else if arg == *OPTION_SIZE {
                option_size = true;
            } else if ref_commit_or_branch.is_empty() {
                ref_commit_or_branch = arg;
            } else if ref_path.is_empty() {
                ref_path = arg;
            }
        }
        Ok(LsTree {
            tree: Self::get_commit_tree(ref_commit_or_branch, repo_paths)?,
            ref_path,
            option_size,
            format,
        })
    }

    /// get the tree from the branch/commit.
    fn get_commit_tree(
        ref_commit_or_branch: String,
        repo_paths: &RepoPaths,
    ) -> Result<Tree, ErrorType> {
        if ref_commit_or_branch.is_empty() {
            return Err(ErrorType::FormatError(
                "ERROR, ls-tree requier one argument (Branch/Hash commit/HEAD)".to_string(),
            ));
        }
        let reference = if ref_commit_or_branch == *REF_HEAD {
            let branch = BranchRef::open(repo_paths.get_head(), &repo_paths.get_refs_heads())?;
            let last_commit = match branch.get_last_commit(&repo_paths.get_objects())? {
                None => {
                    return Err(ErrorType::RepositoryError(
                        "Error in get commit HEAD".to_string(),
                    ))
                }
                Some(commit) => commit,
            };
            last_commit.get_tree().clone()
        } else {
            match GitHash::new(&ref_commit_or_branch) {
                Ok(hash) => {
                    let commit = GitObject::read_commit(&hash, &repo_paths.get_objects())?;
                    commit.get_tree().clone()
                }
                Err(_) => {
                    match Branch::open(&repo_paths.get_refs_heads(), &ref_commit_or_branch) {
                        Ok(branch) => {
                            let last_commit = branch.get_last_commit(&repo_paths.get_objects())?;
                            last_commit.get_tree().clone()
                        }
                        Err(_) => {
                            // caso de TAG
                            let hash_tag =
                                Tag::get_hash_of_tag(repo_paths, ref_commit_or_branch.clone())?;
                            let hash = GitHash::new(&hash_tag)?;
                            let commit = GitObject::read_commit(&hash, &repo_paths.get_objects())?;
                            commit.get_tree().clone()
                        }
                    }
                }
            }
        };
        Ok(reference)
    }

    fn output_info(&self) -> Result<String, ErrorType> {
        let mut list_output = Vec::new();
        for (path, hash) in self.tree.get_files_vec() {
            let path_file = match path.to_str() {
                None => "",
                Some(a) => a,
            };
            if !self.ref_path.is_empty() && !path_file.contains(&self.ref_path) {
                continue;
            }
            let (file_type, mode) = self.get_type_mode(&hash)?;
            let name = hash.to_string();
            let file_size = match mode == MODE_BLOB {
                true => cat_file("-s", &hash.to_string())?,
                false => "-".to_string(),
            };
            match self.format.is_empty() {
                true => self.line_default(
                    mode,
                    file_type,
                    name,
                    file_size,
                    path_file.to_string(),
                    &mut list_output,
                )?,
                false => self.line_format(
                    mode,
                    file_type,
                    name,
                    file_size,
                    path_file.to_string(),
                    &mut list_output,
                )?,
            }
        }
        Ok(list_output.join("\n"))
    }

    fn line_default(
        &self,
        mode: String,
        file_type: String,
        name: String,
        file_size: String,
        path_file: String,
        list_output: &mut Vec<String>,
    ) -> Result<(), ErrorType> {
        match self.option_size {
            true => {
                list_output.push(format!(
                    "{} {} {} {}  {}",
                    mode, file_type, name, file_size, path_file
                ));
            }
            false => list_output.push(format!("{} {} {}  {}", mode, file_type, name, path_file)),
        }
        Ok(())
    }

    fn line_format(
        &self,
        mode: String,
        file_type: String,
        name: String,
        file_size: String,
        path_file: String,
        list_output: &mut Vec<String>,
    ) -> Result<(), ErrorType> {
        let line_whitout_format = self.format.replace(OPTION_FORMAT, "");
        let line_whit_mode = line_whitout_format.replace(FORMAT_MODE, &mode);
        let line_whit_type = line_whit_mode.replace(FORMAT_TYPE, &file_type);
        let line_whit_hash = line_whit_type.replace(FORMAT_HASH, &name);
        let line_whit_size = line_whit_hash.replace(FORMAT_SIZE, &file_size);
        let line_whit_path = line_whit_size.replace(FORMAT_PATH, &path_file);
        list_output.push(line_whit_path);
        Ok(())
    }

    fn get_type_mode(&self, hash: &GitHash) -> Result<(String, String), ErrorType> {
        let file_type = cat_file("-t", &hash.to_string())?;
        let mode = match &*file_type {
            "blob" => MODE_BLOB,
            "tree" => MODE_TREE,
            "commit" => MODE_LINK_COMMIT,
            _ => {
                return Err(ErrorType::ObjectType(
                    "blob/tree/commit".to_string(),
                    file_type.clone(),
                ))
            }
        };
        Ok((file_type, mode.to_string()))
    }
}
