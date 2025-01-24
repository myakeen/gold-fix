#[derive(Debug, Clone)]
pub struct Field {
    tag: i32,
    value: String,
}

impl Field {
    pub fn new(tag: i32, value: impl Into<String>) -> Self {
        Field {
            tag,
            value: value.into(),
        }
    }

    pub fn tag(&self) -> i32 {
        self.tag
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

// Common FIX field tags
pub const BEGIN_STRING: i32 = 8;
pub const BODY_LENGTH: i32 = 9;
pub const MSG_TYPE: i32 = 35;
pub const SENDER_COMP_ID: i32 = 49;
pub const TARGET_COMP_ID: i32 = 56;
pub const MSG_SEQ_NUM: i32 = 34;
pub const SENDING_TIME: i32 = 52;
pub const CHECKSUM: i32 = 10;
