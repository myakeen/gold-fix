use super::{Message, Field};
use crate::error::FixError;
use crate::Result;

pub struct MessageParser;

impl MessageParser {
    pub fn parse(data: &str) -> Result<Message> {
        let fields: Vec<&str> = data.split('\u{1}').collect();
        let mut message = None;

        for field_str in fields {
            if field_str.is_empty() {
                continue;
            }

            let parts: Vec<&str> = field_str.split('=').collect();
            if parts.len() != 2 {
                return Err(FixError::ParseError(
                    format!("Invalid field format: {}", field_str)
                ));
            }

            let tag = parts[0].parse::<i32>().map_err(|_| {
                FixError::ParseError(format!("Invalid tag: {}", parts[0]))
            })?;

            if tag == 35 {  // MsgType
                message = Some(Message::new(parts[1]));
            }

            if let Some(ref mut msg) = message {
                msg.set_field(Field::new(tag, parts[1].to_string()));
            }
        }

        message.ok_or_else(|| FixError::ParseError("Missing message type".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_logon_message() {
        let msg_str = "8=FIX.4.2\u{1}9=73\u{1}35=A\u{1}49=SENDER\u{1}56=TARGET\u{1}34=1\u{1}52=20210713-12:00:00\u{1}10=123\u{1}";
        let result = MessageParser::parse(msg_str);
        assert!(result.is_ok());
        let message = result.unwrap();
        assert_eq!(message.msg_type(), "A");
    }
}
