use std::fmt;
use std::error::Error;
use serde_json;
use rustls;
use rustls::pki_types::InvalidDnsNameError;
use std::io;

#[derive(Debug)]
pub enum FixError {
    ParseError(String),
    SessionError(String),
    ConfigError(String),
    TransportError(String),
    StoreError(String),
    IoError(io::Error),
    SerializationError(serde_json::Error),
    SslError(String),
    ConnectionError(String),
    CertificateError(String),
    SessionNotFound(String),  // Added SessionNotFound variant
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
            FixError::SslError(msg) => write!(f, "SSL error: {}", msg),
            FixError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            FixError::CertificateError(msg) => write!(f, "Certificate error: {}", msg),
            FixError::SessionNotFound(msg) => write!(f, "Session not found: {}", msg),
        }
    }
}

impl Error for FixError {}

impl From<io::Error> for FixError {
    fn from(err: io::Error) -> FixError {
        FixError::IoError(err)
    }
}

impl From<serde_json::Error> for FixError {
    fn from(err: serde_json::Error) -> FixError {
        FixError::SerializationError(err)
    }
}

impl From<rustls::Error> for FixError {
    fn from(err: rustls::Error) -> FixError {
        FixError::SslError(err.to_string())
    }
}

impl From<InvalidDnsNameError> for FixError {
    fn from(err: InvalidDnsNameError) -> FixError {
        FixError::CertificateError(format!("Invalid DNS name in certificate: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = FixError::TransportError("Connection failed".to_string());
        assert_eq!(err.to_string(), "Transport error: Connection failed");

        let err = FixError::SslError("Invalid certificate".to_string());
        assert_eq!(err.to_string(), "SSL error: Invalid certificate");

        let err = FixError::SessionNotFound("TEST_SESSION".to_string());
        assert_eq!(err.to_string(), "Session not found: TEST_SESSION");
    }

    #[test]
    fn test_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let fix_err: FixError = io_err.into();
        assert!(matches!(fix_err, FixError::IoError(_)));

        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let fix_err: FixError = json_err.into();
        assert!(matches!(fix_err, FixError::SerializationError(_)));
    }
}