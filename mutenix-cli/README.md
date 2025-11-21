# Mutenix CLI

A command-line interface for controlling Mutenix HID devices with Microsoft Teams integration.

## Features

- **Terminal UI**: Beautiful ncurses-style interface with real-time status updates
  - Device and Teams connection status
  - Live message logs (device/HID and Teams)
  - Meeting state indicators (mute, video, hand raised, etc.)
  - Version information
- **HID Device Control**: Connects to Mutenix devices via USB HID
- **Teams Integration**: Controls Microsoft Teams via WebSocket API
- **Button Actions**: Execute Teams actions (mute, video, hand raise, etc.) via hardware buttons
- **LED Status**: Display Teams meeting state on device LEDs
- **Long Press Support**: Different actions for short and long button presses
- **Configurable**: YAML-based configuration for flexible device setup
- **Background Mode**: Optional no-UI mode for running as a service

## Building

```bash
cargo build --release
```

## Usage

With terminal UI (default):

```bash
./target/release/mutenix-cli
```

In the terminal UI:
- Press `q` or `ESC` to quit
- The UI auto-updates every 100ms with current status
- Logs scroll automatically to show the latest messages

With custom config file:

```bash
./target/release/mutenix-cli --config /path/to/config.yaml
```

Background mode (no UI):

```bash
./target/release/mutenix-cli --no-ui
```

## Command-Line Options

- `-c, --config <CONFIG>` - Path to the configuration file (default: `mutenix.yaml`)
- `-t, --teams-uri <TEAMS_URI>` - Teams WebSocket URI (default: `ws://localhost:8124`)
- `-T, --token-file <TOKEN_FILE>` - Token file path (default: `.mutenix_token`)
- `--no-ui` - Disable terminal UI and run in background mode
- `-h, --help` - Print help
- `-V, --version` - Print version

## Terminal UI

The terminal UI provides a comprehensive view of the system status:

### Layout

```
┌─ Device Status ─────────────┬─ Teams Status ──────────────┐
│ Status: Connected           │ Status: Connected            │
│ Product: Mutenix Keypad     │ In Meeting: Yes              │
│ Manufacturer: Mutenix       │ Muted: No | Video: On        │
│ Serial: 12345678            │ Hand Raised: No | Recording: No│
├─ Device / HID Messages ─────┼─ Teams Messages ─────────────┤
│ [15:30:45] INFO  Button 1   │ [15:30:46] INFO  Sending     │
│            pressed           │            action: ToggleMute │
│ [15:30:45] INFO  Executing  │ [15:30:47] INFO  State update│
│            action for btn 1  │            muted=true...     │
│ ...                          │ ...                          │
└──────────────────────────────┴──────────────────────────────┘
│ Mutenix CLI v0.1.0 | Press 'q' or ESC to quit              │
└───────────────────────────────────────────────────────────────┘
```

### Features

- **Real-time Status**: Connection states update every 500ms
- **Color-coded Logs**: Info (white), Warn (yellow), Error (red), Debug (gray)
- **Meeting State Indicators**: Visual feedback for mute, video, hand raised, recording
- **Scrolling Logs**: Automatically shows the most recent messages
- **Clean Exit**: Press `q` or `ESC` to quit cleanly

## Configuration

The CLI uses a YAML configuration file. See `mutenix.yaml` for an example.

### Configuration Sections

#### Device Identifications
Specify the USB vendor and product IDs for your device:

```yaml
device_identifications:
  - vendor_id: 7504
    product_id: 24969
```

#### Button Actions
Define what happens when buttons are pressed:

```yaml
actions:
  - button_id: 1
    actions:
      - activate_teams: false
        meeting_action: toggle-mute
```

#### Long Press Actions
Define different actions for long presses:

```yaml
longpress_action:
  - button_id: 3
    actions:
      - activate_teams: false
        meeting_action: toggle-video
```

#### LED Status
Configure LED indicators based on Teams state:

```yaml
led_status:
  - button_id: 1
    off: false
    webhook: false
    teams_state:
      teams_state: is-muted
      color_on: green
      color_off: red
```

### Available Meeting Actions

- `toggle-mute` - Toggle microphone mute
- `toggle-video` - Toggle video on/off
- `toggle-hand` - Raise/lower hand
- `leave-call` - Leave the meeting
- `toggle-background-blur` - Toggle background blur
- `query-state` - Query current meeting state

### Available Reactions

```yaml
actions:
  - activate_teams: false
    teams_reaction:
      reaction: like  # or: love, applause, laugh, wow
```

### Available Teams States for LEDs

- `is-muted` - Microphone is muted
- `is-hand-raised` - Hand is raised
- `is-video-on` - Video is on
- `is-in-meeting` - Currently in a meeting
- `is-recording-on` - Meeting is being recorded
- `is-background-blurred` - Background is blurred
- `is-sharing` - Screen is being shared
- `has-unread-messages` - Unread chat messages exist

### Available LED Colors

- `black` (off)
- `red`
- `green`
- `blue`
- `yellow`
- `cyan`
- `magenta`
- `white`

## How It Works

1. **Device Connection**: On startup, the CLI connects to the configured HID device(s)
2. **Teams Connection**: Connects to the Microsoft Teams WebSocket API (requires Teams desktop app with API enabled)
3. **Button Monitoring**: Listens for button press/release events from the device
4. **Action Execution**: Executes configured actions when buttons are pressed
5. **LED Updates**: Updates device LEDs every 500ms based on Teams meeting state
6. **Token Management**: Saves authentication tokens for automatic reconnection

## Dependencies

- `mutenix-hid` - HID device communication library
- `teams-api` - Microsoft Teams WebSocket client library
- `tokio` - Async runtime
- `serde` / `serde_yaml` - Configuration parsing
- `clap` - Command-line argument parsing
- `ratatui` - Terminal UI framework
- `crossterm` - Cross-platform terminal manipulation
- `chrono` - Date and time handling for logs

## Architecture

The CLI is structured into several components:

- **App Module** (`src/app.rs`): Application state management
  - Device and Teams status tracking
  - Log entry management with timestamps
  - Thread-safe state updates via RwLock
- **Config Module** (`src/config.rs`): Parses and validates YAML configuration
- **UI Module** (`src/ui.rs`): Terminal user interface
  - Ratatui-based rendering
  - Split-pane layout for status and logs
  - Color-coded log levels
  - Keyboard event handling
- **Main Module** (`src/main.rs`):
  - Device callback handling for button events
  - Teams callback handling for state updates
  - LED update loop
  - Token persistence
  - Async task orchestration

### Event Flow

```text
Button Press → HID Device → Callback → Action Lookup → Teams API Call
                                                      ↓
Teams State Update → Teams Callback → LED Update → HID Device
                                   ↓
                            UI State Update → Terminal Rendering
```

## Troubleshooting

### Device Not Found

- Ensure the device is connected via USB
- Check that the vendor_id and product_id in the config match your device
- On Linux, you may need udev rules for USB access

### Teams Connection Failed

- Ensure Microsoft Teams desktop app is running
- Enable the Teams WebSocket API in Teams settings
- Check that the Teams URI is correct (default: `ws://localhost:8124`)

### LEDs Not Updating

- Verify Teams is connected (check logs)
- Ensure LED status is configured in the YAML
- Check that the button_id values match your device

## License

MIT - See license headers in source files
