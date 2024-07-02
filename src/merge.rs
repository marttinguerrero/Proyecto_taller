use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use crate::{
    blob::Blob,
    branch::Branch,
    commit::Commit,
    config::RepoConfig,
    diff::{Diff, ModificationType},
    git_errors::{command_error::CommandError::IncorrectAmount, errors::ErrorType},
    git_object::GitObject,
    hash::GitHash,
    index::Index,
    refs::BranchRef,
    repo_paths::RepoPaths,
    user::User,
};
type TreeFileVector = Vec<(PathBuf, Blob)>;
pub struct Merge;

impl Merge {
    pub fn merge_command(repo_paths: RepoPaths, args: Vec<String>) -> Result<(), ErrorType> {
        if args.len() != 1 {
            return Err(ErrorType::CommandError(IncorrectAmount(
                "1".to_string(),
                args.len(),
            )));
        }
        let path_branches = repo_paths.get_refs_heads();
        let path_index = repo_paths.get_index();
        let path_home = repo_paths.get_home();

        let index = Index::open(&path_index)?;
        index.check_for_changes(&path_home, &repo_paths.get_ignore())?;

        let mut head = BranchRef::open(repo_paths.get_head(), &path_branches)?;

        let mut head_branch =
            match head.get_branch() {
                Some(b) => b,
                None => return Err(ErrorType::RepositoryError(
                    "can't merge because HEAD isn't pointing to any branch. You must commit first"
                        .to_string(),
                )),
            };

        let branch_name = &args[0];
        let branch = Branch::open(&path_branches, branch_name)?;

        let (modified_files, conflict_files) = Self::merge(
            &mut head_branch,
            branch.clone(),
            repo_paths.clone(),
            None,
            None,
        )?;
        if conflict_files.is_empty() {
            head.checkout_to(
                head_branch.clone(),
                index,
                &path_home,
                &repo_paths.get_objects(),
                &path_index,
            )?;
            head.save()?;
            if modified_files.is_empty() {
                println!("Fast-forward merge successfully completed"); //add verbose flag
            } else {
                println!("Three-way merge completed successfully");
            }
        } else {
            println!("There were conflicts while merging. Both branches modified:\n");
            for (file_path, _) in &conflict_files {
                println!("\t{}\n", file_path.display());
            }
            let all_files: Vec<(PathBuf, Blob)> =
                modified_files.into_iter().chain(conflict_files).collect();
            for (file_path, blob) in all_files {
                let mut file = File::create(&file_path)?;
                file.write_all(blob.get_content().as_bytes())?;
            }
            println!("Solve them manually and then close the merge with add and commit.");
            fs::write(repo_paths.get_head_merge(), branch.get_name().as_bytes())?;
        }

        Ok(())
    }

    pub fn merge(
        head_branch: &mut Branch,
        branch: Branch,
        repo_paths: RepoPaths,
        message: Option<String>,
        user: Option<User>,
    ) -> Result<(TreeFileVector, TreeFileVector), ErrorType> {
        let path_objects = repo_paths.get_objects();

        let head_commit = head_branch.get_last_commit(&path_objects)?;
        let branch_commit = branch.get_last_commit(&path_objects)?;

        let last_common_ancestor: Commit = branch_commit
            .last_common_ancestor(&head_commit, &path_objects)?
            .ok_or(ErrorType::RepositoryError(
                "No common commit ancestor between both branch tips".to_string(),
            ))?;

        if last_common_ancestor == head_commit {
            head_branch.set_last_commit_hash(branch_commit.get_hash());
            head_branch.save()?;
            Ok((Vec::new(), Vec::new()))
        } else {
            let files = Self::real_merge(
                head_branch,
                branch,
                last_common_ancestor,
                repo_paths,
                message,
                user,
            )?;
            Ok(files)
        }
    }

