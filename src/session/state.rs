use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Created,
    Connecting,
    Connected,
    Disconnecting,
    Disconnected,
}

pub struct SessionState {
    status: Status,
    next_outgoing_seq: i32,
    next_incoming_seq: i32,
    last_send_time: u64,
    last_receive_time: u64,
}

impl SessionState {
    pub fn new() -> Self {
        SessionState {
            status: Status::Created,
            next_outgoing_seq: 1,
            next_incoming_seq: 1,
            last_send_time: 0,
            last_receive_time: 0,
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
}
