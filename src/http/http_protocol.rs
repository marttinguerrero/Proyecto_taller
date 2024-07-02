use std::io::Read;
use std::sync::RwLock;
use std::{
    collections::{HashMap, HashSet},
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    path::{Path, PathBuf},
    sync::{mpsc::Sender, Arc, Mutex},
};

use serde_json::Value;

use crate::pull_request::PullRequest;
use crate::repository_access_permission::{
    get_permision_for_reposiory_from_repository_access_permission, RepositoryAccessPermission,
};
use crate::{git_errors::errors::ErrorType, server_paths::ServerPaths};
use crate::{
    http::{http_error::HTTPError::BadRequest, http_response::HTTPResponse},
    log_file::send_info_from_client_http,
};

pub struct HTTPRequest {
    // method: String,
    // uri: String,
    // version: String,
    // headers: HashMap<String, String>,
    // body: String,
}

impl HTTPRequest {
    pub fn handle_http_request(
        mut reader: BufReader<&mut TcpStream>,
        base_path: PathBuf,
        sender: Arc<Mutex<Sender<String>>>,
        method: String,
        repository_permission: Arc<RwLock<RepositoryAccessPermission>>,
    ) -> Result<(), ErrorType> {
        let mut http_request = read_full_request(&mut reader)?;
        http_request[0] = method.clone() + &http_request[0];

        let headers = match parse_headers(&http_request[1..]) {
            Ok(m) => m,
            Err(e) => {
                let result = Err(e);
                let stream = reader.get_mut();
                Self::write_response(&result, method.as_str(), stream, sender)?;
                result?;
                return Ok(());
            }
        };

        let request_line = http_request[0].split_whitespace().collect::<Vec<&str>>();
        let method = request_line[0];
        let uri = request_line[1];

        let uri_parts: Vec<&str> = uri.split('/').collect();

        let content_length = match headers.get("Content-Length") {
            Some(s) => match s.parse::<usize>() {
                Ok(n) => n,
                Err(_) => {
                    let result = Err(ErrorType::HTTPError(BadRequest(
                        "HTTP Request must include Content-Length header".to_string(),
                    )));
                    let stream = reader.get_mut();
                    Self::write_response(&result, method, stream, sender)?;
                    result?;
                    return Ok(());
                }
            },
            None => 0,
        };

        let body = match content_length {
            0 => None,
            _ => match get_http_request_body(&mut reader, content_length) {
                Ok(s) => Some(s),
                Err(e) => {
                    let result = Err(e);
                    let stream = reader.get_mut();
                    Self::write_response(&result, method, stream, sender)?;
                    result?;
                    return Ok(());
                }
            },
        };

        send_info_from_client_http(
            true,
            format!(
                "HTTP Command. Method: {}, URI: {}, body: {}",
                method,
                uri,
                body.clone().unwrap_or("".to_string())
            ),
            &sender,
        );

        let params = verify_request_validity(body, method, uri, &uri_parts);

        let result = match params {
            Err(e) => Err(e),
            Ok(body) => {
                // permiso del repo
                let repo_name = uri_parts[2];
                let repo_path = base_path.join(repo_name);
                let lock_repository =
                    get_permision_for_reposiory_from_repository_access_permission(
                        &repository_permission,
                        repo_path.clone(),
                    )?;
                //

                match method {
                    "GET" => {
                        // bloqueo de lectura repositorio
                        let _read_guard = match lock_repository.read() {
                            Ok(guard) => guard,
                            Err(_) => {
                                return Err(ErrorType::ConfigError(
                                    "Error in lock read repository (http GET).".to_string(),
                                ))
                            }
                        };
                        Self::get_http(repo_path, uri_parts[3..].to_vec())
                    }
                    "POST" => {
                        // bloqueo de escrtitura repositorio
                        let _write_guard = match lock_repository.write() {
                            Ok(guard) => guard,
                            Err(_) => {
                                return Err(ErrorType::ConfigError(
                                    "Error in lock write repository (http POST).".to_string(),
                                ))
                            }
                        };
                        Self::post_http(repo_path, &body)
                    }
                    "PUT" => {
                        // bloqueo de escrtitura repositorio
                        let _write_guard = match lock_repository.write() {
                            Ok(guard) => guard,
                            Err(_) => {
                                return Err(ErrorType::ConfigError(
                                    "Error in lock write repository (http PUT).".to_string(),
                                ))
                            }
                        };
                        Self::put_http(repo_path, uri_parts[3..].to_vec(), body)
                    }
                    "PATCH" => {
                        // bloqueo de escrtitura repositorio
                        let _write_guard = match lock_repository.write() {
                            Ok(guard) => guard,
                            Err(_) => {
                                return Err(ErrorType::ConfigError(
                                    "Error in lock write repository (http PATCH).".to_string(),
                                ))
                            }
                        };
                        Self::patch_http(repo_path, uri_parts[3..].to_vec(), body)
                    }
                    // "DELETE" => {
                    //     // Handle "DELETE" request
                    //     // "Command DELETE".to_string()
                    // }
                    _ => Err(ErrorType::HTTPError(BadRequest(method.to_string()))),
                }
            }
        };

        let stream = reader.get_mut();
        Self::write_response(&result, method, stream, sender)?;

        result?;
        Ok(())
    }

