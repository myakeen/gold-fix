use goldfix::{
    FixEngine,
    config::{EngineConfig, SessionConfig, LogConfig},
    transport::TransportConfig,
    message::{Field, field},
};
use std::path::PathBuf;
use std::time::Duration;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Test case 1: Invalid certificate handling
    println!("Testing invalid certificate handling...");
    let config = TransportConfig {
        use_ssl: true,
        cert_file: Some(PathBuf::from("certs/invalid.crt")),
        key_file: Some(PathBuf::from("certs/invalid.key")),
        ca_file: Some(PathBuf::from("certs/ca.crt")),
        verify_peer: true,
        buffer_size: 4096,
        connection_timeout: Duration::from_secs(30),
    };

    let session_config = SessionConfig {
        begin_string: "FIX.4.2".to_string(),
        sender_comp_id: "EDGE_CLIENT".to_string(),
        target_comp_id: "EDGE_SERVER".to_string(),
        target_addr: "127.0.0.1:8444".to_string(),
        heart_bt_int: 30,
        reset_on_logon: true,
        reset_on_logout: true,
        reset_on_disconnect: true,
        transport_config: Some(config),
    };

    // Test case 2: Connection timeout
    println!("Testing connection timeout...");
    let timeout_config = TransportConfig {
        use_ssl: false,
        cert_file: None,
        key_file: None,
        ca_file: None,
        verify_peer: false,
        buffer_size: 4096,
        connection_timeout: Duration::from_millis(100),
    };

    let timeout_session = SessionConfig {
        begin_string: "FIX.4.2".to_string(),
        sender_comp_id: "TIMEOUT_CLIENT".to_string(),
        target_comp_id: "TIMEOUT_SERVER".to_string(),
        target_addr: "192.0.2.1:12345".to_string(), // Non-existent address
        heart_bt_int: 30,
        reset_on_logon: true,
        reset_on_logout: true,
        reset_on_disconnect: true,
        transport_config: Some(timeout_config),
    };

    // Test case 3: Buffer overflow handling
    println!("Testing buffer overflow handling...");
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let buffer_config = TransportConfig {
        use_ssl: false,
        cert_file: None,
        key_file: None,
        ca_file: None,
        verify_peer: false,
        buffer_size: 16, // Very small buffer to test overflow
        connection_timeout: Duration::from_secs(30),
    };

    let buffer_session = SessionConfig {
        begin_string: "FIX.4.2".to_string(),
        sender_comp_id: "BUFFER_CLIENT".to_string(),
        target_comp_id: "BUFFER_SERVER".to_string(),
        target_addr: addr.to_string(),
        heart_bt_int: 30,
        reset_on_logon: true,
        reset_on_logout: true,
        reset_on_disconnect: true,
        transport_config: Some(buffer_config),
    };

    // Create and configure the engine
    let engine_config = EngineConfig {
        log_config: LogConfig {
            log_directory: PathBuf::from("logs"),
            log_level: "DEBUG".to_string(),
            log_events: true,
            log_messages: true,
        },
        sessions: vec![
            session_config,
            timeout_session,
            buffer_session,
        ],
    };

    let engine = FixEngine::new(engine_config);

    // Start the engine and test scenarios
    engine.start().await?;

    // Wait for test completion
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Stop the engine
    engine.stop().await?;

    Ok(())
}
