# Teams API Tests

This directory contains integration tests for the Teams WebSocket API.

## Running Tests

```bash
# Run all tests
cargo test

# Run tests with logging output
RUST_LOG=debug cargo test -- --nocapture

# Run specific test
cargo test test_message_serialization
```

## Test Coverage

- Message serialization/deserialization
- State management thread safety
- Client message ID generation
- URL building with parameters