    fn write_response(
        result: &Result<String, ErrorType>,
        method: &str,
        stream: &mut TcpStream,
        sender: Arc<Mutex<Sender<String>>>,
    ) -> Result<(), ErrorType> {
        let response = HTTPResponse::new(result, method);
        stream.write_all(response.as_bytes().as_slice())?;

        let (is_info, text) = match &result {
            Ok(_) => (
                true,
                format!(
                    "HTTP Request completed succesfully with status: {}",
                    response.get_status()
                ),
            ),
            Err(e) => (
                false,
                format!(
                    "HTTP Request failed to complete due to: {}. Status: {}",
                    e,
                    response.get_status()
                ),
            ),
        };
        send_info_from_client_http(is_info, text, &sender);
        Ok(())
    }

    fn get_http(repo_path: PathBuf, uri_parts: Vec<&str>) -> Result<String, ErrorType> {
        // given the URI was /repos/{repo}/pulls... then uri parts will be ["pulls", ...]
        match uri_parts.len() {
            1 => {
                // GET /repos/{repo}/pulls
                // obtener todos los PR
                Self::get_all_prs_http(&repo_path, uri_parts)
            }
            2 => {
                // GET /repos/{repo}/pulls/{pull_number}
                // get one PR by id
                let pull_number = uri_parts[1];
                let pr = PullRequest::get_pull_request(&repo_path, pull_number)?;
                Ok(pr.to_json_string()?)
            }
            3 => {
                // GET /repos/{repo}/pulls/{pull_number}/commits
                // get al commits of the PR since it was created
                if uri_parts[2] != "commits" {
                    return Err(ErrorType::HTTPError(BadRequest(
                        "Invalid URI request".to_string(),
                    )));
                }
                let pull_number = uri_parts[1];
                let list = PullRequest::list_commits_pull_request(repo_path, pull_number)?;

                match serde_json::to_string(&list) {
                    Ok(s) => Ok(s),
                    Err(_) => Err(ErrorType::RepositoryError(
                        "Error serializing commit list into JSON array".to_string(),
                    )),
                }
            }
            _ => Err(ErrorType::HTTPError(BadRequest(
                "Invalid URI request".to_string(),
            ))),
        }
    }

