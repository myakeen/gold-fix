use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::net::TcpStream;
use tokio::time::{self, Duration};
use std::path::PathBuf;

use crate::config::SessionConfig;
use crate::logging::Logger;
use crate::message::{Message, Field, field, MessagePool};
use crate::transport::Transport;
use crate::Result;
use crate::store::MessageStore;
use chrono;

pub mod state;

#[derive(Clone)]
pub struct Session {
    pub config: SessionConfig,
    state: Arc<Mutex<state::SessionState>>,
    transport: Arc<Mutex<Option<Transport>>>,
    logger: Arc<Logger>,
    store: Arc<MessageStore>,
    message_pool: Arc<MessagePool>,
}

impl Session {
    pub fn new(
        config: SessionConfig,
        logger: Arc<Logger>,
        store: Arc<MessageStore>,
        message_pool: Arc<MessagePool>,
    ) -> Self {
        let session_id = format!("{}_{}", config.sender_comp_id, config.target_comp_id);
        let store_dir = PathBuf::from("store/sessions");

        let state = state::SessionState::with_config(
            &session_id,
            store_dir,
            10,  // logon timeout
            config.heart_bt_int as u64,
            2,   // test request delay
        );

        Session {
            config,
            state: Arc::new(Mutex::new(state)),
            transport: Arc::new(Mutex::new(None)),
            logger,
            store,
            message_pool,
        }
    }

    pub async fn start(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        state.set_status(state::Status::Connecting);
        drop(state);

        // Connect to counterparty
        let stream = TcpStream::connect(&self.config.target_addr).await?;
        let transport = Transport::new(stream);

        {
            let mut transport_guard = self.transport.lock().await;
            *transport_guard = Some(transport);
        }

        // Initiate logon sequence
        self.initiate_logon().await?;

        // Start heartbeat monitoring
        self.start_heartbeat_monitor().await;

        // Start message processing
        self.start_message_processor().await;

        Ok(())
    }

    async fn initiate_logon(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        state.set_status(state::Status::InitiateLogon);
        state.update_send_time();

        let logon = self.create_logon_message().await;
        drop(state);

        if let Some(transport) = self.transport.lock().await.as_mut() {
            transport.send(&logon).await?;
        }

        Ok(())
    }

    async fn start_heartbeat_monitor(&self) {
        let transport_clone = Arc::clone(&self.transport);
        let state_clone = Arc::clone(&self.state);
        let config_clone = self.config.clone();
        let logger_clone = Arc::clone(&self.logger);
        let message_pool_clone = Arc::clone(&self.message_pool);

        tokio::spawn(async move {
            let heartbeat_interval = Duration::from_secs(config_clone.heart_bt_int as u64);
            let mut interval = time::interval(heartbeat_interval);

            loop {
                interval.tick().await;
                let mut state = state_clone.lock().await;

                if state.should_disconnect() {
                    state.set_status(state::Status::Disconnecting);
                    drop(state);
                    break;
                }

                // Check if we need to send a test request
                if state.should_send_test_request() {
                    state.increment_test_request_counter();
                    drop(state);

                    // Get test request message from pool
                    let mut test_request = message_pool_clone.get_message(field::values::TEST_REQUEST).await;
                    if let Err(e) = test_request.set_field(Field::new(field::TEST_REQ_ID,
                        format!("TEST_REQ_{}", chrono::Utc::now().timestamp()))) {
                            logger_clone.log_event("ERROR", &format!("Failed to set TEST_REQ_ID: {}", e)).ok();
                            break;
                    }

                    // Send test request
                    if let Some(transport) = transport_clone.lock().await.as_mut() {
                        if let Err(e) = transport.send(&test_request).await {
                            logger_clone.log_event("ERROR", &format!("Failed to send test request: {}", e)).ok();
                            break;
                        }
                    }

                    // Return message to pool
                    message_pool_clone.return_message(test_request).await;
                }

                // Send regular heartbeat if connected
                let state = state_clone.lock().await;
                if *state.status() == state::Status::Connected {
                    drop(state);

                    // Get heartbeat message from pool
                    let heartbeat = message_pool_clone.get_message(field::values::HEARTBEAT).await;
                    if let Some(transport) = transport_clone.lock().await.as_mut() {
                        if let Err(e) = transport.send(&heartbeat).await {
                            logger_clone.log_event("ERROR", &format!("Failed to send heartbeat: {}", e)).ok();
                            break;
                        }
                    }
                    // Return message to pool
                    message_pool_clone.return_message(heartbeat).await;
                }
            }
        });
    }