    fn real_merge(
        head_branch: &mut Branch,
        branch: Branch,
        last_common_ancestor: Commit,
        repo_paths: RepoPaths,
        message: Option<String>,
        user: Option<User>,
    ) -> Result<(TreeFileVector, TreeFileVector), ErrorType> {
        let objects = &repo_paths.get_objects();
        let head_commit = head_branch.get_last_commit(objects)?;
        let branch_commit = branch.get_last_commit(objects)?;

        let head_files = head_commit.get_files_vec();
        let branch_files = branch_commit.get_files_vec();
        let lca_files = last_common_ancestor.get_files_vec();

        let (modified_files, conflict_files) =
            Self::compare_files(head_files, branch_files, lca_files, &repo_paths)?;

        let mut tree = head_commit.get_tree().clone();
        for (file, blob) in &modified_files {
            GitObject::save_blob(blob.get_content().as_bytes().to_vec(), objects)?;
            tree.add(&format!("{}", file.display()), blob.get_hash());
        }

        if !conflict_files.is_empty() {
            return Ok((modified_files, conflict_files));
        }

        let message = match message {
            Some(m) => m,
            None => format!(
                "Merge branch <{}> into <{}>",
                branch.get_name(),
                head_branch.get_name()
            ),
        };

        let user = match user{
            Some(a) => a,
            None => RepoConfig::open(repo_paths.get_config())?.get_user().ok_or(ErrorType::ConfigError("User name and mail should be set before commiting (use 'git-rustico config --user-name <name> --user-mail <mail>')".to_string()))?
        };

        let commit = Commit::new(
            tree,
            Some(head_commit.get_hash()),
            Some(branch_commit.get_hash()),
            &message,
            user,
        )?;

        commit.save(objects)?;
        head_branch.set_last_commit_hash(commit.get_hash());
        head_branch.save()?;

        Ok((modified_files, conflict_files))
    }

    pub fn compare_files(
        head_files: Vec<(PathBuf, GitHash)>,
        branch_files: Vec<(PathBuf, GitHash)>,
        lca_files: Vec<(PathBuf, GitHash)>,
        repo_paths: &RepoPaths,
    ) -> Result<(TreeFileVector, TreeFileVector), ErrorType> {
        let path_objects = repo_paths.get_objects();
        let mut conflict_files = Vec::new();
        let mut modified_files = Vec::new();
        // maybe sacar afuera

        let head_files = head_files
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<HashMap<PathBuf, GitHash>>();
        let lca_files = lca_files
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<HashMap<_, _>>();

        for (branch_file_path, branch_blob_hash) in branch_files {
            if let Some(head_blob_hash) = head_files.get(&branch_file_path) {
                //both have it
                if head_blob_hash == &branch_blob_hash
                // both have it but with the same changes
                {
                    continue;
                };
                let lca_blob_hash = match lca_files.get(&branch_file_path) {
                    Some(a) => a,
                    None => {
                        return Err(ErrorType::RepositoryError(
                            "No common ancestor between branches commits".to_string(),
                        ))
                    }
                };
                if lca_blob_hash == &branch_blob_hash {
                    // only head has changes
                    continue;
                }
                // both have changes

                let head_content =
                    GitObject::read_blob(head_blob_hash, &path_objects)?.get_content();
                let branch_content =
                    GitObject::read_blob(&branch_blob_hash, &path_objects)?.get_content();

                let lca_content = GitObject::read_blob(lca_blob_hash, &path_objects)?.get_content();

                let mut merged_content: String = String::new();

                if Self::three_way_merge(
                    &lca_content,
                    &head_content,
                    &branch_content,
                    &mut merged_content,
                )? {
                    conflict_files.push((branch_file_path.clone(), Blob::new(merged_content)));
                } else {
                    modified_files.push((branch_file_path, Blob::new(merged_content)));
                }
            } else {
                // only branch has it
                let blob = GitObject::read_blob(&branch_blob_hash, &path_objects)?;
                modified_files.push((branch_file_path, blob));
            }
        }
        Ok((modified_files, conflict_files))
    }