    fn get_all_prs_http(repo_path: &Path, uri_parts: Vec<&str>) -> Result<String, ErrorType> {
        let mut full_list = PullRequest::list_pull_requests(repo_path)?;
        let mut map: HashSet<PullRequest> = full_list.clone().into_iter().collect();

        let query_params = parse_query(uri_parts[0])?;
        if !query_params.is_empty() {
            for (query, field) in query_params.into_iter() {
                match query.as_str() {
                    "state" => match field.as_str() {
                        "open" => {
                            for pr in &full_list {
                                if !pr.is_open() {
                                    map.remove(pr);
                                }
                            }
                        }
                        "closed" => {
                            for pr in &full_list {
                                if pr.is_open() {
                                    map.remove(pr);
                                }
                            }
                        }
                        "all" => continue,
                        _ => {
                            return Err(ErrorType::HTTPError(BadRequest(format!(
                                "Invalid state query: {}",
                                field
                            ))))
                        }
                    },
                    "base" => {
                        for pr in &full_list {
                            if pr.get_base() != field {
                                map.remove(pr);
                            }
                        }
                    }
                    _ => {
                        return Err(ErrorType::HTTPError(BadRequest(format!(
                            "Invalid query {query}"
                        ))))
                    }
                }
                full_list = map.clone().into_iter().collect();
            }
        }
        let json_array = PullRequest::to_json_array(&full_list)?;
        Ok(json_array)
    }

    fn post_http(repo_path: PathBuf, body: &str) -> Result<String, ErrorType> {
        // POST /repos/{repo}/pulls
        // create PR
        let (base_name, target_name, title) = Self::parse_json_create_body(body)?;
        let pr = PullRequest::create_pull_request(repo_path, base_name, target_name, title)?;

        pr.to_json_string()
    }

    fn parse_json_create_body(
        json_body: &str,
    ) -> Result<(String, String, Option<String>), ErrorType> {
        let values: Value = match serde_json::from_str(json_body) {
            Ok(b) => b,
            Err(_) => {
                return Err(ErrorType::RepositoryError(
                    "Error deserializing http request body".to_string(),
                ))
            }
        };

        let base_name = match values["base"].as_str() {
            Some(b) => b,
            None => {
                return Err(ErrorType::RepositoryError(
                    "Error reading branch_base from http request body".to_string(),
                ))
            }
        };
        let target_name = match values["head"].as_str() {
            Some(b) => b,
            None => {
                return Err(ErrorType::RepositoryError(
                    "Error reading branch_target from http request body".to_string(),
                ))
            }
        };

        let title = values["title"].as_str().map(|x| x.to_string());

        Ok((base_name.to_string(), target_name.to_string(), title))
    }

    fn put_http(
        repo_path: PathBuf,
        uri_parts: Vec<&str>,
        body: String,
    ) -> Result<String, ErrorType> {
        // PUT /repos/{repo}/pulls/{pull_number}/merge

        // given the URI was /repos/{repo}/pulls/{pull_number}/merge then uri parts will be ["pulls", "{pull_number}", "merge"]
        if uri_parts.len() != 3 || uri_parts[0] != "pulls" || uri_parts[2] != "merge" {
            return Err(ErrorType::HTTPError(BadRequest(
                "URI for PUT must be /repos/{repo}/pulls/{pull_number}/merge".to_string(),
            )));
        }
        let pull_number = uri_parts[1];

        // sacar el parseo de json body
        let message = PullRequest::merge_pull_request(repo_path, pull_number, &body)?;
        let json_message = format!("{{\"merged\": true, \"message\": \"{}\"}}", message);

        println!("Http method PUT: {}", message);
        Ok(json_message)
    }

    fn patch_http(
        server_repo_path: PathBuf,
        uri_parts: Vec<&str>,
        body: String,
    ) -> Result<String, ErrorType> {
        // PATCH /repos/{repo}/pulls/{pull_number}
        // given the URI was /repos/{repo}/pulls/{pull_number} then uri parts will be ["pulls", "{pull_number}"]

        if uri_parts.len() != 2 {
            return Err(ErrorType::HTTPError(BadRequest(
                "URI for PATCH must be /repos/{repo}/pulls/{pull_number}".to_string(),
            )));
        }

        let number = uri_parts[1];
        let mut pr = PullRequest::get_pull_request(&server_repo_path, number)?;

        let patches: Value = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(_) => {
                return Err(ErrorType::RepositoryError(
                    "Error deserializing http request body".to_string(),
                ))
            }
        };

        let server_paths = ServerPaths::new(&server_repo_path);
        let repo_paths = server_paths.get_repo_paths()?;

