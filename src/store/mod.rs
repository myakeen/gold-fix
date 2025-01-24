use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::message::Message;
use crate::Result;
use crate::error::FixError;

/// Stores FIX messages for gap fill and resend requests
pub struct MessageStore {
    /// Messages stored by sequence number
    messages: Arc<Mutex<HashMap<i32, Message>>>,
    /// Next expected sequence number for each session
    sequence_numbers: Arc<Mutex<HashMap<String, i32>>>,
}

impl MessageStore {
    pub fn new() -> Self {
        MessageStore {
            messages: Arc::new(Mutex::new(HashMap::new())),
            sequence_numbers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn store_message(&self, session_id: &str, seq_num: i32, message: Message) -> Result<()> {
        let mut messages = self.messages.lock().await;
        messages.insert(seq_num, message);
        
        let mut seq_nums = self.sequence_numbers.lock().await;
        seq_nums.insert(session_id.to_string(), seq_num + 1);
        
        Ok(())
    }

    pub async fn get_message(&self, seq_num: i32) -> Result<Option<Message>> {
        let messages = self.messages.lock().await;
        Ok(messages.get(&seq_num).cloned())
    }

    pub async fn get_messages_range(&self, start: i32, end: i32) -> Result<Vec<Message>> {
        let messages = self.messages.lock().await;
        let mut result = Vec::new();
        
        for seq_num in start..=end {
            if let Some(msg) = messages.get(&seq_num) {
                result.push(msg.clone());
            }
        }
        
        Ok(result)
    }

    pub async fn get_next_seq_num(&self, session_id: &str) -> Result<i32> {
        let seq_nums = self.sequence_numbers.lock().await;
        Ok(*seq_nums.get(session_id).unwrap_or(&1))
    }

    pub async fn reset_sequence_numbers(&self, session_id: &str) -> Result<()> {
        let mut seq_nums = self.sequence_numbers.lock().await;
        seq_nums.insert(session_id.to_string(), 1);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{Field, field};

    #[tokio::test]
    async fn test_message_store() {
        let store = MessageStore::new();
        let session_id = "TEST_SESSION";

        // Create test message
        let mut msg = Message::new(field::values::NEW_ORDER_SINGLE);
        msg.set_field(Field::new(field::CL_ORD_ID, "12345"));
        
        // Store message
        store.store_message(session_id, 1, msg.clone()).await.unwrap();
        
        // Retrieve message
        let retrieved = store.get_message(1).await.unwrap();
        assert!(retrieved.is_some());
        
        let retrieved_msg = retrieved.unwrap();
        assert_eq!(retrieved_msg.msg_type(), field::values::NEW_ORDER_SINGLE);
        
        // Check sequence number
        let next_seq = store.get_next_seq_num(session_id).await.unwrap();
        assert_eq!(next_seq, 2);
    }

    #[tokio::test]
    async fn test_message_range_retrieval() {
        let store = MessageStore::new();
        let session_id = "TEST_SESSION";

        // Store multiple messages
        for i in 1..=5 {
            let mut msg = Message::new(field::values::NEW_ORDER_SINGLE);
            msg.set_field(Field::new(field::CL_ORD_ID, format!("ORDER_{}", i)));
            store.store_message(session_id, i, msg).await.unwrap();
        }

        // Retrieve range of messages
        let messages = store.get_messages_range(2, 4).await.unwrap();
        assert_eq!(messages.len(), 3);
    }

    #[tokio::test]
    async fn test_sequence_number_reset() {
        let store = MessageStore::new();
        let session_id = "TEST_SESSION";

        // Store a message to increment sequence number
        let msg = Message::new(field::values::NEW_ORDER_SINGLE);
        store.store_message(session_id, 1, msg).await.unwrap();

        // Reset sequence numbers
        store.reset_sequence_numbers(session_id).await.unwrap();

        // Check if sequence number was reset
        let next_seq = store.get_next_seq_num(session_id).await.unwrap();
        assert_eq!(next_seq, 1);
    }
}
