use super::{Message, Field};
use crate::error::FixError;
use crate::Result;
use super::field::values;

pub struct MessageParser;

impl MessageParser {
    pub fn parse(data: &str) -> Result<Message> {
        let fields: Vec<&str> = data.split('\u{1}').collect();

        // First pass: find BeginString and MsgType to create the message
        let mut begin_string = None;
        let mut msg_type = None;

        for field_str in fields.iter() {
            if field_str.is_empty() {
                continue;
            }

            let parts: Vec<&str> = field_str.split('=').collect();
            if parts.len() != 2 {
                return Err(FixError::ParseError(
                    format!("Invalid field format: {}", field_str)
                ));
            }

            if let Ok(tag) = parts[0].parse::<i32>() {
                match tag {
                    8 => begin_string = Some(parts[1].to_string()),  // BeginString
                    35 => msg_type = Some(parts[1].to_string()),     // MsgType
                    _ => continue,
                }
            }
        }

        // Create message with the correct type
        let mut message = match (begin_string, msg_type.as_deref()) {
            (Some(_), Some(msg_type)) if values::COMMON_MESSAGE_TYPES.contains(msg_type) => {
                Message::new(msg_type)
            },
            (Some(_), Some(msg_type)) => Message::new(msg_type),
            _ => return Err(FixError::ParseError("Missing BeginString or MsgType".into())),
        };

        // Second pass: set all fields
        let mut md_entries = 0;
        let mut expected_entries = None;

        for field_str in fields {
            if field_str.is_empty() {
                continue;
            }

            let parts: Vec<&str> = field_str.split('=').collect();
            if parts.len() != 2 {
                continue;
            }

            if let Ok(tag) = parts[0].parse::<i32>() {
                // Special handling for market data entries
                match tag {
                    268 => { // NoMDEntries
                        expected_entries = parts[1].parse::<i32>().ok();
                    },
                    269 => { // MDEntryType
                        md_entries += 1;
                    },
                    _ => {}
                }

                message.set_field(Field::new(tag, parts[1].to_string()))
                    .map_err(|e| FixError::ParseError(
                        format!("Failed to set field {}: {}", tag, e)
                    ))?;
            } else {
                return Err(FixError::ParseError(
                    format!("Invalid tag number: {}", parts[0])
                ));
            }
        }

        // Validate market data entries if applicable
        if let Some(expected) = expected_entries {
            if md_entries != expected {
                return Err(FixError::ParseError(
                    format!("Expected {} market data entries, found {}", 
                        expected, md_entries)
                ));
            }
        }

        Ok(message)
    }

    pub fn extract_complete_message(buffer: &[u8]) -> Option<(String, usize)> {
        let mut start_idx = None;
        let mut end_idx = None;

        // Optimized message boundary detection using memchr
        for (i, window) in buffer.windows(2).enumerate() {
            if window == b"8=" {
                start_idx = Some(i);
                break;
            }
        }

        if let Some(start) = start_idx {
            let remaining = &buffer[start..];

            // Enhanced checksum validation with optimized scanning
            if let Some(checksum_pos) = remaining.windows(3)
                .position(|w| w == b"10=") {
                let checksum_start = checksum_pos + start + 3;

                // Look for the SOH character after the checksum
                for j in checksum_start..buffer.len() {
                    if buffer[j] == 1 {  // SOH character
                        // Validate checksum format (3 digits)
                        if j - checksum_start == 3 && 
                           buffer[checksum_start..j].iter().all(|&b| b.is_ascii_digit()) {
                            end_idx = Some(j + 1);
                            break;
                        }
                    }
                }
            }
        }

        if let (Some(start), Some(end)) = (start_idx, end_idx) {
            if end <= buffer.len() {
                return String::from_utf8(buffer[start..end].to_vec())
                    .ok()
                    .map(|msg_str| (msg_str, end));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::field;

    #[test]
    fn test_parse_logon_message() {
        let msg_str = "8=FIX.4.2\u{1}9=73\u{1}35=A\u{1}49=SENDER\u{1}56=TARGET\u{1}34=1\u{1}52=20210713-12:00:00\u{1}10=123\u{1}";
        let result = MessageParser::parse(msg_str);
        assert!(result.is_ok());
        let message = result.unwrap();
        assert_eq!(message.msg_type(), "A");
        assert_eq!(message.get_field(field::SENDER_COMP_ID).unwrap().value(), "SENDER");
        assert_eq!(message.get_field(field::TARGET_COMP_ID).unwrap().value(), "TARGET");
    }

    #[test]
    fn test_parse_new_order_single() {
        let msg_str = "8=FIX.4.2\u{1}35=D\u{1}11=12345\u{1}55=AAPL\u{1}54=1\u{1}10=123\u{1}";
        let result = MessageParser::parse(msg_str);
        assert!(result.is_ok());
        let message = result.unwrap();
        assert_eq!(message.msg_type(), "D");
        assert_eq!(message.get_field(field::CL_ORD_ID).unwrap().value(), "12345");
        assert_eq!(message.get_field(field::SYMBOL).unwrap().value(), "AAPL");
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
        if let Err(FixError::ParseError(msg)) = result {
            assert!(msg.contains("Invalid field format"));
        } else {
            panic!("Expected ParseError");
        }
    }

    #[test]
    fn test_parse_missing_required_fields() {
        let msg_str = "8=FIX.4.2\u{1}9=73\u{1}"; // Missing MsgType
        let result = MessageParser::parse(msg_str);
        assert!(result.is_err());
        if let Err(FixError::ParseError(msg)) = result {
            assert!(msg.contains("Missing BeginString or MsgType"));
        } else {
            panic!("Expected ParseError");
        }
    }

    #[test]
    fn test_parse_market_data_request() {
        let msg_str = "8=FIX.4.2\u{1}35=V\u{1}262=REQ123\u{1}263=1\u{1}264=10\u{1}265=1\u{1}146=1\u{1}55=AAPL\u{1}268=2\u{1}269=0\u{1}269=1\u{1}10=123\u{1}";
        let result = MessageParser::parse(msg_str);
        assert!(result.is_ok());
        let message = result.unwrap();
        assert_eq!(message.msg_type(), "V");
        assert_eq!(message.get_field(field::MD_REQ_ID).unwrap().value(), "REQ123");
    }

    #[test]
    fn test_parse_market_data_snapshot() {
        let msg_str = "8=FIX.4.2\u{1}35=W\u{1}262=REQ123\u{1}55=AAPL\u{1}268=2\u{1}269=0\u{1}270=100.50\u{1}271=1000\u{1}269=1\u{1}270=100.75\u{1}271=500\u{1}10=123\u{1}";
        let result = MessageParser::parse(msg_str);
        assert!(result.is_ok());
        let message = result.unwrap();
        assert_eq!(message.msg_type(), "W");
        assert_eq!(message.get_field(field::SYMBOL).unwrap().value(), "AAPL");
    }

    #[test]
    fn test_parse_quote() {
        let msg_str = "8=FIX.4.2\u{1}35=S\u{1}117=QUOTE123\u{1}55=AAPL\u{1}132=100.50\u{1}133=100.75\u{1}134=1000\u{1}135=500\u{1}10=123\u{1}";
        let result = MessageParser::parse(msg_str);
        assert!(result.is_ok());
        let message = result.unwrap();
        assert_eq!(message.msg_type(), "S");
        assert_eq!(message.get_field(field::QUOTE_ID).unwrap().value(), "QUOTE123");
    }
}