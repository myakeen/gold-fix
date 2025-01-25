//! GoldFix: A FIX protocol engine implementation in Rust
//! A high-performance implementation focusing on reliability and atomic operations

pub mod message;
pub mod session;
pub mod config;
pub mod transport;
pub mod error;
pub mod logging;
pub mod store;

use std::sync::Arc;
use tokio::sync::Mutex;

pub use error::FixError;
pub type Result<T> = std::result::Result<T, FixError>;

/// The main FIX engine instance
pub struct FixEngine {
    sessions: Arc<Mutex<Vec<session::Session>>>,
    logger: Arc<logging::Logger>,
    store: Arc<store::MessageStore>,
    message_pool: Arc<message::MessagePool>,  // Add message pool
}

impl FixEngine {
    pub fn new(config: config::EngineConfig) -> Self {
        let logger = Arc::new(logging::Logger::new(&config.log_config));
        let store = Arc::new(store::MessageStore::new());
        let message_pool = Arc::new(message::MessagePool::new());  // Initialize message pool

        FixEngine {
            sessions: Arc::new(Mutex::new(Vec::new())),
            logger,
            store,
            message_pool,
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
        let session = session::Session::new(
            session_config, 
            Arc::clone(&self.logger),
            Arc::clone(&self.store),
            Arc::clone(&self.message_pool),  // Pass message pool to session
        );
        sessions.push(session);
        Ok(())
    }

    /// Get a reference to the message pool
    pub fn message_pool(&self) -> Arc<message::MessagePool> {
        Arc::clone(&self.message_pool)
    }

    /// Get a session by its ID
    pub async fn get_session(&self, session_id: &str) -> Result<Arc<session::Session>> {
        let sessions = self.sessions.lock().await;
        let session = sessions.iter()
            .find(|s| {
                let config = &s.config;
                format!("{}_{}", config.sender_comp_id, config.target_comp_id) == session_id
            })
            .ok_or_else(|| error::FixError::SessionNotFound(session_id.to_string()))?;

        // Create a new Arc pointing to the session
        Ok(Arc::new(session.clone()))
    }

    /// Get all session IDs
    pub async fn get_session_ids(&self) -> Vec<String> {
        let sessions = self.sessions.lock().await;
        sessions.iter()
            .map(|s| format!("{}_{}", s.config.sender_comp_id, s.config.target_comp_id))
            .collect()
    }
}