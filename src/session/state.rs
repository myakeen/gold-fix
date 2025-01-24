use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq)]
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
}

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
}

impl SessionState {
    pub fn new() -> Self {
        SessionState {
            status: Status::Created,
            next_outgoing_seq: 1,
            next_incoming_seq: 1,
            expected_target_seq_num: 1,
            last_send_time: 0,
            last_receive_time: 0,
            test_request_counter: 0,
            logon_timeout: 10,           // 10 seconds
            heartbeat_interval: 30,      // 30 seconds
            test_request_delay: 2,       // 2 seconds
        }
    }

    pub fn with_config(logon_timeout: u64, heartbeat_interval: u64, test_request_delay: u64) -> Self {
        SessionState {
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
        }
    }

    pub fn set_status(&mut self, status: Status) {
        self.status = status;
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    pub fn next_outgoing_seq(&self) -> i32 {
        self.next_outgoing_seq
    }

    pub fn increment_outgoing_seq(&mut self) {
        self.next_outgoing_seq += 1;
    }

    pub fn next_incoming_seq(&self) -> i32 {
        self.next_incoming_seq
    }

    pub fn increment_incoming_seq(&mut self) {
        self.next_incoming_seq += 1;
    }

    pub fn expected_target_seq_num(&self) -> i32 {
        self.expected_target_seq_num
    }

    pub fn set_expected_target_seq_num(&mut self, seq_num: i32) {
        self.expected_target_seq_num = seq_num;
    }

    pub fn increment_test_request_counter(&mut self) {
        self.test_request_counter += 1;
    }

    pub fn reset_test_request_counter(&mut self) {
        self.test_request_counter = 0;
    }

    pub fn test_request_counter(&self) -> i32 {
        self.test_request_counter
    }

    pub fn update_send_time(&mut self) {
        self.last_send_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    pub fn update_receive_time(&mut self) {
        self.last_receive_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_state_transitions() {
        let mut state = SessionState::new();
        assert_eq!(*state.status(), Status::Created);

        state.set_status(Status::Connecting);
        assert_eq!(*state.status(), Status::Connecting);

        state.set_status(Status::Connected);
        assert_eq!(*state.status(), Status::Connected);
    }

    #[test]
    fn test_sequence_numbers() {
        let mut state = SessionState::new();
        assert_eq!(state.next_outgoing_seq(), 1);
        assert_eq!(state.next_incoming_seq(), 1);

        state.increment_outgoing_seq();
        state.increment_incoming_seq();

        assert_eq!(state.next_outgoing_seq(), 2);
        assert_eq!(state.next_incoming_seq(), 2);
    }

    #[test]
    fn test_test_request_handling() {
        let mut state = SessionState::with_config(10, 30, 2);
        assert_eq!(state.test_request_counter(), 0);

        state.increment_test_request_counter();
        assert_eq!(state.test_request_counter(), 1);

        state.reset_test_request_counter();
        assert_eq!(state.test_request_counter(), 0);
    }
}