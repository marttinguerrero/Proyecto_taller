use std::{fmt, num::ParseIntError, string::FromUtf8Error};

#[derive(Debug)]

pub enum ParseError {
    Utf8(FromUtf8Error),
    Int(ParseIntError),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Utf8(e) => write!(f, "from u8 to utf8: {e}"),
            ParseError::Int(e) => write!(f, "from string to int: {e}"),
        }
    }
}

impl From<ParseIntError> for ParseError {
    fn from(value: ParseIntError) -> Self {
        ParseError::Int(value)
    }
}

impl From<FromUtf8Error> for ParseError {
    fn from(value: FromUtf8Error) -> Self {
        ParseError::Utf8(value)
    }
}
