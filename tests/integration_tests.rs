use goldfix::{
    FixEngine,
    config::{EngineConfig, LogConfig},
    message::{Message, Field},
};
use std::path::PathBuf;
use tokio::time::Duration;

mod test_utils;
use test_utils::*;

#[tokio::test]
async fn test_engine_startup() {
    let config = create_test_engine_config();
    let engine = FixEngine::new(config);

    let result = engine.start().await;
    assert!(result.is_ok());

    // Proper cleanup
    let _ = engine.stop().await;
}

#[tokio::test]
async fn test_initiator_session() {
    let mut config = create_test_engine_config();
    config.sessions = vec![create_test_initiator_config()];
    let engine = FixEngine::new(config);

    let result = engine.start().await;
    assert!(result.is_ok());

    // Allow time for connection establishment
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Proper cleanup
    let _ = engine.stop().await;
}

#[tokio::test]
async fn test_acceptor_session() {
    let mut config = create_test_engine_config();
    config.sessions = vec![create_test_acceptor_config()];
    let engine = FixEngine::new(config);

    let result = engine.start().await;
    assert!(result.is_ok());

    // Allow time for connection setup
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Proper cleanup
    let _ = engine.stop().await;
}

#[tokio::test]
async fn test_message_creation() {
    let mut message = Message::new("D"); // New Order Single
    let _ = message.set_field(Field::new(11, "12345")); // ClOrdID
    let _ = message.set_field(Field::new(55, "IBM")); // Symbol

    let msg_str = message.to_string().unwrap();
    assert!(msg_str.contains("35=D"));
    assert!(msg_str.contains("11=12345"));
    assert!(msg_str.contains("55=IBM"));
}

#[tokio::test]
async fn test_session_roles() {
    // Test initiator role
    let initiator_config = create_test_initiator_config();
    assert!(initiator_config.is_initiator());
    assert!(!initiator_config.is_acceptor());

    // Test acceptor role
    let acceptor_config = create_test_acceptor_config();
    assert!(acceptor_config.is_acceptor());
    assert!(!acceptor_config.is_initiator());
}

#[tokio::test]
async fn test_session_connection() {
    let (initiator_config, acceptor_config) = create_test_session_pair().await.unwrap();

    let mut engine_config = create_test_engine_config();
    engine_config.sessions = vec![initiator_config, acceptor_config];

    let engine = FixEngine::new(engine_config);
    let result = engine.start().await;
    assert!(result.is_ok());

    // Allow time for connection establishment
    tokio::time::sleep(Duration::from_secs(1)).await;

    let session_ids = engine.get_session_ids().await;
    assert_eq!(session_ids.len(), 2);

    // Proper cleanup
    let _ = engine.stop().await;
}

fn create_test_engine_config() -> EngineConfig {
    EngineConfig {
        log_config: LogConfig {
            log_directory: PathBuf::from("/tmp"),
            log_level: "INFO".to_string(),
            log_events: true,
            log_messages: true,
        },
        sessions: vec![], // Initialize with an empty vector
    }
}