use std::fmt;
use std::error::Error;

#[derive(Debug)]
pub enum FixError {
    ParseError(String),
    SessionError(String),
    ConfigError(String),
    TransportError(String),
    IoError(std::io::Error),
}

impl fmt::Display for FixError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FixError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            FixError::SessionError(msg) => write!(f, "Session error: {}", msg),
            FixError::ConfigError(msg) => write!(f, "Config error: {}", msg),
            FixError::TransportError(msg) => write!(f, "Transport error: {}", msg),
            FixError::IoError(err) => write!(f, "IO error: {}", err),
        }
    }
}

impl Error for FixError {}

impl From<std::io::Error> for FixError {
    fn from(err: std::io::Error) -> FixError {
        FixError::IoError(err)
    }
}
