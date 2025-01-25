//! FIX Protocol Initiator implementation
//! Handles client-side connection initiation and message flow

use crate::{
    config::SessionConfig,
    error::FixError,
    logging::Logger,
    message::MessagePool,
    session::Session,
    store::MessageStore,
    Result,
};
use std::sync::Arc;
use tokio::sync::Mutex;

/// FIX Initiator implementation
pub struct Initiator {
    pub sessions: Arc<Mutex<Vec<Session>>>,
    logger: Arc<Logger>,
    store: Arc<MessageStore>,
    message_pool: Arc<MessagePool>,
}

impl Initiator {
    pub fn new(logger: Arc<Logger>, store: Arc<MessageStore>, message_pool: Arc<MessagePool>) -> Self {
        Initiator {
            sessions: Arc::new(Mutex::new(Vec::new())),
            logger,
            store,
            message_pool,
        }
    }

    /// Start a new initiator session
    pub async fn start_session(&self, config: SessionConfig) -> Result<()> {
        let mut sessions = self.sessions.lock().await;
        let session = Session::new(
            config.clone(),
            Arc::clone(&self.logger),
            Arc::clone(&self.store),
            Arc::clone(&self.message_pool),
        );

        // Validate initiator-specific configuration
        if !config.is_initiator() {
            return Err(FixError::ConfigError("Session must be configured as initiator".into()));
        }

        session.start().await?;
        sessions.push(session);
        Ok(())
    }

    /// Stop all initiator sessions
    pub async fn stop_all(&self) -> Result<()> {
        let sessions = self.sessions.lock().await;
        for session in sessions.iter() {
            session.stop().await?;
        }
        Ok(())
    }

    /// Get active session count
    pub async fn active_session_count(&self) -> usize {
        let sessions = self.sessions.lock().await;
        let mut count = 0;
        for session in sessions.iter() {
            if session.is_connected().await {
                count += 1;
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LogConfig;
    use std::path::PathBuf;

    async fn create_test_initiator() -> Initiator {
        let logger = Arc::new(Logger::new(&LogConfig {
            log_directory: PathBuf::from("/tmp"),
            log_level: "DEBUG".to_string(),
            log_events: true,
            log_messages: true,
        }));
        let store = Arc::new(MessageStore::new());
        let message_pool = Arc::new(MessagePool::new());
        Initiator::new(logger, store, message_pool)
    }

    #[tokio::test]
    async fn test_initiator_session_management() {
        let initiator = create_test_initiator().await;

        let config = SessionConfig {
            begin_string: "FIX.4.2".to_string(),
            sender_comp_id: "TEST_INITIATOR".to_string(),
            target_comp_id: "TEST_ACCEPTOR".to_string(),
            target_addr: "127.0.0.1:0".to_string(),
            heart_bt_int: 30,
            reset_on_logon: true,
            reset_on_logout: true,
            reset_on_disconnect: true,
            transport_config: None,
            role: crate::config::SessionRole::Initiator,
        };

        // Test starting a session
        assert!(initiator.start_session(config).await.is_ok());
        assert_eq!(initiator.active_session_count().await, 1);

        // Test stopping all sessions
        assert!(initiator.stop_all().await.is_ok());
        assert_eq!(initiator.active_session_count().await, 0);
    }
}