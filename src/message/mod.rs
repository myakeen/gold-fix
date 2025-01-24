pub mod field;
pub mod parser;
pub mod validator;

use chrono;
use std::collections::HashMap;
pub use field::Field;  // Re-export Field for use in other modules
use crate::Result;

#[derive(Debug, Clone)]
pub struct Message {
    fields: HashMap<i32, Field>,
    msg_type: String,
}

impl Message {
    pub fn new(msg_type: &str) -> Self {
        let mut msg = Message {
            fields: HashMap::new(),
            msg_type: msg_type.to_string(),
        };
        msg.set_default_headers();
        msg
    }

    fn set_default_headers(&mut self) {
        // Set BeginString
        self.set_field(Field::new(field::BEGIN_STRING, "FIX.4.2"));
        // Set MsgType
        self.set_field(Field::new(field::MSG_TYPE, &self.msg_type));
        // Set current time as SendingTime
        let timestamp = chrono::Utc::now().format("%Y%m%d-%H:%M:%S").to_string();
        self.set_field(Field::new(field::SENDING_TIME, timestamp));
    }

    pub fn set_field(&mut self, field: Field) {
        self.fields.insert(field.tag(), field);
    }

    pub fn get_field(&self, tag: i32) -> Option<&Field> {
        self.fields.get(&tag)
    }

    pub fn msg_type(&self) -> &str {
        &self.msg_type
    }

    pub fn to_string(&self) -> Result<String> {
        let mut msg = String::new();

        // Start with BeginString (tag 8)
        if let Some(begin_str) = self.get_field(field::BEGIN_STRING) {
            msg.push_str(&format!("8={}\u{1}", begin_str.value()));
        }

        // Add BodyLength placeholder
        msg.push_str("9=0000\u{1}");

        // Add MsgType
        msg.push_str(&format!("35={}\u{1}", self.msg_type));

        // Add all other fields except BeginString, BodyLength, and Checksum
        for (&tag, field) in self.fields.iter() {
            if tag != field::BEGIN_STRING && tag != field::BODY_LENGTH && tag != field::CHECKSUM {
                msg.push_str(&format!("{}={}\u{1}", tag, field.value()));
            }
        }

        // Calculate body length (excluding BeginString and Checksum)
        let body_start = msg.find("9=").unwrap_or(0);
        let body_length = msg[body_start..].len();

        // Replace body length placeholder
        let body_length_str = format!("9={:04}\u{1}", body_length);
        msg = msg.replace("9=0000\u{1}", &body_length_str);

        // Calculate and add checksum
        let checksum = calculate_checksum(&msg);
        msg.push_str(&format!("10={:03}\u{1}", checksum));

        Ok(msg)
    }
}

fn calculate_checksum(msg: &str) -> u32 {
    msg.bytes().map(|b| b as u32).sum::<u32>() % 256
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::field::values;

    #[test]
    fn test_message_creation() {
        let mut msg = Message::new(values::NEW_ORDER_SINGLE);
        msg.set_field(Field::new(field::CL_ORD_ID, "12345"));
        msg.set_field(Field::new(field::SYMBOL, "AAPL"));
        msg.set_field(Field::new(field::SIDE, values::BUY));

        assert_eq!(msg.msg_type(), values::NEW_ORDER_SINGLE);
        assert_eq!(msg.get_field(field::SYMBOL).unwrap().value(), "AAPL");
    }

    #[test]
    fn test_message_serialization() {
        let mut msg = Message::new(values::HEARTBEAT);
        msg.set_field(Field::new(field::SENDER_COMP_ID, "SENDER"));
        msg.set_field(Field::new(field::TARGET_COMP_ID, "TARGET"));
        msg.set_field(Field::new(field::MSG_SEQ_NUM, "1"));

        let result = msg.to_string();
        assert!(result.is_ok());
        let msg_str = result.unwrap();
        assert!(msg_str.starts_with("8=FIX.4.2"));
        assert!(msg_str.contains("35=0"));
        assert!(msg_str.contains("49=SENDER"));
        assert!(msg_str.contains("56=TARGET"));
    }
}