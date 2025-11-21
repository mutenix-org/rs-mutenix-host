# Mutenix HID Communication Library

A Rust library for HID communication with Mutenix devices, ported from the Python reference implementation.

## Features

- ✅ Async HID device communication using tokio
- ✅ Automatic device discovery and reconnection
- ✅ LED control and device configuration
- ✅ Firmware update protocol implementation
- ✅ Status monitoring and version information
- ✅ Chunked file transfer with acknowledgment
- ✅ Comprehensive error handling

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
mutenix-hid = { path = "../lib-dev" }
tokio = { version = "1.42", features = ["full"] }
```

## Quick Start

```rust
use mutenix_hid::{HidDevice, LedColor, SetLed};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create device handler (auto-discovers mutenix devices)
    let device = HidDevice::new_auto();

    // Register callback for incoming messages
    device.register_callback(|data| {
        println!("Received: {:?}", data);
    }).await;

    // Start device processing
    tokio::spawn(async move {
        device.process().await
    });

    // Send LED command
    let cmd = SetLed::new(0, LedColor::Red);
    device.send_command(cmd).await?;

    Ok(())
}
```

## Examples

See the `examples/` directory for complete examples:

- `basic_usage.rs` - Device connection and LED control

Run examples with:
```bash
cargo run --example basic_usage
```

## Architecture

See [ADR.md](ADR.md) for detailed architecture decisions, assumptions, and limitations.

### Module Overview

- **constants** - Protocol constants (report IDs, chunk sizes, commands)
- **chunks** - File transfer chunk types (FileStart, FileChunk, FileEnd, etc.)
- **device_messages** - Device response parsing (ChunkAck, UpdateError, LogMessage)
- **hid_commands** - HID command structures (SetLed, UpdateConfig, etc.)
- **hid_device** - Main device handler with async communication
- **device_update** - Firmware update functionality

## Device Updates

To update device firmware:

```rust
use mutenix_hid::{HidDevice, perform_hid_upgrade};
use std::path::Path;

// Get raw device access
let device = HidDevice::new_auto();
// ... wait for connection ...

if let Some(raw_device) = device.raw_device().await {
    let files = vec![
        Path::new("firmware/main.py"),
        Path::new("firmware/config.py"),
    ];
    
    perform_hid_upgrade(&raw_device, files).await?;
}
```

## Known Limitations

1. **Python Minification**: Not implemented - pre-process Python files before update
2. **TAR Archives**: Not supported - extract archives before updating
3. **Progress Bars**: Uses logging instead of visual progress indicators
4. **Callbacks**: Receive raw bytes - parse with `parse_input_message()` if needed

See [ADR.md](ADR.md) for complete list of assumptions and workarounds.

## Protocol Details

### HID Reports

- **Report ID 1**: Communication commands (LED, config, ping, etc.)
- **Report ID 2**: File transfer data

### Message Format

Communication messages are 8 bytes:
```
[command_id][param1][param2][param3][param4][param5][param6][counter]
```

Transfer chunks are 60 bytes:
```
[type:2][id:2][total:2][package:2][data:52]
```

### Update Protocol

1. Send PREPARE_UPDATE command
2. For each file:
   - Send FileStart chunk
   - Send FileChunk packets (wait for ACK)
   - Send FileEnd chunk
3. Send Completed packet
4. Send RESET command

## Development

### Building

```bash
cargo build
```

### Running Examples

```bash
RUST_LOG=debug cargo run --example basic_usage
```

### Documentation

```bash
cargo doc --open
```

## License

MIT License - Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

## References

- Python reference implementation: See `.py` files in this directory
- Related project: `lib-teams` (Teams API integration)
