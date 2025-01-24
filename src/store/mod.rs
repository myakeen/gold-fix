use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use crate::message::Message;
use crate::Result;
use crate::error::FixError;

// Transaction state for atomic message operations
#[derive(Debug)]
struct Transaction {
    messages: Vec<(i32, Message)>,
    started: bool,
    version: u64,  // Added version tracking
}

// Stores FIX messages with persistence and queueing capabilities
pub struct MessageStore {
    // Messages stored by session and sequence number
    messages: Arc<Mutex<HashMap<String, HashMap<i32, (Message, u64)>>>>,  // Added version number
    // Next expected sequence number for each session
    sequence_numbers: Arc<Mutex<HashMap<String, i32>>>,
    // Store directory for persistence
    store_dir: PathBuf,
    // Active transactions
    transactions: Arc<Mutex<HashMap<String, Transaction>>>,
    // Version counter for optimistic locking
    version_counter: Arc<Mutex<u64>>,
}

impl MessageStore {
    pub fn new() -> Self {
        let store_dir = PathBuf::from("store");
        fs::create_dir_all(&store_dir).expect("Failed to create store directory");

        // Create session state directory
        let session_dir = store_dir.join("sessions");
        fs::create_dir_all(&session_dir).expect("Failed to create session directory");

        MessageStore {
            messages: Arc::new(Mutex::new(HashMap::new())),
            sequence_numbers: Arc::new(Mutex::new(HashMap::new())),
            store_dir,
            transactions: Arc::new(Mutex::new(HashMap::new())),
            version_counter: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn begin_transaction(&self, session_id: &str) -> Result<()> {
        let mut transactions = self.transactions.lock().await;
        if transactions.contains_key(session_id) {
            return Err(FixError::StoreError("Transaction already in progress".into()));
        }

        let version = {
            let mut counter = self.version_counter.lock().await;
            *counter += 1;
            *counter
        };

        transactions.insert(session_id.to_string(), Transaction {
            messages: Vec::new(),
            started: true,
            version,
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

        // Drop the transactions lock before storing messages
        drop(transactions);

        // Start atomic persistence
        let file_path = self.store_dir.join(format!("{}.messages", session_id));
        let temp_path = file_path.with_extension("tmp");

        // Write to temporary file first
        {
            let mut temp_file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&temp_path)
                .map_err(|e| FixError::IoError(e))?;

            // Store all messages in the transaction
            let messages = transaction.messages.clone(); // Clone to avoid move
            for (seq_num, message) in messages.iter() {
                let msg_str = message.to_string()?;
                writeln!(temp_file, "{}|{}|{}", seq_num, transaction.version, msg_str)
                    .map_err(|e| FixError::IoError(e))?;
            }

            temp_file.flush().map_err(|e| FixError::IoError(e))?;
        }

        // Atomic rename of temporary file to actual file
        fs::rename(&temp_path, &file_path)
            .map_err(|e| FixError::IoError(e))?;

        // Update in-memory state
        let mut messages = self.messages.lock().await;
        let session_messages = messages.entry(session_id.to_string()).or_insert_with(HashMap::new);

        for (seq_num, message) in transaction.messages.iter() {
            session_messages.insert(*seq_num, (message.clone(), transaction.version));
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
        {
            let mut transactions = self.transactions.lock().await;
            if let Some(transaction) = transactions.get_mut(session_id) {
                transaction.messages.push((seq_num, message.clone()));
                return Ok(());
            }
        }

        // Get current version
        let version = {
            let mut counter = self.version_counter.lock().await;
            *counter += 1;
            *counter
        };

        // Store in memory with version
        let mut messages = self.messages.lock().await;
        let session_messages = messages.entry(session_id.to_string()).or_insert_with(HashMap::new);
        session_messages.insert(seq_num, (message.clone(), version));
        drop(messages);

        let mut seq_nums = self.sequence_numbers.lock().await;
        seq_nums.insert(session_id.to_string(), seq_num + 1);
        drop(seq_nums);

        // Persist to file with version
        self.persist_message(session_id, seq_num, &message, version).await?;

        Ok(())
    }

    async fn persist_message(&self, session_id: &str, seq_num: i32, message: &Message, version: u64) -> Result<()> {
        let file_path = self.store_dir.join(format!("{}.messages", session_id));
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .map_err(|e| FixError::IoError(e))?;

        // Format: sequence_number|version|message_string
        let msg_str = message.to_string()?;
        writeln!(file, "{}|{}|{}", seq_num, version, msg_str)
            .map_err(|e| FixError::IoError(e))?;

        // Ensure data is written to disk
        file.flush()
            .map_err(|e| FixError::IoError(e))?;

        Ok(())
    }

    pub async fn get_message(&self, session_id: &str, seq_num: i32) -> Result<Option<Message>> {
        let messages = self.messages.lock().await;
        Ok(messages.get(session_id)
            .and_then(|session_msgs| session_msgs.get(&seq_num))
            .map(|(msg, _)| msg.clone()))
    }

    pub async fn get_message_with_version(&self, session_id: &str, seq_num: i32) -> Result<Option<(Message, u64)>> {
        let messages = self.messages.lock().await;
        Ok(messages.get(session_id)
            .and_then(|session_msgs| session_msgs.get(&seq_num))
            .map(|(msg, ver)| (msg.clone(), *ver)))
    }

    pub async fn get_messages_range(&self, session_id: &str, start: i32, end: i32) -> Result<Vec<Message>> {
        let messages = self.messages.lock().await;
        let mut result = Vec::new();

        if let Some(session_msgs) = messages.get(session_id) {
            for seq_num in start..=end {
                if let Some((msg, _)) = session_msgs.get(&seq_num) {
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
        drop(seq_nums);

        // Clear session messages from memory
        let mut messages = self.messages.lock().await;
        messages.remove(session_id);
        drop(messages);

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
        let mut max_version = 0;

        for line in reader.lines() {
            let line = line.map_err(|e| FixError::IoError(e))?;
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() != 3 {
                continue;
            }

            if let (Ok(seq_num), Ok(version), Ok(message)) = (
                parts[0].parse::<i32>(),
                parts[1].parse::<u64>(),
                Message::from_string(parts[2])
            ) {
                session_messages.insert(seq_num, (message, version));
                max_seq = max_seq.max(seq_num);
                max_version = max_version.max(version);
            }
        }

        // Update sequence number and version counter
        drop(messages);
        if max_seq > 0 {
            let mut seq_nums = self.sequence_numbers.lock().await;
            seq_nums.insert(session_id.to_string(), max_seq + 1);
        }
        if max_version > 0 {
            let mut version = self.version_counter.lock().await;
            *version = max_version;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{Field, field};
    use tempfile::tempdir;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_message_transaction() {
        let store = MessageStore::new();
        let session_id = "TEST_SESSION";

        let test = async {
            // Begin transaction
            store.begin_transaction(session_id).await.unwrap();

            // Store messages in transaction
            let mut msg1 = Message::new(field::values::NEW_ORDER_SINGLE);
            msg1.set_field(Field::new(field::CL_ORD_ID, "12345")).unwrap();
            store.store_message(session_id, 1, msg1.clone()).await.unwrap();

            let mut msg2 = Message::new(field::values::NEW_ORDER_SINGLE);
            msg2.set_field(Field::new(field::CL_ORD_ID, "12346")).unwrap();
            store.store_message(session_id, 2, msg2.clone()).await.unwrap();

            // Commit transaction
            store.commit_transaction(session_id).await.unwrap();

            // Verify messages were stored with versions
            let message1 = store.get_message_with_version(session_id, 1).await.unwrap();
            let message2 = store.get_message_with_version(session_id, 2).await.unwrap();

            assert!(message1.is_some());
            assert!(message2.is_some());

            let (msg1_stored, ver1) = message1.unwrap();
            let (msg2_stored, ver2) = message2.unwrap();

            assert_eq!(msg1_stored.get_field(field::CL_ORD_ID).unwrap().value(), "12345");
            assert_eq!(msg2_stored.get_field(field::CL_ORD_ID).unwrap().value(), "12346");
            assert_eq!(ver1, ver2);  // Same transaction, same version
        };

        timeout(Duration::from_secs(5), test).await.unwrap();
    }

    #[tokio::test]
    async fn test_message_transaction_rollback() {
        let store = MessageStore::new();
        let session_id = "TEST_SESSION";

        let test = async {
            // Begin transaction
            store.begin_transaction(session_id).await.unwrap();

            // Store message in transaction
            let mut msg = Message::new(field::values::NEW_ORDER_SINGLE);
            msg.set_field(Field::new(field::CL_ORD_ID, "12345")).unwrap();
            store.store_message(session_id, 1, msg.clone()).await.unwrap();

            // Rollback transaction
            store.rollback_transaction(session_id).await.unwrap();

            // Verify message was not stored
            let message = store.get_message(session_id, 1).await.unwrap();
            assert!(message.is_none());
        };

        timeout(Duration::from_secs(5), test).await.unwrap();
    }

    #[tokio::test]
    async fn test_message_persistence() {
        let temp_dir = tempdir().unwrap();
        let store = MessageStore {
            messages: Arc::new(Mutex::new(HashMap::new())),
            sequence_numbers: Arc::new(Mutex::new(HashMap::new())),
            store_dir: temp_dir.path().to_path_buf(),
            transactions: Arc::new(Mutex::new(HashMap::new())),
            version_counter: Arc::new(Mutex::new(0)),
        };

        let session_id = "TEST_SESSION";

        let test = async {
            // Begin transaction
            store.begin_transaction(session_id).await.unwrap();

            // Store test message
            let mut msg = Message::new(field::values::NEW_ORDER_SINGLE);
            msg.set_field(Field::new(field::CL_ORD_ID, "12345")).unwrap();
            msg.set_field(Field::new(field::MSG_SEQ_NUM, "1")).unwrap();

            // Store message
            store.store_message(session_id, 1, msg.clone()).await.unwrap();

            // Commit transaction
            store.commit_transaction(session_id).await.unwrap();

            // Create new store instance and load messages
            let new_store = MessageStore {
                messages: Arc::new(Mutex::new(HashMap::new())),
                sequence_numbers: Arc::new(Mutex::new(HashMap::new())),
                store_dir: temp_dir.path().to_path_buf(),
                transactions: Arc::new(Mutex::new(HashMap::new())),
                version_counter: Arc::new(Mutex::new(0)),
            };

            new_store.load_messages(session_id).await.unwrap();

            // Verify message was loaded with correct version
            let loaded = new_store.get_message_with_version(session_id, 1).await.unwrap();
            assert!(loaded.is_some());

            let (loaded_msg, loaded_ver) = loaded.unwrap();
            assert_eq!(loaded_msg.msg_type(), field::values::NEW_ORDER_SINGLE);
            assert_eq!(loaded_msg.get_field(field::CL_ORD_ID).unwrap().value(), "12345");
            assert_eq!(loaded_ver, 1);  // First version should be 1
        };

        timeout(Duration::from_secs(5), test).await.unwrap();
    }
}