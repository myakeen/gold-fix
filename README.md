# Rust FIX Protocol Engine

A high-performance Financial Information eXchange (FIX) protocol engine implementation in Rust, featuring robust error handling and atomic message operations. Based on QuickFix/n architecture with Rust-specific optimizations.

## Core Features

### 1. Message Store with Transaction Support
The message store provides atomic operations through transactions:
- Atomic message storage with rollback capability
- Persistent storage with automatic recovery
- Thread-safe concurrent access using `Arc<Mutex<_>>`
- Efficient in-memory caching with disk persistence

Example usage:
```rust
// Begin a transaction
store.begin_transaction(session_id).await?;

// Store messages atomically
store.store_message(session_id, 1, message1).await?;
store.store_message(session_id, 2, message2).await?;

// Commit or rollback
store.commit_transaction(session_id).await?;
// Or: store.rollback_transaction(session_id).await?;
```

### 2. Comprehensive Error Handling
Structured error handling using custom `FixError` enum:
```rust
pub enum FixError {
    ParseError(String),
    SessionError(String),
    ConfigError(String),
    TransportError(String),
    StoreError(String),
    IoError(std::io::Error),
}
```

### 3. Message Processing
- **Field Formatting**: Type-safe field formatting with support for:
  - DateTime formatting
  - Integer validation
  - Decimal precision handling
  - Character field validation
  - String sanitization

- **Field Validation**: Comprehensive validation including:
  - Required fields checking
  - Field value type validation
  - Conditional field validation
  - Message type-specific rules


### 4. Session Management
- Clean state transitions
- Sequence number tracking
- Heartbeat monitoring
- Test request handling

## Technical Implementation Details

### 1. Thread Safety and Concurrency
- Use of `Arc<Mutex<_>>` for thread-safe state sharing
- Async/await support throughout the codebase
- Safe concurrent message processing

### 2. Message Store Implementation
```rust
pub struct MessageStore {
    messages: Arc<Mutex<HashMap<String, HashMap<i32, Message>>>>,
    sequence_numbers: Arc<Mutex<HashMap<String, i32>>>,
    store_dir: PathBuf,
    transactions: Arc<Mutex<HashMap<String, Transaction>>>,
}
```

Key features:
- Session-based message storage
- Sequence number management
- Transaction support
- Persistent storage

### 3. Error Handling Strategy
- Custom error types with detailed context
- Proper error propagation
- Recovery mechanisms
- Comprehensive error logging

## Usage Examples

### Basic Message Processing
```rust
// Create a new message
let mut msg = Message::new(values::NEW_ORDER_SINGLE);
msg.set_field(Field::new(field::CL_ORD_ID, "12345"))?;
msg.set_field(Field::new(field::SYMBOL, "AAPL"))?;
msg.set_field(Field::new(field::SIDE, values::BUY))?;

// Add custom formatters
msg.set_formatter(field::SENDING_TIME, DateTimeFormatter);
msg.set_formatter(field::PRICE, DecimalFormatter::new(2));
```

### Session Management
```rust
// Create and configure a session
let session = Session::new(config, logger, store);
session.start().await?;

// Initiate logon sequence
session.initiate_logon().await?;

// Start message processing
session.start_message_processor().await;
```

## Current Status and Next Steps
1. ✅ Core message store implementation with transaction support
2. ✅ Field formatting and validation
3. ✅ Basic session management
4. ✅ Error handling framework
5. 🔄 Message persistence optimization
6. 📝 Session recovery mechanisms
7. 📝 Performance optimization

## Development Setup

### Prerequisites
- Rust 1.54 or higher
- Cargo
- Tokio runtime for async support

### Building and Testing
```bash
# Build the project
cargo build

# Run tests
cargo test
```

## Contributing
Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## License
This project is licensed under the MIT License - see the LICENSE file for details.