use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::io::{BufRead, BufReader, Write as IOWrite};
use std::net::TcpStream;
use std::{fs::File, path::PathBuf};

use crate::git_errors::command_error::CommandError;
use crate::git_errors::errors::ErrorType;
use crate::protocol::pkt_line::create_pkt_line;

// const BASE_ADDRESS_IP_DAEMON: &str = "127.0.0.1";
// const BASE_ADDRESS_PORT_DAEMON: &str = "9418";
const COMMAND_ADD: &str = "add"; // el comando CLONE deberia crear un remote con nombre "origin"
                                 // puede aceptar el -f o --fetch para ejecutar fetch despues de ejecutarse
const COMMAND_REMOVE: &str = "rm";
const COMMAND_REMOVE_2: &str = "remove";
const COMMAND_RENAME: &str = "rename";
const COMMAND_GET_URL: &str = "get-url"; // puede tener push y all
                                         // const COMMAND_SHOW: &str = "show";
                                         // const COMMAND_PRUNE: &str = "prune";

const EXTRA_PARAMETERS: &str = "version=1";
const DEFAULT_PORT: &str = "9418";

pub struct Remote {
    // remote name: URL
    remotes: HashMap<String, String>,
    // local branch name: (remote name, remote branch name)
    branches: HashMap<String, (String, String)>,
    path_remote: PathBuf,
}

impl Remote {
    pub fn remote_command(args: Vec<String>, path_remote: PathBuf) -> Result<(), ErrorType> {
        let args = args.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
        match args.as_slice() {
            [] => Self::print_remotes(path_remote)?,
            [COMMAND_ADD, name, url] => {
                Self::add(path_remote, name.to_string(), url.to_string())?;
            }
            [COMMAND_REMOVE | COMMAND_REMOVE_2, name] => {
                if Self::remove(path_remote, name.to_string())?.is_none() {
                    return Err(ErrorType::CommandError(CommandError::InvalidArgument(
                        format!("no such remote: {}", name),
                    )));
                }
            }
            [COMMAND_RENAME, old_name, new_name] => {
                if Self::rename(path_remote, old_name.to_string(), new_name.to_string())?.is_none()
                {
                    return Err(ErrorType::CommandError(CommandError::InvalidArgument(
                        format!("no such remote: {}", old_name),
                    )));
                }
            }
            [COMMAND_GET_URL, name] => match Self::get_url(path_remote, name)? {
                Some(url) => println!("{}", url),
                None => {
                    return Err(ErrorType::CommandError(CommandError::InvalidArgument(
                        format!("no such remote '{}'", name),
                    )))
                }
            },
            _ => {
                return Err(ErrorType::CommandError(CommandError::FormatError(
                    "".to_string(),
                )))
            }
        }
        Ok(())
    }

    fn from_file(path_remote: PathBuf) -> Result<Self, ErrorType> {
        let file = File::open(&path_remote)?;
        let reader = BufReader::new(file);

        let mut remotes = HashMap::new();
        let mut branches = HashMap::new();

        for line in reader.lines() {
            let line = line?;
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();

            if let ["remote", name, url] = parts.as_slice() {
                remotes.insert(name.to_string(), url.to_string());
            } else if let ["branch", local_branch_name, remote_name, remote_branch_name] =
                parts.as_slice()
            {
                branches.insert(
                    local_branch_name.to_string(),
                    (remote_name.to_string(), remote_branch_name.to_string()),
                );
            } else {
                return Err(ErrorType::FormatError(format!(
                    "invalid line in remote file: {}",
                    line
                )));
            }
        }
        Ok(Self {
            remotes,
            branches,
            path_remote,
        })
    }

    pub fn add(path_remote: PathBuf, name: String, url: String) -> Result<(), ErrorType> {
        let mut remote = Self::from_file(path_remote)?;
        remote.remotes.insert(name, url);
        remote.save()
    }

    pub fn rename(
        path_remote: PathBuf,
        name: String,
        new_name: String,
    ) -> Result<Option<String>, ErrorType> {
        let mut remote = Self::from_file(path_remote)?;

        if let Some(url) = remote.remotes.remove(&name) {
            remote.remotes.insert(new_name.clone(), url);
            remote.save()?;
            Ok(None)
        } else {
            Ok(Some(new_name))
        }
    }

    pub fn remove(path_remote: PathBuf, name: String) -> Result<Option<String>, ErrorType> {
        let mut remote = Self::from_file(path_remote)?;

        match remote.remotes.remove(&name) {
            None => Ok(None),
            Some(url) => {
                remote.save()?;
                Ok(Some(url))
            }
        }
    }

