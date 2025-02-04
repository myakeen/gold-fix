# GoldFix: Rust FIX Protocol Engine

A high-performance Financial Information eXchange (FIX) protocol engine implementation in Rust, featuring robust error handling, atomic message operations, SSL/TLS support, and comprehensive session management.

## Architecture Overview

The engine implements a modular, thread-safe architecture optimized for high-frequency trading environments:

### Core Components

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
- **Active Connection Management**:
  - Implements automatic session recovery with configurable retry policies
  - Maintains sequence number integrity across reconnections
  - Handles connection state transitions atomically

- **Memory Optimization**:
  - Utilizes a message pool pattern for efficient message reuse
  - Implements zero-copy message parsing where possible
  - Employs arena allocation for session-scoped data

- **Security Features**:
  - Full SSL/TLS support with client certificate authentication
  - Configurable cipher suite selection
  - Certificate chain validation
  - Optional peer verification

#### Acceptor (Server) Features
- **Concurrent Session Handling**:
  - Tokio-powered async I/O for maximum throughput
  - Lock-free session state management
  - Efficient connection backlog handling

- **Resource Management**:
  - Automatic cleanup of expired sessions
  - Configurable connection limits
  - Memory-efficient message buffering

- **Security Implementation**:
  - TLS 1.3 support with backward compatibility
  - Certificate-based authentication
  - Secure session key management

### Session Management
- **State Machine Implementation**:
  ```
  Created → Connecting → InitiateLogon → LogonReceived → Connected
      ↑                                                      |
      +---------------------- Recovering <------------------+|
      |                           ↑                         |
      +--- Disconnected ← Disconnecting ← Error ←-----------+
  ```

- **Recovery Mechanisms**:
  - Automatic sequence number synchronization
  - Configurable reset policies on logon/logout/disconnect
  - Transaction-safe message persistence
  - Gap fill processing for missed messages

- **Performance Optimizations**:
  - Lock-free state transitions where possible
  - Batch message processing capabilities
  - Efficient message pooling and reuse
  - Zero-allocation path for common operations

### Transport Layer
1. **SSL/TLS Implementation**:
   - Native Rust TLS stack using `rustls`
   - Zero-copy certificate handling
   - Configurable verification policies:
     ```rust
     pub struct TransportConfig {
         pub use_ssl: bool,
         pub cert_file: Option<PathBuf>,
         pub key_file: Option<PathBuf>,
         pub ca_file: Option<PathBuf>,
         pub verify_peer: bool,
         // ...
     }
     ```

2. **Connection Management**:
   - Configurable buffer sizes for optimal performance
   - Automatic buffer resizing based on message patterns
   - Connection pooling with reuse
   - Intelligent timeout handling:
     - Configurable connection timeouts
     - Heartbeat interval management
     - Test request timing

3. **Error Handling Strategy**:
   - Comprehensive error type system
   - Automatic recovery procedures
   - Detailed error reporting with context
   - Edge case handling:
     - Network partitions
     - SSL handshake failures
     - Buffer overflow protection
     - Sequence gap detection

### State Management
1. **Session States**:
   - Each state has specific entry/exit conditions
   - Atomic state transitions
   - Persistent state tracking
   - Recovery procedures for each state

2. **Persistence Layer**:
   - Transaction-safe updates
   - JSON-based storage format
   - Automatic state recovery
   - Configurable storage backends

## Implementation Status

✅ **Core Features** (Implemented):
- **Session Management**:
  - Complete state machine implementation
  - Atomic state transitions
  - Comprehensive session lifecycle management
  - Robust recovery mechanisms

- **Transport Layer**:
  - Full SSL/TLS support
  - Certificate-based authentication
  - Connection pooling
  - Buffer management

- **Message Handling**:
  - Efficient message pooling
  - Zero-copy parsing where possible
  - Sequence number management
  - Gap detection and recovery

- **Monitoring and Control**:
  - Heartbeat monitoring
  - Test request handling
  - Connection health checks
  - Session state tracking

🔄 **In Development**:
- **Performance Optimizations**:
  - Lock-free data structures
  - Zero-allocation message paths
  - Batch processing optimizations
  - Memory pool improvements

- **Advanced Features**:
  - Market data handling
  - Order management
  - Administrative message support
  - Custom field dictionaries

- **Infrastructure**:
  - Load balancing
  - Session clustering
  - High availability support
  - Real-time monitoring

## Prerequisites
- Rust 1.54+
- OpenSSL development libraries
- Tokio runtime

## Build Instructions
```bash
# Build the project
cargo build

# Run tests
cargo test

# Run specific example
cargo run --example ssl_client
```

## Example Usage

### Basic Client Configuration
```rust
// Example configuration for a FIX client
let config = SessionConfig {
    begin_string: "FIX.4.2".to_string(),
    sender_comp_id: "CLIENT1".to_string(),
    target_comp_id: "BROKER1".to_string(),
    target_addr: "127.0.0.1:8000".to_string(),
    heart_bt_int: 30,
    reset_on_logon: true,
    reset_on_logout: true,
    reset_on_disconnect: true,
    transport_config: Some(TransportConfig {
        use_ssl: true,
        cert_file: Some("certs/client.crt".into()),
        key_file: Some("certs/client.key".into()),
        ca_file: Some("certs/ca.crt".into()),
        verify_peer: true,
        buffer_size: 4096,
        connection_timeout: Duration::from_secs(30),
    }),
    role: SessionRole::Initiator,
};
```

### Creating and Starting the Engine
```rust
// Initialize the engine with configuration
let engine = FixEngine::new(config);

// Start the engine
engine.start().await?;

// Add a new session
engine.add_session(session_config).await?;

// Get session by ID
let session = engine.get_session("CLIENT1_BROKER1").await?;

// Stop the engine
engine.stop().await?;
```

## Testing Strategy

1. **Unit Tests**:
   - Component-level testing
   - State machine verification
   - Error handling coverage
   - Message parsing validation

2. **Integration Tests**:
   - End-to-end session flows
   - Recovery scenarios
   - Edge case handling
   - Performance benchmarks

3. **Performance Testing**:
   - Message throughput
   - Latency measurements
   - Memory usage patterns
   - Connection handling capacity

## Contributing
Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## License
This project is licensed under the MIT License - see the LICENSE file for details.