use crate::branch::Branch;
use crate::commit::Commit;
use crate::git_errors::errors::ErrorType;
use crate::repo_paths::RepoPaths;
use std::cmp::min;
use std::fs;

pub struct Rebase;

impl Rebase {
    pub fn rebase(repo_paths: RepoPaths, args: Vec<String>) -> Result<String, ErrorType> {
        match args.len() {
            0 => Err(ErrorType::FormatError(
                "Error, requier some argument.".to_string(),
            )),
            1 => Self::rebase_self(args[0].clone(), repo_paths),
            _ => Self::rebase_exlplicit(args[0].clone(), args[1].clone(), repo_paths),
        }
    }

    fn rebase_self(brach: String, repo_paths: RepoPaths) -> Result<String, ErrorType> {
        Self::rebase_exlplicit(
            fs::read_to_string(repo_paths.get_head())?,
            brach,
            repo_paths,
        )
    }

    fn rebase_exlplicit(
        base: String,
        other: String,
        repo_paths: RepoPaths,
    ) -> Result<String, ErrorType> {
        if base == other {
            return Err(ErrorType::FormatError(
                "Error, branches cannot be equal in rebase.".to_string(),
            ));
        }
        let other = match other == *"HEAD" {
            true => fs::read_to_string(repo_paths.get_head())?,
            false => other,
        };
        let base_branch = Branch::open(&repo_paths.get_refs_heads(), &base)?;
        let mut other_branch = Branch::open(&repo_paths.get_refs_heads(), &other)?;
        let last_commit_base = base_branch.get_last_commit(&repo_paths.get_objects())?;
        let last_commit_other = other_branch.get_last_commit(&repo_paths.get_objects())?;
        let mut commits_history_base =
            last_commit_base.get_commits_history_rec(&repo_paths.get_objects())?;
        let mut commits_history_other =
            last_commit_other.get_commits_history_rec(&repo_paths.get_objects())?;
        commits_history_base.reverse();
        commits_history_other.reverse();
        let val_diff = Self::get_pos_for_first_diff(&commits_history_other, &commits_history_base);
        let mut commits_for_delete = commits_history_other.clone();
        commits_for_delete.drain(0..val_diff);
        for j in val_diff..commits_history_other.len() {
            let before_commit: Commit = match j == val_diff {
                false => commits_history_other[j - 1].clone(),
                true => last_commit_base.clone(),
            };
            let mut commit: Commit = commits_history_other[j].clone();
            commit.set_parent_hash(Some(before_commit.get_hash()))?;
            commit.save(&repo_paths.get_objects())?;
            commits_history_other[j] = commit;
        }
        other_branch.set_last_commit_hash(
            commits_history_other[commits_history_other.len() - 1].get_hash(),
        );
        other_branch.save()?;
        for commit in commits_for_delete {
            commit.delete(&repo_paths.get_objects())?;
        }
        Ok(format!("Branch base: {} for: {}.", base, other))
    }

    fn get_pos_for_first_diff(
        commits_history_other: &[Commit],
        commits_history_base: &[Commit],
    ) -> usize {
        let mut val_diff = 0;
        for i in 1..min(commits_history_other.len(), commits_history_base.len()) {
            if commits_history_other[i].get_message() == commits_history_base[i].get_message() {
                continue;
            }
            val_diff = i;
            break;
        }
        val_diff
    }
}
