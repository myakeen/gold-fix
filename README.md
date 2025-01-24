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
        |    Validation   |    |   Heartbeat  |     |    SSL/TLS   |
        |    Formatting   |    |   Sequence   |     |  Socket Mgmt |
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
- **Enhanced Message Processing**: 
  - Optimized parsing for high-frequency message types
  - Efficient handling of market data and quote messages
  - Smart message pooling with adaptive sizing
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

### 3. Advanced Transport Layer
- **SSL/TLS Support**:
  - Certificate-based authentication
  - Custom CA support
  - Client certificate handling
  - Secure socket management
  - Efficient buffer handling with configurable sizes
  - Automatic TLS negotiation
  - Certificate validation and verification
- **Socket Management**:
  - Connection pooling
  - Automatic reconnection
  - Timeout handling
  - Configurable buffer sizes
  - Non-blocking I/O operations
- **Error Handling**:
  - Comprehensive SSL error handling
  - Connection error recovery
  - Certificate validation
  - Transport-specific error types
  - Detailed error reporting and logging

### 4. Message Store with Transaction Support
The message store provides atomic operations through transactions:
- Atomic message storage with rollback capability
- Persistent storage with automatic recovery
- Thread-safe concurrent access using `Arc<Mutex<_>>`
- Efficient in-memory caching with disk persistence
- Message versioning for optimistic concurrency control
- Atomic file operations for reliable persistence

### 5. Advanced Message Pooling System
The enhanced message pooling system provides:
- Adaptive pool sizing based on message type frequency
- Automatic cleanup of underutilized pools
- Performance monitoring and statistics
- Thread-safe message recycling
- Type-specific pool optimization

Example usage:
```rust
// Configure SSL transport
let config = TransportConfig {
    use_ssl: true,
    cert_file: Some(PathBuf::from("certs/client.crt")),
    key_file: Some(PathBuf::from("certs/client.key")),
    ca_file: Some(PathBuf::from("certs/ca.crt")),
    verify_peer: true,
    buffer_size: 4096,
    connection_timeout: Duration::from_secs(30),
};

// Create and connect transport
let mut transport = Transport::new_with_config(config);
transport.connect("localhost:8443").await?;

// Send and receive messages
transport.send(&message).await?;
if let Some(response) = transport.receive().await? {
    // Handle response
}
```

## Technical Implementation Details

### 1. Thread Safety and Concurrency
- Use of `Arc<Mutex<_>>` for thread-safe state sharing
- Async/await support throughout the codebase
- Safe concurrent message processing
- Optimistic concurrency control with message versioning

### 2. Message Pool Implementation
```rust
pub struct MessagePool {
    pools: Arc<Mutex<HashMap<String, Vec<Message>>>>,
    config: PoolConfig,
    stats: Arc<Mutex<HashMap<String, PoolStats>>>,
}
```

Key features:
- Dynamic pool sizing based on message type
- Performance monitoring and statistics
- Automatic resource cleanup
- Memory usage optimization
- Pool utilization tracking

### 3. Enhanced Transport Implementation
```rust
pub struct Transport {
    connection: Option<ConnectionType>,
    config: TransportConfig,
    buffer: Vec<u8>,
}
```

Features:
- SSL/TLS support with certificate management
- Efficient buffer handling
- Connection type abstraction
- Automatic reconnection
- Comprehensive error handling

## Current Features
✅ Implemented:
- Message versioning and optimistic locking
- Atomic transaction support
- Persistent message store with recovery
- Thread-safe concurrent access
- Message validation and formatting
- Session management
- Advanced message pooling system
- Market data optimization
- Quote handling improvements
- SSL/TLS support with certificate management
- Advanced transport layer with configurable settings
- Comprehensive error handling
- Efficient buffer management
- Automatic TLS negotiation
- Custom CA support

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
- OpenSSL development libraries (for SSL/TLS support)

### Building and Testing
```bash
# Build the project
cargo build

# Run tests
cargo test

# Run examples
cargo run --example simple_client
```

## Contributing
Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## License
This project is licensed under the MIT License - see the LICENSE file for details.