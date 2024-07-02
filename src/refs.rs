use crate::{branch::Branch, commit::Commit, index::Index, repo_paths::RepoPaths};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

use crate::{
    git_errors::{command_error::CommandError::IncorrectAmount, errors::ErrorType},
    git_object::GitObject,
    hash::GitHash,
};

pub struct BranchRef {
    branch: Option<Branch>,
    path_ref: PathBuf, // podria guardar path branches
}

impl BranchRef {
    pub fn open(
        path_ref: PathBuf,
        path_branches: &Path,
        // path_objects: &Path,
    ) -> Result<Self, ErrorType> {
        if !path_ref.exists() {
            return Err(ErrorType::RepositoryError(format!(
                "Inexistent ref: {}",
                path_ref.display()
            )));
        }
        let branch_name = fs::read_to_string(&path_ref)?;
        if !path_branches.join(&branch_name).exists() {
            return Ok(Self {
                branch: None,
                path_ref,
            });
        }
        let branch = Branch::open(path_branches, &branch_name)?;
        Ok(Self {
            branch: Some(branch),
            path_ref,
        })
    }

    pub fn new(branch: Option<Branch>, path_ref: PathBuf) -> Self {
        // meter save adentro?
        Self { branch, path_ref }
    }

    // obtiene el commmit al que apunta head. None si no existe (cuando no hay commits head no apunta a nada)
    pub fn get_last_commit(&self, path_objects: &Path) -> Result<Option<Commit>, ErrorType> {
        if let Some(branch) = &self.branch {
            let hash = branch.get_last_commit_hash();
            let commit = GitObject::read_commit(&hash, path_objects)?;
            return Ok(Some(commit));
        }
        Ok(None)
    }

    pub fn get_branch_name(&self) -> Option<String> {
        self.branch.as_ref().map(|b| b.get_name())
    }

    pub fn checkout_command(repo_paths: RepoPaths, args: Vec<String>) -> Result<String, ErrorType> {
        if args.len() != 1 {
            return Err(ErrorType::CommandError(IncorrectAmount(
                1.to_string(),
                args.len(),
            )));
        }

        let path_index = repo_paths.get_index();
        let path_home = repo_paths.get_home();
        let path_objects = repo_paths.get_objects();
        let path_branches = repo_paths.get_refs_heads();

        let index = Index::new(File::open(&path_index)?)?;
        index.check_for_changes(&path_home, &repo_paths.get_ignore())?;

        let branch_name: &str = &args[0];
        let branch = Branch::open(&path_branches, branch_name)?;

        let mut head = Self::open(repo_paths.get_head(), &path_branches)?;

        head.checkout_to(branch, index, &path_home, &path_objects, &path_index)?;
        head.save()?;

        let result = format!("checkout successfull to {} branch.", branch_name);
        println!("{}", result);
        Ok(result)
    }

    // switch branch-name -> creates a new one from the remote one with the same name
    // switch branch-name remote-branch-name -> creates a new one from the remote one with the name branch-name
    pub fn switch_command(repo_paths: RepoPaths, args: Vec<String>) -> Result<String, ErrorType> {
        let path_index = repo_paths.get_index();
        let path_home = repo_paths.get_home();

        let index = Index::new(File::open(&path_index)?)?;
        index.check_for_changes(&path_home, &repo_paths.get_ignore())?;

        let local_branch_name: &str = &args[0];

        let remote_branch_name = match args.len() {
            1 => local_branch_name,
            2 => &args[1],
            _ => {
                return Err(ErrorType::CommandError(IncorrectAmount(
                    "1 or 2".to_string(),
                    args.len(),
                )))
            }
        };

        if repo_paths.get_refs_heads().join(local_branch_name).exists() {
            return Err(ErrorType::RepositoryError(format!(
                "Branch {} already exists locally",
                local_branch_name
            )));
        }
        let remote_branch = Branch::open(&repo_paths.get_refs_remote(), remote_branch_name)?;
        let local_branch = Branch::new(
            local_branch_name,
            &repo_paths.get_refs_heads(),
            remote_branch.get_last_commit_hash(),
        )?;

        let mut head = Self::open(repo_paths.get_head(), &repo_paths.get_refs_heads())?;
        head.checkout_to(
            local_branch,
            index,
            &path_home,
            &repo_paths.get_objects(),
            &path_index,
        )?;
        head.save()?;

        let result = format!("switch successfull to {} branch.", local_branch_name);
        println!("{}", result);
        Ok(result)
    }

