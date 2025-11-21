// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

use crate::emulator::DeviceEmulator;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use mutenix_hid::HardwareType;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    /// HID command from client
    #[serde(rename = "command")]
    Command { data: Vec<u8> },
    
    /// HID response from emulator
    #[serde(rename = "response")]
    Response { data: Vec<u8> },
    
    /// State update
    #[serde(rename = "state")]
    State { state: crate::emulator::EmulatorState },
    
    /// Button action from client
    #[serde(rename = "button")]
    Button { button: u8, pressed: bool },
    
    /// Request version info
    #[serde(rename = "get_version")]
    GetVersion,
    
    /// Error message
    #[serde(rename = "error")]
    Error { message: String },
}

/// Shared server state
struct ServerState {
    emulator: Arc<DeviceEmulator>,
}

/// Web server for device emulation
pub struct WebServer {
    emulator: Arc<DeviceEmulator>,
    port: u16,
}

impl WebServer {
    /// Create a new web server
    pub fn new(hardware_type: HardwareType, port: u16) -> Self {
        Self {
            emulator: Arc::new(DeviceEmulator::new(hardware_type)),
            port,
        }
    }

    /// Run the web server
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let state = Arc::new(ServerState {
            emulator: self.emulator,
        });

        let app = Router::new()
            .route("/", get(index_handler))
            .route("/ws", get(websocket_handler))
            .route("/health", get(health_handler))
            .layer(CorsLayer::permissive())
            .with_state(state);

