use goldfix::{
    FixEngine,
    config::{EngineConfig, SessionConfig, LogConfig},
    transport::TransportConfig,
};
use std::path::PathBuf;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create SSL/TLS transport configuration
    let transport_config = TransportConfig {
        use_ssl: true,
        cert_file: Some(PathBuf::from("certs/client.crt")),
        key_file: Some(PathBuf::from("certs/client.key")),
        ca_file: Some(PathBuf::from("certs/ca.crt")),
        verify_peer: true,
        buffer_size: 8192,  // Increased buffer size
        connection_timeout: Duration::from_secs(30),
    };

    // Create engine configuration
    let config = EngineConfig {
        log_config: LogConfig {
            log_directory: PathBuf::from("logs"),
            log_level: "DEBUG".to_string(),
            log_events: true,
            log_messages: true,
        },
        sessions: vec![
            SessionConfig {
                begin_string: "FIX.4.2".to_string(),
                sender_comp_id: "SSL_CLIENT".to_string(),
                target_comp_id: "SSL_SERVER".to_string(),
                target_addr: "127.0.0.1:8443".to_string(),
                heart_bt_int: 30,
                reset_on_logon: true,
                reset_on_logout: true,
                reset_on_disconnect: true,
                transport_config: Some(transport_config),
            }
        ],
    };

    // Create and start the engine
    let engine = FixEngine::new(config);
    engine.start().await?;

    // Wait for initialization
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Test error scenarios
    test_error_scenarios(&engine).await?;

    // Stop the engine
    engine.stop().await?;

    Ok(())
}

async fn test_error_scenarios(engine: &FixEngine) -> Result<(), Box<dyn std::error::Error>> {
    // Test invalid certificate handling
    let message_pool = engine.message_pool();

    // 1. Test with invalid certificate (should fail)
    let config_invalid_cert = TransportConfig {
        use_ssl: true,
        cert_file: Some(PathBuf::from("certs/invalid.crt")),
        key_file: Some(PathBuf::from("certs/invalid.key")),
        ca_file: Some(PathBuf::from("certs/ca.crt")),
        verify_peer: true,
        buffer_size: 8192,
        connection_timeout: Duration::from_secs(30),
    };

    // Add session with invalid cert (should fail)
    let result = engine.add_session(SessionConfig {
        begin_string: "FIX.4.2".to_string(),
        sender_comp_id: "INVALID_CERT_CLIENT".to_string(),
        target_comp_id: "SSL_SERVER".to_string(),
        target_addr: "127.0.0.1:8443".to_string(),
        heart_bt_int: 30,
        reset_on_logon: true,
        reset_on_logout: true,
        reset_on_disconnect: true,
        transport_config: Some(config_invalid_cert),
    }).await;

    assert!(result.is_err(), "Expected error with invalid certificate");

    // 2. Test with expired certificate scenario
    // In real implementation, you would use an actual expired certificate

    // 3. Test with revoked certificate scenario
    // In real implementation, you would use an actual revoked certificate

    // 4. Test with mismatched hostname in certificate
    let config_wrong_hostname = TransportConfig {
        use_ssl: true,
        cert_file: Some(PathBuf::from("certs/client.crt")),
        key_file: Some(PathBuf::from("certs/client.key")),
        ca_file: Some(PathBuf::from("certs/ca.crt")),
        verify_peer: true,
        buffer_size: 8192,
        connection_timeout: Duration::from_secs(30),
    };

    let result = engine.add_session(SessionConfig {
        begin_string: "FIX.4.2".to_string(),
        sender_comp_id: "WRONG_HOST_CLIENT".to_string(),
        target_comp_id: "SSL_SERVER".to_string(),
        target_addr: "wrong.example.com:8443".to_string(),
        heart_bt_int: 30,
        reset_on_logon: true,
        reset_on_logout: true,
        reset_on_disconnect: true,
        transport_config: Some(config_wrong_hostname),
    }).await;

    assert!(result.is_err(), "Expected error with wrong hostname");

    Ok(())
}