use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use super::Message;
use super::field::values;

const DEFAULT_POOL_SIZE: usize = 100;
const COMMON_MESSAGE_TYPES: &[&str] = &[
    values::NEW_ORDER_SINGLE,
    values::EXECUTION_REPORT,
    values::QUOTE_REQUEST,
    values::MARKET_DATA_REQUEST,
    values::HEARTBEAT,
];

/// A thread-safe pool of pre-allocated messages for improved performance
pub struct MessagePool {
    pools: Arc<Mutex<HashMap<String, Vec<Message>>>>,
}

impl MessagePool {
    pub fn new() -> Self {
        let mut pools = HashMap::new();
        
        // Pre-allocate pools for common message types
        for &msg_type in COMMON_MESSAGE_TYPES {
            let mut messages = Vec::with_capacity(DEFAULT_POOL_SIZE);
            for _ in 0..DEFAULT_POOL_SIZE {
                messages.push(Message::new(msg_type));
            }
            pools.insert(msg_type.to_string(), messages);
        }

        MessagePool {
            pools: Arc::new(Mutex::new(pools)),
        }
    }

    /// Get a message from the pool, or create a new one if the pool is empty
    pub async fn get_message(&self, msg_type: &str) -> Message {
        let mut pools = self.pools.lock().await;
        
        if let Some(pool) = pools.get_mut(msg_type) {
            if let Some(message) = pool.pop() {
                return message;
            }
        }
        
        // If no pooled message is available, create a new one
        Message::new(msg_type)
    }

    /// Return a message to the pool for reuse
    pub async fn return_message(&self, message: Message) {
        let mut pools = self.pools.lock().await;
        
        if let Some(pool) = pools.get_mut(message.msg_type()) {
            if pool.len() < DEFAULT_POOL_SIZE {
                pool.push(message);
            }
        }
    }

    /// Pre-warm the pool by ensuring it has the minimum number of messages
    pub async fn ensure_capacity(&self, msg_type: &str, capacity: usize) {
        let mut pools = self.pools.lock().await;
        
        let pool = pools.entry(msg_type.to_string())
            .or_insert_with(Vec::new);
            
        while pool.len() < capacity {
            pool.push(Message::new(msg_type));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_message_pool() {
        let pool = MessagePool::new();
        
        // Get a message from pool
        let msg1 = pool.get_message(values::NEW_ORDER_SINGLE).await;
        assert_eq!(msg1.msg_type(), values::NEW_ORDER_SINGLE);
        
        // Return message to pool
        pool.return_message(msg1).await;
        
        // Get another message (should be pooled)
        let msg2 = pool.get_message(values::NEW_ORDER_SINGLE).await;
        assert_eq!(msg2.msg_type(), values::NEW_ORDER_SINGLE);
    }

    #[tokio::test]
    async fn test_pool_capacity() {
        let pool = MessagePool::new();
        let custom_type = "CUSTOM";
        
        // Ensure capacity for custom type
        pool.ensure_capacity(custom_type, 5).await;
        
        // Get messages from pool
        let mut messages = Vec::new();
        for _ in 0..5 {
            messages.push(pool.get_message(custom_type).await);
        }
        
        // All messages should be of correct type
        for msg in messages {
            assert_eq!(msg.msg_type(), custom_type);
        }
    }
}
