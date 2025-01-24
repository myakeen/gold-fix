pub mod state;

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::net::TcpStream;

use crate::config::SessionConfig;
use crate::logging::Logger;
use crate::message::Message;
use crate::transport::Transport;
use crate::Result;

pub struct Session {
    config: SessionConfig,
    state: Arc<Mutex<state::SessionState>>,
    transport: Option<Transport>,
    logger: Arc<Logger>,
}

impl Session {
    pub fn new(config: SessionConfig, logger: Arc<Logger>) -> Self {
        Session {
            config,
            state: Arc::new(Mutex::new(state::SessionState::new())),
            transport: None,
            logger,
        }
    }

    pub async fn start(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        state.set_status(state::Status::Connecting);
        
        // Connect to counterparty
        let stream = TcpStream::connect(&self.config.target_addr).await?;
        let transport = Transport::new(stream);
        
        // Send logon message
        let logon = self.create_logon_message();
        transport.send(&logon).await?;
        
        state.set_status(state::Status::Connected);
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        state.set_status(state::Status::Disconnecting);
        
        // Send logout message if connected
        if let Some(transport) = &self.transport {
            let logout = self.create_logout_message();
            transport.send(&logout).await?;
        }
        
        state.set_status(state::Status::Disconnected);
        Ok(())
    }

    fn create_logon_message(&self) -> Message {
        let mut msg = Message::new("A");  // A = Logon
        // Add required logon fields
        // Implementation details here
        msg
    }

    fn create_logout_message(&self) -> Message {
        let mut msg = Message::new("5");  // 5 = Logout
        // Add required logout fields
        // Implementation details here
        msg
    }
}
