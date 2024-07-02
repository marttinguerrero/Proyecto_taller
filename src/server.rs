use git_rustico::branch::Branch;
use git_rustico::git_errors::errors::ErrorType;
use git_rustico::git_object::GitObject;
use git_rustico::hash::GitHash;
use git_rustico::http::http_protocol::HTTPRequest;
use git_rustico::init::git_init;
use git_rustico::log_file::{send_info_from_server, LogFile};
use git_rustico::network_commands::get_packfile_objects;
use git_rustico::protocol::pack_file::{build_packfile, read_packfile};
use git_rustico::protocol::pkt_line::{self, create_pkt_line, read_pkt_line};
use git_rustico::refs::BranchRef;
use git_rustico::repo_paths::RepoPaths;
use git_rustico::repository_access_permission::{
    get_permision_for_reposiory_from_repository_access_permission, RepositoryAccessPermission,
};
use git_rustico::server_paths::ServerPaths;
use std::collections::HashSet;
use std::fs;
// use std::fs;
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{mpsc, RwLock};
use std::sync::{Arc, Mutex};
use std::{
    io,
    net::{TcpListener, TcpStream},
    thread,
};

const ADDRESS_IP: &str = "127.0.0.1";
const DEFAULT_ADDRESS_PORT: &str = "9418";
const COMMAND_UPLOAD: &str = "git-upload-pack"; // fetch y clone
const COMMAND_RECEIVE: &str = "git-receive-pack"; // PUSH

const TYPE_MESSAGE_CONSOLE: &str = "CONSOLE";
const TYPE_MESSAGE_CONSOLE_ERROR: &str = "CONSOLE_ERR";
const TYPE_MESSAGE_ERROR: &str = "ERROR";
const TYPE_MESSAGE_INFO: &str = "INFO";
const USER_CLIENT: &str = "CLIENT";
const USER_BASE: &str = "SERVER";
const USER_LISTENER: &str = "LISTENER";

fn main() {
    use std::env;
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("ERROR: Server format is 'git-rustico-server <base path>' or 'git-rustico-server create-empty-repo <base_path> <repo name>'");
        return;
    }

    if args[1] == "create-empty-repo" {
        if args.len() != 4 {
            eprint!("You must include a repo name for this command");
        }

        match create_empty_repo(PathBuf::from(&args[2]), args[3].clone()) {
            Ok(_) => println!("Empty repo named {} created", args[3]),
            Err(_) => eprintln!("Failed to create an empty repo"),
        }
        return;
    }
    let base_path = PathBuf::from(&args[1]);
    let log_file = base_path.join(".LOG_SERVER");

    let (sender, receiver) = mpsc::channel::<String>();
    let thread_log_file = thread::spawn(move || {
        LogFile::init_log_file(log_file, receiver);
    });
    let sender_permission = Arc::new(Mutex::new(sender));
    let sender_permission_to_clients = sender_permission.clone();

    // hilo de aceptacion de clientes
    let accept_new_clients = Arc::new(Mutex::new(true));
    let accept_new_clients_for_listener = accept_new_clients.clone();
    let thread = thread::spawn(move || {
        server_listener(
            accept_new_clients_for_listener,
            sender_permission_to_clients,
            base_path,
        );
    });

    // comandos para servidor
    loop {
        let command = match input_by_console() {
            Ok(input) => input.to_lowercase(),
            Err(_) => {
                let error_message = "Error in read input command".to_string();
                println!("{}", error_message.clone());
                send_info_from_server(
                    TYPE_MESSAGE_CONSOLE_ERROR.to_string(),
                    error_message,
                    USER_BASE.to_string(),
                    &sender_permission,
                );
                break;
            }
        };
        send_info_from_server(
            TYPE_MESSAGE_CONSOLE.to_string(),
            format!("Receive command {}.", command),
            USER_BASE.to_string(),
            &sender_permission,
        );
        if command == *"quit" || command == *"q" {
            let mut new_clients = match accept_new_clients.lock() {
                Ok(new_clients) => new_clients,
                Err(_) => {
                    println!("Error in accept new clients");
                    break;
                }
            };
            *new_clients = false;
            break;
        }
    }
    let wait_message = "Wait last client.".to_string();
    send_info_from_server(
        TYPE_MESSAGE_INFO.to_string(),
        wait_message.clone(),
        USER_BASE.to_string(),
        &sender_permission,
    );
    println!("{}", wait_message);
    if let Err(e) = send_finish_conecction() {
        send_info_from_server(
            TYPE_MESSAGE_ERROR.to_string(),
            format!("Sending conecction to close listener failed. {}.", e),
            USER_BASE.to_string(),
            &sender_permission,
        );
    }
    let _ = thread.join();
    let finish_message = "Finish.".to_string();
    send_info_from_server(
        TYPE_MESSAGE_INFO.to_string(),
        finish_message.clone(),
        USER_BASE.to_string(),
        &sender_permission,
    );
    drop(sender_permission);
    let _ = thread_log_file.join();
    println!("{}", finish_message);
}

