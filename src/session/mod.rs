pub mod state;

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::net::TcpStream;

use crate::config::SessionConfig;
use crate::logging::Logger;
use crate::message::Message;
use crate::transport::Transport;
use crate::Result;
use crate::message::field;


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
        let mut msg = Message::new("A");  // A = Logon
        // Add required logon fields
        msg.set_field(field::Field::new(field::BEGIN_STRING, &self.config.begin_string));
        msg.set_field(field::Field::new(field::SENDER_COMP_ID, &self.config.sender_comp_id));
        msg.set_field(field::Field::new(field::TARGET_COMP_ID, &self.config.target_comp_id));
        msg.set_field(field::Field::new(field::HEART_BT_INT, self.config.heart_bt_int.to_string()));
        msg
    }

    fn create_logout_message(&self) -> Message {
        let mut msg = Message::new("5");  // 5 = Logout
        msg.set_field(field::Field::new(field::BEGIN_STRING, &self.config.begin_string));
        msg.set_field(field::Field::new(field::SENDER_COMP_ID, &self.config.sender_comp_id));
        msg.set_field(field::Field::new(field::TARGET_COMP_ID, &self.config.target_comp_id));
        msg
    }
}