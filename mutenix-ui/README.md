# Mutenix UI

A modern system tray application for controlling HID devices and Microsoft Teams meetings.

## Features

- **System Tray Integration**: Runs in the background with a system tray icon showing connection status
- **Device Control**: Interface with HID devices for button actions
- **Teams Integration**: Control Microsoft Teams meetings (mute, video, hand raise, reactions)
- **Customizable Actions**: Configure button mappings for keyboard, mouse, webhooks, and Teams actions
- **LED Status**: Visual feedback on device LEDs based on Teams meeting state
- **Long Press Support**: Different actions for short and long button presses

## Requirements

- macOS 10.15 or later
- Microsoft Teams (for Teams integration features)
- Compatible HID device (see configuration)

## Configuration

Create a `mutenix.yaml` file in the same directory as the application. See the mutenix-cli README for configuration format details.

Example minimal configuration:

```yaml
version: 1
device_identifications:
  - vendor_id: 0x1234
    product_id: 0x5678

actions:
  - button_id: 1
    actions:
      - meeting_action: toggle-mute

led_status:
  - button_id: 1
    teams_state:
      teams_state: is-muted
      color_on: red
      color_off: green
```

## Building

```bash
cd mutenix-ui
cargo build --release
```

## Running

```bash
./target/release/mutenix-ui
```

The application will start in the system tray. Click the tray icon to show the status window.

## System Tray

The tray icon shows:
- 🟢 Green: Both device and Teams connected
- 🟡 Yellow: Partially connected
- 🔴 Red: Not connected

Click the tray icon to:
- View connection status
- View device and Teams logs
- Quit the application

## License

MIT License - Copyright (c) 2025 Matthias Bilger
