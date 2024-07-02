use core::fmt;
use std::fmt::Display;

#[derive(Debug)]
pub enum HTTPError {
    BadRequest(String),
    // Unauthorized,
    // Forbidden,
    NotFound(String),
    MethodNotAllowed(String),
    // InternalServerError,
}

impl Display for HTTPError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HTTPError::BadRequest(s) => write!(f, "400 Bad Request: {}", s),
            // HTTPError::Unauthorized => write!(f, "401 Unauthorized"),
            // HTTPError::Forbidden => write!(f, "403 Forbidden"),
            HTTPError::NotFound(s) => write!(f, "404 Not Found: {}", s),
            HTTPError::MethodNotAllowed(s) => write!(f, "405 Method Not Allowed: {}", s),
            // HTTPError::InternalServerError => write!(f, "500 Internal Server Error: {}",),
        }
    }
}
