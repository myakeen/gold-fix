//! A FIX protocol engine implementation in Rust
//! Inspired by QuickFix/n

pub mod message;
pub mod session;
pub mod config;
pub mod transport;
pub mod error;
pub mod logging;

use std::sync::Arc;
use tokio::sync::Mutex;

pub use error::FixError;
pub type Result<T> = std::result::Result<T, FixError>;

/// The main FIX engine instance
pub struct FixEngine {
    sessions: Arc<Mutex<Vec<session::Session>>>,
    config: config::EngineConfig,
    logger: Arc<logging::Logger>,
}

impl FixEngine {
    pub fn new(config: config::EngineConfig) -> Self {
        let logger = Arc::new(logging::Logger::new(&config.log_config));
        FixEngine {
            sessions: Arc::new(Mutex::new(Vec::new())),
            config,
            logger,
        }
    }

    pub async fn start(&self) -> Result<()> {
        let sessions = self.sessions.lock().await;
        for session in sessions.iter() {
            session.start().await?;
        }
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let sessions = self.sessions.lock().await;
        for session in sessions.iter() {
            session.stop().await?;
        }
        Ok(())
    }

    pub async fn add_session(&self, session_config: config::SessionConfig) -> Result<()> {
        let mut sessions = self.sessions.lock().await;
        let session = session::Session::new(session_config, Arc::clone(&self.logger));
        sessions.push(session);
        Ok(())
    }
}
