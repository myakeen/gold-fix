# GoldFix: Rust FIX Protocol Engine

A high-performance Financial Information eXchange (FIX) protocol engine implementation in Rust, featuring robust error handling, atomic message operations, SSL/TLS support, and comprehensive session management.

## Architecture Overview

### Core Components

The engine is built on a modular architecture with clear separation of concerns:

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
- Active connection initiation and management
- Automatic session recovery and reconnection
- Robust sequence number tracking and gap detection
- Message pooling for optimal memory usage
- SSL/TLS client authentication support
- Atomic persistence of session state

#### Acceptor (Server) Features
- Multi-session support with concurrent connection handling
- Efficient async I/O powered by Tokio
- SSL/TLS server authentication
- Connection backlog management
- Session state persistence
- Automatic recovery mechanisms

### Session Management
- Comprehensive state machine implementation
- Persistent session state with atomic updates
- Automatic heartbeat monitoring
- Test request handling
- Advanced sequence number synchronization
- Robust session recovery mechanisms

### Transport Layer
1. **SSL/TLS Support**
   - Certificate-based authentication
   - Configurable verification policies
   - Support for custom certificate chains
   - Automatic certificate validation

2. **Connection Management**
   - Configurable buffer sizes
   - Connection pooling
   - Automatic reconnection
   - Timeout handling

3. **Error Handling**
   - Comprehensive error types
   - Automatic recovery procedures
   - Detailed error reporting
   - Connection edge case handling

### State Management
1. **Session States**
   - Created
   - Connecting
   - InitiateLogon
   - ResendRequest
   - LogonReceived
   - Connected
   - Disconnecting
   - Disconnected
   - Error
   - Recovering

2. **Persistence**
   - Atomic state updates
   - JSON-based storage
   - Automatic recovery
   - Transaction support

## Implementation Status

✅ Implemented:
- Core session management with state machine
- Robust error handling and recovery mechanisms
- Full SSL/TLS support with client/server authentication
- Message pooling for memory optimization
- Atomic message operations
- Comprehensive logging system
- Sequence number management
- Heartbeat monitoring
- Test request handling
- Message persistence
- Connection recovery
- Edge case handling for network issues

🔄 In Progress:
- Performance optimizations for high-frequency trading
- Additional FIX message types
- Extended market data support
- Advanced recovery scenarios
- Connection pooling improvements
- Message routing enhancements
- Load balancing capabilities
- Session clustering support
- Real-time monitoring dashboard
- Message validation rules engine
- Custom field dictionary support
- Administrative API endpoints

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

## Contributing
Contributions are welcome! Please feel free to submit a Pull Request.

## License
This project is licensed under the MIT License - see the LICENSE file for details.