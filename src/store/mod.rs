use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use crate::message::Message;
use crate::Result;
use crate::error::FixError;

/// Transaction state for atomic message operations
#[derive(Debug)]
struct Transaction {
    session_id: String,
    messages: Vec<(i32, Message)>,
    started: bool,
}

/// Stores FIX messages with persistence and queueing capabilities
pub struct MessageStore {
    /// Messages stored by session and sequence number
    messages: Arc<Mutex<HashMap<String, HashMap<i32, Message>>>>,
    /// Next expected sequence number for each session
    sequence_numbers: Arc<Mutex<HashMap<String, i32>>>,
    /// Queue of outgoing messages
    outgoing_queue: Arc<Mutex<Vec<Message>>>,
    /// Store directory for persistence
    store_dir: PathBuf,
    /// Active transactions
    transactions: Arc<Mutex<HashMap<String, Transaction>>>,
}

impl MessageStore {
    pub fn new() -> Self {
        let store_dir = PathBuf::from("store");
        std::fs::create_dir_all(&store_dir).expect("Failed to create store directory");

        // Create session state directory
        let session_dir = store_dir.join("sessions");
        std::fs::create_dir_all(&session_dir).expect("Failed to create session directory");

        MessageStore {
            messages: Arc::new(Mutex::new(HashMap::new())),
            sequence_numbers: Arc::new(Mutex::new(HashMap::new())),
            outgoing_queue: Arc::new(Mutex::new(Vec::new())),
            store_dir,
            transactions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn begin_transaction(&self, session_id: &str) -> Result<()> {
        let mut transactions = self.transactions.lock().await;
        if transactions.contains_key(session_id) {
            return Err(FixError::StoreError("Transaction already in progress".into()));
        }

        transactions.insert(session_id.to_string(), Transaction {
            session_id: session_id.to_string(),
            messages: Vec::new(),
            started: true,
        });

        Ok(())
    }

    pub async fn commit_transaction(&self, session_id: &str) -> Result<()> {
        let mut transactions = self.transactions.lock().await;
        let transaction = transactions.remove(session_id)
            .ok_or_else(|| FixError::StoreError("No transaction in progress".into()))?;

        if !transaction.started {
            return Err(FixError::StoreError("Transaction not started".into()));
        }

        // Store all messages in the transaction
        for (seq_num, message) in transaction.messages {
            self.store_message(session_id, seq_num, message).await?;
        }

        Ok(())
    }

    pub async fn rollback_transaction(&self, session_id: &str) -> Result<()> {
        let mut transactions = self.transactions.lock().await;
        transactions.remove(session_id)
            .ok_or_else(|| FixError::StoreError("No transaction in progress".into()))?;
        Ok(())
    }

    pub async fn store_message(&self, session_id: &str, seq_num: i32, message: Message) -> Result<()> {
        // Check if part of a transaction
        let mut transactions = self.transactions.lock().await;
        if let Some(transaction) = transactions.get_mut(session_id) {
            transaction.messages.push((seq_num, message.clone()));
            return Ok(());
        }
        drop(transactions);

        // Store in memory
        let mut messages = self.messages.lock().await;
        let session_messages = messages.entry(session_id.to_string()).or_insert_with(HashMap::new);
        session_messages.insert(seq_num, message.clone());

        let mut seq_nums = self.sequence_numbers.lock().await;
        seq_nums.insert(session_id.to_string(), seq_num + 1);

        // Persist to file with error handling
        self.persist_message(session_id, seq_num, &message).await?;

        Ok(())
    }

    async fn persist_message(&self, session_id: &str, seq_num: i32, message: &Message) -> Result<()> {
        let file_path = self.store_dir.join(format!("{}.messages", session_id));
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .map_err(|e| FixError::IoError(e))?;

        // Format: sequence_number|message_string
        let msg_str = message.to_string()?;
        writeln!(file, "{}|{}", seq_num, msg_str)
            .map_err(|e| FixError::IoError(e))?;

        // Ensure data is written to disk
        file.flush()
            .map_err(|e| FixError::IoError(e))?;

        Ok(())
    }

    pub async fn load_messages(&self, session_id: &str) -> Result<()> {
        let file_path = self.store_dir.join(format!("{}.messages", session_id));
        if !file_path.exists() {
            return Ok(());
        }

        let file = File::open(&file_path)
            .map_err(|e| FixError::IoError(e))?;
        let reader = BufReader::new(file);
        let mut messages = self.messages.lock().await;
        let session_messages = messages.entry(session_id.to_string()).or_insert_with(HashMap::new);
        let mut max_seq = 0;

        for line in reader.lines() {
            let line = line.map_err(|e| FixError::IoError(e))?;
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() != 2 {
                continue;
            }

            if let Ok(seq_num) = parts[0].parse::<i32>() {
                if let Ok(message) = Message::from_string(parts[1]) {
                    session_messages.insert(seq_num, message);
                    max_seq = max_seq.max(seq_num);
                }
            }
        }

        // Update sequence number
        if max_seq > 0 {
            let mut seq_nums = self.sequence_numbers.lock().await;
            seq_nums.insert(session_id.to_string(), max_seq + 1);
        }

        Ok(())
    }

    pub async fn get_message(&self, session_id: &str, seq_num: i32) -> Result<Option<Message>> {
        let messages = self.messages.lock().await;
        Ok(messages.get(session_id)
            .and_then(|session_msgs| session_msgs.get(&seq_num))
            .cloned())
    }

    pub async fn get_messages_range(&self, session_id: &str, start: i32, end: i32) -> Result<Vec<Message>> {
        let messages = self.messages.lock().await;
        let mut result = Vec::new();

        if let Some(session_msgs) = messages.get(session_id) {
            for seq_num in start..=end {
                if let Some(msg) = session_msgs.get(&seq_num) {
                    result.push(msg.clone());
                }
            }
        }

        Ok(result)
    }

    pub async fn get_next_seq_num(&self, session_id: &str) -> Result<i32> {
        let seq_nums = self.sequence_numbers.lock().await;
        Ok(*seq_nums.get(session_id).unwrap_or(&1))
    }

    pub async fn reset_sequence_numbers(&self, session_id: &str) -> Result<()> {
        // Reset in-memory sequence number
        let mut seq_nums = self.sequence_numbers.lock().await;
        seq_nums.insert(session_id.to_string(), 1);

        // Clear session messages from memory
        let mut messages = self.messages.lock().await;
        messages.remove(session_id);

        // Truncate the message file for this session
        let file_path = self.store_dir.join(format!("{}.messages", session_id));
        if file_path.exists() {
            OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(file_path)
                .map_err(|e| FixError::IoError(e))?;
        }

        Ok(())
    }

    pub async fn queue_outgoing_message(&self, message: Message) -> Result<()> {
        let mut queue = self.outgoing_queue.lock().await;
        queue.push(message);
        Ok(())
    }

    pub async fn get_next_outgoing_message(&self) -> Option<Message> {
        let mut queue = self.outgoing_queue.lock().await;
        queue.pop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{Field, field};
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_message_persistence() {
        let temp_dir = tempdir().unwrap();
        let store = MessageStore {
            messages: Arc::new(Mutex::new(HashMap::new())),
            sequence_numbers: Arc::new(Mutex::new(HashMap::new())),
            outgoing_queue: Arc::new(Mutex::new(Vec::new())),
            store_dir: temp_dir.path().to_path_buf(),
            transactions: Arc::new(Mutex::new(HashMap::new())),
        };

        let session_id = "TEST_SESSION";

        // Create and store test message
        let mut msg = Message::new(field::values::NEW_ORDER_SINGLE);
        msg.set_field(Field::new(field::CL_ORD_ID, "12345"));
        msg.set_field(Field::new(field::MSG_SEQ_NUM, "1"));

        // Store message
        store.store_message(session_id, 1, msg.clone()).await.unwrap();

        // Verify sequence number
        let seq_num = store.get_next_seq_num(session_id).await.unwrap();
        assert_eq!(seq_num, 2);

        // Create new store instance and load messages
        let new_store = MessageStore {
            messages: Arc::new(Mutex::new(HashMap::new())),
            sequence_numbers: Arc::new(Mutex::new(HashMap::new())),
            outgoing_queue: Arc::new(Mutex::new(Vec::new())),
            store_dir: temp_dir.path().to_path_buf(),
            transactions: Arc::new(Mutex::new(HashMap::new())),
        };

        new_store.load_messages(session_id).await.unwrap();

        // Verify message was loaded
        let loaded_msg = new_store.get_message(session_id, 1).await.unwrap();
        assert!(loaded_msg.is_some());

        let loaded_msg = loaded_msg.unwrap();
        assert_eq!(loaded_msg.msg_type(), field::values::NEW_ORDER_SINGLE);
        assert_eq!(loaded_msg.get_field(field::CL_ORD_ID).unwrap().value(), "12345");
    }

    #[tokio::test]
    async fn test_message_transaction() {
        let store = MessageStore::new();
        let session_id = "TEST_SESSION";

        // Begin transaction
        store.begin_transaction(session_id).await.unwrap();

        // Store messages in transaction
        let mut msg1 = Message::new(field::values::NEW_ORDER_SINGLE);
        msg1.set_field(Field::new(field::CL_ORD_ID, "12345"));
        store.store_message(session_id, 1, msg1.clone()).await.unwrap();

        let mut msg2 = Message::new(field::values::NEW_ORDER_SINGLE);
        msg2.set_field(Field::new(field::CL_ORD_ID, "12346"));
        store.store_message(session_id, 2, msg2.clone()).await.unwrap();

        // Commit transaction
        store.commit_transaction(session_id).await.unwrap();

        // Verify messages were stored
        let messages = store.get_messages_range(session_id, 1, 2).await.unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].get_field(field::CL_ORD_ID).unwrap().value(), "12345");
        assert_eq!(messages[1].get_field(field::CL_ORD_ID).unwrap().value(), "12346");
    }

    #[tokio::test]
    async fn test_message_transaction_rollback() {
        let store = MessageStore::new();
        let session_id = "TEST_SESSION";

        // Begin transaction
        store.begin_transaction(session_id).await.unwrap();

        // Store message in transaction
        let mut msg = Message::new(field::values::NEW_ORDER_SINGLE);
        msg.set_field(Field::new(field::CL_ORD_ID, "12345"));
        store.store_message(session_id, 1, msg.clone()).await.unwrap();

        // Rollback transaction
        store.rollback_transaction(session_id).await.unwrap();

        // Verify message was not stored
        let messages = store.get_messages_range(session_id, 1, 1).await.unwrap();
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn test_message_range_retrieval() {
        let store = MessageStore::new();
        let session_id = "TEST_SESSION";

        // Store multiple messages
        for i in 1..=5 {
            let mut msg = Message::new(field::values::NEW_ORDER_SINGLE);
            msg.set_field(Field::new(field::CL_ORD_ID, &format!("ORDER_{}", i)));
            store.store_message(session_id, i, msg).await.unwrap();
        }

        // Retrieve range of messages
        let messages = store.get_messages_range(session_id, 2, 4).await.unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].get_field(field::CL_ORD_ID).unwrap().value(), "ORDER_2");
        assert_eq!(messages[2].get_field(field::CL_ORD_ID).unwrap().value(), "ORDER_4");
    }

    #[tokio::test]
    async fn test_sequence_number_reset() {
        let store = MessageStore::new();
        let session_id = "TEST_SESSION";

        // Store a message
        let mut msg = Message::new(field::values::NEW_ORDER_SINGLE);
        msg.set_field(Field::new(field::CL_ORD_ID, "12345"));
        store.store_message(session_id, 1, msg).await.unwrap();

        // Reset sequence numbers
        store.reset_sequence_numbers(session_id).await.unwrap();

        // Verify sequence number is reset
        let seq_num = store.get_next_seq_num(session_id).await.unwrap();
        assert_eq!(seq_num, 1);

        // Verify messages are cleared
        let messages = store.get_messages_range(session_id, 1, 1).await.unwrap();
        assert!(messages.is_empty());
    }
}