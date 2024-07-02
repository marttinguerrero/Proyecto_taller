use crate::config::RepoConfig;
use crate::git_errors::errors::ErrorType;
use crate::utils::get_current_date_time;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

const SEPARATOR: &str = "; ";
const SEPARATOR_USER: &str = " - ";
const SEPARATOR_ARGS: &str = " ";
const NOT_ARGS: &str = "";

pub struct LogFile {
    file: File,
}

impl LogFile {
    pub fn write_log_file_whitout_thread(file_path: PathBuf, text_lines: Vec<String>) {
        let mut log = match Self::open_log_file(file_path) {
            Ok(log) => log,
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        };
        for text in text_lines {
            match writeln!(log.file, "{}", text) {
                Ok(_) => {}
                Err(e) => eprintln!("{}", e),
            };
        }
    }

    pub fn init_log_file(file_path: PathBuf, reciver: Receiver<String>) {
        let mut log = match Self::open_log_file(file_path) {
            Ok(log) => log,
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        };
        log.write_logs(reciver);
    }

    fn open_log_file(file_path: PathBuf) -> Result<Self, ErrorType> {
        if !file_path.exists() {
            {
                File::create(file_path.clone())?;
            }
        }
        let file = OpenOptions::new()
            .read(true) // Abre el archivo para lectura.
            .append(true) // Esto deberia hacer que la escritura sea al final del archivo sin rescribir nada de lo anterior
            .open(file_path)?;
        Ok(LogFile { file })
    }

    // fn write_logs(&mut self, reciver: Receiver<String>, write_permission: Arc<Mutex<bool>>) {
    fn write_logs(&mut self, reciver: Receiver<String>) {
        for text in reciver {
            // match write_permission.lock() {
            //     Ok(_) => {}
            //     Err(_) => return,
            // }
            match writeln!(self.file, "{}", text) {
                Ok(_) => {}
                Err(e) => eprintln!("{}", e),
            };
        }
    }
}

pub fn send_text_to_log_initial(
    command: String,
    args: Vec<String>,
    path_config: PathBuf,
) -> String {
    let text = match args.is_empty() {
        true => NOT_ARGS.to_string(),
        false => args.join(SEPARATOR_ARGS),
    };
    send_info("EXEC".to_string(), command, text, path_config)
}

pub fn send_text_to_log_finish(
    command: String,
    text: String,
    path_config: PathBuf,
    is_ok: bool,
) -> String {
    let finish = match is_ok {
        true => "OK".to_string(),
        false => "ERROR".to_string(),
    };
    send_info(finish, command, text, path_config)
}

fn send_info(result: String, command: String, text_extra: String, path_config: PathBuf) -> String {
    let timestamp = match get_current_date_time() {
        Ok(time) => time,
        Err(_) => "Error getting current time.".to_string(),
    };
    let user_data = match get_user_data(path_config) {
        Ok(data) => data,
        Err(_) => "No data user in config.".to_string(),
    };
    [result, timestamp, user_data, command, text_extra].join(SEPARATOR)
}

fn get_user_data(path_config: PathBuf) -> Result<String, ErrorType> {
    let config = RepoConfig::open(path_config)?;
    let user = match config.get_user() {
        None => {
            return Err(ErrorType::ConfigError(
                "Error not data user yet.".to_string(),
            ))
        }
        Some(user) => user,
    };
    Ok(format!(
        "Name={}{}Mail={}",
        user.get_name(),
        SEPARATOR_USER,
        user.get_mail()
    ))
}

pub fn send_info_from_server(
    type_of_message: String,
    text: String,
    user: String,
    arc_sender: &Arc<Mutex<Sender<String>>>,
) {
    let timestamp = match get_current_date_time() {
        Ok(time) => time,
        Err(_) => "Error getting current time.".to_string(),
    };
    let info = [type_of_message, timestamp, user, text].join(SEPARATOR);
    // envio de datos
    let sender = match arc_sender.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Error, send server log: {}", e);
            return;
        }
    };
    match sender.send(info) {
        Ok(_) => {}
        Err(e) => eprintln!("Error, send log (server): {}", e),
    };
}

pub fn send_info_from_client_http(
    is_info: bool,
    text: String,
    arc_sender: &Arc<Mutex<Sender<String>>>,
) {
    let type_of_message = if is_info {
        "INFO".to_string()
    } else {
        "ERROR".to_string()
    };
    send_info_from_server(type_of_message, text, "CLIENT".to_string(), arc_sender)
}
