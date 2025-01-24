use std::time::{SystemTime, UNIX_EPOCH};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use crate::Result;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Status {
    Created,
    Connecting,
    InitiateLogon,
    ResendRequest,
    LogonReceived,
    Connected,
    Disconnecting,
    Disconnected,
    Error,
    Recovering,  // New state for recovery
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionState {
    status: Status,
    next_outgoing_seq: i32,
    next_incoming_seq: i32,
    expected_target_seq_num: i32,
    last_send_time: u64,
    last_receive_time: u64,
    test_request_counter: i32,
    logon_timeout: u64,
    heartbeat_interval: u64,
    test_request_delay: u64,
    session_id: String,  // Added for persistence
    store_dir: PathBuf,  // Added for persistence
}

impl SessionState {
    pub fn new(session_id: &str, store_dir: PathBuf) -> Self {
        let state = SessionState {
            status: Status::Created,
            next_outgoing_seq: 1,
            next_incoming_seq: 1,
            expected_target_seq_num: 1,
            last_send_time: 0,
            last_receive_time: 0,
            test_request_counter: 0,
            logon_timeout: 10,
            heartbeat_interval: 30,
            test_request_delay: 2,
            session_id: session_id.to_string(),
            store_dir,
        };
        state.persist().unwrap_or_default();
        state
    }

    pub fn with_config(
        session_id: &str,
        store_dir: PathBuf,
        logon_timeout: u64,
        heartbeat_interval: u64,
        test_request_delay: u64
    ) -> Self {
        let state = SessionState {
            status: Status::Created,
            next_outgoing_seq: 1,
            next_incoming_seq: 1,
            expected_target_seq_num: 1,
            last_send_time: 0,
            last_receive_time: 0,
            test_request_counter: 0,
            logon_timeout,
            heartbeat_interval,
            test_request_delay,
            session_id: session_id.to_string(),
            store_dir,
        };
        state.persist().unwrap_or_default();
        state
    }

    // Recovery methods
    pub fn load_or_create(session_id: &str, store_dir: PathBuf) -> Result<Self> {
        let state_path = store_dir.join(format!("{}_state.json", session_id));
        if state_path.exists() {
            let mut file = File::open(&state_path)?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            let mut state: SessionState = serde_json::from_str(&contents)?;
            state.status = Status::Recovering;
            Ok(state)
        } else {
            Ok(Self::new(session_id, store_dir))
        }
    }

    fn persist(&self) -> Result<()> {
        fs::create_dir_all(&self.store_dir)?;
        let state_path = self.store_dir.join(format!("{}_state.json", self.session_id));
        let temp_path = state_path.with_extension("tmp");

        // Write to temporary file first
        let json = serde_json::to_string_pretty(self)?;
        let mut temp_file = File::create(&temp_path)?;
        temp_file.write_all(json.as_bytes())?;
        temp_file.sync_all()?;

        // Atomic rename
        fs::rename(&temp_path, &state_path)?;
        Ok(())
    }

    pub fn set_status(&mut self, status: Status) {
        self.status = status;
        self.persist().unwrap_or_default();
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    pub fn next_outgoing_seq(&self) -> i32 {
        self.next_outgoing_seq
    }

    pub fn increment_outgoing_seq(&mut self) {
        self.next_outgoing_seq += 1;
        self.persist().unwrap_or_default();
    }

    pub fn next_incoming_seq(&self) -> i32 {
        self.next_incoming_seq
    }

    pub fn increment_incoming_seq(&mut self) {
        self.next_incoming_seq += 1;
        self.persist().unwrap_or_default();
    }

    pub fn expected_target_seq_num(&self) -> i32 {
        self.expected_target_seq_num
    }

    pub fn set_expected_target_seq_num(&mut self, seq_num: i32) {
        self.expected_target_seq_num = seq_num;
        self.persist().unwrap_or_default();
    }

    pub fn increment_test_request_counter(&mut self) {
        self.test_request_counter += 1;
        self.persist().unwrap_or_default();
    }

    pub fn reset_test_request_counter(&mut self) {
        self.test_request_counter = 0;
        self.persist().unwrap_or_default();
    }

    pub fn test_request_counter(&self) -> i32 {
        self.test_request_counter
    }

    pub fn update_send_time(&mut self) {
        self.last_send_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.persist().unwrap_or_default();
    }

    pub fn update_receive_time(&mut self) {
        self.last_receive_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.persist().unwrap_or_default();
    }

    pub fn should_send_test_request(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now - self.last_receive_time > self.heartbeat_interval + self.test_request_delay
    }

    pub fn should_disconnect(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        match self.status {
            Status::InitiateLogon => now - self.last_send_time > self.logon_timeout,
            Status::Connected => {
                self.test_request_counter >= 2 ||
                now - self.last_receive_time > 2 * self.heartbeat_interval
            },
            _ => false,
        }
    }

    pub fn reset_sequence_numbers(&mut self) {
        self.next_outgoing_seq = 1;
        self.next_incoming_seq = 1;
        self.expected_target_seq_num = 1;
        self.persist().unwrap_or_default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_session_state_persistence() {
        let temp_dir = tempdir().unwrap();
        let session_id = "TEST_SESSION";

        // Create new state
        let mut state = SessionState::new(session_id, temp_dir.path().to_path_buf());
        state.set_status(Status::Connected);
        state.increment_outgoing_seq();
        state.increment_incoming_seq();

        // Load persisted state
        let loaded_state = SessionState::load_or_create(session_id, temp_dir.path().to_path_buf()).unwrap();

        assert_eq!(loaded_state.status, Status::Recovering);
        assert_eq!(loaded_state.next_outgoing_seq(), 2);
        assert_eq!(loaded_state.next_incoming_seq(), 2);
    }

    #[test]
    fn test_session_state_transitions() {
        let temp_dir = tempdir().unwrap();
        let mut state = SessionState::new("TEST", temp_dir.path().to_path_buf());
        assert_eq!(*state.status(), Status::Created);

        state.set_status(Status::Connecting);
        assert_eq!(*state.status(), Status::Connecting);

        state.set_status(Status::Connected);
        assert_eq!(*state.status(), Status::Connected);
    }

    #[test]
    fn test_sequence_numbers() {
        let temp_dir = tempdir().unwrap();
        let mut state = SessionState::new("TEST", temp_dir.path().to_path_buf());
        assert_eq!(state.next_outgoing_seq(), 1);
        assert_eq!(state.next_incoming_seq(), 1);

        state.increment_outgoing_seq();
        state.increment_incoming_seq();

        assert_eq!(state.next_outgoing_seq(), 2);
        assert_eq!(state.next_incoming_seq(), 2);
    }

    #[test]
    fn test_test_request_handling() {
        let temp_dir = tempdir().unwrap();
        let mut state = SessionState::with_config(
            "TEST",
            temp_dir.path().to_path_buf(),
            10, 30, 2
        );
        assert_eq!(state.test_request_counter(), 0);

        state.increment_test_request_counter();
        assert_eq!(state.test_request_counter(), 1);

        state.reset_test_request_counter();
        assert_eq!(state.test_request_counter(), 0);
    }

    #[test]
    fn test_recovery_state() {
        let temp_dir = tempdir().unwrap();
        let session_id = "TEST_RECOVERY";

        // Create initial state
        let mut state = SessionState::new(session_id, temp_dir.path().to_path_buf());
        state.set_status(Status::Connected);
        state.increment_outgoing_seq();

        // Simulate crash and recovery
        let recovered_state = SessionState::load_or_create(session_id, temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(recovered_state.status, Status::Recovering);
        assert_eq!(recovered_state.next_outgoing_seq(), 2);
        assert!(recovered_state.should_send_test_request());
    }
}