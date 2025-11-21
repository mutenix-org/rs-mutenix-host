# Architecture Decision Record: Rust HID Communication Library

**Date:** 2025-11-21  
**Status:** Implemented  
**Author:** GitHub Copilot CLI (based on Python reference implementation)

## Context

A Python-based HID communication library exists in `lib-dev` for communicating with Mutenix hardware devices. A Rust implementation is needed to provide similar functionality with better performance, type safety, and integration with other Rust-based components.

## Decision

Implement a Rust HID communication library that mirrors the Python implementation's functionality while leveraging Rust's type system and async capabilities.

## Architecture

### Module Structure

The library is organized into the following modules:

1. **constants.rs** - Protocol constants and configuration values
2. **chunks.rs** - File transfer chunk types and structures
3. **device_messages.rs** - Device response message parsing
4. **hid_commands.rs** - HID command structures and types
5. **hid_device.rs** - Main device communication handler
6. **device_update.rs** - Firmware update functionality

### Key Components

#### 1. HID Device Handler (`hid_device.rs`)

- **Async Architecture**: Uses tokio for async operations, matching the lib-teams pattern
- **Connection Management**: Automatic device discovery and reconnection
- **Message Processing**: Separate read/write/ping loops running concurrently via tokio::select!
- **Callback System**: Allows registration of message handlers
- **Concurrent Task Design**:
  - `read_loop`: Continuously reads from device, yields after each read to prevent busy-looping
  - `write_loop`: Processes outbound commands from async channel, blocks on recv() which efficiently waits
  - `ping_loop`: Uses `tokio::time::interval` for precise periodic pings without blocking other tasks

#### 2. Message Types (`hid_commands.rs`)

- **Input Messages**: Status, VersionInfo, StatusRequest
- **Output Commands**: SetLed, UpdateConfig, SimpleCommand (Ping, PrepareUpdate, Reset)
- **Type Safety**: Strong typing with enums for hardware types, commands, and LED colors

#### 3. Update Protocol (`device_update.rs`, `chunks.rs`)

- **Chunked Transfer**: Files split into 52-byte chunks (60 bytes - 8 byte header)
- **Acknowledgment**: Each chunk must be acknowledged by device
- **File Operations**: Support for file transfer and deletion
- **Progress Tracking**: Track transfer state per file

#### 4. Device Messages (`device_messages.rs`)

- **Protocol Messages**: ChunkAck, UpdateError, LogMessage
- **Parsing**: Safe parsing from raw byte buffers
- **Validation**: Identifier validation for message types

## Technology Choices

### Dependencies

1. **hidapi (2.6)** - HID device access
   - **Rationale**: Cross-platform, well-maintained, standard for HID in Rust
   - **Alternative Considered**: rusb (lower-level, more complex)

2. **tokio (1.42)** - Async runtime
   - **Rationale**: Consistency with lib-teams, industry standard
   - **Features**: Full feature set for comprehensive async support

3. **thiserror (2.0)** - Error handling
   - **Rationale**: Ergonomic error types, matches lib-teams pattern

4. **serde (1.0)** - Serialization
   - **Rationale**: Standard for data serialization, future extensibility

5. **log (0.4)** - Logging facade
   - **Rationale**: Allows library users to choose logging implementation

6. **bytes (1.9)** - Byte buffer utilities
   - **Rationale**: Efficient buffer manipulation

## Assumptions and Limitations

### Assumptions

1. **HID Protocol Compatibility**: The HID protocol used by Python version is assumed to be the same
   - Device uses report IDs 1 (communication) and 2 (transfer)
   - Message formats are identical to Python implementation

2. **Device Behavior**: Device behavior matches Python implementation expectations
   - Sends acknowledgments for each chunk
   - Supports log messages during update
   - Uses same error message format

3. **Platform Support**: Library targets platforms supported by hidapi
   - Linux (with static hidraw feature)
   - macOS (via IOKit)
   - Windows (via Windows HID API)

4. **Single Device**: Primary use case is single device connection
   - Multi-device support possible but not primary focus

5. **Async Context**: Library users have tokio runtime available
   - All public APIs are async

### Known Limitations

1. **Python Minification Not Implemented**
   - Python version uses `python_minifier` for .py files
   - Rust version does not minify - assumes pre-minified or binary files
   - **Workaround**: Pre-process files before update, or implement custom minification

2. **No TAR Archive Support**
   - Python version supports .tar.gz archives for updates
   - Rust version expects individual file paths
   - **Workaround**: Extract archives before calling update functions

3. **Progress Reporting**
   - Python version uses tqdm for progress bars
   - Rust version uses logging only
   - **Workaround**: Users can implement custom progress tracking via logging

4. **Callback Type Limitation**
   - Callbacks receive raw byte buffers, not parsed messages
   - **Rationale**: Flexibility - users can parse as needed
   - **Workaround**: Use device_messages::parse_input_message()

5. **Device Search Strategy**
   - Auto-discovery searches for "mutenix" in product string
   - May not work if device uses different naming
   - **Workaround**: Use DeviceInfo with specific VID/PID

