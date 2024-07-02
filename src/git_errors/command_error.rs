use core::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub enum CommandError {
    UnknownOption(String, String),
    IncorrectAmount(String, usize),
    IncorrectOptionAmount(usize, usize),
    InvalidHash(String),
    InvalidBranch(String),
    FormatError(String),
    InexistentPath(PathBuf),
    InvalidArgument(String),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::UnknownOption(expected, received) => write!(
                f,
                "ERROR:[Error in option. Received: {received}, allowed: {expected}]"
            ),
            CommandError::IncorrectAmount(expected, received) => write!(
                f,
                "ERROR:[Error in amount of parameters sent. Expected {expected}, got {received}]"
            ),
            CommandError::IncorrectOptionAmount(expected, received) => write!(
                f,
                "ERROR:[Error in amount of options sent. Expected {expected}, got {received}]"
            ),
            CommandError::InvalidHash(s) => write!(f, "ERROR:[Error in hash passed: {s}]"),
            CommandError::InvalidBranch(s) => write!(f, "ERROR[Invalid branch input: {s}]"),
            CommandError::FormatError(s) => write!(f, "ERROR[Error in command format.{s}]"),
            CommandError::InexistentPath(p) => write!(
                f,
                "ERROR[Inexistent path passed as argument: {}]",
                p.display()
            ),
            CommandError::InvalidArgument(s) => write!(f, "ERROR[Invalid argument: {s}]"),
        }
    }
}
