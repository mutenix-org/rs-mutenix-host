# Mutenix WebSocket Device Emulator

A WebSocket-based web server that emulates a Mutenix HID device for testing and development purposes.

## Features

- **WebSocket Server**: Real-time bidirectional communication with device emulator
- **Web UI**: Interactive browser-based interface to test device functionality
- **Device Emulation**: Simulates device behavior without physical hardware
  - Button press/release events
  - LED state management
  - Version information
  - Command processing (SetLed, Ping, UpdateConfig)
- **No Firmware Updates**: Emulator excludes device update functionality for safety

## Usage

### Running the Example Server

```bash
cargo run --example server
```

Then open http://localhost:3000 in your browser.

### Using as a Library

```rust
use mutenix_hid::HardwareType;
use mutenix_webdev::WebServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = WebServer::new(HardwareType::FiveButtonUsb, 3000);
    server.run().await?;
    Ok(())
}
```

### WebSocket Protocol

The WebSocket endpoint at `/ws` accepts JSON messages:

#### Message Types

**Command** (Client → Server):
```json
{
  "type": "command",
  "data": [1, 2, 3, ...]  // Raw HID command bytes
}
```

**Button** (Client → Server):
```json
{
  "type": "button",
  "button": 0,
  "pressed": true
}
```

**Get Version** (Client → Server):
```json
{
  "type": "get_version"
}
```

**Response** (Server → Client):
```json
{
  "type": "response",
  "data": [1, 2, 3, ...]  // Raw HID response bytes
}
```

**State** (Server → Client):
```json
{
  "type": "state",
  "state": {
    "version": "1.0.0",
    "hardware_type": 3,
    "leds": [...],
    "buttons": [...],
    "serial_number": "EMULATOR001"
  }
}
```

**Error** (Server → Client):
```json
{
  "type": "error",
  "message": "Error description"
}
```

## Supported Commands

The emulator supports the following HID commands from `lib-dev`:

- **SetLed** (0x01): Set LED colors
- **Ping** (0xF0): Keep-alive ping
- **UpdateConfig** (0xE2): Update device configuration

The following commands are **NOT** supported for safety:
- PrepareUpdate (0xE0)
- Reset (0xE1)
- Any firmware update operations

## Architecture

- `server.rs`: Axum-based web server with WebSocket support
- `emulator.rs`: Virtual device state and command processing
- Built on top of `mutenix-hid` for message type compatibility

## Dependencies

- `axum`: Web framework with WebSocket support
- `tokio`: Async runtime
- `mutenix-hid`: Device protocol definitions from lib-dev
- `serde`/`serde_json`: JSON serialization
- `tower-http`: CORS and middleware support