    async fn start_message_processor(&self) {
        let transport_clone = Arc::clone(&self.transport);
        let state_clone = Arc::clone(&self.state);
        let logger_clone = Arc::clone(&self.logger);
        let store_clone = Arc::clone(&self.store);
        let config_clone = self.config.clone();
        let message_pool_clone = Arc::clone(&self.message_pool);

        tokio::spawn(async move {
            loop {
                let state = state_clone.lock().await;
                if *state.status() == state::Status::Disconnected {
                    break;
                }
                drop(state);

                if let Some(transport) = transport_clone.lock().await.as_mut() {
                    match transport.receive().await {
                        Ok(Some(msg)) => {
                            let session_id = format!("{}_{}", config_clone.sender_comp_id, config_clone.target_comp_id);

                            // Store incoming message
                            if let Some(seq_num) = msg.get_field(field::MSG_SEQ_NUM) {
                                if let Ok(seq_num) = seq_num.value().parse::<i32>() {
                                    let mut state = state_clone.lock().await;

                                    // Check for sequence gaps
                                    if seq_num > state.next_incoming_seq() {
                                        // Get resend request from pool
                                        let mut resend_request = message_pool_clone.get_message(field::values::RESEND_REQUEST).await;
                                        match resend_request.set_field(Field::new(field::BEGIN_SEQ_NO, 
                                            state.next_incoming_seq().to_string())) {
                                                Ok(_) => {
                                                    match resend_request.set_field(Field::new(field::END_SEQ_NO, seq_num.to_string())) {
                                                        Ok(_) => {
                                                            if let Err(e) = transport.send(&resend_request).await {
                                                                logger_clone.log_event("ERROR", &format!("Failed to send resend request: {}", e)).ok();
                                                            }
                                                        },
                                                        Err(e) => {
                                                            logger_clone.log_event("ERROR", &format!("Failed to set END_SEQ_NO: {}", e)).ok();
                                                        }
                                                    }
                                                },
                                                Err(e) => {
                                                    logger_clone.log_event("ERROR", &format!("Failed to set BEGIN_SEQ_NO: {}", e)).ok();
                                                }
                                        }
                                        message_pool_clone.return_message(resend_request).await;
                                        continue;
                                    }

                                    // Store message
                                    if let Err(e) = store_clone.store_message(&session_id, seq_num, msg.clone()).await {
                                        logger_clone.log_event("ERROR", &format!("Failed to store message: {}", e)).ok();
                                    }

                                    state.increment_incoming_seq();
                                }
                            }

                            // Process message based on type
                            match msg.msg_type() {
                                field::values::LOGON => {
                                    let mut state = state_clone.lock().await;
                                    if *state.status() == state::Status::InitiateLogon {
                                        state.set_status(state::Status::Connected);
                                        state.reset_test_request_counter();
                                    }
                                },
                                field::values::TEST_REQUEST => {
                                    let mut heartbeat = message_pool_clone.get_message(field::values::HEARTBEAT).await;
                                    if let Some(test_req_id) = msg.get_field(field::TEST_REQ_ID) {
                                        if let Err(e) = heartbeat.set_field(Field::new(field::TEST_REQ_ID, test_req_id.value())) {
                                            logger_clone.log_event("ERROR", &format!("Failed to set TEST_REQ_ID in heartbeat: {}", e)).ok();
                                        } else if let Err(e) = transport.send(&heartbeat).await {
                                            logger_clone.log_event("ERROR", &format!("Failed to send heartbeat: {}", e)).ok();
                                        }
                                    }
                                    message_pool_clone.return_message(heartbeat).await;
                                },
                                field::values::HEARTBEAT => {
                                    let mut state = state_clone.lock().await;
                                    state.reset_test_request_counter();
                                },
                                field::values::LOGOUT => {
                                    let mut state = state_clone.lock().await;
                                    state.set_status(state::Status::Disconnected);
                                    break;
                                },
                                _ => {
                                    logger_clone.log_event("INFO", &format!("Received message type: {}", msg.msg_type())).ok();
                                }
                            }
                        },
                        Ok(None) => {
                            time::sleep(Duration::from_millis(100)).await;
                        },
                        Err(e) => {
                            logger_clone.log_event("ERROR", &format!("Error receiving message: {}", e)).ok();
                            let mut state = state_clone.lock().await;
                            state.set_status(state::Status::Error);
                            break;
                        }
                    }
                }
            }
        });
    }

    pub async fn stop(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        state.set_status(state::Status::Disconnecting);

        // Send logout message if connected
        if let Some(mut transport) = self.transport.lock().await.take() {
            let logout = self.create_logout_message().await;
            transport.send(&logout).await?;
        }

        state.set_status(state::Status::Disconnected);
        Ok(())
    }

