# Rust FIX Protocol Engine

A robust Financial Information eXchange (FIX) protocol engine implementation in Rust, based on QuickFix/n. This engine provides high-performance, reliable message processing and session management for financial trading systems.

## Overview

This FIX engine is designed to handle the complexities of the FIX protocol while providing a clean, safe, and efficient Rust implementation. Our implementation focuses on:

- Message parsing and validation
- Session management and state handling
- Persistent message storage with transaction support
- Sequence number management
- Heartbeat monitoring and connection management

### Key Features

#### Message Processing
- **Parser (`message/parser.rs`)**
  - Robust FIX message parsing
  - Support for standard FIX message types
  - Proper handling of field separators and message boundaries
  - Complete message extraction from byte streams
  - Detailed error handling for malformed messages

- **Validator (`message/validator.rs`)**
  - Comprehensive message validation
  - Required field checks
  - Field value validation
  - Conditional field validation
  - Message type-specific validation rules

- **Field Management (`message/field.rs`)**
  - Field definitions and constants
  - Common FIX field tags
  - Field value type safety
  - Proper string handling

#### Session Management
- **Session State (`session/state.rs`)**
  - Clean state transitions
  - Sequence number tracking
  - Heartbeat monitoring
  - Test request handling
  - Connection status management

- **Session Handler (`session/mod.rs`)**
  - Session lifecycle management
  - Message sequencing
  - Heartbeat processing
  - Test request handling
  - Message resend support

#### Message Store
- **Persistent Storage (`store/mod.rs`)**
  - Atomic message operations with transaction support
  - Persistent message storage
  - Sequence number management
  - Message retrieval by range
  - Efficient in-memory caching

### Implementation Details

#### Message Store Transactions
The message store supports atomic operations through transactions:

```rust
// Begin a transaction
store.begin_transaction(session_id).await?;

// Store messages atomically
store.store_message(session_id, 1, message1).await?;
store.store_message(session_id, 2, message2).await?;

// Commit the transaction
store.commit_transaction(session_id).await?;
```

Transaction support ensures message consistency and allows for atomic operations when storing multiple related messages. The store also supports rollback operations:

```rust
// Begin a transaction
store.begin_transaction(session_id).await?;

// Store some messages
store.store_message(session_id, 1, message1).await?;

// If something goes wrong, rollback the transaction
if error_condition {
    store.rollback_transaction(session_id).await?;
}
```

#### Session State Management
Sessions are managed through a state machine:

```rust
pub enum Status {
    Created,
    Connecting,
    InitiateLogon,
    ResendRequest,
    LogonReceived,
    Connected,
    Disconnecting,
    Disconnected,
    Error,
}
```

The session manager handles state transitions and ensures proper message flow:

```rust
// Session creation and startup
let session = Session::new(config, logger, store);
session.start().await?;

// Logon sequence
session.initiate_logon().await?;

// Message processing
session.start_message_processor().await;
```

#### Message Validation
Comprehensive message validation ensures FIX protocol compliance:

```rust
pub fn validate(message: &Message) -> Result<()> {
    // Validate required header fields
    validate_header_fields(message)?;

    // Validate message-specific fields
    validate_message_fields(message)?;

    // Validate field values
    validate_field_values(message)?;

    // Validate conditional fields
    validate_conditional_fields(message)?;

    Ok(())
}
```

#### Error Handling
The engine uses a comprehensive error handling system:

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

This allows for precise error reporting and handling across different components.

## Usage

### Basic Example

```rust
use fix_engine::{FixEngine, config::{EngineConfig, SessionConfig}};

#[tokio::main]
async fn main() -> Result<()> {
    // Create engine configuration
    let config = EngineConfig {
        // ... engine configuration
    };

    // Initialize the FIX engine
    let engine = FixEngine::new(config);

    // Create session configuration
    let session_config = SessionConfig {
        begin_string: "FIX.4.2".to_string(),
        sender_comp_id: "SENDER".to_string(),
        target_comp_id: "TARGET".to_string(),
        target_addr: "127.0.0.1:5001".to_string(),
        heart_bt_int: 30,
        reset_on_logon: true,
        reset_on_logout: true,
        reset_on_disconnect: true,
    };

    // Add a session
    engine.add_session(session_config).await?;

    // Start the engine
    engine.start().await?;
}
```

### Message Store Usage

```rust
use fix_engine::store::MessageStore;

#[tokio::main]
async fn main() -> Result<()> {
    let store = MessageStore::new();
    let session_id = "TEST_SESSION";

    // Begin a transaction
    store.begin_transaction(session_id).await?;

    // Store messages atomically
    store.store_message(session_id, 1, message1).await?;
    store.store_message(session_id, 2, message2).await?;

    // Commit the transaction
    store.commit_transaction(session_id).await?;

    // Retrieve messages
    let messages = store.get_messages_range(session_id, 1, 2).await?;

    // Reset sequence numbers if needed
    store.reset_sequence_numbers(session_id).await?;
}
```

## Development Setup

### Prerequisites
- Rust 1.54 or higher
- Cargo
- Tokio runtime

### Building
```bash
cargo build
```

### Running Tests
```bash
cargo test
```

## Project Structure
```
src/
├── message/
│   ├── mod.rs      # Message handling
│   ├── field.rs    # Field definitions
│   ├── parser.rs   # Message parsing
│   └── validator.rs # Message validation
├── session/
│   ├── mod.rs      # Session management
│   └── state.rs    # Session state
├── store/
│   └── mod.rs      # Message persistence
├── transport/
│   └── mod.rs      # Network handling
├── error.rs        # Error definitions
├── config.rs       # Configuration
└── lib.rs         # Library entry point
```

## Testing

The engine includes comprehensive test coverage:

- Unit tests for all components
- Integration tests for engine functionality
- Property-based tests for message handling
- Performance tests for critical paths

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --test integration_tests

# Run tests with logging
RUST_LOG=debug cargo test
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Based on the QuickFIX/n implementation
- Built with Rust and Tokio
- Thanks to the FIX Protocol community