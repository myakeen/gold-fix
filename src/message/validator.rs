use std::collections::{HashMap, HashSet};
use crate::message::{Message, field};
use crate::Result;
use crate::error::FixError;

pub struct MessageValidator;

impl MessageValidator {
    pub fn validate(message: &Message) -> Result<()> {
        // Check required header fields
        let required_header = vec![
            field::BEGIN_STRING,
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

        // Validate field values
        Self::validate_field_values(message)?;

        // Validate conditional fields
        Self::validate_conditional_fields(message)?;

        // Validate sequence numbers
        Self::validate_sequence_numbers(message)?;

        // Validate sending time
        Self::validate_sending_time(message)?;

        Ok(())
    }

    fn validate_field_values(message: &Message) -> Result<()> {
        for (&tag, field) in message.fields() {
            match tag {
                field::MSG_SEQ_NUM => {
                    if field.value().parse::<i32>().is_err() || field.value().parse::<i32>().unwrap() <= 0 {
                        return Err(FixError::ParseError(
                            format!("Invalid MsgSeqNum value: {}", field.value())
                        ));
                    }
                },
                field::PRICE | field::QUANTITY | field::STOP_PX | field::MIN_QTY | field::MAX_FLOOR |
                field::MD_ENTRY_PX | field::MD_ENTRY_SIZE | field::BID_PX | field::OFFER_PX |
                field::BID_SIZE | field::OFFER_SIZE => {
                    if let Err(_) = field.value().parse::<f64>() {
                        return Err(FixError::ParseError(
                            format!("Invalid numeric value for tag {}: {}", tag, field.value())
                        ));
                    }
                },
                field::SENDING_TIME | field::TRANSACTION_TIME | field::EXPIRE_TIME |
                field::MD_ENTRY_TIME | field::VALID_UNTIL_TIME => {
                    if !Self::is_valid_timestamp(field.value()) {
                        return Err(FixError::ParseError(
                            format!("Invalid timestamp format: {}", field.value())
                        ));
                    }
                },
                field::SIDE => {
                    if !Self::is_valid_side(field.value()) {
                        return Err(FixError::ParseError(
                            format!("Invalid side value: {}", field.value())
                        ));
                    }
                },
                field::ORD_TYPE => {
                    if !Self::is_valid_order_type(field.value()) {
                        return Err(FixError::ParseError(
                            format!("Invalid order type: {}", field.value())
                        ));
                    }
                },
                field::TIME_IN_FORCE => {
                    if !Self::is_valid_time_in_force(field.value()) {
                        return Err(FixError::ParseError(
                            format!("Invalid time in force: {}", field.value())
                        ));
                    }
                },
                field::MD_ENTRY_TYPE => {
                    if !Self::is_valid_md_entry_type(field.value()) {
                        return Err(FixError::ParseError(
                            format!("Invalid market data entry type: {}", field.value())
                        ));
                    }
                },
                field::QUOTE_CONDITION => {
                    if !Self::is_valid_quote_condition(field.value()) {
                        return Err(FixError::ParseError(
                            format!("Invalid quote condition: {}", field.value())
                        ));
                    }
                },
                _ => {}
            }
        }
        Ok(())
    }

    fn validate_conditional_fields(message: &Message) -> Result<()> {
        match message.msg_type() {
            field::values::EXECUTION_REPORT => {
                Self::validate_execution_report(message)?;
            },
            field::values::NEW_ORDER_SINGLE => {
                Self::validate_new_order_single(message)?;
            },
            field::values::ORDER_CANCEL_REPLACE_REQUEST => {
                Self::validate_order_cancel_replace(message)?;
            },
            field::values::QUOTE_REQUEST => {
                Self::validate_quote_request(message)?;
            },
            field::values::MARKET_DATA_REQUEST => {
                Self::validate_market_data_request(message)?;
            },
            field::values::QUOTE => {
                Self::validate_quote(message)?;
            },
            field::values::MARKET_DATA_SNAPSHOT => {
                Self::validate_market_data_snapshot(message)?;
            },
            _ => {}
        }
        Ok(())
    }

    fn validate_market_data_request(message: &Message) -> Result<()> {
        // Required fields for market data request
        let required_fields = vec![
            field::MD_REQ_ID,
            field::SUBSCRIPTION_REQ_TYPE,
            field::MARKET_DEPTH,
            field::NO_MD_ENTRIES,
            field::SYMBOL,
        ];

        for &tag in &required_fields {
            if message.get_field(tag).is_none() {
                return Err(FixError::ParseError(
                    format!("Missing required field for market data request: {}", tag)
                ));
            }
        }

        // Validate subscription type
        if let Some(sub_type) = message.get_field(field::SUBSCRIPTION_REQ_TYPE) {
            let sub_type_val = sub_type.value().parse::<i32>()
                .map_err(|_| FixError::ParseError("Invalid SubscriptionRequestType".into()))?;
            if !(0..=2).contains(&sub_type_val) {
                return Err(FixError::ParseError("Invalid SubscriptionRequestType value".into()));
            }
        }

        // Validate market depth
        if let Some(depth) = message.get_field(field::MARKET_DEPTH) {
            let depth_val = depth.value().parse::<i32>()
                .map_err(|_| FixError::ParseError("Invalid MarketDepth".into()))?;
            if depth_val < 0 {
                return Err(FixError::ParseError("MarketDepth must be non-negative".into()));
            }
        }

        // Validate NoMDEntries
        if let Some(no_entries) = message.get_field(field::NO_MD_ENTRIES) {
            if let Err(_) = no_entries.value().parse::<usize>() {
                return Err(FixError::ParseError("Invalid NoMDEntries".into()));
            }
        }

        Ok(())
    }

    fn validate_market_data_snapshot(message: &Message) -> Result<()> {
        // Check required fields
        if message.get_field(field::SYMBOL).is_none() {
            return Err(FixError::ParseError("Missing Symbol in market data snapshot".into()));
        }

        // Set up repeating group if not already present
        if message.get_group(field::NO_MD_ENTRIES).is_none() {
            return Err(FixError::ParseError("Missing market data entries group".into()));
        }

        if let Some(no_entries) = message.get_field(field::NO_MD_ENTRIES) {
            let expected_count = no_entries.value().parse::<usize>()
                .map_err(|_| FixError::ParseError("Invalid NoMDEntries value".into()))?;

            let group = message.get_group(field::NO_MD_ENTRIES)
                .ok_or_else(|| FixError::ParseError("Missing market data entries group".into()))?;

            // Validate group structure
            if let Err(e) = group.validate() {
                return Err(FixError::ParseError(format!("Invalid market data group: {}", e)));
            }

            // Validate entry count
            if group.entry_count() != expected_count {
                return Err(FixError::ParseError(
                    format!("NoMDEntries ({}) doesn't match actual entries ({})",
                        expected_count, group.entry_count())
                ));
            }

            // Validate each entry
            for i in 0..group.entry_count() {
                // Check entry type
                let entry_type = group.get_field_at(field::MD_ENTRY_TYPE, i)
                    .ok_or_else(|| FixError::ParseError(format!("Missing MDEntryType at position {}", i)))?;

                if !Self::is_valid_md_entry_type(entry_type.value()) {
                    return Err(FixError::ParseError(
                        format!("Invalid MDEntryType: {} at position {}", entry_type.value(), i)
                    ));
                }

                // Check price and size
                let price = group.get_field_at(field::MD_ENTRY_PX, i)
                    .ok_or_else(|| FixError::ParseError(format!("Missing MDEntryPx at position {}", i)))?;

                let size = group.get_field_at(field::MD_ENTRY_SIZE, i)
                    .ok_or_else(|| FixError::ParseError(format!("Missing MDEntrySize at position {}", i)))?;

                // Validate numeric values
                if price.value().parse::<f64>().is_err() {
                    return Err(FixError::ParseError(
                        format!("Invalid price format at position {}: {}", i, price.value())
                    ));
                }

                if size.value().parse::<f64>().is_err() {
                    return Err(FixError::ParseError(
                        format!("Invalid size format at position {}: {}", i, size.value())
                    ));
                }
            }
        }

        Ok(())
    }

    fn validate_quote(message: &Message) -> Result<()> {
        // Quote must have either bid or offer
        if message.get_field(field::BID_PX).is_none() && message.get_field(field::OFFER_PX).is_none() {
            return Err(FixError::ParseError("Quote must have either BidPx or OfferPx".into()));
        }

        // If bid present, must have bid size
        if message.get_field(field::BID_PX).is_some() && message.get_field(field::BID_SIZE).is_none() {
            return Err(FixError::ParseError("BidSize is required when BidPx is present".into()));
        }

        // If offer present, must have offer size
        if message.get_field(field::OFFER_PX).is_some() && message.get_field(field::OFFER_SIZE).is_none() {
            return Err(FixError::ParseError("OfferSize is required when OfferPx is present".into()));
        }

        // Validate price and size values
        if let Some(bid_px) = message.get_field(field::BID_PX) {
            if bid_px.value().parse::<f64>().is_err() {
                return Err(FixError::ParseError("Invalid BidPx value".into()));
            }
        }

        if let Some(offer_px) = message.get_field(field::OFFER_PX) {
            if offer_px.value().parse::<f64>().is_err() {
                return Err(FixError::ParseError("Invalid OfferPx value".into()));
            }
        }

        // Validate bid <= offer if both present
        if let (Some(bid_px), Some(offer_px)) = (
            message.get_field(field::BID_PX).and_then(|f| f.value().parse::<f64>().ok()),
            message.get_field(field::OFFER_PX).and_then(|f| f.value().parse::<f64>().ok())
        ) {
            if bid_px > offer_px {
                return Err(FixError::ParseError("BidPx cannot be greater than OfferPx".into()));
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
            field::values::MARKET_DATA_REQUEST => {
                fields.insert(field::MD_REQ_ID);
                fields.insert(field::SUBSCRIPTION_REQ_TYPE);
                fields.insert(field::MARKET_DEPTH);
                fields.insert(field::NO_MD_ENTRIES);
            }
            field::values::QUOTE_REQUEST => {
                fields.insert(field::QUOTE_REQ_ID);
                fields.insert(field::SYMBOL);
            }
            field::values::QUOTE => {
                fields.insert(field::QUOTE_ID);
                fields.insert(field::SYMBOL);
            }
            _ => {}
        }
        fields
    }

    fn is_valid_timestamp(timestamp: &str) -> bool {
        let re = regex::Regex::new(r"^\d{8}-\d{2}:\d{2}:\d{2}(\.\d{3})?$").unwrap();
        re.is_match(timestamp)
    }

    fn is_valid_side(side: &str) -> bool {
        matches!(side, 
            field::values::BUY | field::values::SELL | 
            field::values::BUY_MINUS | field::values::SELL_PLUS |
            field::values::SELL_SHORT | field::values::SELL_SHORT_EXEMPT
        )
    }

    fn is_valid_md_entry_type(entry_type: &str) -> bool {
        matches!(entry_type,
            field::values::MD_ENTRY_BID | field::values::MD_ENTRY_OFFER |
            field::values::MD_ENTRY_TRADE | field::values::MD_ENTRY_INDEX |
            field::values::MD_ENTRY_OPENING | field::values::MD_ENTRY_CLOSING |
            field::values::MD_ENTRY_HIGH | field::values::MD_ENTRY_LOW
        )
    }

    fn is_valid_quote_condition(condition: &str) -> bool {
        matches!(condition,
            field::values::QUOTE_OPEN | field::values::QUOTE_CLOSED |
            field::values::QUOTE_OUTRIGHT | field::values::QUOTE_CROSS |
            field::values::QUOTE_LOCKED | field::values::QUOTE_CROSSED
        )
    }

    fn is_valid_order_type(ord_type: &str) -> bool {
        matches!(ord_type,
            field::values::MARKET | field::values::LIMIT |
            field::values::STOP | field::values::STOP_LIMIT |
            field::values::MARKET_ON_CLOSE | field::values::WITH_OR_WITHOUT |
            field::values::LIMIT_OR_BETTER | field::values::LIMIT_WITH_OR_WITHOUT
        )
    }

    fn is_valid_time_in_force(tif: &str) -> bool {
        matches!(tif,
            field::values::DAY | field::values::GOOD_TILL_CANCEL |
            field::values::AT_THE_OPENING | field::values::IMMEDIATE_OR_CANCEL |
            field::values::FILL_OR_KILL | field::values::GOOD_TILL_DATE
        )
    }

    fn is_valid_exec_type_ord_status(exec_type: &str, ord_status: &str) -> bool {
        let valid_combinations: HashMap<&str, HashSet<&str>> = [
            ("0", vec!["0", "1"].into_iter().collect()),
            ("1", vec!["1", "2"].into_iter().collect()),
            ("2", vec!["2"].into_iter().collect()),
            ("4", vec!["4"].into_iter().collect()),
            ("C", vec!["C"].into_iter().collect()),
        ].iter().cloned().collect();

        valid_combinations.get(exec_type)
            .map_or(false, |valid_statuses| valid_statuses.contains(ord_status))
    }

    fn validate_sending_time(message: &Message) -> Result<()> {
        if let Some(sending_time) = message.get_field(field::SENDING_TIME) {
            if !Self::is_valid_timestamp(sending_time.value()) {
                return Err(FixError::ParseError(
                    format!("Invalid SendingTime format: {}", sending_time.value())
                ));
            }
        }
        Ok(())
    }

    fn validate_sequence_numbers(message: &Message) -> Result<()> {
        if let Some(seq_num) = message.get_field(field::MSG_SEQ_NUM) {
            let seq = seq_num.value().parse::<i32>()
                .map_err(|_| FixError::ParseError("Invalid MsgSeqNum".into()))?;
            if seq <= 0 {
                return Err(FixError::ParseError("MsgSeqNum must be positive".into()));
            }
        }
        Ok(())
    }

    fn validate_execution_report(message: &Message) -> Result<()> {
        if let (Some(exec_type), Some(ord_status)) = (
            message.get_field(field::EXEC_TYPE),
            message.get_field(field::ORD_STATUS)
        ) {
            if !Self::is_valid_exec_type_ord_status(exec_type.value(), ord_status.value()) {
                return Err(FixError::ParseError(
                    format!("Invalid ExecType({}) and OrdStatus({}) combination",
                        exec_type.value(), ord_status.value())
                ));
            }
        }
        Ok(())
    }

    fn validate_new_order_single(message: &Message) -> Result<()> {
        // Validate price for LIMIT orders
        if let Some(ord_type) = message.get_field(field::ORD_TYPE) {
            match ord_type.value() {
                field::values::LIMIT => {
                    if message.get_field(field::PRICE).is_none() {
                        return Err(FixError::ParseError("Price is required for LIMIT orders".into()));
                    }
                },
                field::values::STOP => {
                    if message.get_field(field::STOP_PX).is_none() {
                        return Err(FixError::ParseError("StopPx is required for STOP orders".into()));
                    }
                },
                field::values::STOP_LIMIT => {
                    if message.get_field(field::PRICE).is_none() || message.get_field(field::STOP_PX).is_none() {
                        return Err(FixError::ParseError(
                            "Both Price and StopPx are required for STOP LIMIT orders".into()
                        ));
                    }
                },
                _ => {}
            }
        }
        Ok(())
    }

    fn validate_order_cancel_replace(message: &Message) -> Result<()> {
        // Original order ID is required
        if message.get_field(field::ORDER_ID).is_none() {
            return Err(FixError::ParseError("OrderID is required for cancel/replace".into()));
        }
        Ok(())
    }

    fn validate_quote_request(message: &Message) -> Result<()> {
        // Required fields for quote requests
        let required_fields = vec![
            field::QUOTE_REQ_ID,
            field::SYMBOL,
        ];

        for &tag in &required_fields {
            if message.get_field(tag).is_none() {
                return Err(FixError::ParseError(
                    format!("Missing required field for quote request: {}", tag)
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Field;
    use crate::message::field::values;

    #[test]
    fn test_validate_new_order_types() {
        let mut msg = Message::new(values::NEW_ORDER_SINGLE);
        // Add required fields
        msg.set_field(Field::new(field::BEGIN_STRING, "FIX.4.2")).unwrap();
        msg.set_field(Field::new(field::MSG_TYPE, values::NEW_ORDER_SINGLE)).unwrap();
        msg.set_field(Field::new(field::SENDER_COMP_ID, "SENDER")).unwrap();
        msg.set_field(Field::new(field::TARGET_COMP_ID, "TARGET")).unwrap();
        msg.set_field(Field::new(field::MSG_SEQ_NUM, "1")).unwrap();
        msg.set_field(Field::new(field::SENDING_TIME, "20250124-12:00:00")).unwrap();
        msg.set_field(Field::new(field::CL_ORD_ID, "123456")).unwrap();
        msg.set_field(Field::new(field::SYMBOL, "AAPL")).unwrap();
        msg.set_field(Field::new(field::SIDE, values::BUY)).unwrap();
        msg.set_field(Field::new(field::QUANTITY, "100")).unwrap();
        msg.set_field(Field::new(field::TIME_IN_FORCE, values::DAY)).unwrap();

        // Test LIMIT order
        msg.set_field(Field::new(field::ORD_TYPE, values::LIMIT)).unwrap();
        msg.set_field(Field::new(field::PRICE, "150.50")).unwrap();
        assert!(MessageValidator::validate(&msg).is_ok());

        // Test STOP order
        msg.set_field(Field::new(field::ORD_TYPE, values::STOP)).unwrap();
        msg.set_field(Field::new(field::STOP_PX, "155.00")).unwrap();
        assert!(MessageValidator::validate(&msg).is_ok());
    }


    #[test]
    fn test_validate_logon() {
        let mut msg = Message::new(values::LOGON);
        // Add header fields with proper error handling
        let _ = msg.set_field(Field::new(field::BEGIN_STRING, "FIX.4.2"));
        let _ = msg.set_field(Field::new(field::MSG_TYPE, field::values::LOGON));
        let _ = msg.set_field(Field::new(field::SENDER_COMP_ID, "SENDER"));
        let _ = msg.set_field(Field::new(field::TARGET_COMP_ID, "TARGET"));
        let _ = msg.set_field(Field::new(field::MSG_SEQ_NUM, "1"));
        let _ = msg.set_field(Field::new(field::SENDING_TIME, "20250124-12:00:00"));

        // Add Logon-specific fields
        let _ = msg.set_field(Field::new(field::ENCRYPT_METHOD, "0"));
        let _ = msg.set_field(Field::new(field::HEART_BT_INT, "30"));

        assert!(MessageValidator::validate(&msg).is_ok());
    }

    #[test]
    fn test_validate_missing_required_field() {
        let mut msg = Message::new(field::values::LOGON);
        // Missing some required fields
        let _ = msg.set_field(Field::new(field::BEGIN_STRING, "FIX.4.2"));
        let _ = msg.set_field(Field::new(field::MSG_TYPE, field::values::LOGON));

        assert!(MessageValidator::validate(&msg).is_err());
    }

    #[test]
    fn test_validate_new_order_single() {
        let mut msg = Message::new(field::values::NEW_ORDER_SINGLE);
        // Add header fields
        let _ = msg.set_field(Field::new(field::BEGIN_STRING, "FIX.4.2"));
        let _ = msg.set_field(Field::new(field::MSG_TYPE, field::values::NEW_ORDER_SINGLE));
        let _ = msg.set_field(Field::new(field::SENDER_COMP_ID, "SENDER"));
        let _ = msg.set_field(Field::new(field::TARGET_COMP_ID, "TARGET"));
        let _ = msg.set_field(Field::new(field::MSG_SEQ_NUM, "2"));
        let _ = msg.set_field(Field::new(field::SENDING_TIME, "20250124-12:00:00"));

        // Add order fields
        let _ = msg.set_field(Field::new(field::CL_ORD_ID, "123456"));
        let _ = msg.set_field(Field::new(field::SYMBOL, "AAPL"));
        let _ = msg.set_field(Field::new(field::SIDE, "1")); // Buy
        let _ = msg.set_field(Field::new(field::ORD_TYPE, "2")); // Limit
        let _ = msg.set_field(Field::new(field::QUANTITY, "100"));
        let _ = msg.set_field(Field::new(field::TIME_IN_FORCE, "0")); // Day
        let _ = msg.set_field(Field::new(field::PRICE, "150.50")); // Required for LIMIT orders

        assert!(MessageValidator::validate(&msg).is_ok());
    }

    #[test]
    fn test_validate_resend_request() {
        let mut msg = Message::new(field::values::RESEND_REQUEST);
        // Add header fields
        let _ = msg.set_field(Field::new(field::BEGIN_STRING, "FIX.4.2"));
        let _ = msg.set_field(Field::new(field::MSG_TYPE, field::values::RESEND_REQUEST));
        let _ = msg.set_field(Field::new(field::SENDER_COMP_ID, "SENDER"));
        let _ = msg.set_field(Field::new(field::TARGET_COMP_ID, "TARGET"));
        let _ = msg.set_field(Field::new(field::MSG_SEQ_NUM, "1"));
        let _ = msg.set_field(Field::new(field::SENDING_TIME, "20250124-12:00:00"));

        // Add ResendRequest fields
        let _ = msg.set_field(Field::new(field::BEGIN_SEQ_NO, "1"));
        let _ = msg.set_field(Field::new(field::END_SEQ_NO, "10"));

        assert!(MessageValidator::validate(&msg).is_ok());
    }

    #[test]
    fn test_validate_quote_request() {
        let mut msg = Message::new(values::QUOTE_REQUEST);
        // Add required fields
        msg.set_field(Field::new(field::BEGIN_STRING, "FIX.4.2")).unwrap();
        msg.set_field(Field::new(field::MSG_TYPE, values::QUOTE_REQUEST)).unwrap();
        msg.set_field(Field::new(field::SENDER_COMP_ID, "SENDER")).unwrap();
        msg.set_field(Field::new(field::TARGET_COMP_ID, "TARGET")).unwrap();
        msg.set_field(Field::new(field::MSG_SEQ_NUM, "1")).unwrap();
        msg.set_field(Field::new(field::SENDING_TIME, "20250124-12:00:00")).unwrap();
        msg.set_field(Field::new(field::QUOTE_REQ_ID, "QR001")).unwrap(); // Added QUOTE_REQ_ID
        msg.set_field(Field::new(field::SYMBOL, "AAPL")).unwrap();

        assert!(MessageValidator::validate(&msg).is_ok());
    }

    #[test]
    fn test_validate_market_data_request() {
        let mut msg = Message::new(values::MARKET_DATA_REQUEST);
        // Add required fields
        msg.set_field(Field::new(field::BEGIN_STRING, "FIX.4.2")).unwrap();
        msg.set_field(Field::new(field::MSG_TYPE, values::MARKET_DATA_REQUEST)).unwrap();
        msg.set_field(Field::new(field::SENDER_COMP_ID, "SENDER")).unwrap();
        msg.set_field(Field::new(field::TARGET_COMP_ID, "TARGET")).unwrap();
        msg.set_field(Field::new(field::MSG_SEQ_NUM, "1")).unwrap();
        msg.set_field(Field::new(field::SENDING_TIME, "20250124-12:00:00")).unwrap();

        // Add market data specific fields
        msg.set_field(Field::new(field::MD_REQ_ID, "MDR001")).unwrap();
        msg.set_field(Field::new(field::SUBSCRIPTION_REQ_TYPE, "1")).unwrap();
        msg.set_field(Field::new(field::MARKET_DEPTH, "0")).unwrap();
        msg.set_field(Field::new(field::NO_MD_ENTRIES, "2")).unwrap();
        msg.set_field(Field::new(field::SYMBOL, "AAPL")).unwrap();

        assert!(MessageValidator::validate(&msg).is_ok());
    }

    #[test]
    fn test_validate_market_data_snapshot() {
        let mut msg = Message::new(values::MARKET_DATA_SNAPSHOT);

        // Add required header fields
        msg.set_field(Field::new(field::BEGIN_STRING, "FIX.4.2")).unwrap();
        msg.set_field(Field::new(field::MSG_TYPE, values::MARKET_DATA_SNAPSHOT)).unwrap();
        msg.set_field(Field::new(field::SENDER_COMP_ID, "SENDER")).unwrap();
        msg.set_field(Field::new(field::TARGET_COMP_ID, "TARGET")).unwrap();
        msg.set_field(Field::new(field::MSG_SEQ_NUM, "1")).unwrap();
        msg.set_field(Field::new(field::SENDING_TIME, "20250124-12:00:00")).unwrap();

        // Add Symbol field
        msg.set_field(Field::new(field::SYMBOL, "AAPL")).unwrap();

        // Setup market data group
        msg.add_group(field::NO_MD_ENTRIES, field::MD_ENTRY_TYPE, 
            vec![field::MD_ENTRY_PX, field::MD_ENTRY_SIZE]).unwrap();
        msg.set_field(Field::new(field::NO_MD_ENTRIES, "2")).unwrap();

        let group = msg.get_group_mut(field::NO_MD_ENTRIES).unwrap();

        // Add first entry
        let entry = group.add_entry();
        entry.insert(field::MD_ENTRY_TYPE, Field::new(field::MD_ENTRY_TYPE, values::MD_ENTRY_BID));
        entry.insert(field::MD_ENTRY_PX, Field::new(field::MD_ENTRY_PX, "150.25"));
        entry.insert(field::MD_ENTRY_SIZE, Field::new(field::MD_ENTRY_SIZE, "100"));

        // Add second entry
        let entry = group.add_entry();
        entry.insert(field::MD_ENTRY_TYPE, Field::new(field::MD_ENTRY_TYPE, values::MD_ENTRY_OFFER));
        entry.insert(field::MD_ENTRY_PX, Field::new(field::MD_ENTRY_PX, "150.50"));
        entry.insert(field::MD_ENTRY_SIZE, Field::new(field::MD_ENTRY_SIZE, "200"));

        assert!(MessageValidator::validate(&msg).is_ok());
    }

    #[test]
    fn test_validate_market_data_snapshot_with_entries() {
        let mut msg = Message::new(values::MARKET_DATA_SNAPSHOT);
        // Add required header fields
        msg.set_field(Field::new(field::BEGIN_STRING, "FIX.4.2")).unwrap();
        msg.set_field(Field::new(field::MSG_TYPE, values::MARKET_DATA_SNAPSHOT)).unwrap();
        msg.set_field(Field::new(field::SENDER_COMP_ID, "SENDER")).unwrap();
        msg.set_field(Field::new(field::TARGET_COMP_ID, "TARGET")).unwrap();
        msg.set_field(Field::new(field::MSG_SEQ_NUM, "1")).unwrap();
        msg.set_field(Field::new(field::SENDING_TIME, "20250124-12:00:00")).unwrap();

        // Add Symbol and group setup
        msg.set_field(Field::new(field::SYMBOL, "AAPL")).unwrap();
        msg.add_group(field::NO_MD_ENTRIES, field::MD_ENTRY_TYPE,
            vec![field::MD_ENTRY_PX, field::MD_ENTRY_SIZE]).unwrap();
        msg.set_field(Field::new(field::NO_MD_ENTRIES, "2")).unwrap();

        let group = msg.get_group_mut(field::NO_MD_ENTRIES).unwrap();

        // Add entries
        let entry = group.add_entry();
        entry.insert(field::MD_ENTRY_TYPE, Field::new(field::MD_ENTRY_TYPE, values::MD_ENTRY_BID));
        entry.insert(field::MD_ENTRY_PX, Field::new(field::MD_ENTRY_PX, "150.25"));
        entry.insert(field::MD_ENTRY_SIZE, Field::new(field::MD_ENTRY_SIZE, "100"));

        let entry = group.add_entry();
        entry.insert(field::MD_ENTRY_TYPE, Field::new(field::MD_ENTRY_TYPE, values::MD_ENTRY_OFFER));
        entry.insert(field::MD_ENTRY_PX, Field::new(field::MD_ENTRY_PX, "150.50"));
        entry.insert(field::MD_ENTRY_SIZE, Field::new(field::MD_ENTRY_SIZE, "200"));

        assert!(MessageValidator::validate(&msg).is_ok());
    }

    #[test]
    fn test_validate_market_data_snapshot_invalid_entries() {
        let mut msg = Message::new(values::MARKET_DATA_SNAPSHOT);
        // Add required fields
        msg.set_field(Field::new(field::BEGIN_STRING, "FIX.4.2")).unwrap();
        msg.set_field(Field::new(field::MSG_TYPE, values::MARKET_DATA_SNAPSHOT)).unwrap();
        msg.set_field(Field::new(field::SENDER_COMP_ID, "SENDER")).unwrap();
        msg.set_field(Field::new(field::TARGET_COMP_ID, "TARGET")).unwrap();
        msg.set_field(Field::new(field::MSG_SEQ_NUM, "1")).unwrap();
        msg.set_field(Field::new(field::SENDING_TIME, "20250124-12:00:00")).unwrap();

        // Add market data fields with mismatched entry count
        msg.set_field(Field::new(field::SYMBOL, "AAPL")).unwrap();
        msg.set_field(Field::new(field::NO_MD_ENTRIES, "2")).unwrap();

        // Add only one entry when we specified two
        msg.set_field(Field::new(field::MD_ENTRY_TYPE, values::MD_ENTRY_BID)).unwrap();
        msg.set_field(Field::new(field::MD_ENTRY_PX, "150.25")).unwrap();
        msg.set_field(Field::new(field::MD_ENTRY_SIZE, "100")).unwrap();

        assert!(MessageValidator::validate(&msg).is_err());
    }

    #[test]
    fn test_validate_market_data_snapshot_missing_price() {
        let mut msg = Message::new(values::MARKET_DATA_SNAPSHOT);
        // Add required fields
        msg.set_field(Field::new(field::BEGIN_STRING, "FIX.4.2")).unwrap();
        msg.set_field(Field::new(field::MSG_TYPE, values::MARKET_DATA_SNAPSHOT)).unwrap();
        msg.set_field(Field::new(field::SENDER_COMP_ID, "SENDER")).unwrap();
        msg.set_field(Field::new(field::TARGET_COMP_ID, "TARGET")).unwrap();
        msg.set_field(Field::new(field::MSG_SEQ_NUM, "1")).unwrap();
        msg.set_field(Field::new(field::SENDING_TIME, "20250124-12:00:00")).unwrap();

        // Add market data fields but missing price
        msg.set_field(Field::new(field::SYMBOL, "AAPL")).unwrap();
        msg.set_field(Field::new(field::NO_MD_ENTRIES, "1")).unwrap();
        msg.set_field(Field::new(field::MD_ENTRY_TYPE, values::MD_ENTRY_BID)).unwrap();
        msg.set_field(Field::new(field::MD_ENTRY_SIZE, "100")).unwrap();

        assert!(MessageValidator::validate(&msg).is_err());
    }

}