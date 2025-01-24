use goldfix::{
    FixEngine,
    config::{EngineConfig, SessionConfig, LogConfig},
    transport::TransportConfig,
    message::{Message, Field, field},
};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const NUM_MESSAGES: usize = 10_000;
const BATCH_SIZE: usize = 100;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transport_config = TransportConfig {
        use_ssl: false,
        cert_file: None,
        key_file: None,
        ca_file: None,
        verify_peer: false,
        buffer_size: 16384,  // Larger buffer for performance
        connection_timeout: Duration::from_secs(30),
    };

    let config = EngineConfig {
        log_config: LogConfig {
            log_directory: PathBuf::from("logs"),
            log_level: "INFO".to_string(),
            log_events: true,
            log_messages: true,
        },
        sessions: vec![
            SessionConfig {
                begin_string: "FIX.4.2".to_string(),
                sender_comp_id: "PERF_CLIENT".to_string(),
                target_comp_id: "PERF_SERVER".to_string(),
                target_addr: "127.0.0.1:8002".to_string(),
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

    // Wait for connection establishment
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Run performance tests
    println!("Starting performance test...");
    let start = Instant::now();
    test_message_throughput(&engine).await?;
    let duration = start.elapsed();
    
    println!("Performance test results:");
    println!("Total messages: {}", NUM_MESSAGES);
    println!("Total time: {:?}", duration);
    println!("Messages per second: {:.2}", NUM_MESSAGES as f64 / duration.as_secs_f64());

    // Stop the engine
    engine.stop().await?;

    Ok(())
}

async fn test_message_throughput(engine: &FixEngine) -> Result<(), Box<dyn std::error::Error>> {
    let message_pool = engine.message_pool();
    let (tx, mut rx) = mpsc::channel(BATCH_SIZE);
    let processed_count = Arc::new(AtomicUsize::new(0));
    let processed_count_clone = processed_count.clone();

    // Spawn message processor
    tokio::spawn(async move {
        while let Some(_) = rx.recv().await {
            processed_count_clone.fetch_add(1, Ordering::SeqCst);
        }
    });

    // Send messages in batches
    for batch in 0..(NUM_MESSAGES / BATCH_SIZE) {
        let mut futures = Vec::with_capacity(BATCH_SIZE);
        
        for i in 0..BATCH_SIZE {
            let msg_id = batch * BATCH_SIZE + i;
            let mut msg = message_pool.get_message(field::values::MARKET_DATA_REQUEST).await;
            msg.set_field(Field::new(field::MD_REQ_ID, &format!("REQ_{}", msg_id)))?;
            
            let tx = tx.clone();
            futures.push(async move {
                let _ = tx.send(()).await;
                message_pool.return_message(msg).await;
            });
        }

        futures::future::join_all(futures).await;
    }

    // Wait for all messages to be processed
    while processed_count.load(Ordering::SeqCst) < NUM_MESSAGES {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Ok(())
}
