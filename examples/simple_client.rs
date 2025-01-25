use goldfix::{
    FixEngine,
    config::{EngineConfig, SessionConfig, LogConfig, SessionRole},
    transport::TransportConfig,
};
use std::path::PathBuf;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create transport configuration with SSL/TLS support
    let transport_config = TransportConfig {
        use_ssl: true,
        cert_file: Some(PathBuf::from("certs/client.crt")),
        key_file: Some(PathBuf::from("certs/client.key")),
        ca_file: Some(PathBuf::from("certs/ca.crt")),
        verify_peer: true,
        buffer_size: 4096,
        connection_timeout: Duration::from_secs(30),
    };

    // Create configuration
    let config = EngineConfig {
        log_config: LogConfig {
            log_directory: PathBuf::from("/tmp"),
            log_level: "INFO".to_string(),
            log_events: true,
            log_messages: true,
        },
        sessions: vec![
            SessionConfig {
                begin_string: "FIX.4.2".to_string(),
                sender_comp_id: "CLIENT".to_string(),
                target_comp_id: "SERVER".to_string(),
                target_addr: "127.0.0.1:8000".to_string(),
                heart_bt_int: 30,
                reset_on_logon: true,
                reset_on_logout: true,
                reset_on_disconnect: true,
                transport_config: Some(transport_config),
                role: SessionRole::Initiator,  // Explicitly set as initiator
            }
        ],
    };

    // Create and start the engine
    let engine = FixEngine::new(config);
    engine.start().await?;

    // Wait for a while to observe the connection
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Stop the engine
    engine.stop().await?;

    Ok(())
}