use core::fmt;
use std::io;
use std::num::ParseIntError;

use crate::git_errors::command_error::CommandError;
use crate::git_errors::parse_error::ParseError;
use crate::http::http_error::HTTPError;

// esto esta horrible. posible refactor:
// - CommandError para errores de entrada del usuario
// - Format Error para errores en alguno de los archivos de .git-rustico
// - RepositoryError para errores de la estructura de .git-rustico. por ejemplo no existe index u objects
#[derive(Debug)]
pub enum ErrorType {
    IOError(io::Error),
    Parse(ParseError),
    FormatError(String),
    FileNotFound(String),
    CommandError(CommandError),
    FileNotInIndex(String),
    InvalidPath(String),
    InvalidHash(String),
    RepositoryError(String),
    ConfigError(String),
    ObjectType(String, String),
    ProtocolError(String),
    HTTPError(HTTPError),
}

impl fmt::Display for ErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorType::IOError(ref cause) => write!(
                f,
                "ERROR:[Error while reading or writing to file: {}]",
                cause
            ),
            ErrorType::FormatError(s) => write!(f, "ERROR:[Error in file format: {}]", s),
            ErrorType::FileNotFound(s) => {
                write!(f, "ERROR:[The given path does not match any files: {}]", s)
            }
            ErrorType::FileNotInIndex(s) => {
                write!(
                    f,
                    "ERROR:[The given path {} is not included in the index file]",
                    s
                )
            }
            ErrorType::CommandError(e) => write!(f, "{}", e),
            ErrorType::InvalidPath(p) => write!(f, "ERROR:[Invalid path name: {}]", p),
            ErrorType::RepositoryError(s) => write!(f, "ERROR[{}]", s),
            ErrorType::ConfigError(s) => write!(f, "ERROR:[Configuration error: {s}]"),
            ErrorType::ObjectType(e, g) => {
                write!(f, "ERROR:[Wrong ObjectType. Expected {e}, got {g}]")
            }
            ErrorType::InvalidHash(s) => write!(
                f,
                "ERROR:[Invalid hash '{s}'. Hashes must be 40 characters long]"
            ),
            ErrorType::Parse(p) => write!(f, "ERROR[couldn't parse {p}]"),
            ErrorType::ProtocolError(s) => write!(f, "Error[git-protocol: {s}]"),
            ErrorType::HTTPError(e) => write!(f, "ERROR[{e}]"),
        }
    }
}

impl From<io::Error> for ErrorType {
    fn from(value: io::Error) -> Self {
        ErrorType::IOError(value)
    }
}

impl From<std::fmt::Error> for ErrorType {
    fn from(value: std::fmt::Error) -> Self {
        ErrorType::RepositoryError(format!(
            "Couldn't format text when writing to a string: {value}"
        ))
    }
}

impl From<ParseIntError> for ErrorType {
    fn from(value: ParseIntError) -> Self {
        let e = ParseError::Int(value);
        ErrorType::Parse(e)
    }
}