        let addr = format!("0.0.0.0:{}", self.port);
        log::info!("Starting WebSocket server on {}", addr);

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

/// Index page handler
async fn index_handler() -> Html<&'static str> {
    Html(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Mutenix Device Emulator</title>
    <style>
        body { font-family: Arial, sans-serif; max-width: 800px; margin: 50px auto; padding: 20px; }
        h1 { color: #333; }
        .status { padding: 10px; background: #f0f0f0; border-radius: 5px; margin: 10px 0; }
        .button-grid { display: grid; grid-template-columns: repeat(5, 1fr); gap: 10px; margin: 20px 0; }
        .button { padding: 20px; background: #4CAF50; color: white; border: none; border-radius: 5px; cursor: pointer; font-size: 16px; }
        .button:active { background: #45a049; }
        .led-grid { display: grid; grid-template-columns: repeat(5, 1fr); gap: 10px; margin: 20px 0; }
        .led { width: 50px; height: 50px; border-radius: 50%; border: 2px solid #333; }
        .log { background: #000; color: #0f0; padding: 10px; border-radius: 5px; font-family: monospace; font-size: 12px; height: 200px; overflow-y: auto; }
        .connected { color: green; }
        .disconnected { color: red; }
    </style>
</head>
<body>
    <h1>Mutenix Device Emulator</h1>
    <div class="status">
        Connection: <span id="connection" class="disconnected">Disconnected</span>
    </div>
    <div class="status">
        <div>Version: <span id="version">N/A</span></div>
        <div>Hardware: <span id="hardware">N/A</span></div>
    </div>
    
    <h2>Buttons</h2>
    <div class="button-grid" id="buttons"></div>
    
    <h2>LEDs</h2>
    <div class="led-grid" id="leds"></div>
    
    <h2>Log</h2>
    <div class="log" id="log"></div>

    <script>
        let ws = null;
        let state = null;

        function log(message) {
            const logEl = document.getElementById('log');
            const timestamp = new Date().toISOString().substr(11, 8);
            logEl.innerHTML += `[${timestamp}] ${message}\n`;
            logEl.scrollTop = logEl.scrollHeight;
        }

        function connect() {
            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            ws = new WebSocket(`${protocol}//${window.location.host}/ws`);

            ws.onopen = () => {
                document.getElementById('connection').textContent = 'Connected';
                document.getElementById('connection').className = 'connected';
                log('WebSocket connected');
                
                // Request version info
                ws.send(JSON.stringify({ type: 'get_version' }));
            };

            ws.onclose = () => {
                document.getElementById('connection').textContent = 'Disconnected';
                document.getElementById('connection').className = 'disconnected';
                log('WebSocket disconnected');
                setTimeout(connect, 1000);
            };

            ws.onerror = (error) => {
                log('WebSocket error: ' + error);
            };

            ws.onmessage = (event) => {
                try {
                    const msg = JSON.parse(event.data);
                    handleMessage(msg);
                } catch (e) {
                    log('Error parsing message: ' + e);
                }
            };
        }

        function handleMessage(msg) {
            switch (msg.type) {
                case 'state':
                    state = msg.state;
                    updateUI();
                    log(`State updated: ${msg.state.leds.length} LEDs, ${msg.state.buttons.length} buttons`);
                    break;
                case 'response':
                    log(`Response: [${msg.data.map(b => b.toString(16).padStart(2, '0')).join(' ')}]`);
                    break;
                case 'error':
                    log(`Error: ${msg.message}`);
                    break;
            }
        }

        function updateUI() {
            if (!state) return;

            // Update version info
            document.getElementById('version').textContent = state.version;
            const hwTypes = ['Unknown', '', 'Five Button USB V1', 'Five Button USB', 'Five Button BT', 'Ten Button USB', 'Ten Button BT'];
            document.getElementById('hardware').textContent = hwTypes[state.hardware_type] || 'Unknown';

            // Update buttons
            const buttonsEl = document.getElementById('buttons');
            buttonsEl.innerHTML = '';
            state.buttons.forEach(btn => {
                const buttonEl = document.createElement('button');
                buttonEl.className = 'button';
                buttonEl.textContent = `BTN ${btn.button}`;
                buttonEl.style.background = btn.pressed ? '#45a049' : '#4CAF50';
                
                buttonEl.onmousedown = () => {
                    ws.send(JSON.stringify({ type: 'button', button: btn.button, pressed: true }));
                };
                buttonEl.onmouseup = () => {
                    ws.send(JSON.stringify({ type: 'button', button: btn.button, pressed: false }));
                };
                buttonEl.ontouchstart = (e) => {
                    e.preventDefault();
                    ws.send(JSON.stringify({ type: 'button', button: btn.button, pressed: true }));
                };
                buttonEl.ontouchend = (e) => {
                    e.preventDefault();
                    ws.send(JSON.stringify({ type: 'button', button: btn.button, pressed: false }));
                };
                
                buttonsEl.appendChild(buttonEl);
            });

            // Update LEDs
            const ledsEl = document.getElementById('leds');
            ledsEl.innerHTML = '';
            state.leds.forEach(led => {
                const ledEl = document.createElement('div');
                ledEl.className = 'led';
                ledEl.style.background = `rgb(${led.r * 25}, ${led.g * 25}, ${led.b * 25})`;
                ledEl.title = `LED ${led.id}: RGBW(${led.r}, ${led.g}, ${led.b}, ${led.w})`;
                ledsEl.appendChild(ledEl);
            });
        }

        // Connect on load
        connect();
    </script>
</body>
</html>
"#)
}

/// Health check handler
async fn health_handler() -> impl IntoResponse {
    "OK"
}

/// WebSocket handler
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<ServerState>) {
    let (mut sender, mut receiver) = socket.split();

    // Send initial state
    let initial_state = state.emulator.get_state().await;
    let msg = WsMessage::State { state: initial_state };
    if let Ok(json) = serde_json::to_string(&msg) {
        let _ = sender.send(Message::Text(json)).await;
    }

    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                log::error!("WebSocket error: {}", e);
                break;
            }
        };

        match msg {
            Message::Text(text) => {
                if let Err(e) = handle_text_message(&text, &state, &mut sender).await {
                    log::error!("Error handling message: {}", e);
                }
            }
            Message::Binary(data) => {
                if let Err(e) = handle_binary_message(&data, &state, &mut sender).await {
                    log::error!("Error handling binary message: {}", e);
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}

/// Handle text WebSocket message
async fn handle_text_message(
    text: &str,
    state: &Arc<ServerState>,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
) -> Result<(), Box<dyn std::error::Error>> {
    use futures_util::SinkExt;

    let msg: WsMessage = serde_json::from_str(text)?;

    match msg {
        WsMessage::Command { data } => {
            let response_data = state.emulator.process_command(&data).await?;
            
            // Send response if any
            if !response_data.is_empty() {
                let response = WsMessage::Response { data: response_data };
                sender.send(Message::Text(serde_json::to_string(&response)?)).await?;
            }

            // Send updated state
            let current_state = state.emulator.get_state().await;
            let state_msg = WsMessage::State { state: current_state };
            sender.send(Message::Text(serde_json::to_string(&state_msg)?)).await?;
        }
        WsMessage::Button { button, pressed } => {
            let response_data = if pressed {
                state.emulator.press_button(button).await?
            } else {
                state.emulator.release_button(button).await?
            };

            // Send button status
            let response = WsMessage::Response { data: response_data };
            sender.send(Message::Text(serde_json::to_string(&response)?)).await?;

            // Send updated state
            let current_state = state.emulator.get_state().await;
            let state_msg = WsMessage::State { state: current_state };
            sender.send(Message::Text(serde_json::to_string(&state_msg)?)).await?;
        }
        WsMessage::GetVersion => {
            let version_data = state.emulator.get_version_info().await;
            let response = WsMessage::Response { data: version_data };
            sender.send(Message::Text(serde_json::to_string(&response)?)).await?;
        }
        _ => {}
    }

    Ok(())
}

/// Handle binary WebSocket message (raw HID data)
async fn handle_binary_message(
    data: &[u8],
    state: &Arc<ServerState>,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
) -> Result<(), Box<dyn std::error::Error>> {
    use futures_util::SinkExt;

    let response_data = state.emulator.process_command(data).await?;

    if !response_data.is_empty() {
        sender.send(Message::Binary(response_data)).await?;
    }

    // Send updated state
    let current_state = state.emulator.get_state().await;
    let state_msg = WsMessage::State { state: current_state };
    sender.send(Message::Text(serde_json::to_string(&state_msg)?)).await?;

    Ok(())
}