    /// Given an original common base text and two different modified versions of the base it merges them.
    /// It keeps the parts that are the same in the three of them and applies changes where just one text
    /// made a change or marks a conflict where both texts modified the same part.
    fn three_way_merge(
        lca_content: &str,
        head_content: &str,
        branch_content: &str,
        buffer: &mut String,
    ) -> Result<bool, ErrorType> {
        let mut conflict = false;

        let diff_head = Diff::diff(lca_content, head_content);
        let diff_branch = Diff::diff(lca_content, branch_content);

        let mut i = 0;
        let mut j = 0;

        let mut lines = Vec::new();

        while i < diff_head.len() || j < diff_branch.len() {
            if i >= diff_head.len() {
                match &diff_branch[j] {
                    ModificationType::Remove(_) => continue,
                    ModificationType::Same(l) => lines.push(l.clone()),
                    ModificationType::Add(l) => lines.push(l.clone()),
                }
                j += 1;
            } else if j >= diff_branch.len() {
                match &diff_head[i] {
                    ModificationType::Remove(_) => continue,
                    ModificationType::Same(l) => lines.push(l.clone()),
                    ModificationType::Add(l) => lines.push(l.clone()),
                }
                i += 1;
            } else {
                match (&diff_head[i], &diff_branch[j]) {
                    (ModificationType::Same(l), ModificationType::Same(_)) => {
                        lines.push(l.clone());
                        i += 1;
                        j += 1;
                    }
                    (ModificationType::Same(_), ModificationType::Add(l)) => {
                        lines.push(l.clone());
                        j += 1;
                    }
                    (ModificationType::Add(l), ModificationType::Same(_)) => {
                        lines.push(l.clone());
                        i += 1;
                    }
                    (ModificationType::Same(_), ModificationType::Remove(_))
                    | (ModificationType::Remove(_), ModificationType::Same(_))
                    | (ModificationType::Remove(_), ModificationType::Remove(_)) => {
                        i += 1;
                        j += 1;
                    }
                    (ModificationType::Add(l1), ModificationType::Add(l2)) => {
                        if l1 == l2 {
                            lines.push(l1.clone());
                            i += 1;
                            j += 1;
                        } else {
                            //conflicto
                            conflict = true;
                            let conflict_block =
                                Self::conflict(&diff_head, &mut i, &diff_branch, &mut j);
                            lines.push(conflict_block);
                        }
                    }
                    // conflicto
                    _ => {
                        conflict = true;
                        let conflict_block =
                            Self::conflict(&diff_head, &mut i, &diff_branch, &mut j);
                        lines.push(conflict_block);
                    }
                }
            }
        }

        *buffer = lines.join("\n");
        Ok(conflict)
    }

