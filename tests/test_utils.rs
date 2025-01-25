use goldfix::{
    config::{SessionConfig, SessionRole},
};
use tokio::net::TcpListener;
use std::time::Duration;

/// Creates a test configuration for an initiator (client) session
pub fn create_test_initiator_config() -> SessionConfig {
    SessionConfig {
        begin_string: "FIX.4.2".to_string(),
        sender_comp_id: "TEST_INITIATOR".to_string(),
        target_comp_id: "TEST_ACCEPTOR".to_string(),
        target_addr: "127.0.0.1:0".to_string(),
        heart_bt_int: 30,
        reset_on_logon: true,
        reset_on_logout: true,
        reset_on_disconnect: true,
        transport_config: None,
        role: SessionRole::Initiator,
    }
}

/// Creates a test configuration for an acceptor (server) session
pub fn create_test_acceptor_config() -> SessionConfig {
    SessionConfig {
        begin_string: "FIX.4.2".to_string(),
        sender_comp_id: "TEST_ACCEPTOR".to_string(),
        target_comp_id: "TEST_INITIATOR".to_string(),
        target_addr: "127.0.0.1:0".to_string(),
        heart_bt_int: 30,
        reset_on_logon: true,
        reset_on_logout: true,
        reset_on_disconnect: true,
        transport_config: None,
        role: SessionRole::Acceptor,
    }
}

/// Creates a TCP listener for testing
pub async fn create_test_listener() -> std::io::Result<(TcpListener, String)> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?.to_string();
    Ok((listener, addr))
}

/// Creates a pair of connected initiator and acceptor sessions for testing
pub async fn create_test_session_pair() -> std::io::Result<(SessionConfig, SessionConfig)> {
    let (listener, addr) = create_test_listener().await?;
    drop(listener); // We don't need the listener anymore

    let mut initiator_config = create_test_initiator_config();
    initiator_config.target_addr = addr.clone();

    let mut acceptor_config = create_test_acceptor_config();
    acceptor_config.target_addr = addr;

    Ok((initiator_config, acceptor_config))
}