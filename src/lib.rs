//! GoldFix: A FIX protocol engine implementation in Rust
//! A high-performance implementation focusing on reliability and atomic operations

pub mod message;
pub mod session;
pub mod config;
pub mod transport;
pub mod error;
pub mod logging;
pub mod store;
pub mod initiator;  // New module
pub mod acceptor;   // New module

use std::sync::Arc;
use tokio::sync::Mutex;

pub use error::FixError;
pub use initiator::Initiator;
pub use acceptor::Acceptor;
pub type Result<T> = std::result::Result<T, FixError>;

/// The main FIX engine instance
/// Can be configured as either an initiator or acceptor
pub struct FixEngine {
    initiator: Option<Arc<Initiator>>,
    acceptor: Option<Arc<Mutex<Acceptor>>>,
    #[allow(dead_code)]
    logger: Arc<logging::Logger>,
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
        if let Some(initiator) = &self.initiator {
            for session in &self.config.sessions {
                if session.is_initiator() {
                    initiator.start_session(session.clone()).await?;
                }
            }
        }

        // Start acceptor if configured
        if let Some(acceptor) = &self.acceptor {
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
        if let Some(initiator) = &self.initiator {
            initiator.stop_all().await?;
        }
        if let Some(acceptor) = &self.acceptor {
            let mut acceptor = acceptor.lock().await;
            acceptor.stop().await?;
        }
        Ok(())
    }

    pub async fn add_session(&self, session_config: config::SessionConfig) -> Result<()> {
        if session_config.is_initiator() {
            if let Some(initiator) = &self.initiator {
                initiator.start_session(session_config).await?;
            }
        }
        Ok(())
    }

    /// Get a reference to the message pool
    pub fn message_pool(&self) -> Arc<message::MessagePool> {
        Arc::clone(&self.message_pool)
    }

    /// Get all session IDs
    pub async fn get_session_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();

        if let Some(initiator) = &self.initiator {
            let sessions = initiator.sessions.lock().await;
            ids.extend(sessions.iter().map(|s| {
                format!("{}_{}", s.config.sender_comp_id, s.config.target_comp_id)
            }));
        }

        if let Some(acceptor) = &self.acceptor {
            let acceptor = acceptor.lock().await;
            let sessions = acceptor.sessions.lock().await;
            ids.extend(sessions.iter().map(|s| {
                format!("{}_{}", s.config.sender_comp_id, s.config.target_comp_id)
            }));
        }

        ids
    }
}