pub mod field;
pub mod parser;
pub mod validator;
pub mod formatter;
pub mod pool;
pub mod group;

use chrono;
use std::collections::HashMap;
use std::sync::Arc;
use std::fmt;
pub use self::field::Field;
use crate::message::parser::MessageParser;
use crate::Result;
use crate::error::FixError;
use crate::message::formatter::FieldFormatter;
pub use self::pool::MessagePool;
use self::group::RepeatingGroup;

#[derive(Clone)]
pub struct Message {
    fields: HashMap<i32, Field>,
    msg_type: String,
    formatters: HashMap<i32, Arc<dyn FieldFormatter>>,
    groups: HashMap<i32, RepeatingGroup>,
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Message")
            .field("fields", &self.fields)
            .field("msg_type", &self.msg_type)
            .field("groups", &self.groups)
            .finish_non_exhaustive() // Skip formatters field in Debug output
    }
}

impl Message {
    pub fn new(msg_type: &str) -> Self {
        let mut msg = Message {
            fields: HashMap::new(),
            msg_type: msg_type.to_string(),
            formatters: HashMap::new(),
            groups: HashMap::new(),
        };
        msg.set_default_headers();
        msg
    }

    pub fn from_string(msg_str: &str) -> Result<Self> {
        MessageParser::parse(msg_str)
    }

    pub fn set_formatter(&mut self, tag: i32, formatter: impl FieldFormatter + 'static) {
        self.formatters.insert(tag, Arc::new(formatter));
    }

    fn set_default_headers(&mut self) {
        // Set BeginString
        self.set_field(Field::new(self::field::BEGIN_STRING, "FIX.4.2")).unwrap();
        // Set MsgType
        self.set_field(Field::new(self::field::MSG_TYPE, &self.msg_type)).unwrap();
        // Set SendingTime
        let timestamp = chrono::Utc::now().format("%Y%m%d-%H:%M:%S").to_string();
        self.set_field(Field::new(self::field::SENDING_TIME, timestamp)).unwrap();
        // Set default sender and target comp ids
        self.set_field(Field::new(self::field::SENDER_COMP_ID, "SENDER")).unwrap();
        self.set_field(Field::new(self::field::TARGET_COMP_ID, "TARGET")).unwrap();
        // Set initial sequence number
        self.set_field(Field::new(self::field::MSG_SEQ_NUM, "1")).unwrap();
    }

