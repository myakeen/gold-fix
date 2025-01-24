pub mod state;

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::net::TcpStream;
use tokio::time::{self, Duration};

use crate::config::SessionConfig;
use crate::logging::Logger;
use crate::message::{Message, Field, field};
use crate::transport::Transport;
use crate::Result;
use crate::store::MessageStore;
use chrono;

pub struct Session {
    config: SessionConfig,
    state: Arc<Mutex<state::SessionState>>,
    transport: Arc<Mutex<Option<Transport>>>,
    logger: Arc<Logger>,
    store: Arc<MessageStore>,
}

impl Session {
    pub fn new(config: SessionConfig, logger: Arc<Logger>, store: Arc<MessageStore>) -> Self {
        let state = state::SessionState::with_config(
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

        let logon = self.create_logon_message();
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

        tokio::spawn(async move {
            let heartbeat_interval = Duration::from_secs(config_clone.heart_bt_int as u64);
            let mut interval = time::interval(heartbeat_interval);

            loop {
                interval.tick().await;
                let mut state = state_clone.lock().await;

                // Check for disconnection conditions
                if state.should_disconnect() {
                    state.set_status(state::Status::Disconnecting);
                    drop(state);
                    break;
                }

                // Check if we need to send a test request
                if state.should_send_test_request() {
                    state.increment_test_request_counter();
                    drop(state);

                    // Send test request
                    if let Some(transport) = transport_clone.lock().await.as_mut() {
                        let mut test_request = Message::new(field::values::TEST_REQUEST);
                        test_request.set_field(Field::new(field::TEST_REQ_ID,
                            format!("TEST_REQ_{}", chrono::Utc::now().timestamp())));

                        if let Err(e) = transport.send(&test_request).await {
                            logger_clone.log_event("ERROR", &format!("Failed to send test request: {}", e)).ok();
                            break;
                        }
                    }
                } else {
                    drop(state);
                }

                // Send regular heartbeat if connected
                let state = state_clone.lock().await;
                if *state.status() == state::Status::Connected {
                    drop(state);
                    if let Some(transport) = transport_clone.lock().await.as_mut() {
                        let heartbeat = Message::new(field::values::HEARTBEAT);
                        if let Err(e) = transport.send(&heartbeat).await {
                            logger_clone.log_event("ERROR", &format!("Failed to send heartbeat: {}", e)).ok();
                            break;
                        }
                    }
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
                                        let mut resend_request = Message::new(field::values::RESEND_REQUEST);
                                        resend_request.set_field(Field::new(field::BEGIN_SEQ_NO, state.next_incoming_seq().to_string()));
                                        resend_request.set_field(Field::new(field::END_SEQ_NO, seq_num.to_string()));

                                        if let Err(e) = transport.send(&resend_request).await {
                                            logger_clone.log_event("ERROR", &format!("Failed to send resend request: {}", e)).ok();
                                        }
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
                                    let mut heartbeat = Message::new(field::values::HEARTBEAT);
                                    if let Some(test_req_id) = msg.get_field(field::TEST_REQ_ID) {
                                        heartbeat.set_field(Field::new(field::TEST_REQ_ID, test_req_id.value()));
                                    }
                                    if let Err(e) = transport.send(&heartbeat).await {
                                        logger_clone.log_event("ERROR", &format!("Failed to send heartbeat: {}", e)).ok();
                                    }
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



    async fn process_message(&self, msg: &Message) -> Result<()> {
        let mut state = self.state.lock().await;
        let session_id = format!("{}_{}", self.config.sender_comp_id, self.config.target_comp_id);

        // Store incoming message
        if let Some(seq_num) = msg.get_field(field::MSG_SEQ_NUM) {
            let seq_num = seq_num.value().parse::<i32>()
                .map_err(|_| FixError::ParseError("Invalid MsgSeqNum".into()))?;

            // Check for sequence gaps
            if seq_num > state.next_incoming_seq() {
                // Gap detected, request resend
                drop(state);
                self.send_resend_request(seq_num).await?;
                return Ok(());
            }

            // Store message for potential resend requests
            self.store.store_message(&session_id, seq_num, msg.clone()).await?;
        }

        match msg.msg_type() {
            field::values::RESEND_REQUEST => {
                let begin_seq = msg.get_field(field::BEGIN_SEQ_NO)
                    .ok_or_else(|| FixError::ParseError("Missing BeginSeqNo".into()))?
                    .value()
                    .parse::<i32>()
                    .map_err(|_| FixError::ParseError("Invalid BeginSeqNo".into()))?;

                let end_seq = msg.get_field(field::END_SEQ_NO)
                    .ok_or_else(|| FixError::ParseError("Missing EndSeqNo".into()))?
                    .value()
                    .parse::<i32>()
                    .map_err(|_| FixError::ParseError("Invalid EndSeqNo".into()))?;

                drop(state);
                self.resend_messages(begin_seq, end_seq).await?;
            },
            field::values::LOGON => {
                if *state.status() == state::Status::InitiateLogon {
                    state.set_status(state::Status::Connected);
                    state.reset_test_request_counter();
                }
            },
            field::values::TEST_REQUEST => {
                let mut heartbeat = Message::new(field::values::HEARTBEAT);
                if let Some(test_req_id) = msg.get_field(field::TEST_REQ_ID) {
                    heartbeat.set_field(Field::new(field::TEST_REQ_ID, test_req_id.value()));
                }
                drop(state);
                self.transport.lock().await.as_mut().unwrap().send(&heartbeat).await.ok();
            },
            field::values::HEARTBEAT => {
                state.reset_test_request_counter();
            },
            field::values::LOGOUT => {
                state.set_status(state::Status::Disconnected);
                drop(state);
                //break; //removed break to prevent unexpected behavior
            },
            _ => {
                state.increment_incoming_seq();
                self.logger.log_event("INFO", &format!("Received message type: {}", msg.msg_type())).ok();
            }
        }
        Ok(())
    }



    async fn send_resend_request(&self, expected_seq_num: i32) -> Result<()> {
        let mut resend_request = Message::new(field::values::RESEND_REQUEST);
        resend_request.set_field(Field::new(field::BEGIN_SEQ_NO, expected_seq_num.to_string()));
        resend_request.set_field(Field::new(field::END_SEQ_NO, "0")); // Request all messages

        if let Some(transport) = self.transport.lock().await.as_mut() {
            transport.send(&resend_request).await?;
        }

        Ok(())
    }

    async fn resend_messages(&self, begin_seq: i32, end_seq: i32) -> Result<()> {
        let session_id = format!("{}_{}", self.config.sender_comp_id, self.config.target_comp_id);
        let messages = self.store.get_messages_range(begin_seq, end_seq).await?;

        if let Some(transport) = self.transport.lock().await.as_mut() {
            for msg in messages {
                // Mark message as possible duplicate
                let mut resend_msg = msg.clone();
                resend_msg.set_field(Field::new(field::POSS_DUP_FLAG, "Y"));
                transport.send(&resend_msg).await?;
            }
        }

        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        state.set_status(state::Status::Disconnecting);

        // Send logout message if connected
        if let Some(mut transport) = self.transport.lock().await.take() {
            let logout = self.create_logout_message();
            transport.send(&logout).await?;
        }

        state.set_status(state::Status::Disconnected);
        Ok(())
    }

    fn create_logon_message(&self) -> Message {
        let mut msg = Message::new(field::values::LOGON);
        msg.set_field(Field::new(field::BEGIN_STRING, &self.config.begin_string));
        msg.set_field(Field::new(field::SENDER_COMP_ID, &self.config.sender_comp_id));
        msg.set_field(Field::new(field::TARGET_COMP_ID, &self.config.target_comp_id));
        msg.set_field(Field::new(field::HEART_BT_INT, self.config.heart_bt_int.to_string()));
        msg.set_field(Field::new(field::ENCRYPT_METHOD, "0")); // No encryption
        msg.set_field(Field::new(field::RESET_SEQ_NUM_FLAG, "Y")); // Reset sequence numbers
        msg
    }

    fn create_logout_message(&self) -> Message {
        let mut msg = Message::new(field::values::LOGOUT);
        msg.set_field(Field::new(field::BEGIN_STRING, &self.config.begin_string));
        msg.set_field(Field::new(field::SENDER_COMP_ID, &self.config.sender_comp_id));
        msg.set_field(Field::new(field::TARGET_COMP_ID, &self.config.target_comp_id));
        msg
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::config::LogConfig;

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
        };

        let log_config = LogConfig {
            log_directory: PathBuf::from("/tmp"),
            log_level: "INFO".to_string(),
            log_events: true,
            log_messages: true,
        };

        let logger = Arc::new(Logger::new(&log_config));
        let store = Arc::new(MessageStore::new()); 
        let session = Session::new(config, logger, store); 

        // Test message creation
        let logon = session.create_logon_message();
        assert_eq!(logon.msg_type(), field::values::LOGON);
        assert!(logon.get_field(field::HEART_BT_INT).is_some());

        let logout = session.create_logout_message();
        assert_eq!(logout.msg_type(), field::values::LOGOUT);
    }
}