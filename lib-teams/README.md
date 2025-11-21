# Teams WebSocket Client - Rust Implementation

A Rust implementation of a Teams WebSocket client that supports asynchronous communication with the Teams WebSocket server using Tokio.

## Features

- **Asynchronous**: Built on Tokio runtime for efficient async I/O
- **State Management**: Maintains the most recent state that can be read at any time
- **Auto-reconnection**: Automatically reconnects on connection failures
- **Message Queue**: Asynchronous message sending with internal queue
- **Callback Support**: Register callbacks to handle incoming server messages
- **Type-Safe**: Full type safety with Rust's type system and Serde for JSON serialization

## Architecture

The implementation consists of three main modules:

### 1. `teams_messages.rs`
Defines all message types exchanged with the Teams server:
- `MeetingPermissions`: Permissions for meeting actions
- `MeetingState`: Current state of the meeting
- `MeetingUpdate`: Updates to meeting state
- `ServerMessage`: Messages received from Teams
- `ClientMessage`: Messages sent to Teams
- `MeetingAction`: Available actions (mute, unmute, raise hand, etc.)

### 2. `state.rs`
Manages the shared state:
- `ConnectionState`: Connection status (Connected/Disconnected)
- `TeamsState`: Thread-safe shared state using `Arc<RwLock<T>>`
  - Connection status
  - Most recent server message
  - Last received timestamp

### 3. `websocket_client.rs`
The main WebSocket client implementation:
- `Identifier`: Authentication and device identification
- `TeamsWebSocketClient`: Main client with:
  - Async send/receive loops
  - Automatic reconnection with configurable retry interval
  - Message callback system
  - Thread-safe state updates

## Usage

### Basic Example

```rust
use teams_api::{
    ClientMessage, Identifier, MeetingAction, TeamsState, TeamsWebSocketClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Create shared state
    let state = TeamsState::new();

    // Create identifier for authentication
    let identifier = Identifier::new(
        "MyManufacturer",
        "MyDevice",
        "MyApp",
        "1.0.0",
    )
    .with_token("your-auth-token-here");

    // Create WebSocket client
    let client = std::sync::Arc::new(TeamsWebSocketClient::new(
        state.clone(),
        "ws://localhost:8124",
        identifier,
    ));

    // Register a callback for incoming messages
    client
        .register_callback(|message| {
            println!("Received message: {:?}", message);
        })
        .await;

    // Spawn the client processing task
    let client_clone = client.clone();
    tokio::spawn(async move {
        if let Err(e) = client_clone.process().await {
            eprintln!("Client error: {}", e);
        }
    });

    // Wait for connection
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Send messages
    client.send_message(ClientMessage::action(MeetingAction::QueryMeetingState))?;
    client.send_message(ClientMessage::action(MeetingAction::ToggleMute))?;

    // Read the current state at any time
    let current_state = state.state().await;
    println!("Current state: {:?}", current_state);

    // Stop the client
    client.stop().await;

    Ok(())
}
```

### Sending Different Message Types

```rust
// Simple actions
client.send_message(ClientMessage::action(MeetingAction::Mute))?;
client.send_message(ClientMessage::action(MeetingAction::RaiseHand))?;
client.send_message(ClientMessage::action(MeetingAction::LeaveCall))?;

// Reactions
use teams_api::ClientMessageParameterType;
client.send_message(ClientMessage::reaction(
    ClientMessageParameterType::ReactLike
))?;

// Toggle UI elements
client.send_message(ClientMessage::toggle_ui(
    ClientMessageParameterType::ToggleUiChat
))?;
```

### Reading State

```rust
// Connection status
let status = state.connection_status().await;
println!("Connected: {:?}", status);

// Meeting state
let current_state = state.state().await;
if let Some(update) = current_state.meeting_update {
    if let Some(meeting_state) = update.meeting_state {
        println!("Muted: {}", meeting_state.is_muted);
        println!("In meeting: {}", meeting_state.is_in_meeting);
        println!("Hand raised: {}", meeting_state.is_hand_raised);
    }
}

// Last received timestamp
if let Some(timestamp) = state.last_received_timestamp().await {
    println!("Last message at: {}", timestamp);
}
```

### Background Processing

```rust
// Spawn a background task to monitor state changes
tokio::spawn({
    let state = state.clone();
    async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            let current_state = state.state().await;
            // Process state...
        }
    }
});
```

## API Reference

### `Identifier`

```rust
pub struct Identifier {
    pub protocol_version: String,
    pub manufacturer: String,
    pub device: String,
    pub app: String,
    pub app_version: String,
    pub token: String,
}
```

Methods:
- `new(manufacturer, device, app, app_version)` - Create new identifier
- `with_token(token)` - Set authentication token

### `TeamsWebSocketClient`

Methods:
- `new(state, uri, identifier)` - Create new client
- `register_callback(callback)` - Register message callback
- `send_message(message)` - Send message to server (non-blocking)
- `process()` - Start client loop (blocks until stopped)
- `stop()` - Stop the client
- `is_running()` - Check if client is running

### `TeamsState`

Methods:
- `new()` - Create new state
- `connection_status()` - Get current connection status
- `state()` - Get current server message state
- `last_received_timestamp()` - Get last message timestamp

### `ClientMessage`

Static methods:
- `action(MeetingAction)` - Create action message
- `reaction(ClientMessageParameterType)` - Create reaction message
- `toggle_ui(ClientMessageParameterType)` - Create UI toggle message

## Dependencies

```toml
[dependencies]
tokio = { version = "1.42", features = ["full"] }
tokio-tungstenite = "0.24"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
futures-util = "0.3"
url = "2.5"
log = "0.4"
thiserror = "2.0"
```

## Error Handling

The client uses the `WebSocketError` type for error handling:

```rust
pub enum WebSocketError {
    Connection(String),
    Send(String),
    Receive(String),
    Json(serde_json::Error),
    WebSocket(tokio_tungstenite::tungstenite::Error),
    Stopped,
}
```

All errors implement `std::error::Error` for easy propagation.

## Testing

Run the tests:

```bash
cargo test
```

Run the example:

```bash
cargo run --example basic_usage
```

## License

SPDX-License-Identifier: MIT
Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>
