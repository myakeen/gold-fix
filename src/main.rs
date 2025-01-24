use fix_engine::{FixEngine, config::EngineConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = EngineConfig::from_file("config.toml")?;
    
    // Create FIX engine instance
    let engine = FixEngine::new(config.clone());
    
    // Add sessions from config
    for session_config in config.sessions {
        engine.add_session(session_config).await?;
    }
    
    // Start the engine
    engine.start().await?;
    
    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    
    // Stop the engine
    engine.stop().await?;
    
    Ok(())
}