6. **Synchronous HID Writes**
   - hidapi crate doesn't provide async write operations
   - Write operations block tokio thread briefly
   - **Impact**: Minimal for small HID reports (8-60 bytes)

7. **Error Recovery**
   - Automatic reconnection on device disconnect
   - No retry mechanism for failed chunk transfers
   - **Future Enhancement**: Could add configurable retry logic

## Implementation Details

### Async Task Coordination (2025-11-21 Update)

The `hid_device.rs` implementation uses three concurrent async loops that run via `tokio::select!`:

**Issue Identified and Resolved:**
- Initial implementation had blocking sleep calls that delayed message processing
- Locks were held during sleep operations, causing contention
- `sleep_until` was incorrectly calculating timing based on stale values

**Current Implementation:**

1. **Read Loop**
   - Uses `tokio::task::yield_now()` after successful reads instead of blocking sleeps
   - Prevents busy-looping while allowing immediate message processing
   - Only sleeps (100ms) when device is not connected

2. **Write Loop**
   - Holds receiver lock for entire loop duration (correct pattern for exclusive receiver)
   - `recv().await` efficiently blocks until messages arrive - no manual sleep needed
   - Returns `None` only when channel is closed, triggering clean loop exit

3. **Ping Loop**
   - Uses `tokio::time::interval()` instead of manual `sleep()` for precise periodic execution
   - `interval.tick().await` properly yields to the async scheduler
   - `MissedTickBehavior::Skip` prevents backlog if pings are delayed
   - Non-blocking: other tasks can process messages while waiting for next ping interval

**Benefits:**
- Messages are processed immediately without artificial delays
- All three loops execute concurrently without blocking each other
- Efficient CPU usage - async runtime schedules tasks optimally
- Predictable ping timing without drift

## API Design Differences from Python

### Python Version
```python
device = HidDevice(state, device_identifications)
device.register_callback(callback)
await device.process()
await device.send_msg(msg)
```

### Rust Version
```rust
let device = HidDevice::new(device_info);
device.register_callback(|data| { ... }).await;
tokio::spawn(async move { device.process().await });
device.send_command(command).await?;
```

**Key Differences:**
1. Rust uses generic command types instead of runtime polymorphism
2. Callbacks in Rust are `Fn(&[u8])` instead of `Callable[[HidInputMessage], None]`
3. Error handling uses Result types instead of exceptions
4. Device state is internal, not externally managed

## Testing Strategy

### Unit Tests (Not Yet Implemented)
- Message parsing from byte buffers
- Chunk creation and packet generation
- Command serialization

### Integration Tests (Recommended)
- Device connection/disconnection
- Message round-trip
- Update process with mock device

### Example Programs
- `basic_usage.rs` - Simple device connection and LED control
- Additional examples recommended:
  - Firmware update example
  - Status monitoring example
  - Multi-command example

## Future Enhancements

1. **Archive Support**: Add tar.gz extraction for updates
2. **Progress Callbacks**: Add progress reporting API
3. **Python File Minification**: Integrate Python minifier if needed
4. **Retry Logic**: Configurable retry for failed operations
5. **Device Discovery Events**: Callbacks for device connect/disconnect
6. **Sync API**: Optional blocking API for non-async contexts
7. **Mock Device**: Testing utilities with mock HID device

## Migration Path from Python

For existing Python code using the library:

1. **Device Setup**:
   - Python: `HidDevice(state, device_identifications)`
   - Rust: `HidDevice::new(device_info)` or `HidDevice::new_auto()`

2. **Commands**:
   - Python: `SetLed(id, led_color)` returns `HidCommand`
   - Rust: `SetLed::new(id, color)` implements `HidOutputCommand`

3. **Callbacks**:
   - Python: Receives parsed `HidInputMessage`
   - Rust: Receives raw `&[u8]`, parse with `parse_input_message()`

4. **Updates**:
   - Python: `perform_upgrade_with_file(device, file_stream)` handles tar.gz
   - Rust: `perform_hid_upgrade(device, vec![path1, path2])` expects file paths

## Verification

To verify implementation correctness:

1. ✅ All Python constants mapped to Rust constants
2. ✅ All message types implemented
3. ✅ All chunk types implemented
4. ✅ All HID commands implemented
5. ✅ Device connection lifecycle handled
6. ✅ Update protocol implemented
7. ⚠️ Python minification not implemented (documented assumption)
8. ⚠️ TAR archive support not implemented (documented assumption)
9. ✅ Async architecture matching lib-teams
10. ✅ Concurrent task loops properly coordinated without blocking (2025-11-21)

## Conclusion

This Rust implementation provides equivalent functionality to the Python version with the documented assumptions and limitations. The architecture leverages Rust's strengths (type safety, async, error handling) while maintaining compatibility with the HID protocol. Users should be aware of the limitations around file preprocessing and implement workarounds as needed.

## References

- Python implementation: `/Users/matthiasbilger/git/rust-mutenix-host/lib-dev/*.py`
- lib-teams reference: `/Users/matthiasbilger/git/rust-mutenix-host/lib-teams/`
- hidapi-rs: https://github.com/ruabmbua/hidapi-rs
- tokio: https://tokio.rs/
