use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use crate::transport::TransportConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub log_config: LogConfig,
    pub sessions: Vec<SessionConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    pub log_directory: PathBuf,
    pub log_level: String,
    pub log_events: bool,
    pub log_messages: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SessionRole {
    #[serde(rename = "initiator")]
    Initiator,
    #[serde(rename = "acceptor")]
    Acceptor,
}

impl Default for SessionRole {
    fn default() -> Self {
        SessionRole::Initiator
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub begin_string: String,
    pub sender_comp_id: String,
    pub target_comp_id: String,
    pub target_addr: String,
    pub heart_bt_int: u32,
    pub reset_on_logon: bool,
    pub reset_on_logout: bool,
    pub reset_on_disconnect: bool,
    pub transport_config: Option<TransportConfig>,
    #[serde(default)]
    pub role: SessionRole,
}

impl SessionConfig {
    pub fn is_initiator(&self) -> bool {
        self.role == SessionRole::Initiator
    }

    pub fn is_acceptor(&self) -> bool {
        self.role == SessionRole::Acceptor
    }

    pub fn validate(&self) -> crate::Result<()> {
        if self.begin_string.is_empty() {
            return Err(crate::error::FixError::ConfigError("begin_string cannot be empty".into()));
        }
        if self.sender_comp_id.is_empty() {
            return Err(crate::error::FixError::ConfigError("sender_comp_id cannot be empty".into()));
        }
        if self.target_comp_id.is_empty() {
            return Err(crate::error::FixError::ConfigError("target_comp_id cannot be empty".into()));
        }
        if self.target_addr.is_empty() {
            return Err(crate::error::FixError::ConfigError("target_addr cannot be empty".into()));
        }
        if self.heart_bt_int == 0 {
            return Err(crate::error::FixError::ConfigError("heart_bt_int must be greater than 0".into()));
        }
        Ok(())
    }
}

impl EngineConfig {
    pub fn from_file(path: &str) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: EngineConfig = toml::from_str(&content)
            .map_err(|e| crate::error::FixError::ConfigError(e.to_string()))?;

        // Validate all session configurations
        for session in &config.sessions {
            session.validate()?;
        }

        Ok(config)
    }

    pub fn validate(&self) -> crate::Result<()> {
        if self.sessions.is_empty() {
            return Err(crate::error::FixError::ConfigError("At least one session must be configured".into()));
        }

        for session in &self.sessions {
            session.validate()?;
        }

        Ok(())
    }
}