use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use super::Message;
use super::field::values;

// Configuration for message pools
pub struct PoolConfig {
    default_size: usize,
    type_specific_sizes: HashMap<String, usize>,
    max_pool_size: usize,
}

impl Default for PoolConfig {
    fn default() -> Self {
        let mut type_specific_sizes = HashMap::new();
        type_specific_sizes.insert(values::MARKET_DATA_REQUEST.to_string(), 200);
        type_specific_sizes.insert(values::QUOTE_REQUEST.to_string(), 150);
        type_specific_sizes.insert(values::NEW_ORDER_SINGLE.to_string(), 100);

        PoolConfig {
            default_size: 50,
            type_specific_sizes,
            max_pool_size: 1000,
        }
    }
}

// Statistics for pool monitoring
#[derive(Debug, Clone)]
pub struct PoolStats {
    hits: usize,
    misses: usize,
    returns: usize,
    current_size: usize,
}

/// A thread-safe pool of pre-allocated messages for improved performance
pub struct MessagePool {
    pools: Arc<Mutex<HashMap<String, Vec<Message>>>>,
    config: PoolConfig,
    stats: Arc<Mutex<HashMap<String, PoolStats>>>,
}

impl MessagePool {
    pub fn new() -> Self {
        Self::with_config(PoolConfig::default())
    }

    pub fn with_config(config: PoolConfig) -> Self {
        let mut pools = HashMap::new();
        let mut stats = HashMap::new();

        // Pre-allocate pools for common message types
        for (&msg_type, &size) in values::COMMON_MESSAGE_TYPES.iter()
            .chain(config.type_specific_sizes.iter().map(|(k, v)| (k.as_str(), v)))
        {
            let size = config.type_specific_sizes
                .get(msg_type)
                .copied()
                .unwrap_or(config.default_size);

            let mut messages = Vec::with_capacity(size);
            for _ in 0..size {
                messages.push(Message::new(msg_type));
            }
            pools.insert(msg_type.to_string(), messages);

            stats.insert(msg_type.to_string(), PoolStats {
                hits: 0,
                misses: 0,
                returns: 0,
                current_size: size,
            });
        }

        MessagePool {
            pools: Arc::new(Mutex::new(pools)),
            config,
            stats: Arc::new(Mutex::new(stats)),
        }
    }

    /// Get a message from the pool, or create a new one if the pool is empty
    pub async fn get_message(&self, msg_type: &str) -> Message {
        let mut pools = self.pools.lock().await;
        let mut stats = self.stats.lock().await;

        if let Some(pool) = pools.get_mut(msg_type) {
            if let Some(message) = pool.pop() {
                if let Some(stat) = stats.get_mut(msg_type) {
                    stat.hits += 1;
                    stat.current_size -= 1;
                }
                return message;
            }
        }

        // Log cache miss
        if let Some(stat) = stats.get_mut(msg_type) {
            stat.misses += 1;
        }

        // If no pooled message is available, create a new one
        Message::new(msg_type)
    }

    /// Return a message to the pool for reuse
    pub async fn return_message(&self, message: Message) {
        let mut pools = self.pools.lock().await;
        let mut stats = self.stats.lock().await;
        let msg_type = message.msg_type().to_string(); // Clone the message type

        if let Some(pool) = pools.get_mut(&msg_type) {
            let max_size = self.config.type_specific_sizes
                .get(&msg_type)
                .copied()
                .unwrap_or(self.config.default_size);

            if pool.len() < max_size && pool.len() < self.config.max_pool_size {
                pool.push(message);
                if let Some(stat) = stats.get_mut(&msg_type) {
                    stat.returns += 1;
                    stat.current_size += 1;
                }
            }
        }
    }

    /// Get statistics for a specific message type
    pub async fn get_stats(&self, msg_type: &str) -> Option<PoolStats> {
        self.stats.lock().await.get(msg_type).cloned()
    }

    /// Pre-warm the pool by ensuring it has the minimum number of messages
    pub async fn ensure_capacity(&self, msg_type: &str, capacity: usize) {
        let mut pools = self.pools.lock().await;
        let mut stats = self.stats.lock().await;

        let pool = pools.entry(msg_type.to_string())
            .or_insert_with(Vec::new);

        let target_capacity = capacity.min(self.config.max_pool_size);

        while pool.len() < target_capacity {
            pool.push(Message::new(msg_type));
        }

        if let Some(stat) = stats.get_mut(msg_type) {
            stat.current_size = pool.len();
        }
    }

    /// Resize the pool for a specific message type
    pub async fn resize_pool(&self, msg_type: &str, new_size: usize) {
        let mut pools = self.pools.lock().await;
        let mut stats = self.stats.lock().await;

        if let Some(pool) = pools.get_mut(msg_type) {
            let target_size = new_size.min(self.config.max_pool_size);

            // Grow pool if needed
            while pool.len() < target_size {
                pool.push(Message::new(msg_type));
            }

            // Shrink pool if needed
            if pool.len() > target_size {
                pool.truncate(target_size);
            }

            if let Some(stat) = stats.get_mut(msg_type) {
                stat.current_size = pool.len();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_message_pool_with_config() {
        let mut config = PoolConfig::default();
        config.type_specific_sizes.insert(values::NEW_ORDER_SINGLE.to_string(), 5);

        let pool = MessagePool::with_config(config);

        // Get messages until pool is empty
        for _ in 0..6 {
            let msg = pool.get_message(values::NEW_ORDER_SINGLE).await;
            assert_eq!(msg.msg_type(), values::NEW_ORDER_SINGLE);
        }

        // Check stats
        let stats = pool.get_stats(values::NEW_ORDER_SINGLE).await.unwrap();
        assert_eq!(stats.hits, 5); // First 5 from pool
        assert_eq!(stats.misses, 1); // 6th created new
    }

    #[tokio::test]
    async fn test_pool_resize() {
        let pool = MessagePool::new();

        // Initial size
        pool.ensure_capacity(values::QUOTE_REQUEST, 5).await;
        let stats = pool.get_stats(values::QUOTE_REQUEST).await.unwrap();
        assert_eq!(stats.current_size, 5);

        // Resize larger
        pool.resize_pool(values::QUOTE_REQUEST, 10).await;
        let stats = pool.get_stats(values::QUOTE_REQUEST).await.unwrap();
        assert_eq!(stats.current_size, 10);

        // Resize smaller
        pool.resize_pool(values::QUOTE_REQUEST, 3).await;
        let stats = pool.get_stats(values::QUOTE_REQUEST).await.unwrap();
        assert_eq!(stats.current_size, 3);
    }

    #[tokio::test]
    async fn test_message_recycling() {
        let pool = MessagePool::new();

        let msg1 = pool.get_message(values::NEW_ORDER_SINGLE).await;
        pool.return_message(msg1).await;

        let msg2 = pool.get_message(values::NEW_ORDER_SINGLE).await;
        assert_eq!(msg2.msg_type(), values::NEW_ORDER_SINGLE);

        let stats = pool.get_stats(values::NEW_ORDER_SINGLE).await.unwrap();
        assert_eq!(stats.returns, 1);
        assert_eq!(stats.hits, 1);
    }

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