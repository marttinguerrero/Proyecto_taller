use crate::config::RepoConfig;
use crate::git_errors::errors::ErrorType;
use crate::refs::BranchRef;
use crate::repo_paths::RepoPaths;
use crate::utils::get_current_date_time;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{BufRead, Write};
use std::path::PathBuf;

const LIST_TAGS_OPTION: &str = "-l";
const DELETE_TAGS_OPTION: &str = "-d";
const VERIFY_TAGS_OPTION: &str = "-v";
const MESSAGE_TAGS_OPTION: &str = "-m";
const TAG_FILE: &str = "tags";
const SEPARATOR: &str = ";";

pub struct Tag {
    name: String,
    creator: String,
    hash_commit: String,
    message: Option<String>,
    date_time: String,
}

impl Tag {
    // Funcion para cliente
    pub fn command_tag(args: Vec<String>, repo_paths: &RepoPaths) -> Result<String, ErrorType> {
        let pathbuf_file = repo_paths.get_refs_tags().join(TAG_FILE);
        if !pathbuf_file.exists() {
            {
                File::create(pathbuf_file.clone())?;
            }
        }
        if args.is_empty() {
            Self::list_tag(pathbuf_file)
        } else if args.contains(&LIST_TAGS_OPTION.to_string()) {
            let arg = Self::delete_option(&args, &LIST_TAGS_OPTION.to_string());
            if arg.len() != 1 {
                return Err(ErrorType::RepositoryError(
                    "Error, in argument to list tags.".to_string(),
                ));
            }
            Self::list_tag_whit_patron(pathbuf_file, arg[0].clone())
        } else if args.contains(&VERIFY_TAGS_OPTION.to_string()) {
            let arg = Self::delete_option(&args, &VERIFY_TAGS_OPTION.to_string());
            if arg.len() != 1 {
                return Err(ErrorType::RepositoryError(
                    "Error, in argument to Verify tags.".to_string(),
                ));
            }
            Self::verify_tag(pathbuf_file, arg[0].clone())
        } else if args.contains(&DELETE_TAGS_OPTION.to_string()) {
            let arg = Self::delete_option(&args, &DELETE_TAGS_OPTION.to_string());
            if arg.len() != 1 {
                return Err(ErrorType::RepositoryError(
                    "Error, in argument to Delete tags.".to_string(),
                ));
            }
            Self::delete_tag(pathbuf_file, arg[0].clone())
        } else {
            Self::create_tag(args, pathbuf_file, repo_paths)
        }
    }

    fn create_tag(
        args: Vec<String>,
        pathbuf_file: PathBuf,
        repo_paths: &RepoPaths,
    ) -> Result<String, ErrorType> {
        if args.is_empty() {
            return Err(ErrorType::RepositoryError(
                "Error, in argument to create tag, no arguments.".to_string(),
            ));
        }
        let creator = Self::get_user_data_for_tags(repo_paths.get_config())?; // si no hay usuario cancela ensegida.
        let hash_commit = Self::get_hash_commit(repo_paths)?;
        let map = Self::read_file_tags(pathbuf_file.clone())?;
        let name = args[0].clone();
        if map.contains_key(&name) {
            return Err(ErrorType::RepositoryError(format!(
                "Error, in name to create tag, tag {} already exist.",
                name
            )));
        }
        let message = if args.len() > 2 && args[1] == *MESSAGE_TAGS_OPTION {
            Some(args[2].clone())
        } else {
            None
        };
        let date_time = get_current_date_time()?;
        let tag = Tag {
            name,
            creator,
            hash_commit,
            message,
            date_time,
        };
        Self::append_file_tags(pathbuf_file, tag)?;
        let text = format!("Tag: {}, created successfully.", args[0].clone());
        println!("{}", text);
        Ok(text)
    }

    fn delete_tag(pathbuf_file: PathBuf, tag_to_delete: String) -> Result<String, ErrorType> {
        let mut map = Self::read_file_tags(pathbuf_file.clone())?;
        let result = match map.remove(&tag_to_delete) {
            None => Err(ErrorType::RepositoryError(format!(
                "Error, Not exist tag: {} to delete.",
                tag_to_delete.clone()
            ))),
            Some(_) => Ok(format!(
                "Tag {} successfully removed.",
                tag_to_delete.clone()
            )),
        };
        Self::write_file_tags(pathbuf_file, map)?;
        println!("Tag {} successfully removed.", tag_to_delete);
        result
    }

    fn verify_tag(pathbuf_file: PathBuf, tag_to_verify: String) -> Result<String, ErrorType> {
        let map = Self::read_file_tags(pathbuf_file)?;
        let tags_names = map.contains_key(&tag_to_verify);
        let result = match tags_names {
            true => format!("Exist tag: {}.", tag_to_verify),
            false => format!("Not exist tag: {}.", tag_to_verify),
        };
        println!("{}", result);
        Ok(result)
    }

    fn list_tag(pathbuf_file: PathBuf) -> Result<String, ErrorType> {
        let map = Self::read_file_tags(pathbuf_file)?;
        let mut tags_names: Vec<String> = map.keys().map(|x| x.to_string()).collect();
        tags_names.sort();
        println!("{}", tags_names.join("\n"));
        Ok(format!("{} tags were listed", tags_names.len()))
    }