        if let Some(new_title) = patches["title"].as_str() {
            pr.set_title(new_title);
        };
        if let Some(new_state) = patches["state"].as_str() {
            pr.set_state(new_state)?;
        };
        if let Some(new_base) = patches["base"].as_str() {
            pr.set_base(new_base, &repo_paths.get_refs_heads())?;
        }
        pr.save(&server_paths.get_pull_requests_path())?;

        pr.to_json_string()
    }
}

fn parse_headers(http_request: &[String]) -> Result<HashMap<String, String>, ErrorType> {
    let mut headers = HashMap::new();
    for line in http_request {
        if let Some((field, value)) = line.split_once(": ") {
            headers.insert(field.to_string(), value.to_string());
        } else {
            return Err(ErrorType::HTTPError(BadRequest(format!(
                "Invalid http request header: {}",
                line
            ))));
        }
    }
    Ok(headers)
}

fn parse_query(uri_end: &str) -> Result<HashMap<String, String>, ErrorType> {
    let mut map = HashMap::new();
    if let Some((_, queries)) = uri_end.split_once('?') {
        let queries: Vec<&str> = queries.split('&').collect();
        for query in queries {
            if let Some((field, value)) = query.split_once('=') {
                map.insert(field.to_string(), value.to_string());
            } else {
                return Err(ErrorType::HTTPError(BadRequest(format!(
                    "Invalid query format: {}",
                    query
                ))));
            }
        }
    }
    Ok(map)
}

fn verify_request_validity(
    body: Option<String>,
    method: &str,
    uri: &str,
    uri_parts: &[&str],
) -> Result<String, ErrorType> {
    // if !uri.starts_with("/repos/") ||  uri_parts.len() < 3 {
    if !uri.starts_with("/repos/") || !uri_parts[3].contains("pulls") || uri_parts.len() < 3 {
        return Err(ErrorType::HTTPError(BadRequest(
            "Invalid URI request".to_string(),
        )));
    }

    match body {
        Some(b) => Ok(b),
        None => {
            if method == "POST" || method == "PUT" || method == "PATCH" {
                return Err(ErrorType::HTTPError(BadRequest(
                    "POST, PUT and PATCH must include a body".to_string(),
                )));
            }

            Ok("".to_string())
        }
    }
}

// fn check_repo_exists(repo_path: &Path, repo_name: &str) -> Result<(), ErrorType> {
//     let path = repo_path.join(".git-rustico");
//     match path.exists() {
//         true => Ok(()),
//         false => Err(ErrorType::HTTPError(
//             super::http_error::HTTPError::NotFound(format!("Repository {} not found", repo_name)),
//         )),
//     }
// }

fn get_http_request_body(
    reader: &mut BufReader<&mut TcpStream>,
    content_len: usize,
) -> Result<String, ErrorType> {
    let mut buffer = vec![0; content_len];
    reader.read_exact(&mut buffer)?;
    match String::from_utf8(buffer) {
        Ok(s) => Ok(s),
        Err(_) => Err(ErrorType::HTTPError(BadRequest(
            "Invalid HTTP request body".to_string(),
        ))),
    }
    // let body = http_request
    //     .iter()
    //     .skip_while(|line| !line.is_empty())
    //     .skip(1)
    //     .cloned()
    //     .collect::<Vec<String>>()
    //     .join("\n");
    // if body.is_empty() {
    //     None
    // } else {
    //     Some(body)
    // }
}

fn read_full_request(reader: &mut BufReader<&mut TcpStream>) -> Result<Vec<String>, ErrorType> {
    // let mut content = Vec::new();
    // reader.read_to_end(&mut content)?;

    // let lines = String::from_utf8_lossy(&content)
    //     .split("\r\n")
    //     .map(|s| s.to_string())
    //     .collect::<Vec<String>>();

    // Ok(lines)
    let mut http_request = Vec::new();
    for line_result in reader.lines() {
        match line_result {
            Ok(line) => {
                if line.is_empty() {
                    break;
                }
                http_request.push(line);
            }
            Err(_) => {
                return Err(ErrorType::ProtocolError(
                    "Invalid http request line".to_string(),
                ));
            }
        }
    }
    Ok(http_request)
}