fn create_empty_repo(server_path: PathBuf, arg: String) -> Result<(), ErrorType> {
    let repo_path = server_path.join(arg);

    fs::create_dir(&repo_path)?;

    let pull_requests_path = repo_path.join("pull_requests");
    fs::create_dir(pull_requests_path)?;

    let repo_folder_path = repo_path.join("repo");
    fs::create_dir(&repo_folder_path)?;

    let repo_paths = RepoPaths::new(repo_folder_path)?;
    git_init(repo_paths)?;

    Ok(())
}

fn input_by_console() -> Result<String, String> {
    let mut buffer = String::new();
    match io::stdin().read_line(&mut buffer) {
        Ok(_) => {}
        Err(_) => return Err(String::from("Console input failure.\n")),
    };
    Ok(buffer.trim().to_string())
}

/// Listen loop to accept new TCP connections
fn server_listener(
    accept_new_clients: Arc<Mutex<bool>>,
    sender: Arc<Mutex<Sender<String>>>,
    base_path: PathBuf,
) {
    let config_path = base_path.join(".config");
    let port = match read_config_file(config_path) {
        Ok(Some(port)) => port,
        Ok(None) => DEFAULT_ADDRESS_PORT.to_string(),
        Err(_) => {
            println!("Error reading config file");
            return;
        }
    };

    let address = format!("{}:{}", ADDRESS_IP, port);
    let listener = match TcpListener::bind(address) {
        Ok(listener) => listener,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    println!("Ready to rustble");

    send_info_from_server(
        TYPE_MESSAGE_INFO.to_string(),
        format!(
            "Listening initiated on: {}:{}.",
            ADDRESS_IP, DEFAULT_ADDRESS_PORT
        ),
        USER_LISTENER.to_string(),
        &sender,
    );

    // bloqueador de repositorios esto debe evitar que dos clientes accedan al mismo repo a la vez
    let repository_permission: Arc<RwLock<RepositoryAccessPermission>> = Arc::new(RwLock::new(
        RepositoryAccessPermission::init_repository_access_permission(),
    ));
    //

    let mut threads = Vec::new();
    for stream in listener.incoming() {
        let new_clients = match accept_new_clients.lock() {
            Ok(new_clients) => *new_clients,
            Err(_) => {
                println!("Error reading new users");
                break;
            }
        };
        if !new_clients {
            break;
        }
        let mut stream = match stream {
            Ok(tcp_stream) => tcp_stream,
            Err(err) => {
                println!("{}", err);
                send_info_from_server(
                    TYPE_MESSAGE_ERROR.to_string(),
                    format!("{}", err),
                    USER_LISTENER.to_string(),
                    &sender,
                );
                return;
            }
        };
        let new_conection_message = "Connection established!".to_string();
        println!("{}", new_conection_message.clone());
        let base_path_clone = base_path.clone();
        send_info_from_server(
            TYPE_MESSAGE_INFO.to_string(),
            new_conection_message,
            USER_LISTENER.to_string(),
            &sender,
        );
        let sender_to_client = sender.clone();
        let repository_permission_to_client = repository_permission.clone();
        let thread = thread::spawn(move || {
            let sender_to_handle_client = sender_to_client.clone();
            match handle_connection(
                &mut stream,
                base_path_clone,
                sender_to_handle_client,
                repository_permission_to_client,
            ) {
                Ok(_) => {
                    send_info_from_server(
                        TYPE_MESSAGE_INFO.to_string(),
                        "Connection completed with successful execution of the command".to_string(),
                        USER_CLIENT.to_string(),
                        &sender_to_client,
                    );
                }
                Err(err) => {
                    send_info_from_server(
                        TYPE_MESSAGE_ERROR.to_string(),
                        format!("{}", err),
                        USER_CLIENT.to_string(),
                        &sender_to_client,
                    );
                    println!("{}", err)
                }
            };
        });
        threads.push(thread);
    }
    let waiting_message = "Waiting for completion of communications from clients".to_string();
    println!("{}", waiting_message);
    send_info_from_server(
        TYPE_MESSAGE_INFO.to_string(),
        waiting_message,
        USER_LISTENER.to_string(),
        &sender,
    );
    for thread in threads {
        let _ = thread.join();
    }
    let end_message = "End of acceptance of new clients".to_string();
    println!("{}", end_message);
    send_info_from_server(
        TYPE_MESSAGE_INFO.to_string(),
        end_message,
        USER_LISTENER.to_string(),
        &sender,
    );
}

fn read_config_file(config_path: PathBuf) -> Result<Option<String>, ErrorType> {
    if !config_path.exists() {
        return Ok(None);
    }
    Ok(Some(fs::read_to_string(config_path)?))
}

/// Individual connection with client
fn handle_connection(
    stream: &mut TcpStream,
    base_path: PathBuf,
    sender: Arc<Mutex<Sender<String>>>,
    repository_permission: Arc<RwLock<RepositoryAccessPermission>>,
) -> Result<(), ErrorType> {
    println!("Connection from {}", stream.peer_addr()?);

    let mut reader = BufReader::new(stream);
    let mut buffer = [0; 4];
    reader.read_exact(&mut buffer)?;

    let header = String::from_utf8_lossy(&buffer).to_string();

    if header == "PUT " || header == "GET " || header == "POST" || header == "PATC" {
        // Handle HTTP request
        println!("Handling HTTP request");
        HTTPRequest::handle_http_request(reader, base_path, sender, header, repository_permission)?;
    } else {
        // Handle Git pkt-line request
        println!("Handling Git-Rustico Client request");
        handle_git_request(reader, base_path, sender, buffer, repository_permission)?;
    }
    Ok(())
}

fn handle_git_request(
    mut reader: BufReader<&mut TcpStream>,
    base_path: PathBuf,
    sender: Arc<Mutex<Sender<String>>>,
    buffer: [u8; 4],
    repository_permission: Arc<RwLock<RepositoryAccessPermission>>,
) -> Result<(), ErrorType> {
    let line_size = pkt_line::pkt_line_size(buffer)?;
    if line_size == 0 {
        // Flush packet
        return Err(ErrorType::ProtocolError(
            "expected a valid request pkt-line, got flush line".to_string(),
        ));
    }

    let line_content = pkt_line::pkt_line_content(line_size, &mut reader)?;

    let mut split_values = line_content.split_ascii_whitespace().map(String::from);
    let command = split_values
        .next()
        .ok_or(ErrorType::ProtocolError("invalid request".to_string()))?;

    let extra_data: Vec<String> = split_values
        .next()
        .ok_or(ErrorType::ProtocolError("invalid request".to_string()))?
        .split('\0')
        .map(String::from)
        .collect();

    if extra_data.len() < 2 {
        return Err(ErrorType::ProtocolError("invalid request".to_string()));
    }

    let host = extra_data[1].clone();
    // println!("{}", host);

    let repo = extra_data[0].clone();
    let repo = repo.strip_prefix('/').unwrap_or(&repo);
    let server_repo_path = base_path.join(repo);
    let server_paths = ServerPaths::new(&server_repo_path);

    if !server_paths.get_repo_path().join(".git-rustico").exists() {
        return Err(ErrorType::ProtocolError(format!(
            "invalid request: repository '{}' not found in base path",
            repo
        )));
    }
    let repo_paths = server_paths.get_repo_paths()?;

    // acceso de lectura al gestor de permisos
    let lock_repository = get_permision_for_reposiory_from_repository_access_permission(
        &repository_permission,
        server_repo_path.clone(),
    )?;
    // fin de persisos

    let message = format!("Command {} from {} for repository {}.", command, host, repo);
    send_info_from_server(
        TYPE_MESSAGE_INFO.to_string(),
        message,
        USER_CLIENT.to_string(),
        &sender,
    );

    match command.as_str() {
        COMMAND_UPLOAD => {
            println!("Request upload-pack for '{}'", repo);
            // bloqueo de lectura repositorio
            let _read_guard = match lock_repository.read() {
                Ok(guard) => guard,
                Err(_) => {
                    return Err(ErrorType::ConfigError(
                        "Error in lock read repository.".to_string(),
                    ))
                }
            };
            upload_pack(reader.get_mut(), repo_paths)
        }
        COMMAND_RECEIVE => {
            println!("Request receive-pack for '{}'", repo);
            // bloqueo de escrtitura repositorio
            let _write_guard = match lock_repository.write() {
                Ok(guard) => guard,
                Err(_) => {
                    return Err(ErrorType::ConfigError(
                        "Error in lock write repository.".to_string(),
                    ))
                }
            };
            receive_pack(reader.get_mut(), repo_paths)
        }
        _ => Err(ErrorType::ProtocolError(format!(
            "invalid request command: {}",
            command
        ))),
    }
}

fn upload_pack(stream: &mut TcpStream, repo_paths: RepoPaths) -> Result<(), ErrorType> {
    send_server_refs(stream, &repo_paths)?;

    let mut commits_to_update = Vec::new();

    let mut want_line_content = match read_pkt_line(stream)? {
        Some(c) => c,
        None => return Ok(()),
    };

    loop {
        let parts: Vec<&str> = want_line_content.split(' ').collect();

        let hash = GitHash::new(parts[1].trim())?;
        commits_to_update.push(hash);

        want_line_content = match read_pkt_line(stream)? {
            Some(c) => c,
            None => break,
        }
    }

    // read done
    let done_line = read_pkt_line(stream)?.ok_or(ErrorType::ProtocolError(
        "expected a done line, got a flush line".to_string(),
    ))?;
    if done_line != "done" {
        return Err(ErrorType::ProtocolError(format!(
            "expected a done line, got {done_line}"
        )));
    }

    let packfile_objects =
        get_packfile_objects(commits_to_update, HashSet::new(), &repo_paths.get_objects())?;
    println!("Enumerating objects: {}", packfile_objects.len());
    let packfile = build_packfile(packfile_objects)?;

    stream.write_all(create_pkt_line("NAK")?.as_bytes())?;
    stream.write_all(&packfile)?;

    Ok(())
}

// TODO : chequear "diverging paths"
fn receive_pack(stream: &mut TcpStream, repo_paths: RepoPaths) -> Result<(), ErrorType> {
    send_server_refs(stream, &repo_paths)?;

    let commands = read_commands(stream)?;

    let mut head_command: Option<(GitHash, GitHash, String)> = None;

    //todo : usarlo para verificar que lleguen todos los objetos necesarios
    let _commits_to_update = commits_to_update(&commands, &repo_paths, &mut head_command)?;

    let mut head = BranchRef::open(repo_paths.get_head(), &repo_paths.get_refs_heads())?;

    let server_refs = Branch::list_branches(&repo_paths.get_refs_heads())?;

    if let Some(ref head_command) = head_command {
        if let Some(branch_name) = head.get_branch_name() {
            if let Some(hash) = server_refs.get(&branch_name) {
                if hash != &head_command.1 {
                    eprintln!("unable to update HEAD because new ref doesnt match any branch tip");
                } else {
                    let branch = Branch::open(&repo_paths.get_refs_heads(), &branch_name)?;
                    head.set_branch(branch);
                    head.save()?;
                }
            }
        }
    }

    let mut reader = BufReader::new(stream);
    let packfile_objects = read_packfile(&mut reader)?;

    for (object_type, content) in packfile_objects {
        // todo : verify valid objects are being received and that all commands are satisfied
        // match object_type{
        //     ObjectType::Commit => {
        //         let hash = GitHash::hash_commit(&content);
        //         let commit = Commit::from_object(&hash, content, &repo_paths.get_objects())?;
        //     },
        //     ObjectType::Blob => todo!(),
        //     ObjectType::Tree => todo!(),
        // }
        GitObject::save_object(content, object_type, &repo_paths.get_objects())?;
    }

    reader.get_mut().shutdown(std::net::Shutdown::Both)?;

    Ok(())
}

fn commits_to_update(
    commands: &[(GitHash, GitHash, String)],
    repo_paths: &RepoPaths,
    head_command: &mut Option<(GitHash, GitHash, String)>,
) -> Result<Vec<GitHash>, ErrorType> {
    let mut commits_to_update = Vec::new();
    for command in commands {
        if command.2 == "HEAD" {
            *head_command = Some(command.clone());
            continue;
        }
        if command.0.to_string() == "0000000000000000000000000000000000000000" {
            commits_to_update.push(command.1.clone());
            Branch::new(&command.2, &repo_paths.get_refs_heads(), command.1.clone())?;
        } else if command.1.to_string() == "0000000000000000000000000000000000000000" {
            commits_to_update.push(command.1.clone());
            // Branch::delete_branch(
            //     &command.2,
            //     repo_paths.get_refs_heads(),
            //     repo_paths.get_head(),
            // )?;
        } else {
            let mut branch = Branch::open(&repo_paths.get_refs_heads(), &command.2)?;
            branch.set_last_commit_hash(command.1.clone());
            branch.save()?;
        }
    }
    Ok(commits_to_update)
}

fn read_commands(stream: &mut TcpStream) -> Result<Vec<(GitHash, GitHash, String)>, ErrorType> {
    let mut commands = Vec::new();

    while let Some(line) = read_pkt_line(stream)? {
        // todo esto esta repetido en read server refs o algo asi
        let parts: Vec<&str> = line.trim().split(' ').collect();
        if parts.len() != 3 {
            return Err(ErrorType::ProtocolError(format!(
                "invalid reference line: {line}"
            )));
        }
        let previous_hash = GitHash::new(parts[0])?;
        let new_hash = GitHash::new(parts[1])?;

        let mut ref_name = match parts[2].split_once('\0') {
            Some((n, _)) => n,
            None => parts[2],
        };

        let ref_parts: Vec<&str> = ref_name.trim().split('/').collect();

        if ref_parts.len() == 1 && ref_parts[0] == "HEAD" {
            ref_name = "HEAD";
        } else if ref_parts.len() != 3 || ref_parts.first() != Some(&"refs") {
            // tags are unsupported yet
            return Err(ErrorType::ProtocolError(format!(
                "invalid reference: {ref_name}"
            )));
        } else {
            ref_name = ref_parts[2];
        }
        commands.push((previous_hash, new_hash, ref_name.to_string()));
    }
    Ok(commands)
}

pub fn send_server_refs(stream: &mut TcpStream, repo_paths: &RepoPaths) -> Result<(), ErrorType> {
    let server_head = BranchRef::open(repo_paths.get_head(), &repo_paths.get_refs_heads())?;

    let server_refs = Branch::list_branches(&repo_paths.get_refs_heads())?;

    let mut first = true;
    if let Some(hash) = server_head.get_last_commit_hash() {
        let line = format!("{hash} HEAD\0");
        let pkt_line = create_pkt_line(&line)?;
        stream.write_all(pkt_line.as_bytes())?;
        first = false;
    }

    for (ref_name, hash) in server_refs {
        let line = match first {
            true => {
                first = false;
                format!("{hash} refs/heads/{ref_name}\0")
            }
            false => format!("{hash} refs/heads/{ref_name}"),
        };
        let pkt_line = create_pkt_line(&line)?;
        stream.write_all(pkt_line.as_bytes())?;
    }

    stream.write_all(b"0000")?;
    Ok(())
}

fn send_finish_conecction() -> Result<(), ErrorType> {
    let address = format!("{}:{}", ADDRESS_IP, DEFAULT_ADDRESS_PORT);
    let _ = TcpStream::connect(address)?;
    Ok(())
}
