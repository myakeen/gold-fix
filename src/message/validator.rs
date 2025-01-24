use std::collections::HashSet;
use crate::message::{Message, field};
use crate::Result;
use crate::error::FixError;

pub struct MessageValidator;

impl MessageValidator {
    pub fn validate(message: &Message) -> Result<()> {
        // Check required header fields
        let required_header = vec![
            field::BEGIN_STRING,
            field::BODY_LENGTH,
            field::MSG_TYPE,
            field::SENDER_COMP_ID,
            field::TARGET_COMP_ID,
            field::MSG_SEQ_NUM,
            field::SENDING_TIME,
        ];

        for &tag in &required_header {
            if message.get_field(tag).is_none() {
                return Err(FixError::ParseError(
                    format!("Missing required header field: {}", tag)
                ));
            }
        }

        // Check message-specific required fields
        let required_fields = Self::get_required_fields(message.msg_type());
        for &tag in &required_fields {
            if message.get_field(tag).is_none() {
                return Err(FixError::ParseError(
                    format!("Missing required field for message type {}: {}", 
                        message.msg_type(), tag)
                ));
            }
        }

        Ok(())
    }

    fn get_required_fields(msg_type: &str) -> HashSet<i32> {
        let mut fields = HashSet::new();
        match msg_type {
            field::values::LOGON => {
                fields.insert(field::ENCRYPT_METHOD);
                fields.insert(field::HEART_BT_INT);
            }
            field::values::NEW_ORDER_SINGLE => {
                fields.insert(field::CL_ORD_ID);
                fields.insert(field::SYMBOL);
                fields.insert(field::SIDE);
                fields.insert(field::ORD_TYPE);
                fields.insert(field::QUANTITY);
                fields.insert(field::TIME_IN_FORCE);
            }
            field::values::EXECUTION_REPORT => {
                fields.insert(field::ORDER_ID);
                fields.insert(field::EXEC_ID);
                fields.insert(field::EXEC_TYPE);
                fields.insert(field::ORD_STATUS);
                fields.insert(field::SYMBOL);
                fields.insert(field::SIDE);
                fields.insert(field::QUANTITY);
            }
            _ => {}
        }
        fields
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Field;

    #[test]
    fn test_validate_logon() {
        let mut msg = Message::new(field::values::LOGON);
        // Add header fields
        msg.set_field(Field::new(field::BEGIN_STRING, "FIX.4.2"));
        msg.set_field(Field::new(field::BODY_LENGTH, "100"));
        msg.set_field(Field::new(field::MSG_TYPE, field::values::LOGON));
        msg.set_field(Field::new(field::SENDER_COMP_ID, "SENDER"));
        msg.set_field(Field::new(field::TARGET_COMP_ID, "TARGET"));
        msg.set_field(Field::new(field::MSG_SEQ_NUM, "1"));
        msg.set_field(Field::new(field::SENDING_TIME, "20250124-12:00:00"));
        
        // Add Logon-specific fields
        msg.set_field(Field::new(field::ENCRYPT_METHOD, "0"));
        msg.set_field(Field::new(field::HEART_BT_INT, "30"));

        assert!(MessageValidator::validate(&msg).is_ok());
    }

    #[test]
    fn test_validate_missing_required_field() {
        let mut msg = Message::new(field::values::LOGON);
        // Missing some required fields
        msg.set_field(Field::new(field::BEGIN_STRING, "FIX.4.2"));
        msg.set_field(Field::new(field::MSG_TYPE, field::values::LOGON));
        
        assert!(MessageValidator::validate(&msg).is_err());
    }
}
