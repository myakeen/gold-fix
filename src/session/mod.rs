pub mod state;

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::net::TcpStream;
use tokio::time::{self, Duration};

use crate::config::SessionConfig;
use crate::logging::Logger;
use crate::message::{Message, field};
use crate::transport::Transport;
use crate::Result;

pub struct Session {
    config: SessionConfig,
    state: Arc<Mutex<state::SessionState>>,
    transport: Arc<Mutex<Option<Transport>>>,
    logger: Arc<Logger>,
}

impl Session {
    pub fn new(config: SessionConfig, logger: Arc<Logger>) -> Self {
        Session {
            config,
            state: Arc::new(Mutex::new(state::SessionState::new())),
            transport: Arc::new(Mutex::new(None)),
            logger,
        }
    }

    pub async fn start(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        state.set_status(state::Status::Connecting);

        // Connect to counterparty
        let stream = TcpStream::connect(&self.config.target_addr).await?;
        let mut transport = Transport::new(stream);

        // Send logon message
        let logon = self.create_logon_message();
        transport.send(&logon).await?;

        // Store the transport
        let mut transport_guard = self.transport.lock().await;
        *transport_guard = Some(transport);

        // Start heartbeat monitoring
        let transport_clone = Arc::clone(&self.transport);
        let state_clone = Arc::clone(&self.state);
        let config_clone = self.config.clone();
        let logger_clone = Arc::clone(&self.logger);

        tokio::spawn(async move {
            let heartbeat_interval = Duration::from_secs(config_clone.heart_bt_int as u64);
            let mut interval = time::interval(heartbeat_interval);

            loop {
                interval.tick().await;

                let state = state_clone.lock().await;
                if *state.status() != state::Status::Connected {
                    break;
                }
                drop(state);

                // Send heartbeat
                if let Some(mut transport) = transport_clone.lock().await.as_mut() {
                    let heartbeat = Message::new(field::values::HEARTBEAT);
                    if let Err(e) = transport.send(&heartbeat).await {
                        logger_clone.log_event("ERROR", &format!("Failed to send heartbeat: {}", e)).ok();
                        break;
                    }
                }
            }
        });

        // Start message processing loop
        let transport_clone = Arc::clone(&self.transport);
        let state_clone = Arc::clone(&self.state);
        let logger_clone = Arc::clone(&self.logger);

        tokio::spawn(async move {
            loop {
                let state = state_clone.lock().await;
                if *state.status() != state::Status::Connected {
                    break;
                }
                drop(state);

                if let Some(mut transport) = transport_clone.lock().await.as_mut() {
                    match transport.receive().await {
                        Ok(Some(msg)) => {
                            // Process received message
                            let mut state = state_clone.lock().await;
                            state.update_receive_time();
                            state.increment_incoming_seq();

                            // Log received message
                            logger_clone.log_message("IN", &msg.to_string().unwrap_or_default()).ok();

                            // Handle different message types
                            match msg.msg_type() {
                                field::values::TEST_REQUEST => {
                                    // Respond to test request with heartbeat
                                    let mut heartbeat = Message::new(field::values::HEARTBEAT);
                                    if let Some(test_req_id) = msg.get_field(field::TEST_REQ_ID) {
                                        heartbeat.set_field(field::Field::new(field::TEST_REQ_ID, test_req_id.value()));
                                    }
                                    transport.send(&heartbeat).await.ok();
                                },
                                field::values::HEARTBEAT => {
                                    // Update last receive time
                                    state.update_receive_time();
                                },
                                _ => {
                                    // Handle other message types
                                    logger_clone.log_event("INFO", &format!("Received message type: {}", msg.msg_type())).ok();
                                }
                            }
                        },
                        Ok(None) => {
                            // No message available, continue waiting
                            time::sleep(Duration::from_millis(100)).await;
                        },
                        Err(e) => {
                            logger_clone.log_event("ERROR", &format!("Error receiving message: {}", e)).ok();
                            break;
                        }
                    }
                }
            }
        });

        state.set_status(state::Status::Connected);
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
        msg.set_field(field::Field::new(field::BEGIN_STRING, &self.config.begin_string));
        msg.set_field(field::Field::new(field::SENDER_COMP_ID, &self.config.sender_comp_id));
        msg.set_field(field::Field::new(field::TARGET_COMP_ID, &self.config.target_comp_id));
        msg.set_field(field::Field::new(field::HEART_BT_INT, self.config.heart_bt_int.to_string()));
        msg.set_field(field::Field::new(field::ENCRYPT_METHOD, "0")); // No encryption
        msg
    }

    fn create_logout_message(&self) -> Message {
        let mut msg = Message::new(field::values::LOGOUT);
        msg.set_field(field::Field::new(field::BEGIN_STRING, &self.config.begin_string));
        msg.set_field(field::Field::new(field::SENDER_COMP_ID, &self.config.sender_comp_id));
        msg.set_field(field::Field::new(field::TARGET_COMP_ID, &self.config.target_comp_id));
        msg
    }
}