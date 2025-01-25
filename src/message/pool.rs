use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use super::Message;
use super::field::values;

// Configuration for message pools
#[derive(Clone)]
pub struct PoolConfig {
    default_size: usize,
    type_specific_sizes: HashMap<String, usize>,
    max_pool_size: usize,
}

impl Default for PoolConfig {
    fn default() -> Self {
        let mut type_specific_sizes = HashMap::new();
        // Default pool sizes for common message types
        type_specific_sizes.insert(values::MARKET_DATA_REQUEST.to_string(), 100);
        type_specific_sizes.insert(values::MARKET_DATA_SNAPSHOT.to_string(), 100);
        type_specific_sizes.insert(values::QUOTE_REQUEST.to_string(), 50);
        type_specific_sizes.insert(values::QUOTE.to_string(), 50);
        type_specific_sizes.insert(values::NEW_ORDER_SINGLE.to_string(), 50);
        type_specific_sizes.insert(values::EXECUTION_REPORT.to_string(), 50);

        PoolConfig {
            default_size: 50,
            type_specific_sizes,
            max_pool_size: 1000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    hits: usize,
    misses: usize,
    returns: usize,
    current_size: usize,
    peak_size: usize,
    total_allocations: usize,
}

impl PoolStats {
    fn new() -> Self {
        PoolStats {
            hits: 0,
            misses: 0,
            returns: 0,
            current_size: 0,
            peak_size: 0,
            total_allocations: 0,
        }
    }

    fn update_peak_size(&mut self, size: usize) {
        if size > self.peak_size {
            self.peak_size = size;
        }
    }
}

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
        let pools = Arc::new(Mutex::new(HashMap::new()));
        let stats = Arc::new(Mutex::new(HashMap::new()));

        MessagePool {
            pools,
            config,
            stats,
        }
    }

    pub async fn get_message(&self, msg_type: &str) -> Message {
        let mut pools = self.pools.lock().await;
        let mut stats = self.stats.lock().await;

        let pool = pools.entry(msg_type.to_string())
            .or_insert_with(Vec::new);

        let stat = stats.entry(msg_type.to_string())
            .or_insert_with(PoolStats::new);

        match pool.pop() {
            Some(message) => {
                stat.hits += 1;
                stat.current_size = pool.len();
                message
            },
            None => {
                stat.misses += 1;
                stat.total_allocations += 1;
                Message::new(msg_type)
            }
        }
    }

    pub async fn return_message(&self, message: Message) {
        let mut pools = self.pools.lock().await;
        let mut stats = self.stats.lock().await;
        let msg_type = message.msg_type().to_string();

        let pool = pools.entry(msg_type.clone())
            .or_insert_with(Vec::new);

        let stat = stats.entry(msg_type)
            .or_insert_with(PoolStats::new);

        let max_size = self.config.type_specific_sizes
            .get(&message.msg_type().to_string())
            .copied()
            .unwrap_or(self.config.default_size);

        // Don't add if pool is at or above target size
        let target_size = (max_size / 2).min(self.config.max_pool_size);
        if pool.len() < target_size {
            pool.push(message);
            stat.returns += 1;
            stat.current_size = pool.len();
            stat.update_peak_size(pool.len());
        }

        // Always ensure pool size is within limits
        if pool.len() > target_size {
            pool.truncate(target_size);
            stat.current_size = pool.len();
        }
    }

    pub async fn resize_pool(&self, msg_type: &str, new_size: usize) {
        let mut pools = self.pools.lock().await;
        let mut stats = self.stats.lock().await;

        let pool = pools.entry(msg_type.to_string())
            .or_insert_with(Vec::new);
        let stat = stats.entry(msg_type.to_string())
            .or_insert_with(PoolStats::new);

        let target_size = new_size.min(self.config.max_pool_size);

        while pool.len() < target_size {
            pool.push(Message::new(msg_type));
            stat.total_allocations += 1;
        }

        if pool.len() > target_size {
            pool.truncate(target_size);
        }

        stat.current_size = pool.len();
        stat.update_peak_size(pool.len());
    }

    pub async fn get_stats(&self, msg_type: &str) -> Option<PoolStats> {
        self.stats.lock().await.get(msg_type).cloned()
    }

    pub async fn ensure_capacity(&self, msg_type: &str, capacity: usize) {
        self.resize_pool(msg_type, capacity).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_message_recycling() {
        let pool = MessagePool::new();

        // Get a message and return it
        let msg1 = pool.get_message(values::NEW_ORDER_SINGLE).await;
        pool.return_message(msg1).await;

        // Get another message - should be from pool (hit)
        let msg2 = pool.get_message(values::NEW_ORDER_SINGLE).await;
        assert_eq!(msg2.msg_type(), values::NEW_ORDER_SINGLE);

        let stats = pool.get_stats(values::NEW_ORDER_SINGLE).await.unwrap();
        assert_eq!(stats.hits, 1, "Expected 1 hit from pool");
        assert_eq!(stats.returns, 1, "Expected 1 message return");
    }

    #[tokio::test]
    async fn test_pool_resize() {
        let pool = MessagePool::new();

        // Initial resize to 5
        pool.resize_pool(values::QUOTE_REQUEST, 5).await;
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
    async fn test_pool_cleanup() {
        let pool = MessagePool::new();
        let msg_type = values::MARKET_DATA_REQUEST;

        // Fill pool to max size
        let max_size = 10;
        pool.resize_pool(msg_type, max_size).await;

        // Get all messages
        let mut messages = Vec::new();
        for _ in 0..max_size {
            messages.push(pool.get_message(msg_type).await);
        }

        // Return all messages
        for msg in messages {
            pool.return_message(msg).await;
        }

        // Verify cleanup occurred
        let stats = pool.get_stats(msg_type).await.unwrap();
        assert!(stats.current_size <= max_size / 2, 
            "Pool size should be reduced after cleanup");
    }

    #[tokio::test]
    async fn test_pool_stats_tracking() {
        let pool = MessagePool::new();
        let msg_type = values::NEW_ORDER_SINGLE;

        // First message should be a miss
        let msg1 = pool.get_message(msg_type).await;
        let stats = pool.get_stats(msg_type).await.unwrap();
        assert_eq!(stats.misses, 1, "First message should be a miss");
        assert_eq!(stats.hits, 0, "Should have no hits yet");

        // Return and get again - should be a hit
        pool.return_message(msg1).await;
        let _msg2 = pool.get_message(msg_type).await;
        let stats = pool.get_stats(msg_type).await.unwrap();
        assert_eq!(stats.hits, 1, "Second get should be a hit");
        assert_eq!(stats.returns, 1, "Should have one return");
    }

    #[tokio::test]
    async fn test_pool_stress() {
        let pool = MessagePool::new();
        let msg_type = values::MARKET_DATA_SNAPSHOT;
        let iterations = 1000;

        // Repeatedly get and return messages
        for _ in 0..iterations {
            let msg = pool.get_message(msg_type).await;
            pool.return_message(msg).await;
        }

        let stats = pool.get_stats(msg_type).await.unwrap();
        assert!(stats.hits > 0, "Should have some cache hits after {} iterations", iterations);
        assert!(stats.current_size > 0, "Pool should maintain some messages");
        assert!(stats.current_size <= pool.config.max_pool_size, 
            "Pool size should not exceed maximum");
    }
}