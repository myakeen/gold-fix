pub mod field;
pub mod parser;

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
            msg.push_str(&format!("{}={}\u{1}", field.tag(), field.value()));
        }

        // Calculate body length and checksum
        let body_length = msg.len();
        let checksum = calculate_checksum(&msg);

        // Replace body length placeholder
        msg = msg.replace("9=0\u{1}", &format!("9={}\u{1}", body_length));

        // Add checksum
        msg.push_str(&format!("10={:03}\u{1}", checksum));

        Ok(msg)
    }
}

fn calculate_checksum(msg: &str) -> u32 {
    msg.bytes().map(|b| b as u32).sum::<u32>() % 256
}