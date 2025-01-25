use crate::{
    config::{SessionConfig, SessionRole},
    logging::Logger,
    message::MessagePool,
    session::Session,
    store::MessageStore,
    Result,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;

/// FIX Acceptor implementation
pub struct Acceptor {
    pub sessions: Arc<Mutex<Vec<Session>>>,
    logger: Arc<Logger>,
    store: Arc<MessageStore>,
    message_pool: Arc<MessagePool>,
    listener: Option<Arc<TcpListener>>,
    accept_handle: Option<JoinHandle<()>>,
}

impl Acceptor {
    pub fn new(logger: Arc<Logger>, store: Arc<MessageStore>, message_pool: Arc<MessagePool>) -> Self {
        Acceptor {
            sessions: Arc::new(Mutex::new(Vec::new())),
            logger,
            store,
            message_pool,
            listener: None,
            accept_handle: None,
        }
    }

    /// Handle a new connection
    async fn handle_connection(
        sessions: Arc<Mutex<Vec<Session>>>,
        socket: TcpStream,
        logger: Arc<Logger>,
        store: Arc<MessageStore>,
        message_pool: Arc<MessagePool>,
    ) {
        let peer_addr = socket.peer_addr().unwrap_or_else(|_| "unknown".parse().unwrap());
        let session = Session::new(
            SessionConfig {
                begin_string: "FIX.4.2".to_string(),
                sender_comp_id: "ACCEPTOR".to_string(),
                target_comp_id: "INITIATOR".to_string(),
                target_addr: peer_addr.to_string(),
                heart_bt_int: 30,
                reset_on_logon: true,
                reset_on_logout: true,
                reset_on_disconnect: true,
                transport_config: None,
                role: SessionRole::Acceptor,
            },
            Arc::clone(&logger),
            Arc::clone(&store),
            Arc::clone(&message_pool),
        );

        let mut sessions = sessions.lock().await;
        sessions.push(session);
    }

    /// Start listening for incoming connections
    pub async fn start(&mut self, bind_addr: &str) -> Result<()> {
        let listener = TcpListener::bind(bind_addr).await?;
        self.listener = Some(Arc::new(listener));
        let listener = Arc::clone(self.listener.as_ref().unwrap());

        let sessions = Arc::clone(&self.sessions);
        let logger = Arc::clone(&self.logger);
        let store = Arc::clone(&self.store);
        let message_pool = Arc::clone(&self.message_pool);

        // Spawn the accept loop in a separate task
        let handle = tokio::spawn(async move {
            loop {
                if let Ok((socket, _)) = listener.accept().await {
                    let sessions = Arc::clone(&sessions);
                    let logger = Arc::clone(&logger);
                    let store = Arc::clone(&store);
                    let message_pool = Arc::clone(&message_pool);

                    tokio::spawn(async move {
                        Self::handle_connection(
                            sessions,
                            socket,
                            logger,
                            store,
                            message_pool,
                        ).await;
                    });
                }
            }
        });

        self.accept_handle = Some(handle);
        Ok(())
    }

    /// Stop accepting new connections and close existing sessions
    pub async fn stop(&mut self) -> Result<()> {
        // Abort the accept loop if it's running
        if let Some(handle) = self.accept_handle.take() {
            handle.abort();
        }

        // Clear the listener
        self.listener = None;

        // Stop all sessions
        let mut sessions = self.sessions.lock().await;
        for session in sessions.iter_mut() {
            session.stop().await?;
        }
        Ok(())
    }

    /// Get active session count using async closure
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

    async fn create_test_acceptor() -> Acceptor {
        let logger = Arc::new(Logger::new(&LogConfig {
            log_directory: PathBuf::from("/tmp"),
            log_level: "DEBUG".to_string(),
            log_events: true,
            log_messages: true,
        }));
        let store = Arc::new(MessageStore::new());
        let message_pool = Arc::new(MessagePool::new());
        Acceptor::new(logger, store, message_pool)
    }

    #[tokio::test]
    async fn test_acceptor_lifecycle() {
        let mut acceptor = create_test_acceptor().await;

        // Test starting acceptor
        assert!(acceptor.start("127.0.0.1:0").await.is_ok());

        // Test stopping acceptor
        assert!(acceptor.stop().await.is_ok());
        assert_eq!(acceptor.active_session_count().await, 0);
    }
}