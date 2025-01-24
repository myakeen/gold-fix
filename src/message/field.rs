use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Field {
    tag: i32,
    value: String,
}

impl Field {
    pub fn new(tag: i32, value: impl Into<String>) -> Self {
        Field {
            tag,
            value: value.into(),
        }
    }

    pub fn tag(&self) -> i32 {
        self.tag
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

// Common FIX field tags
pub const BEGIN_STRING: i32 = 8;
pub const BODY_LENGTH: i32 = 9;
pub const MSG_TYPE: i32 = 35;
pub const SENDER_COMP_ID: i32 = 49;
pub const TARGET_COMP_ID: i32 = 56;
pub const MSG_SEQ_NUM: i32 = 34;
pub const SENDING_TIME: i32 = 52;
pub const CHECKSUM: i32 = 10;
pub const HEART_BT_INT: i32 = 108;

// Order-related fields
pub const CL_ORD_ID: i32 = 11;
pub const ORDER_ID: i32 = 37;
pub const EXEC_ID: i32 = 17;
pub const EXEC_TRANS_TYPE: i32 = 20;
pub const EXEC_TYPE: i32 = 150;
pub const ORD_STATUS: i32 = 39;
pub const ORD_TYPE: i32 = 40;
pub const SIDE: i32 = 54;
pub const SYMBOL: i32 = 55;
pub const TIME_IN_FORCE: i32 = 59;
pub const QUANTITY: i32 = 38;
pub const PRICE: i32 = 44;

// Market Data Fields
pub const MD_REQ_ID: i32 = 262;
pub const SUBSCRIPTION_REQ_TYPE: i32 = 263;
pub const MARKET_DEPTH: i32 = 264;
pub const MD_UPDATE_TYPE: i32 = 265;
pub const AGGREGATE_BOOK: i32 = 266;
pub const NO_MD_ENTRIES: i32 = 268;
pub const MD_ENTRY_TYPE: i32 = 269;
pub const MD_ENTRY_PX: i32 = 270;
pub const MD_ENTRY_SIZE: i32 = 271;
pub const MD_ENTRY_DATE: i32 = 272;
pub const MD_ENTRY_TIME: i32 = 273;
pub const TRADING_SESSION_ID: i32 = 336;

// Quote Fields
pub const QUOTE_ID: i32 = 117;
pub const QUOTE_REQ_ID: i32 = 131;
pub const BID_PX: i32 = 132;
pub const OFFER_PX: i32 = 133;
pub const BID_SIZE: i32 = 134;
pub const OFFER_SIZE: i32 = 135;
pub const VALID_UNTIL_TIME: i32 = 62;
pub const QUOTE_CONDITION: i32 = 276;
pub const MIN_QUOTE_LIFE: i32 = 110;
pub const NO_QUOTE_ENTRIES: i32 = 295;
pub const QUOTE_ENTRY_ID: i32 = 299;
pub const QUOTE_REJECT_REASON: i32 = 300;

// Additional Order Fields
pub const TEXT: i32 = 58;
pub const TRANSACTION_TIME: i32 = 60;
pub const SETTLE_TYPE: i32 = 63;
pub const SETTLE_DATE: i32 = 64;
pub const TRADE_DATE: i32 = 75;
pub const POSITION_EFFECT: i32 = 77;
pub const STOP_PX: i32 = 99;
pub const EX_DESTINATION: i32 = 100;
pub const MIN_QTY: i32 = 110;
pub const MAX_FLOOR: i32 = 111;
pub const EXPIRE_TIME: i32 = 126;

// Session management fields
pub const ENCRYPT_METHOD: i32 = 98;
pub const RESET_SEQ_NUM_FLAG: i32 = 141;
pub const TEST_REQ_ID: i32 = 112;
pub const GAP_FILL_FLAG: i32 = 123;
pub const NEW_SEQ_NO: i32 = 36;
pub const LAST_MSG_SEQ_NUM_PROCESSED: i32 = 369;
pub const POSS_DUP_FLAG: i32 = 43;
pub const ORIG_SENDING_TIME: i32 = 122;

// Sequence reset and resend request fields
pub const BEGIN_SEQ_NO: i32 = 7;
pub const END_SEQ_NO: i32 = 16;

// Common values for fields
pub mod values {
    use std::collections::HashSet;
    use std::sync::Arc;

    // Common message types for pool initialization
    pub static COMMON_MESSAGE_TYPES: Arc<Vec<&'static str>> = Arc::new(vec![
        NEW_ORDER_SINGLE,
        EXECUTION_REPORT,
        QUOTE_REQUEST,
        MARKET_DATA_REQUEST,
        MARKET_DATA_SNAPSHOT,
        QUOTE,
        HEARTBEAT,
    ]);

