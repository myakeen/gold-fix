use goldfix::{
    config::{EngineConfig, SessionConfig, LogConfig, SessionRole},
    transport::TransportConfig,
};
use std::path::PathBuf;
use std::time::Duration;
use tokio::net::TcpListener;

/// Creates a test configuration for an initiator (client) session
pub fn create_test_initiator_config() -> SessionConfig {
    let transport_config = TransportConfig {
        use_ssl: false,
        cert_file: None,
        key_file: None,
        ca_file: None,
        verify_peer: false,
        buffer_size: 4096,
        connection_timeout: Duration::from_secs(30),
    };

    SessionConfig {
        begin_string: "FIX.4.2".to_string(),
        sender_comp_id: "TEST_INITIATOR".to_string(),
        target_comp_id: "TEST_ACCEPTOR".to_string(),
        target_addr: "127.0.0.1:0".to_string(),
        heart_bt_int: 30,
        reset_on_logon: true,
        reset_on_logout: true,
        reset_on_disconnect: true,
        transport_config: Some(transport_config),
        role: SessionRole::Initiator,
    }
}

/// Creates a test configuration for an acceptor (server) session
pub fn create_test_acceptor_config() -> SessionConfig {
    let transport_config = TransportConfig {
        use_ssl: false,
        cert_file: None,
        key_file: None,
        ca_file: None,
        verify_peer: false,
        buffer_size: 4096,
        connection_timeout: Duration::from_secs(30),
    };

    SessionConfig {
        begin_string: "FIX.4.2".to_string(),
        sender_comp_id: "TEST_ACCEPTOR".to_string(),
        target_comp_id: "TEST_INITIATOR".to_string(),
        target_addr: "127.0.0.1:0".to_string(),
        heart_bt_int: 30,
        reset_on_logon: true,
        reset_on_logout: true,
        reset_on_disconnect: true,
        transport_config: Some(transport_config),
        role: SessionRole::Acceptor,
    }
}

/// Creates a test engine configuration with both initiator and acceptor sessions
pub fn create_test_engine_config() -> EngineConfig {
    EngineConfig {
        log_config: LogConfig {
            log_directory: PathBuf::from("/tmp"),
            log_level: "DEBUG".to_string(),
            log_events: true,
            log_messages: true,
        },
        sessions: vec![
            create_test_initiator_config(),
            create_test_acceptor_config(),
        ],
    }
}

/// Creates a TCP listener for testing
pub async fn create_test_listener() -> std::io::Result<(TcpListener, String)> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    Ok((listener, addr.to_string()))
}

/// Mock message store for testing
#[derive(Clone)]
pub struct MockMessageStore;

impl MockMessageStore {
    pub fn new() -> Self {
        MockMessageStore
    }
}

/// Test utilities for session testing
pub mod session_utils {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    /// Creates a pair of connected initiator and acceptor sessions for testing
    pub async fn create_test_session_pair() -> std::io::Result<(SessionConfig, SessionConfig)> {
        let (listener, addr) = create_test_listener().await?;
        
        let mut initiator_config = create_test_initiator_config();
        initiator_config.target_addr = addr;
        
        let mut acceptor_config = create_test_acceptor_config();
        acceptor_config.target_addr = addr;
        
        Ok((initiator_config, acceptor_config))
    }
}