    async fn create_logon_message(&self) -> Message {
        let mut msg = self.message_pool.get_message(field::values::LOGON).await;
        let _ = msg.set_field(Field::new(field::BEGIN_STRING, &self.config.begin_string));
        let _ = msg.set_field(Field::new(field::SENDER_COMP_ID, &self.config.sender_comp_id));
        let _ = msg.set_field(Field::new(field::TARGET_COMP_ID, &self.config.target_comp_id));
        let _ = msg.set_field(Field::new(field::HEART_BT_INT, self.config.heart_bt_int.to_string()));
        let _ = msg.set_field(Field::new(field::ENCRYPT_METHOD, "0")); // No encryption
        let _ = msg.set_field(Field::new(field::RESET_SEQ_NUM_FLAG, "Y")); // Reset sequence numbers
        msg
    }

    async fn create_logout_message(&self) -> Message {
        let mut msg = self.message_pool.get_message(field::values::LOGOUT).await;
        let _ = msg.set_field(Field::new(field::BEGIN_STRING, &self.config.begin_string));
        let _ = msg.set_field(Field::new(field::SENDER_COMP_ID, &self.config.sender_comp_id));
        let _ = msg.set_field(Field::new(field::TARGET_COMP_ID, &self.config.target_comp_id));
        msg
    }

    pub async fn recover(&self) -> Result<()> {
        let mut state = self.state.lock().await;

        // Load persisted messages from store
        let session_id = format!("{}_{}", self.config.sender_comp_id, self.config.target_comp_id);
        self.store.load_messages(&session_id).await?;

        // Reset sequence numbers if configured
        if self.config.reset_on_disconnect {
            state.reset_sequence_numbers();
            self.store.reset_sequence_numbers(&session_id).await?;
        }

        // Attempt to reconnect
        if let Some(transport) = self.transport.lock().await.as_mut() {
            let logout = self.create_logout_message().await;
            let _ = transport.send(&logout).await;  // Best effort logout
        }

        // Clear existing transport
        *self.transport.lock().await = None;

        // Update state for recovery
        state.set_status(state::Status::Recovering);

        // Attempt to establish new connection
        match TcpStream::connect(&self.config.target_addr).await {
            Ok(stream) => {
                let transport = Transport::new(stream);
                *self.transport.lock().await = Some(transport);

                // Initiate new logon sequence
                drop(state);
                self.initiate_logon().await?;

                // Start heartbeat monitoring and message processing
                self.start_heartbeat_monitor().await;
                self.start_message_processor().await;

                Ok(())
            },
            Err(e) => {
                state.set_status(state::Status::Error);
                Err(e.into())
            }
        }
    }

    pub async fn disconnect(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        state.set_status(state::Status::Disconnecting);

        if let Some(transport) = self.transport.lock().await.as_mut() {
            let logout = self.create_logout_message().await;
            let _ = transport.send(&logout).await;  // Best effort send
        }

        *self.transport.lock().await = None;
        state.set_status(state::Status::Disconnected);
        Ok(())
    }

    pub async fn logout(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        state.set_status(state::Status::Disconnecting);

        // Send logout message
        if let Some(transport) = self.transport.lock().await.as_mut() {
            let logout = self.create_logout_message().await;
            transport.send(&logout).await?;
        }

        state.set_status(state::Status::Disconnected);
        Ok(())
    }

    pub async fn is_connected(&self) -> bool {
        let state = self.state.lock().await;
        *state.status() == state::Status::Connected
    }

    pub async fn get_state(&self) -> Result<state::SessionState> {
        Ok(self.state.lock().await.clone())
    }

    /// Get the session ID in the format "{sender_comp_id}_{target_comp_id}"
    pub fn session_id(&self) -> String {
        format!("{}_{}", self.config.sender_comp_id, self.config.target_comp_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::config::LogConfig;
    use crate::transport::TransportConfig;
    use crate::config::SessionRole;

    #[tokio::test]
    async fn test_session_lifecycle() {
        let config = SessionConfig {
            begin_string: "FIX.4.2".to_string(),
            sender_comp_id: "SENDER".to_string(),
            target_comp_id: "TARGET".to_string(),
            target_addr: "127.0.0.1:0".to_string(),
            heart_bt_int: 30,
            reset_on_logon: true,
            reset_on_logout: true,
            reset_on_disconnect: true,
            transport_config: Some(TransportConfig::default()),
            role: SessionRole::Initiator,
        };

        let log_config = LogConfig {
            log_directory: PathBuf::from("/tmp"),
            log_level: "INFO".to_string(),
            log_events: true,
            log_messages: true,
        };

        let logger = Arc::new(Logger::new(&log_config));
        let store = Arc::new(MessageStore::new());
        let message_pool = Arc::new(MessagePool::new());
        let session = Session::new(config, logger, store, message_pool);

        // Test message creation
        let logon = session.create_logon_message().await;
        assert_eq!(logon.msg_type(), field::values::LOGON);
        assert!(logon.get_field(field::HEART_BT_INT).is_some());

        let logout = session.create_logout_message().await;
        assert_eq!(logout.msg_type(), field::values::LOGOUT);
    }
}