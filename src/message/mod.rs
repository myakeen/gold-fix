pub mod field;
pub mod parser;
pub mod validator;

use std::collections::HashMap;
use field::Field;
use crate::Result;

#[derive(Debug, Clone)]
pub struct Message {
    fields: HashMap<i32, Field>,
    msg_type: String,
}

impl Message {
    pub fn new(msg_type: &str) -> Self {
        Message {
            fields: HashMap::new(),
            msg_type: msg_type.to_string(),
        }
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
        // Build FIX message string
        // Begin string
        msg.push_str("8=FIX.4.2\u{1}"); // BeginString
        msg.push_str(&format!("9={}\u{1}", 0)); // BodyLength (placeholder)
        msg.push_str(&format!("35={}\u{1}", self.msg_type)); // MsgType

        // Add all other fields
        for (_, field) in self.fields.iter() {
            if field.tag() != field::BEGIN_STRING && 
               field.tag() != field::BODY_LENGTH && 
               field.tag() != field::MSG_TYPE {
                msg.push_str(&format!("{}={}\u{1}", field.tag(), field.value()));
            }
        }

        // Calculate body length and checksum
        let body_length = msg.len();
        let checksum = calculate_checksum(&msg);

        // Replace body length placeholder
        msg = msg.replace("9=0\u{1}", &format!("9={}\u{1}", body_length));

        // Add checksum
        msg.push_str(&format!("10={:03}\u{1}", checksum));

        // Validate message before sending
        validator::MessageValidator::validate(self)?;

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
        msg.set_field(Field::new(field::SENDING_TIME, "20250124-12:00:00"));

        let result = msg.to_string();
        assert!(result.is_ok());
        let msg_str = result.unwrap();
        assert!(msg_str.contains("35=0"));
        assert!(msg_str.contains("49=SENDER"));
    }
}