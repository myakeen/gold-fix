pub mod field;
pub mod parser;
pub mod validator;

use chrono;
use std::collections::HashMap;
pub use field::Field;  // Re-export Field for use in other modules
use crate::Result;
use crate::message::parser::MessageParser;
use crate::error::FixError;

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

    pub fn from_string(msg_str: &str) -> Result<Self> {
        MessageParser::parse(msg_str)
    }

    fn set_default_headers(&mut self) {
        // Set BeginString
        self.set_field(Field::new(field::BEGIN_STRING, "FIX.4.2"));
        // Set MsgType
        self.set_field(Field::new(field::MSG_TYPE, &self.msg_type));
        // Set SendingTime
        let timestamp = chrono::Utc::now().format("%Y%m%d-%H:%M:%S").to_string();
        self.set_field(Field::new(field::SENDING_TIME, timestamp));
        // Set default sender and target comp ids
        self.set_field(Field::new(field::SENDER_COMP_ID, "SENDER"));
        self.set_field(Field::new(field::TARGET_COMP_ID, "TARGET"));
        // Set initial sequence number
        self.set_field(Field::new(field::MSG_SEQ_NUM, "1"));
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

    pub fn fields(&self) -> &HashMap<i32, Field> {
        &self.fields
    }

    pub fn to_string(&self) -> Result<String> {
        let mut msg = String::new();

        // Start with BeginString (tag 8)
        if let Some(begin_str) = self.get_field(field::BEGIN_STRING) {
            msg.push_str(&format!("8={}\u{1}", begin_str.value()));
        } else {
            return Err(FixError::ParseError("Missing BeginString".into()));
        }

        // Add BodyLength placeholder
        msg.push_str("9=0000\u{1}");

        // Add MsgType
        msg.push_str(&format!("35={}\u{1}", self.msg_type));

        // Sort fields by tag number for consistent output
        let mut sorted_fields: Vec<_> = self.fields.iter().collect();
        sorted_fields.sort_by_key(|&(k, _)| *k);

        // Add all other fields except BeginString, BodyLength, and Checksum
        for (&tag, field) in sorted_fields.iter() {
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

    #[test]
    fn test_field_access() {
        let mut msg = Message::new(values::NEW_ORDER_SINGLE);
        msg.set_field(Field::new(field::CL_ORD_ID, "12345"));

        let fields = msg.fields();
        assert!(fields.contains_key(&field::CL_ORD_ID));
        assert!(fields.contains_key(&field::BEGIN_STRING)); // Default header
        assert!(fields.contains_key(&field::MSG_TYPE)); // Default header
    }

    #[test]
    fn test_message_roundtrip() {
        let mut original = Message::new(values::NEW_ORDER_SINGLE);
        original.set_field(Field::new(field::CL_ORD_ID, "12345"));
        original.set_field(Field::new(field::SYMBOL, "AAPL"));
        original.set_field(Field::new(field::SIDE, values::BUY));
        original.set_field(Field::new(field::SENDER_COMP_ID, "SENDER"));
        original.set_field(Field::new(field::TARGET_COMP_ID, "TARGET"));
        original.set_field(Field::new(field::MSG_SEQ_NUM, "1"));

        let msg_str = original.to_string().unwrap();
        let parsed = Message::from_string(&msg_str).unwrap();

        assert_eq!(parsed.msg_type(), original.msg_type());
        assert_eq!(
            parsed.get_field(field::CL_ORD_ID).unwrap().value(),
            original.get_field(field::CL_ORD_ID).unwrap().value()
        );
        assert_eq!(
            parsed.get_field(field::SYMBOL).unwrap().value(),
            original.get_field(field::SYMBOL).unwrap().value()
        );
        assert_eq!(
            parsed.get_field(field::SIDE).unwrap().value(),
            original.get_field(field::SIDE).unwrap().value()
        );
    }
}