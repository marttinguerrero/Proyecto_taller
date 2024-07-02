use crate::branch::Branch;
use crate::git_errors::errors::ErrorType;
// use crate::refs::BranchRef;
use crate::repo_paths::RepoPaths;
use crate::tag::Tag;

pub struct ShowRef {
    head: bool,
    heads: bool,
    tags: bool,
    hash: bool, // faltaria agregar un ref al posible atributo de head
    // verify: bool,
    // dereference: bool,
    // exists: bool,
    // abbrev: bool,
    // quiet: bool,
    // exclude_existing: bool,
    repo_paths: RepoPaths,
}

impl ShowRef {
    /// By default, shows the tags, heads, and remote refs.
    pub fn show_ref(args: Vec<String>) -> Result<String, ErrorType> {
        let refs = Self::options(args)?;
        let mut result = String::new();
        for funtion in refs.show_references() {
            match funtion? {
                None => {}
                Some(a) => {
                    result.push_str(&a);
                }
            };
        }
        Ok(result)
    }

    fn options(args: Vec<String>) -> Result<Self, ErrorType> {
        let repo_paths = RepoPaths::new(std::env::current_dir()?)?;
        if args.is_empty() {
            return Ok(ShowRef {
                head: false,
                heads: false,
                tags: false,
                hash: false,
                repo_paths,
            });
        };
        let head = args.contains(&"--head".to_string());
        let heads = args.contains(&"--heads".to_string());
        let tags = args.contains(&"--tags".to_string());
        let hash = args.contains(&"--hash".to_string()) || args.contains(&"-s".to_string());
        Ok(ShowRef {
            head,
            heads,
            tags,
            hash,
            repo_paths,
        })
    }

    fn show_references(&self) -> Vec<Result<Option<String>, ErrorType>> {
        match (self.tags, self.heads) {
            (false, false) => vec![
                self.references_local(),
                self.references_tags(),
                self.references_remote(),
            ],
            (false, true) => vec![self.references_local()],
            (true, false) => vec![self.references_tags()],
            (true, true) => vec![self.references_local(), self.references_tags()],
        }
    }

    fn references_local(&self) -> Result<Option<String>, ErrorType> {
        let list = Branch::list_branches(&self.repo_paths.get_refs_heads())?;
        let mut result = String::new();
        for (name, hash) in list {
            if self.head && name.to_lowercase() == *"head" {
                continue;
            }
            result.push_str(&self.hashes_references(hash.to_string(), name));
        }
        match result.is_empty() {
            true => Ok(None),
            false => Ok(Some(result)),
        }
    }

    fn references_tags(&self) -> Result<Option<String>, ErrorType> {
        let tags = Tag::get_tags_for_show_refs(&self.repo_paths)?;
        if tags.is_empty() {
            return Ok(None);
        }
        let mut result = Vec::new();
        match self.hash {
            true => {
                for (_, hash) in tags {
                    result.push(hash);
                }
            }
            false => {
                for (name, hash) in tags {
                    result.push(format!("{} {}", hash, name));
                }
            }
        }
        Ok(Some(result.join("\n")))
    }

    fn references_remote(&self) -> Result<Option<String>, ErrorType> {
        // Remote::remote_command(Vec::new(), self.repo_paths.get_remote())
        let list = Branch::list_branches(&self.repo_paths.get_refs_remote())?;
        let mut result = String::new();
        for (name, hash) in list {
            if self.head && name.to_lowercase() == *"head" {
                continue;
            }
            result.push_str(&self.hashes_references(hash.to_string(), name));
        }
        match result.is_empty() {
            true => Ok(None),
            false => Ok(Some(result)),
        }
    }

    fn hashes_references(&self, hash: String, reference: String) -> String {
        match self.hash {
            true => format!("{}\n", hash),
            false => format!("{} {}\n", hash, reference),
        }
    }
}

#[cfg(test)]
mod list_dir_file_paths_tests {
    use crate::show_ref::ShowRef;

    #[test]
    fn options_test() {
        let basic_all_false = ShowRef::options(Vec::new()).unwrap();
        assert!(!basic_all_false.hash);
        assert!(!basic_all_false.head);
        assert!(!basic_all_false.heads);
        assert!(!basic_all_false.tags);
        let basic_all_true = ShowRef::options(vec![
            "-s".to_string(),
            "--head".to_string(),
            "--heads".to_string(),
            "--tags".to_string(),
        ])
        .unwrap();
        assert!(basic_all_true.hash);
        assert!(basic_all_true.head);
        assert!(basic_all_true.heads);
        assert!(basic_all_true.tags);
        let basic_only_hash = ShowRef::options(vec!["--hash".to_string()]).unwrap();
        assert!(basic_only_hash.hash);
        assert!(!basic_only_hash.head);
        assert!(!basic_only_hash.heads);
        assert!(!basic_only_hash.tags);
    }
}
