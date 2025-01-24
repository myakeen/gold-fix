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

    pub fn extract_complete_message(buffer: &[u8]) -> Option<(String, usize)> {
        let mut start_idx = None;
        let mut end_idx = None;

        // Find the start of the message (8=FIX)
        for (i, window) in buffer.windows(2).enumerate() {
            if window == b"8=" {
                start_idx = Some(i);
                break;
            }
        }

        // Find the end of the message (10=xxx<SOH>)
        if let Some(start) = start_idx {
            for (i, window) in buffer[start..].windows(4).enumerate() {
                if window[0] == b'1' && window[1] == b'0' && window[2] == b'=' {
                    // Look for the SOH character after the checksum
                    for j in i + 3..buffer.len() {
                        if buffer[j] == 1 {  // SOH character
                            end_idx = Some(start + j + 1);
                            break;
                        }
                    }
                    break;
                }
            }
        }

        if let (Some(start), Some(end)) = (start_idx, end_idx) {
            if let Ok(msg_str) = String::from_utf8(buffer[start..end].to_vec()) {
                return Some((msg_str, end));
            }
        }

        None
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

    #[test]
    fn test_extract_complete_message() {
        let msg = b"8=FIX.4.2\x0149=SENDER\x0156=TARGET\x0135=0\x0134=1\x0152=20250124-12:00:00\x0110=123\x01";
        let result = MessageParser::extract_complete_message(msg);
        assert!(result.is_some());
        let (msg_str, len) = result.unwrap();
        assert_eq!(len, msg.len());
        assert!(msg_str.starts_with("8=FIX.4.2"));
        assert!(msg_str.ends_with("\u{1}"));
    }

    #[test]
    fn test_parse_invalid_message() {
        let msg_str = "invalid message";
        let result = MessageParser::parse(msg_str);
        assert!(result.is_err());
    }
}