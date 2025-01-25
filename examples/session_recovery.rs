use goldfix::{
    FixEngine,
    config::{EngineConfig, SessionConfig, LogConfig, SessionRole},
    transport::TransportConfig,
    message::{Field, field},
    session::state,  // Fixed import
};
use std::path::PathBuf;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transport_config = TransportConfig {
        use_ssl: false,
        cert_file: None,
        key_file: None,
        ca_file: None,
        verify_peer: false,
        buffer_size: 4096,
        connection_timeout: Duration::from_secs(30),
    };

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
                sender_comp_id: "RECOVERY_CLIENT".to_string(),
                target_comp_id: "RECOVERY_SERVER".to_string(),
                target_addr: "127.0.0.1:8001".to_string(),
                heart_bt_int: 30,
                reset_on_logon: false,  // Don't reset on logon for recovery testing
                reset_on_logout: false,
                reset_on_disconnect: false,
                transport_config: Some(transport_config),
                role: SessionRole::Initiator,
            }
        ],
    };

    // Create and start the engine
    let engine = FixEngine::new(config);
    engine.start().await?;

    // Wait for initial connection
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Test session recovery scenarios
    test_recovery_scenarios(&engine).await?;

    // Stop the engine
    engine.stop().await?;

    Ok(())
}

async fn test_recovery_scenarios(engine: &FixEngine) -> Result<(), Box<dyn std::error::Error>> {
    // Get message pool for creating test messages
    let message_pool = engine.message_pool();

    // Test 1: Message persistence
    println!("Testing message persistence...");
    // Send a batch of messages before disconnection
    for i in 1..=5 {
        let mut msg = message_pool.get_message(field::values::NEW_ORDER_SINGLE).await;
        msg.set_field(Field::new(field::ORDER_ID, &format!("ORDER_{}", i)))?;
        msg.set_field(Field::new(field::SYMBOL, "AAPL"))?;
        msg.set_field(Field::new(field::SIDE, "1"))?;  // Buy
        msg.set_field(Field::new(field::ORDER_QTY, "100"))?;
        msg.set_field(Field::new(field::PRICE, "150.50"))?;
        msg.set_field(Field::new(field::ORD_TYPE, "2"))?;  // Limit order
        // Message will be automatically returned to pool when dropped
    }

    // Test 2: Sequence number recovery
    println!("Testing sequence number recovery...");
    // Simulate network disconnection
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Test 3: Session state recovery
    println!("Testing session state recovery...");
    // Force disconnect and reconnect
    let session = engine.get_session("RECOVERY_CLIENT_RECOVERY_SERVER").await?;
    session.disconnect().await?;
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Test aggressive reconnection
    for _ in 0..3 {
        session.recover().await?;
        tokio::time::sleep(Duration::from_millis(500)).await;
        session.disconnect().await?;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Final recovery attempt
    session.recover().await?;
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify final state
    assert!(session.is_connected().await, "Session should be connected after recovery");
    let state = session.get_state().await?;
    assert_eq!(*state.status(), state::Status::Connected);

    Ok(())
}