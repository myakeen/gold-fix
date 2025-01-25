# GoldFix: Rust FIX Protocol Engine

A high-performance Financial Information eXchange (FIX) protocol engine implementation in Rust, featuring robust error handling, atomic message operations, and transaction support.

## Architecture Overview

### Initiator/Acceptor Pattern Implementation
The engine implements a clean separation between initiator (client) and acceptor (server) roles:

```
                    GoldFix Engine
                         |
            +-----------+-----------+
            |                       |
        Initiator              Acceptor
            |                       |
    +-------+-------+      +-------+-------+
    | Session Mgmt  |      | Connection    |
    | Reconnection  |      | Acceptance    |
    | Sequencing    |      | Session Mgmt  |
    +-------+-------+      +-------+-------+
            |                       |
        Transport Layer        Transport Layer
```

#### Initiator (Client) Features
- Active connection initiation
- Session management with automatic reconnection
- Sequence number tracking and gap detection
- Message pooling for efficient memory usage
- SSL/TLS client authentication

#### Acceptor (Server) Features
- Multi-session support
- Concurrent connection handling
- Efficient async I/O with Tokio
- SSL/TLS server authentication
- Connection backlog management

### Session Management
- Clean state transitions
- Heartbeat monitoring
- Test request handling
- Sequence number synchronization
- Session recovery mechanisms

### Core Components

1. **Message Layer**
   - Optimized parsing
   - Type-safe field formatting
   - Validation rules enforcement
   - Pool-based message management

2. **Transport Layer**
   - SSL/TLS support
   - Efficient buffer management
   - Connection pooling
   - Automatic reconnection

3. **Session Layer**
   - State management
   - Heartbeat handling
   - Message sequencing
   - Recovery procedures

## Project Structure

```
goldfix/
├── src/
│   ├── initiator.rs      # Initiator implementation
│   ├── acceptor.rs       # Acceptor implementation
│   ├── session/          # Session management
│   ├── message/          # Message handling
│   ├── transport/        # Transport layer
│   └── store/           # Message storage
├── tests/
│   ├── test_utils.rs    # Shared test utilities
│   └── integration_tests.rs
└── examples/
    ├── simple_client.rs
    ├── session_recovery.rs
    └── ssl_client.rs
```

## Testing Infrastructure

### Test Organization
1. **Unit Tests**
   - Component-level testing
   - Mocked dependencies
   - Fast execution

2. **Integration Tests**
   - End-to-end scenarios
   - Real network communication
   - Session management validation

3. **Test Utilities**
   - Shared test configurations
   - Mock implementations
   - Helper functions

## Build and Run

### Prerequisites
- Rust 1.54+
- OpenSSL development libraries

### Build Instructions
```bash
# Build the project
cargo build

# Run tests
cargo test

# Run specific example
cargo run --example simple_client
```

### Configuration
Sample configuration for initiator:
```rust
let config = SessionConfig {
    begin_string: "FIX.4.2".to_string(),
    sender_comp_id: "CLIENT".to_string(),
    target_comp_id: "SERVER".to_string(),
    target_addr: "127.0.0.1:8000".to_string(),
    heart_bt_int: 30,
    role: SessionRole::Initiator,
    // ... other settings
};
```

Sample configuration for acceptor:
```rust
let config = SessionConfig {
    begin_string: "FIX.4.2".to_string(),
    sender_comp_id: "SERVER".to_string(),
    target_comp_id: "CLIENT".to_string(),
    target_addr: "0.0.0.0:8000".to_string(),
    heart_bt_int: 30,
    role: SessionRole::Acceptor,
    // ... other settings
};
```

## Current Status

✅ Implemented:
- Initiator/Acceptor pattern
- Session management
- Message validation
- SSL/TLS support
- Comprehensive test suite

🔄 In Progress:
- Performance optimizations
- Additional FIX message types
- Extended market data support

## Contributing
Contributions are welcome! Please feel free to submit a Pull Request.

## License
This project is licensed under the MIT License - see the LICENSE file for details.