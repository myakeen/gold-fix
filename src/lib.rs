//! GoldFix: A FIX protocol engine implementation in Rust
//! A high-performance implementation focusing on reliability and atomic operations

pub mod message;
pub mod session;
pub mod config;
pub mod transport;
pub mod error;
pub mod logging;
pub mod store;
pub mod initiator;
pub mod acceptor;

use std::sync::Arc;
use tokio::sync::Mutex;

pub use error::FixError;
pub use initiator::Initiator;
pub use acceptor::Acceptor;
pub type Result<T> = std::result::Result<T, FixError>;

/// The main FIX engine instance that manages sessions and message handling.
/// Can be configured as either an initiator or acceptor.
pub struct FixEngine {
    initiator: Option<Arc<Initiator>>,
    acceptor: Option<Arc<Mutex<Acceptor>>>,
    /// Logger instance shared across all sessions for consistent logging
    /// Used indirectly through Arc cloning when creating new sessions
    #[allow(dead_code)]
    logger: Arc<logging::Logger>,
    /// Message store for persistence across all sessions
    /// Used indirectly through Arc cloning for session message storage
    #[allow(dead_code)]
    store: Arc<store::MessageStore>,
    message_pool: Arc<message::MessagePool>,
    config: Arc<config::EngineConfig>,
}

impl FixEngine {
    pub fn new(config: config::EngineConfig) -> Self {
        let logger = Arc::new(logging::Logger::new(&config.log_config));
        let store = Arc::new(store::MessageStore::new());
        let message_pool = Arc::new(message::MessagePool::new());
        let config = Arc::new(config);

        FixEngine {
            initiator: Some(Arc::new(Initiator::new(
                Arc::clone(&logger),
                Arc::clone(&store),
                Arc::clone(&message_pool),
            ))),
            acceptor: Some(Arc::new(Mutex::new(Acceptor::new(
                Arc::clone(&logger),
                Arc::clone(&store),
                Arc::clone(&message_pool),
            )))),
            logger,
            store,
            message_pool,
            config,
        }
    }

    pub async fn start(&self) -> Result<()> {
        // Start initiator sessions if configured
        if let Some(ref initiator) = self.initiator {
            for session in &self.config.sessions {
                if session.is_initiator() {
                    initiator.start_session(session.clone()).await?;
                }
            }
        }

        // Start acceptor if configured
        if let Some(ref acceptor) = self.acceptor {
            let mut acceptor = acceptor.lock().await;
            for session in &self.config.sessions {
                if session.is_acceptor() {
                    acceptor.start(&session.target_addr).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        if let Some(ref initiator) = self.initiator {
            initiator.stop_all().await?;
        }
        if let Some(ref acceptor) = self.acceptor {
            let mut acceptor = acceptor.lock().await;
            acceptor.stop().await?;
        }
        Ok(())
    }

    pub async fn add_session(&self, session_config: config::SessionConfig) -> Result<()> {
        if session_config.is_initiator() {
            if let Some(ref initiator) = self.initiator {
                initiator.start_session(session_config).await?;
            }
        }
        Ok(())
    }

    pub fn message_pool(&self) -> Arc<message::MessagePool> {
        Arc::clone(&self.message_pool)
    }

    /// Get a session by its ID (format: "SENDER_COMP_ID_TARGET_COMP_ID")
    pub async fn get_session(&self, session_id: &str) -> Result<Arc<session::Session>> {
        let parts: Vec<&str> = session_id.split('_').collect();
        if parts.len() != 2 {
            return Err(FixError::ConfigError("Invalid session ID format".into()));
        }

        let session_id = session_id.to_string();

        // Check initiator sessions first
        if let Some(ref initiator) = self.initiator {
            // Use a method to get sessions instead of accessing the field directly
            let sessions = initiator.get_sessions().await;
            for session in sessions.iter() {
                if format!("{}_{}", 
                    session.config.sender_comp_id,
                    session.config.target_comp_id) == session_id {
                    return Ok(Arc::new(session.clone()));
                }
            }
        }

        // Then check acceptor sessions
        if let Some(ref acceptor) = self.acceptor {
            let acceptor = acceptor.lock().await;
            // Use a method to get sessions instead of accessing the field directly
            let sessions = acceptor.get_sessions().await;
            for session in sessions.iter() {
                if format!("{}_{}", 
                    session.config.sender_comp_id,
                    session.config.target_comp_id) == session_id {
                    return Ok(Arc::new(session.clone()));
                }
            }
        }

        Err(FixError::SessionNotFound(format!("Session not found: {}", session_id)))
    }

    /// Get all session IDs
    pub async fn get_session_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();

        if let Some(ref initiator) = self.initiator {
            let sessions = initiator.get_sessions().await;
            ids.extend(sessions.iter().map(|s| {
                format!("{}_{}", s.config.sender_comp_id, s.config.target_comp_id)
            }));
        }

        if let Some(ref acceptor) = self.acceptor {
            let acceptor = acceptor.lock().await;
            let sessions = acceptor.get_sessions().await;
            ids.extend(sessions.iter().map(|s| {
                format!("{}_{}", s.config.sender_comp_id, s.config.target_comp_id)
            }));
        }

        ids
    }
}