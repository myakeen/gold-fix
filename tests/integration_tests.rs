use goldfix::{
    FixEngine,
    config::{EngineConfig, SessionConfig, LogConfig},
    message::{Message, Field},
};
use std::path::PathBuf;

#[tokio::test]
async fn test_engine_startup() {
    let config = create_test_config();
    let engine = FixEngine::new(config);

    let result = engine.start().await;
    assert!(result.is_ok());
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

fn create_test_config() -> EngineConfig {
    EngineConfig {
        log_config: LogConfig {
            log_directory: PathBuf::from("/tmp"),
            log_level: "INFO".to_string(),
            log_events: true,
            log_messages: true,
        },
        sessions: vec![
            SessionConfig {
                begin_string: "FIX.4.2".to_string(),
                sender_comp_id: "TEST_SENDER".to_string(),
                target_comp_id: "TEST_TARGET".to_string(),
                target_addr: "127.0.0.1:8001".to_string(),
                heart_bt_int: 30,
                reset_on_logon: true,
                reset_on_logout: true,
                reset_on_disconnect: true,
            }
        ],
    }
}