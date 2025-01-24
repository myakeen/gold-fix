use std::fmt;
use std::error::Error;
use serde_json;

#[derive(Debug)]
pub enum FixError {
    ParseError(String),
    SessionError(String),
    ConfigError(String),
    TransportError(String),
    StoreError(String),
    IoError(std::io::Error),
    SerializationError(serde_json::Error),  // Add serialization error variant
}

impl fmt::Display for FixError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FixError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            FixError::SessionError(msg) => write!(f, "Session error: {}", msg),
            FixError::ConfigError(msg) => write!(f, "Config error: {}", msg),
            FixError::TransportError(msg) => write!(f, "Transport error: {}", msg),
            FixError::StoreError(msg) => write!(f, "Store error: {}", msg),
            FixError::IoError(err) => write!(f, "IO error: {}", err),
            FixError::SerializationError(err) => write!(f, "Serialization error: {}", err),
        }
    }
}

impl Error for FixError {}

impl From<std::io::Error> for FixError {
    fn from(err: std::io::Error) -> FixError {
        FixError::IoError(err)
    }
}

// Add conversion from serde_json::Error
impl From<serde_json::Error> for FixError {
    fn from(err: serde_json::Error) -> FixError {
        FixError::SerializationError(err)
    }
}