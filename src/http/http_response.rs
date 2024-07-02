use std::collections::HashMap;

use crate::git_errors::errors::ErrorType;

use super::http_error::HTTPError;

enum HTTPStatus {
    OK,
    Created,
    // NoContent,
    BadRequest,
    // Unauthorized,
    // Forbidden,
    NotFound,
    MethodNotAllowed,
    InternalServerError,
}

// impl Display for HTTPStatus{
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             HTTPStatus::OK => write!(f, "200 OK"),
//             HTTPStatus::Created => "201 Created".to_string(),
//             // HTTPStatus::NoContent => "204 No Content".to_string(),
//             HTTPStatus::BadRequest => "400 Bad Request".to_string(),
//             // HTTPStatus::Unauthorized => "401 Unauthorized".to_string(),
//             // HTTPStatus::Forbidden => "403 Forbidden".to_string(),
//             HTTPStatus::NotFound => "404 Not Found".to_string(),
//             HTTPStatus::MethodNotAllowed => "405 Method Not Allowed".to_string(),
//             HTTPStatus::InternalServerError => "500 Internal Server Error".to_string(),
//         }
//     }
// }
impl HTTPStatus {
    fn http_to_string(&self) -> String {
        match self {
            HTTPStatus::OK => "200 OK".to_string(),
            HTTPStatus::Created => "201 Created".to_string(),
            // HTTPStatus::NoContent => "204 No Content".to_string(),
            HTTPStatus::BadRequest => "400 Bad Request".to_string(),
            // HTTPStatus::Unauthorized => "401 Unauthorized".to_string(),
            // HTTPStatus::Forbidden => "403 Forbidden".to_string(),
            HTTPStatus::NotFound => "404 Not Found".to_string(),
            HTTPStatus::MethodNotAllowed => "405 Method Not Allowed".to_string(),
            HTTPStatus::InternalServerError => "500 Internal Server Error".to_string(),
        }
    }
}

pub struct HTTPResponse {
    status: HTTPStatus,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl HTTPResponse {
    pub(crate) fn new(result: &Result<String, ErrorType>, method: &str) -> HTTPResponse {
        match result {
            Ok(body) => {
                let status = match method {
                    "POST" => HTTPStatus::Created,
                    _ => HTTPStatus::OK,
                };
                let mut headers = HashMap::new();
                headers.insert("Content-Type:".to_string(), "application/json".to_string());
                HTTPResponse {
                    status,
                    headers,
                    body: body.to_string(),
                }
            }
            Err(err) => {
                let status = match err {
                    ErrorType::HTTPError(ref e) => match e {
                        HTTPError::NotFound(_) => HTTPStatus::NotFound,
                        HTTPError::MethodNotAllowed(_) => HTTPStatus::MethodNotAllowed,
                        HTTPError::BadRequest(_) => HTTPStatus::BadRequest,
                    },
                    _ => HTTPStatus::InternalServerError,
                };
                let mut headers = HashMap::new();
                headers.insert("Content-Type:".to_string(), "text/plain".to_string());
                HTTPResponse {
                    status,
                    headers,
                    body: err.to_string(),
                }
            }
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let mut response = format!("HTTP/1.1 {}\r\n", self.status.http_to_string());
        for (key, value) in &self.headers {
            response.push_str(&format!("{} {}\r\n", key, value));
        }
        response.push_str("\r\n");
        response.push_str(&self.body);
        response.into_bytes()
    }

    pub fn get_status(&self) -> String {
        self.status.http_to_string()
    }
}