    pub fn set_field(&mut self, field: Field) -> Result<()> {
        let tag = field.tag();
        let value = if let Some(formatter) = self.formatters.get(&tag) {
            formatter.format(field.value())?
        } else {
            field.value().to_string()
        };

        self.fields.insert(tag, Field::new(tag, value));
        Ok(())
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
        if let Some(begin_str) = self.get_field(self::field::BEGIN_STRING) {
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
            if tag != self::field::BEGIN_STRING && tag != self::field::BODY_LENGTH && tag != self::field::CHECKSUM {
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

    pub fn add_group(&mut self, tag: i32, delimiter_tag: i32, required_tags: Vec<i32>) -> Result<()> {
        let group = RepeatingGroup::new(delimiter_tag, required_tags);
        self.groups.insert(tag, group);
        Ok(())
    }

    pub fn get_group(&self, tag: i32) -> Option<&RepeatingGroup> {
        self.groups.get(&tag)
    }

    pub fn get_group_mut(&mut self, tag: i32) -> Option<&mut RepeatingGroup> {
        self.groups.get_mut(&tag)
    }

    pub fn get_field_at(&self, tag: i32, position: usize) -> Option<&Field> {
        if let Some(group) = self.groups.values().find(|g| g.has_field(tag)) {
            group.get_field_at(tag, position)
        } else {
            self.fields.get(&tag)
        }
    }

    pub fn set_field_at(&mut self, position: usize, field: Field) -> Result<()> {
        let tag = field.tag();
        if let Some(group) = self.groups.values_mut().find(|g| g.has_field(tag)) {
            group.set_field_at(position, field)
        } else {
            self.fields.insert(tag, field);
            Ok(())
        }
    }
}

fn calculate_checksum(msg: &str) -> u32 {
    msg.bytes().map(|b| b as u32).sum::<u32>() % 256
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::field::values;
    use crate::message::formatter::{DateTimeFormatter, DecimalFormatter};

    #[test]
    fn test_message_creation() {
        let mut msg = Message::new(values::NEW_ORDER_SINGLE);
        msg.set_field(Field::new(super::field::CL_ORD_ID, "12345")).unwrap();
        msg.set_field(Field::new(super::field::SYMBOL, "AAPL")).unwrap();
        msg.set_field(Field::new(super::field::SIDE, values::BUY)).unwrap();

        assert_eq!(msg.msg_type(), values::NEW_ORDER_SINGLE);
        assert_eq!(msg.get_field(super::field::SYMBOL).unwrap().value(), "AAPL");
    }

    #[test]
    fn test_message_serialization() {
        let mut msg = Message::new(values::HEARTBEAT);
        msg.set_field(Field::new(super::field::SENDER_COMP_ID, "SENDER")).unwrap();
        msg.set_field(Field::new(super::field::TARGET_COMP_ID, "TARGET")).unwrap();
        msg.set_field(Field::new(super::field::MSG_SEQ_NUM, "1")).unwrap();

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
        msg.set_field(Field::new(super::field::CL_ORD_ID, "12345")).unwrap();

        let fields = msg.fields();
        assert!(fields.contains_key(&super::field::CL_ORD_ID));
        assert!(fields.contains_key(&super::field::BEGIN_STRING));
        assert!(fields.contains_key(&super::field::MSG_TYPE));
    }

    #[test]
    fn test_message_roundtrip() {
        let mut original = Message::new(values::NEW_ORDER_SINGLE);
        original.set_field(Field::new(super::field::CL_ORD_ID, "12345")).unwrap();
        original.set_field(Field::new(super::field::SYMBOL, "AAPL")).unwrap();
        original.set_field(Field::new(super::field::SIDE, values::BUY)).unwrap();
        original.set_field(Field::new(super::field::SENDER_COMP_ID, "SENDER")).unwrap();
        original.set_field(Field::new(super::field::TARGET_COMP_ID, "TARGET")).unwrap();
        original.set_field(Field::new(super::field::MSG_SEQ_NUM, "1")).unwrap();

        let msg_str = original.to_string().unwrap();
        let parsed = Message::from_string(&msg_str).unwrap();

        assert_eq!(parsed.msg_type(), original.msg_type());
        assert_eq!(
            parsed.get_field(super::field::CL_ORD_ID).unwrap().value(),
            original.get_field(super::field::CL_ORD_ID).unwrap().value()
        );
        assert_eq!(
            parsed.get_field(super::field::SYMBOL).unwrap().value(),
            original.get_field(super::field::SYMBOL).unwrap().value()
        );
        assert_eq!(
            parsed.get_field(super::field::SIDE).unwrap().value(),
            original.get_field(super::field::SIDE).unwrap().value()
        );
    }

    #[test]
    fn test_message_with_formatters() {
        let mut msg = Message::new(values::NEW_ORDER_SINGLE);

        // Add formatters
        msg.set_formatter(super::field::SENDING_TIME, DateTimeFormatter);
        msg.set_formatter(super::field::PRICE, DecimalFormatter::new(2));

        // Set fields with formatting
        msg.set_field(Field::new(super::field::SENDING_TIME, "2025-01-24T12:34:56Z")).unwrap();
        msg.set_field(Field::new(super::field::PRICE, "123.456")).unwrap();

        // Verify formatted values
        assert_eq!(
            msg.get_field(super::field::SENDING_TIME).unwrap().value(),
            "20250124-12:34:56"
        );
        assert_eq!(
            msg.get_field(super::field::PRICE).unwrap().value(),
            "123.46"
        );
    }
}