    /// Given two vectors of different modifications to the lines of a text and the position of a conflict it creates
    /// a divided block of lines containing the conflicting changes made in each modified text separatedly.
    fn conflict(
        diff_head: &[ModificationType],
        head_conflict_line: &mut usize,
        diff_branch: &[ModificationType],
        branch_conflict_line: &mut usize,
    ) -> String {
        let mut result = Vec::new();
        result.push("<<<<<<< HEAD");
        while *head_conflict_line < diff_head.len() {
            match &diff_head[*head_conflict_line] {
                ModificationType::Same(_) => break,
                ModificationType::Add(l) => result.push(l),
                ModificationType::Remove(_) => (),
            }
            *head_conflict_line += 1;
        }
        result.push("=======");

        while *branch_conflict_line < diff_branch.len() {
            match &diff_branch[*branch_conflict_line] {
                ModificationType::Same(_) => break,
                ModificationType::Add(l) => result.push(l),
                ModificationType::Remove(_) => (),
            }
            *branch_conflict_line += 1;
        }
        result.push(">>>>>>> Merge Branch");

        result.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use crate::{git_errors::errors::ErrorType, merge::Merge};

    #[test]
    fn conflict_both_modified_same_line() -> Result<(), ErrorType> {
        let lca_content = "line 1\nline 2\nline 3\n";
        let head_content = "line 1\nline 4\nline 3\n";
        let branch_content = "line 1\nline 5\nline 3\n";
        let mut writer = String::new();
        let result =
            Merge::three_way_merge(lca_content, head_content, branch_content, &mut writer)?;
        assert!(result);
        assert_eq!(
            writer,
            "line 1\n<<<<<<< HEAD\nline 4\n=======\nline 5\n>>>>>>> Merge Branch\nline 3"
        );
        Ok(())
    }

    #[test]
    fn no_conflict_nothing_changed() -> Result<(), ErrorType> {
        let lca_content = "line 1\nline 2\nline 3";
        let head_content = "line 1\nline 2\nline 3";
        let branch_content = "line 1\nline 2\nline 3";
        let mut writer = String::new();
        let result =
            Merge::three_way_merge(lca_content, head_content, branch_content, &mut writer)?;
        assert!(!result);
        assert_eq!(writer, "line 1\nline 2\nline 3");
        Ok(())
    }

    #[test]
    fn no_conflict_both_added_same_line() -> Result<(), ErrorType> {
        let lca_content = "line 1\nline 2\nline 3\n";
        let head_content = "line 1\nline 2\nline 3\nline 4\n";
        let branch_content = "line 1\nline 2\nline 3\nline 4\n";
        let mut writer = String::new();
        let result =
            Merge::three_way_merge(lca_content, head_content, branch_content, &mut writer)?;
        assert!(!result);
        assert_eq!(writer, "line 1\nline 2\nline 3\nline 4");
        Ok(())
    }

    #[test]
    fn no_conflict_both_deleted_same_line() -> Result<(), ErrorType> {
        let lca_content = "line 1\nline 2\nline 3\n";
        let head_content = "line 1\nline 3\n";
        let branch_content = "line 1\nline 3\n";
        let mut writer = String::new();
        let result =
            Merge::three_way_merge(lca_content, head_content, branch_content, &mut writer)?;
        assert!(!result);
        assert_eq!(writer, "line 1\nline 3");
        Ok(())
    }

    #[test]
    fn conflict_both_added_lines_at_beginning_and_end() -> Result<(), ErrorType> {
        let lca_content = "line 1\nline 2\nline 3\n";
        let head_content = "line 0\nline 1\nline 2\nline 3\nline 4\n";
        let branch_content = "line 5\nline 1\nline 2\nline 2.5\nline 3\nline 6\n";
        let mut writer = String::new();
        let result =
            Merge::three_way_merge(lca_content, head_content, branch_content, &mut writer)?;
        assert!(result);
        assert_eq!(
            writer,
            "<<<<<<< HEAD\nline 0\n=======\nline 5\n>>>>>>> Merge Branch\nline 1\nline 2\nline 2.5\nline 3\n<<<<<<< HEAD\nline 4\n=======\nline 6\n>>>>>>> Merge Branch"
        );
        Ok(())
    }

    #[test]
    fn conflict_multi_line_changes() -> Result<(), ErrorType> {
        let lca_content = "line 1\nline 2\nline 3\n";
        let head_content = "line 1\nline a\nline b\nline c\nline 3\n";
        let branch_content = "line 1\nline d\nline e\nline f\nline 3\n";
        let mut writer = String::new();
        let result =
            Merge::three_way_merge(lca_content, head_content, branch_content, &mut writer)?;
        assert!(result);
        assert_eq!(writer, "line 1\n<<<<<<< HEAD\nline a\nline b\nline c\n=======\nline d\nline e\nline f\n>>>>>>> Merge Branch\nline 3");
        Ok(())
    }

    #[test]
    fn no_conflict_one_line() -> Result<(), ErrorType> {
        let lca_content = "line 1";
        let head_content = "line 0\nline 1";
        let branch_content = "line 1\nline 2";
        let mut writer = String::new();
        let result =
            Merge::three_way_merge(lca_content, head_content, branch_content, &mut writer)?;
        assert!(!result);
        assert_eq!(writer, "line 0\nline 1\nline 2");
        Ok(())
    }
}