    // todo : que no reciba index
    pub fn checkout_to(
        &mut self,
        branch: Branch,
        mut index: Index,
        path_home: &Path,
        path_objects: &Path,
        path_index: &PathBuf,
    ) -> Result<(), ErrorType> {
        let files = Self::update_working_dir_files(
            branch.get_last_commit(path_objects)?,
            path_home,
            path_objects,
        )?;
        self.set_branch(branch);

        index.update_to_working_dir(files, path_home)?;
        index.reset_previous_blob_hash();
        index.save(&mut File::create(path_index)?)?;

        Ok(())
    }

    pub fn set_branch(&mut self, branch: Branch) {
        self.branch = Some(branch);
    }

    // borra todo el working directory y arma uno nuevo a partir de un commit
    fn update_working_dir_files(
        branch_commit: Commit,
        path_home: &Path,
        path_objects: &Path,
    ) -> Result<Vec<(PathBuf, GitHash)>, ErrorType> {
        Self::delete_directory_contents_rec(path_home.to_path_buf())?;

        let files = branch_commit.get_files_vec();

        for (local_path, hash) in &files {
            let global_path = path_home.join(local_path);
            if let Some(parent) = global_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let blob_object = GitObject::read_blob(hash, path_objects)?;
            blob_object.write_to_working_directory(&global_path)?;
        }
        Ok(files)
    }

    // todo : usar remove_dir_all. se puede ignorando el .git?
    fn delete_directory_contents_rec(directory_path: PathBuf) -> Result<(), ErrorType> {
        if directory_path.is_dir() {
            let entries = fs::read_dir(directory_path)?;
            for entry in entries {
                let entry = entry?;

                if entry.file_type()?.is_dir() {
                    let entry_name = entry.file_name();
                    if entry_name == *".git-rustico" {
                        continue;
                    }
                    let subdirectory = entry.path();
                    Self::delete_directory_contents_rec(subdirectory)?;
                } else {
                    let file_name = entry.file_name();
                    if file_name == *".gitignore" {
                        continue;
                    }
                    fs::remove_file(entry.path())?;
                }
            }
        }

        Ok(())
    }

    // todo (nice to have) imprmir a donde se encuentra cada branch y ver que pasa cuando hay open merge
    pub fn log_command(repo_paths: RepoPaths) -> Result<usize, ErrorType> {
        let path_objects = repo_paths.get_objects();
        let head = BranchRef::open(repo_paths.get_head(), &repo_paths.get_refs_heads())?;
        let branches = Branch::list_branches(&repo_paths.get_refs_heads())?;

        if let Some(commit) = head.get_last_commit(&repo_paths.get_objects())? {
            let commit_history = commit.get_commits_history(&path_objects)?;
            let number_of_commits = commit_history.len();
            for commit in commit_history {
                let mut refs = Vec::new();
                if let Some(hash) = head.get_last_commit_hash() {
                    if hash == commit.get_hash() {
                        refs.push("HEAD".to_string());
                    }
                }
                if let Some(branch_name) = branches.iter().find(|b| *b.1 == commit.get_hash()) {
                    refs.push(branch_name.0.clone());
                }
                commit.log_display(refs);
            }
            return Ok(number_of_commits);
        }
        Err(ErrorType::RepositoryError(
            "your current branch doesnt have commits yet".to_string(),
        ))
    }

    pub fn set_last_commit(
        &mut self,
        hash: GitHash,
        path_branches: &Path,
    ) -> Result<(), ErrorType> {
        if let Some(branch) = &mut self.branch {
            branch.set_last_commit_hash(hash);
            branch.save()?;
        } else {
            self.branch = Some(Branch::new("master", path_branches, hash)?);
        }
        Ok(())
    }

    pub fn delete(&self) -> Result<(), ErrorType> {
        if self.path_ref.exists() {
            fs::remove_file(&self.path_ref)?;
        }
        Ok(())
    }

    pub fn get_last_commit_hash(&self) -> Option<GitHash> {
        self.branch.as_ref().map(|c| c.get_last_commit_hash())
        // self.get_last_commit().map(|x| x.get_hash())
    }

    pub fn save(&self) -> Result<(), ErrorType> {
        if let Some(branch) = &self.branch {
            fs::write(&self.path_ref, branch.get_name())?;
        }
        Ok(())
    }

    pub(crate) fn get_branch(&self) -> Option<Branch> {
        self.branch.as_ref().cloned()
    }
}
