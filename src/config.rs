use crate::{
    git_errors::{command_error::CommandError::IncorrectOptionAmount, errors::ErrorType},
    repo_paths::RepoPaths,
    user::User,
};
use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
};

const USER_NAME_CATEGORY: &str = "user_name:";
const USER_MAIL_CATEGORY: &str = "user_mail:";

pub struct RepoConfig {
    path_config: PathBuf,
    user_name: Option<String>,
    user_mail: Option<String>,
    // remote : String
}

impl RepoConfig {
    pub fn open(path_config: PathBuf) -> Result<Self, ErrorType> {
        if !path_config.exists() {
            return Err(ErrorType::RepositoryError(
                "Couldn't find config in default path ('.git-rustico/config')".to_string(),
            ));
        }
        let mut user_name = None;
        let mut user_mail = None;

        let file = File::open(&path_config)?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            if let Some((category, value)) = line?.split_once(' ') {
                match category {
                    USER_NAME_CATEGORY => user_name = Some(value.to_string()),
                    USER_MAIL_CATEGORY => user_mail = Some(value.to_string()),
                    _ => {
                        return Err(ErrorType::FormatError(
                            "Invalid category '{category}' in .git-rustico/config".to_string(),
                        ))
                    }
                }
            }
        }
        Ok(Self {
            path_config,
            user_name,
            user_mail,
        })
    }

    pub fn set_user_name(&mut self, name: &str) {
        self.user_name = Some(name.to_string());
    }

    pub fn set_user_mail(&mut self, mail: &str) {
        self.user_mail = Some(mail.to_string());
    }

    pub fn get_user(&self) -> Option<User> {
        match (&self.user_name, &self.user_mail) {
            (Some(n), Some(m)) => Some(User::new(n, m)),
            _ => None,
        }
    }

    pub fn save(&self) -> Result<(), ErrorType> {
        let mut content = Vec::<u8>::new();

        if let Some(uname) = &self.user_name {
            writeln!(
                content,
                "{}",
                format_args!("{USER_NAME_CATEGORY} {}", uname)
            )?;
        }
        if let Some(umail) = &self.user_mail {
            writeln!(
                content,
                "{}",
                format_args!("{USER_MAIL_CATEGORY} {}", umail)
            )?;
        }

        let mut file = File::create(&self.path_config)?;
        file.write_all(&content)?;
        Ok(())
    }

    // user_name: theo
    // user_mail: mail

    //ARGS: ["--user-name", "theo", "--user-mail", "mail"]
    pub fn config_command(repo_paths: RepoPaths, args: Vec<String>) -> Result<String, ErrorType> {
        if args.is_empty() {
            return Err(ErrorType::CommandError(IncorrectOptionAmount(1, 0)));
        }
        let mut config = Self::open(repo_paths.get_config())?;
        let mut found = false;
        let mut option = "";
        let mut result = Vec::new();
        for arg in args.iter() {
            if arg.is_empty() {
                continue;
            }
            if found {
                match option {
                    "--user-name" => {
                        config.set_user_name(arg);
                        result.push(format!("Set user name {}.", arg))
                    }
                    "--user-mail" => {
                        config.set_user_mail(arg);
                        result.push(format!("Set user mail {}.", arg))
                    }
                    _ => {
                        return Err(ErrorType::CommandError(
                            crate::git_errors::command_error::CommandError::UnknownOption(
                                arg.to_string(),
                                "--user-name or --user-mail".to_string(),
                            ),
                        ))
                    }
                };
                found = false;
            }

            if arg.starts_with('-') {
                if arg == "--test" {
                    config.set_user_name("test_username");
                    config.set_user_mail("test_user@fi.uba.ar");
                    break;
                }
                option = arg;
                found = true;
            }
        }
        config.save()?;
        Ok(result.join(" "))
    }
}