    // Message types
    pub const HEARTBEAT: &str = "0";
    pub const TEST_REQUEST: &str = "1";
    pub const RESEND_REQUEST: &str = "2";
    pub const REJECT: &str = "3";
    pub const SEQUENCE_RESET: &str = "4";
    pub const LOGOUT: &str = "5";
    pub const LOGON: &str = "A";
    pub const NEW_ORDER_SINGLE: &str = "D";
    pub const ORDER_CANCEL_REQUEST: &str = "F";
    pub const EXECUTION_REPORT: &str = "8";

    // Market Data Message Types
    pub const MARKET_DATA_REQUEST: &str = "V";
    pub const MARKET_DATA_SNAPSHOT: &str = "W";
    pub const MARKET_DATA_INCREMENT_REFRESH: &str = "X";
    pub const MARKET_DATA_REQUEST_REJECT: &str = "Y";
    pub const SECURITY_STATUS: &str = "f";
    pub const TRADING_SESSION_STATUS: &str = "h";

    // Quote Message Types
    pub const QUOTE_REQUEST: &str = "R";
    pub const QUOTE: &str = "S";
    pub const QUOTE_CANCEL: &str = "Z";
    pub const QUOTE_REQUEST_REJECT: &str = "AG";
    pub const MASS_QUOTE: &str = "i";
    pub const MASS_QUOTE_ACKNOWLEDGEMENT: &str = "b";

    // Additional Message Types
    pub const ORDER_CANCEL_REPLACE_REQUEST: &str = "G";
    pub const ORDER_STATUS_REQUEST: &str = "H";
    pub const ORDER_MASS_CANCEL_REQUEST: &str = "q";
    pub const SECURITY_DEFINITION_REQUEST: &str = "c";
    pub const SECURITY_DEFINITION: &str = "d";
    pub const TRADE_CAPTURE_REPORT: &str = "AE";

    // Side values
    pub const BUY: &str = "1";
    pub const SELL: &str = "2";
    pub const BUY_MINUS: &str = "3";
    pub const SELL_PLUS: &str = "4";
    pub const SELL_SHORT: &str = "5";
    pub const SELL_SHORT_EXEMPT: &str = "6";

    // Market Data Entry Types
    pub const MD_ENTRY_BID: &str = "0";
    pub const MD_ENTRY_OFFER: &str = "1";
    pub const MD_ENTRY_TRADE: &str = "2";
    pub const MD_ENTRY_INDEX: &str = "3";
    pub const MD_ENTRY_OPENING: &str = "4";
    pub const MD_ENTRY_CLOSING: &str = "5";
    pub const MD_ENTRY_HIGH: &str = "7";
    pub const MD_ENTRY_LOW: &str = "8";

    // Quote Conditions
    pub const QUOTE_OPEN: &str = "A";
    pub const QUOTE_CLOSED: &str = "B";
    pub const QUOTE_OUTRIGHT: &str = "C";
    pub const QUOTE_CROSS: &str = "D";
    pub const QUOTE_LOCKED: &str = "E";
    pub const QUOTE_CROSSED: &str = "F";

    // Order types
    pub const MARKET: &str = "1";
    pub const LIMIT: &str = "2";
    pub const STOP: &str = "3";
    pub const STOP_LIMIT: &str = "4";
    pub const MARKET_ON_CLOSE: &str = "5";
    pub const WITH_OR_WITHOUT: &str = "6";
    pub const LIMIT_OR_BETTER: &str = "7";
    pub const LIMIT_WITH_OR_WITHOUT: &str = "8";

    // Time in force
    pub const DAY: &str = "0";
    pub const GOOD_TILL_CANCEL: &str = "1";
    pub const AT_THE_OPENING: &str = "2";
    pub const IMMEDIATE_OR_CANCEL: &str = "3";
    pub const FILL_OR_KILL: &str = "4";
    pub const GOOD_TILL_DATE: &str = "6";

    // Position effect
    pub const OPEN: &str = "O";
    pub const CLOSE: &str = "C";
    pub const ROLLED: &str = "R";
    pub const FIFO: &str = "F";
}