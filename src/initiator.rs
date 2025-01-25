use crate::{
    config::{SessionConfig, SessionRole},
    error::FixError,
    logging::Logger,
    message::MessagePool,
    session::Session,
    store::MessageStore,
    Result,
};
use std::sync::Arc;
use tokio::sync::Mutex;

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

    pub async fn start_session(&self, config: SessionConfig) -> Result<()> {
        // Validate initiator-specific configuration
        if config.role != SessionRole::Initiator {
            return Err(FixError::ParseError("Session must be configured as initiator".into()));
        }

        // Create and initialize new session
        let session = Session::new(
            config.clone(),
            Arc::clone(&self.logger),
            Arc::clone(&self.store),
            Arc::clone(&self.message_pool),
        );

        // Add session to managed sessions before starting
        {
            let mut sessions = self.sessions.lock().await;
            sessions.push(session.clone());
        }

        // Start the session
        match session.start().await {
            Ok(_) => Ok(()),
            Err(e) => {
                // Clean up on error
                let mut sessions = self.sessions.lock().await;
                if let Some(pos) = sessions.iter().position(|s| s.id() == session.id()) {
                    sessions.remove(pos);
                }
                Err(e)
            }
        }
    }

    pub async fn stop_all(&self) -> Result<()> {
        let sessions = self.sessions.lock().await;
        for session in sessions.iter() {
            if let Err(e) = session.stop().await {
                self.logger.log_event("ERROR", &format!("Failed to stop session: {}", e)).ok();
            }
        }
        Ok(())
    }

    pub async fn active_session_count(&self) -> usize {
        let sessions = self.sessions.lock().await;
        sessions.iter()
            .filter(|s| futures::executor::block_on(s.is_connected()))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_initiator_session_management() {
        // Setup test environment
        let logger = Arc::new(Logger::new(&crate::config::LogConfig {
            log_directory: PathBuf::from("/tmp"),
            log_level: "DEBUG".to_string(),
            log_events: true,
            log_messages: true,
        }));
        let store = Arc::new(MessageStore::new());
        let message_pool = Arc::new(MessagePool::new());
        let initiator = Initiator::new(logger, store, message_pool);

        // Create test config with proper role
        let config = SessionConfig {
            begin_string: "FIX.4.2".to_string(),
            sender_comp_id: "TEST_INITIATOR".to_string(),
            target_comp_id: "TEST_ACCEPTOR".to_string(),
            target_addr: "127.0.0.1:0".to_string(),
            heart_bt_int: 30,
            reset_on_logon: true,
            reset_on_logout: true,
            reset_on_disconnect: true,
            role: SessionRole::Initiator,
            transport_config: None,
        };

        assert!(initiator.start_session(config).await.is_ok());
        assert_eq!(initiator.active_session_count().await, 0);
        assert!(initiator.stop_all().await.is_ok());
    }
}