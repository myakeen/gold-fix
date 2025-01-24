# GoldFix: Rust FIX Protocol Engine

A high-performance Financial Information eXchange (FIX) protocol engine implementation in Rust, featuring robust error handling, atomic message operations, and transaction support. Built with modern Rust practices and optimized for performance.

## Technical Architecture

```
                                  GoldFix
                                      |
                 +--------------------+--------------------+
                 |                    |                    |
            Message Layer       Session Layer        Transport Layer
                 |                    |                    |
        +--------+--------+    +------+------+     +------+------+
        |    Messages     |    |   Sessions   |     |  Transport   |
        |    Validation   |    |   Heartbeat  |     |    TCP/IP    |
        |    Formatting   |    |   Sequence   |     |  Messaging   |
        +--------+--------+    +------+------+     +------+------+
                 |                    |                    |
                 |             Message Store               |
                 +----------------+   |   +----------------+
                                |   |   |
                                +---+---+
                                    |
                              Persistence
                           (Atomic Operations)
```

## Core Components

### 1. Message Layer
- **Message Processing**: Handles FIX message creation, parsing, and validation
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

### 2. Session Layer
- **Session Management**: 
  - Clean state transitions
  - Sequence number tracking
  - Heartbeat monitoring
  - Test request handling
- **Connection Handling**:
  - Logon sequence
  - Message sequencing
  - Session recovery

### 3. Message Store with Transaction Support
The message store provides atomic operations through transactions:
- Atomic message storage with rollback capability
- Persistent storage with automatic recovery
- Thread-safe concurrent access using `Arc<Mutex<_>>`
- Efficient in-memory caching with disk persistence
- Message versioning for optimistic concurrency control
- Atomic file operations for reliable persistence

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

// Retrieve message with version information
let (message, version) = store.get_message_with_version(session_id, 1).await?.unwrap();
```

## Technical Implementation Details

### 1. Thread Safety and Concurrency
- Use of `Arc<Mutex<_>>` for thread-safe state sharing
- Async/await support throughout the codebase
- Safe concurrent message processing
- Optimistic concurrency control with message versioning

### 2. Message Store Implementation
```rust
pub struct MessageStore {
    messages: Arc<Mutex<HashMap<String, HashMap<i32, (Message, u64)>>>>,
    sequence_numbers: Arc<Mutex<HashMap<String, i32>>>,
    store_dir: PathBuf,
    transactions: Arc<Mutex<HashMap<String, Transaction>>>,
    version_counter: Arc<Mutex<u64>>,
}
```

Key features:
- Session-based message storage with versioning
- Sequence number management
- Transaction support with atomic commits
- Persistent storage with atomic file operations
- Version-based conflict detection

### 3. Persistence Strategy
- Atomic file operations using temporary files and rename
- Version tracking in persistent storage
- Automatic recovery of message versions
- Transaction-safe persistence with rollback capability

## Current Features
✅ Implemented:
- Message versioning and optimistic locking
- Atomic transaction support
- Persistent message store with recovery
- Thread-safe concurrent access
- Message validation and formatting
- Session management
- Basic FIX message types support

🔄 In Progress:
- Advanced message type support
- Performance optimizations
- Session recovery mechanisms
- Logging enhancements

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