    pub fn get_url(path_remote: PathBuf, remote_name: &str) -> Result<Option<String>, ErrorType> {
        let remote = Self::from_file(path_remote)?;
        Ok(remote.remotes.get(remote_name).cloned())
    }

    fn save(&self) -> Result<(), ErrorType> {
        let mut result = String::new();
        for (remote_name, url) in &self.remotes {
            writeln!(&mut result, "remote {remote_name} {url}")?;
        }
        for (local_branch_name, (remote_name, remote_branch_name)) in &self.branches {
            writeln!(
                &mut result,
                "branch {local_branch_name} {remote_name} {remote_branch_name}"
            )?;
        }
        Ok(fs::write(&self.path_remote, result.as_bytes())?)
    }

    fn print_remotes(path_remote: PathBuf) -> Result<(), ErrorType> {
        let remotes = Self::get_remotes(path_remote)?;

        for (remote_name, _) in remotes.iter() {
            println!("{}", remote_name);
        }
        Ok(())
    }

    pub fn get_remotes(path_remote: PathBuf) -> Result<HashMap<String, String>, ErrorType> {
        let remote = Self::from_file(path_remote)?;
        Ok(remote.remotes.clone())
    }

    pub fn set_upstream(
        path_remote: PathBuf,
        local_branch_name: String,
        remote_name: String,
        remote_branch_name: String,
    ) -> Result<(), ErrorType> {
        let mut remote = Self::from_file(path_remote)?;
        remote
            .branches
            .insert(local_branch_name, (remote_name, remote_branch_name.clone()));
        remote.save()
    }

    pub fn get_upstream(
        path_remote: PathBuf,
        local_branch_name: String,
    ) -> Result<Option<(String, String)>, ErrorType> {
        let remote = Self::from_file(path_remote)?;
        Ok(remote.branches.get(&local_branch_name).cloned())
    }

    //////////////////////////////////////////
    ///            CONNECTION             ///
    ////////////////////////////////////////

    fn get_conection_stream(
        path_remote: PathBuf,
        remote_name: String,
    ) -> Result<(TcpStream, String, String), ErrorType> {
        let url = Self::get_url(path_remote, &remote_name)?.ok_or(ErrorType::CommandError(
            CommandError::InvalidArgument(format!("no such remote '{}'", remote_name)),
        ))?;

        let (address, repo) = Self::parse_url(&url)?;

        let host = address.clone() + ":" + DEFAULT_PORT;
        let stream = TcpStream::connect(&host)?;
        Ok((stream, host, repo))
    }

    fn parse_url(url: &str) -> Result<(String, String), ErrorType> {
        let (protocol, addr) = match url.split_once("://") {
            Some((p, a)) => (p, a),
            None => {
                return Err(ErrorType::FormatError(format!(
                    "Invalid remote address: {url}"
                )))
            }
        };
        if protocol != "git" {
            return Err(ErrorType::ConfigError(format!(
                "unsuported remote transport protocol: {protocol}"
            )));
        }
        let (address, repo) = match addr.split_once('/') {
            Some((h, r)) => (h, r),
            None => {
                return Err(ErrorType::FormatError(format!(
                    "Invalid remote address: {url}"
                )))
            }
        };
        Ok((address.to_string(), format!("/{}", repo)))
    }

    pub fn connect_upload_pack(
        path_remote: PathBuf,
        remote_name: String,
    ) -> Result<TcpStream, ErrorType> {
        let (mut stream, address, repo) = Self::get_conection_stream(path_remote, remote_name)?;
        Self::connection_request(&mut stream, "git-upload-pack", &address, &repo)?;
        Ok(stream)
    }

    pub fn connect_receive_pack(
        path_remote: PathBuf,
        remote_name: String,
    ) -> Result<TcpStream, ErrorType> {
        let (mut stream, address, repo) = Self::get_conection_stream(path_remote, remote_name)?;
        Self::connection_request(&mut stream, "git-receive-pack", &address, &repo)?;
        Ok(stream)
    }

    fn connection_request<W: IOWrite>(
        stream: &mut W,
        request_command: &str,
        host: &str,
        repo: &str,
    ) -> Result<(), ErrorType> {
        let request = format!(
            "{} {}\0host={}\0\0{}\0",
            request_command, repo, host, EXTRA_PARAMETERS
        );
        let line = create_pkt_line(&request)?;
        Ok(stream.write_all(line.as_bytes())?)
    }
}
