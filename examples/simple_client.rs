use goldfix::{
    FixEngine,
    config::{EngineConfig, SessionConfig, LogConfig},
};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
            }
        ],
    };

    // Create and start the engine
    let engine = FixEngine::new(config);
    engine.start().await?;

    // Wait for a while
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Stop the engine
    engine.stop().await?;

    Ok(())
}