    fn list_tag_whit_patron(pathbuf_file: PathBuf, format: String) -> Result<String, ErrorType> {
        let map = Self::read_file_tags(pathbuf_file)?;
        let mut result = Vec::new();
        for key in map.keys() {
            if !key.contains(&format) {
                continue;
            }
            result.push(key.clone());
        }
        result.sort();
        println!("{}", result.join("\n"));
        Ok(format!(
            "{} tags were listed, whit contain {}",
            result.len(),
            format
        ))
    }

    ///////////////////////////////////////////////////////////

    fn read_file_tags(file_path: PathBuf) -> Result<HashMap<String, Tag>, ErrorType> {
        let file = File::open(file_path)?;
        let reader = io::BufReader::new(file);
        let mut map = HashMap::new();
        for line in reader.lines() {
            let line = line?;
            let parts: Vec<&str> = line.split(SEPARATOR).collect();
            if parts.len() != 5 {
                return Err(ErrorType::FormatError(
                    "Error, in read file tags, format of file unkown.".to_string(),
                ));
            }
            let label_name = parts[0].to_string();
            let creator = parts[1].to_string();
            let hash_commit = parts[2].to_string();
            let date_time = parts[3].to_string();
            let message = match parts[4].to_string().is_empty() {
                true => None,
                false => Some(parts[4].to_string()),
            };
            let tag = Tag {
                name: label_name.clone(),
                creator,
                hash_commit,
                message,
                date_time,
            };
            map.insert(label_name, tag);
        }
        Ok(map)
    }

    fn write_file_tags(file_path: PathBuf, map: HashMap<String, Tag>) -> Result<(), ErrorType> {
        let mut file = File::create(file_path)?;
        let mut to_write = Vec::new();
        for (_, tag) in map {
            let line = tag.parse_to_string();
            to_write.push(line);
        }
        let text = to_write.join("\n");
        file.write_all(text.as_bytes())?;
        Ok(())
    }

    fn append_file_tags(file_path: PathBuf, tag: Tag) -> Result<(), ErrorType> {
        let mut file = OpenOptions::new()
            .read(true) // Abre el archivo para lectura.
            .append(true) // Esto deberia hacer que la escritura sea al final del archivo sin rescribir nada de lo anterior
            .open(file_path)?;
        writeln!(file, "{}", tag.parse_to_string())?;
        Ok(())
    }

    fn delete_option(vec_origin: &[String], option: &String) -> Vec<String> {
        let result: Vec<String> = vec_origin
            .iter()
            .filter(|element| element != &option)
            .cloned()
            .collect();
        result
    }

    fn get_user_data_for_tags(path_config: PathBuf) -> Result<String, ErrorType> {
        let config = RepoConfig::open(path_config)?;
        let user = match config.get_user() {
            None => {
                return Err(ErrorType::ConfigError(
                    "Error not data user yet.".to_string(),
                ))
            }
            Some(user) => user,
        };
        Ok(format!("{} {}", user.get_name(), user.get_mail()))
    }

    fn get_hash_commit(repo_paths: &RepoPaths) -> Result<String, ErrorType> {
        let branch = BranchRef::open(repo_paths.get_head(), &repo_paths.get_refs_heads())?;
        let last_commit = match branch.get_last_commit(&repo_paths.get_objects())? {
            None => {
                return Err(ErrorType::RepositoryError(
                    "Error in get last commit HEAD, for tag.".to_string(),
                ))
            }
            Some(commit) => commit,
        };
        Ok(last_commit.get_hash().to_string())
    }

    fn parse_to_string(&self) -> String {
        let message = match &self.message {
            None => "".to_string(),
            Some(message) => message.clone(),
        };
        [
            self.name.clone(),
            self.creator.clone(),
            self.hash_commit.clone(),
            self.date_time.clone(),
            message,
        ]
        .join(SEPARATOR)
    }

    // Funcion para otras funciones
    pub fn get_tags_for_show_refs(
        repo_paths: &RepoPaths,
    ) -> Result<Vec<(String, String)>, ErrorType> {
        let pathbuf_file = repo_paths.get_refs_tags().join(TAG_FILE);
        let map = Self::read_file_tags(pathbuf_file)?;
        let mut result = Vec::new();
        for (name, tag) in map {
            let name_for_show_refs = match tag.get_message() {
                None => name,
                Some(message) => format!("{} {}", name, message),
            };
            result.push((name_for_show_refs, tag.get_hash()))
        }
        Ok(result)
    }

    fn get_hash(&self) -> String {
        self.hash_commit.clone()
    }

    fn get_message(&self) -> Option<String> {
        self.message.clone()
    }

    pub fn get_hash_of_tag(repo_paths: &RepoPaths, name_tag: String) -> Result<String, ErrorType> {
        let pathbuf_file = repo_paths.get_refs_tags().join(TAG_FILE);
        let map = Self::read_file_tags(pathbuf_file)?;
        match map.get(&name_tag) {
            None => Err(ErrorType::RepositoryError(format!(
                "Error, Not exist tag {}.",
                name_tag
            ))),
            Some(tag) => Ok(tag.get_hash()),
        }
    }
}
