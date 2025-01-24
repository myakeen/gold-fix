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

// Session management fields
pub const ENCRYPT_METHOD: i32 = 98;
pub const RESET_SEQ_NUM_FLAG: i32 = 141;
pub const TEST_REQ_ID: i32 = 112;
pub const GAP_FILL_FLAG: i32 = 123;
pub const NEW_SEQ_NO: i32 = 36;
pub const LAST_MSG_SEQ_NUM_PROCESSED: i32 = 369;
pub const POSS_DUP_FLAG: i32 = 43;
pub const ORIG_SENDING_TIME: i32 = 122;

// Common values for fields
pub mod values {
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

    // Side values
    pub const BUY: &str = "1";
    pub const SELL: &str = "2";

    // Order types
    pub const MARKET: &str = "1";
    pub const LIMIT: &str = "2";
    pub const STOP: &str = "3";
    pub const STOP_LIMIT: &str = "4";
}