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
}

impl EngineConfig {
    pub fn from_file(path: &str) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: EngineConfig = toml::from_str(&content)
            .map_err(|e| crate::error::FixError::ConfigError(e.to_string()))?;
        Ok(config)
    }
}