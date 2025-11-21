// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

use crate::state::{ConnectionState, TeamsState};
use crate::teams_messages::{ClientMessage, ServerMessage};
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use url::Url;

#[derive(Debug, Error)]
pub enum WebSocketError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Send error: {0}")]
    Send(String),
    #[error("Receive error: {0}")]
    Receive(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] Box<tokio_tungstenite::tungstenite::Error>),
    #[error("Client stopped")]
    Stopped,
}

pub type Result<T> = std::result::Result<T, WebSocketError>;

/// Callback type for receiving server messages
pub type MessageCallback = Arc<dyn Fn(ServerMessage) + Send + Sync>;

/// Identifier for authenticating with the Teams WebSocket server
#[derive(Debug, Clone)]
pub struct Identifier {
    pub protocol_version: String,
    pub manufacturer: String,
    pub device: String,
    pub app: String,
    pub app_version: String,
    pub token: String,
}

impl Identifier {
    pub fn new(
        manufacturer: impl Into<String>,
        device: impl Into<String>,
        app: impl Into<String>,
        app_version: impl Into<String>,
    ) -> Self {
        Self {
            protocol_version: "2.0.0".to_string(),
            manufacturer: manufacturer.into(),
            device: device.into(),
            app: app.into(),
            app_version: app_version.into(),
            token: String::new(),
        }
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = token.into();
        self
    }

    fn build_uri(&self, base_uri: &str) -> String {
        format!(
            "{}?protocol-version={}&manufacturer={}&device={}&app={}&app-version={}&token={}",
            base_uri,
            self.protocol_version,
            self.manufacturer,
            self.device,
            self.app,
            self.app_version,
            self.token
        )
    }
}

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// Teams WebSocket client
pub struct TeamsWebSocketClient {
    state: TeamsState,
    uri: String,
    send_tx: mpsc::UnboundedSender<ClientMessage>,
    send_rx: Arc<RwLock<mpsc::UnboundedReceiver<ClientMessage>>>,
    callback: Arc<RwLock<Option<MessageCallback>>>,
    running: Arc<RwLock<bool>>,
    retry_interval: Duration,
}

impl TeamsWebSocketClient {
    const RETRY_INTERVAL: Duration = Duration::from_millis(250);
    const RECEIVE_TIMEOUT: Duration = Duration::from_secs(1);
    const SEND_SLEEP: Duration = Duration::from_millis(200);

    /// Create a new Teams WebSocket client
    pub fn new(state: TeamsState, uri: impl Into<String>, identifier: Identifier) -> Self {
        let base_uri = uri.into();
        let uri = identifier.build_uri(&base_uri);
        let (send_tx, send_rx) = mpsc::unbounded_channel();

        Self {
            state,
            uri,
            send_tx,
            send_rx: Arc::new(RwLock::new(send_rx)),
            callback: Arc::new(RwLock::new(None)),
            running: Arc::new(RwLock::new(false)),
            retry_interval: Self::RETRY_INTERVAL,
        }
    }

    /// Register a callback for receiving server messages
    pub async fn register_callback<F>(&self, callback: F)
    where
        F: Fn(ServerMessage) + Send + Sync + 'static,
    {
        *self.callback.write().await = Some(Arc::new(callback));
    }

    /// Send a message to the Teams server
    pub fn send_message(&self, message: ClientMessage) -> Result<()> {
        self.send_tx
            .send(message)
            .map_err(|e| WebSocketError::Send(e.to_string()))
    }

    /// Check if the client is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Start the WebSocket client (runs until stopped)
    pub async fn process(&self) -> Result<()> {
        *self.running.write().await = true;

        while *self.running.read().await {
            match self.connect_and_run().await {
                Ok(_) => {
                    info!("WebSocket connection closed normally");
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    sleep(self.retry_interval).await;
                }
            }
        }

        Ok(())
    }

    /// Stop the WebSocket client
    pub async fn stop(&self) {
        info!("Stopping WebSocket client");
        *self.running.write().await = false;
    }

    async fn connect_and_run(&self) -> Result<()> {
        let ws_stream = self.connect().await?;
        let (write, read) = ws_stream.split();

        let send_handle = {
            let send_rx = self.send_rx.clone();
            let running = self.running.clone();
            tokio::spawn(Self::send_loop(write, send_rx, running))
        };

        let receive_handle = {
            let state = self.state.clone();
            let callback = self.callback.clone();
            let running = self.running.clone();
            tokio::spawn(Self::receive_loop(read, state, callback, running))
        };

        tokio::select! {
            send_result = send_handle => {
                match send_result {
                    Ok(Ok(_)) => debug!("Send loop completed"),
                    Ok(Err(e)) => error!("Send loop error: {}", e),
                    Err(e) => error!("Send task panicked: {}", e),
                }
            }
            receive_result = receive_handle => {
                match receive_result {
                    Ok(Ok(_)) => debug!("Receive loop completed"),
                    Ok(Err(e)) => error!("Receive loop error: {}", e),
                    Err(e) => error!("Receive task panicked: {}", e),
                }
            }
        }

        self.state
            .set_connection_status(ConnectionState::Disconnected)
            .await;

        Ok(())
    }

    async fn connect(&self) -> Result<WsStream> {
        loop {
            self.state
                .set_connection_status(ConnectionState::Disconnected)
                .await;

            match self.do_connect().await {
                Ok(ws_stream) => {
                    self.state
                        .set_connection_status(ConnectionState::Connected)
                        .await;
                    return Ok(ws_stream);
                }
                Err(e) => {
                    if !*self.running.read().await {
                        return Err(e);
                    }
                    error!("Failed to connect, retrying: {}", e);
                    sleep(self.retry_interval).await;
                }
            }
        }
    }

    async fn do_connect(&self) -> Result<WsStream> {
        let url = Url::parse(&self.uri)
            .map_err(|e| WebSocketError::Connection(format!("Invalid URL: {}", e)))?;

        let (ws_stream, _) = connect_async(url.as_str())
            .await
            .map_err(|e| WebSocketError::WebSocket(Box::new(e)))?;

        info!("Connected to WebSocket server at {}", self.uri);
        Ok(ws_stream)
    }

    async fn send_loop(
        mut write: futures_util::stream::SplitSink<WsStream, Message>,
        send_rx: Arc<RwLock<mpsc::UnboundedReceiver<ClientMessage>>>,
        running: Arc<RwLock<bool>>,
    ) -> Result<()> {
        let mut sent_something = true;

        while *running.read().await {
            let message = {
                let mut rx = send_rx.write().await;
                rx.try_recv().ok()
            };

            if let Some(msg) = message {
                let json = serde_json::to_string(&msg)?;
                debug!("Sending message: {}", json);

                write
                    .send(Message::Text(json))
                    .await
                    .map_err(|e| WebSocketError::Send(e.to_string()))?;

                sent_something = true;
            } else {
                if sent_something {
                    debug!("Send queue empty");
                    sent_something = false;
                }
                sleep(Self::SEND_SLEEP).await;
            }
        }

        Ok(())
    }

    async fn receive_loop(
        mut read: futures_util::stream::SplitStream<WsStream>,
        state: TeamsState,
        callback: Arc<RwLock<Option<MessageCallback>>>,
        running: Arc<RwLock<bool>>,
    ) -> Result<()> {
        while *running.read().await {
            match tokio::time::timeout(Self::RECEIVE_TIMEOUT, read.next()).await {
                Ok(Some(Ok(msg))) => {
                    if let Message::Text(text) = msg {
                        debug!("Received message: {}", text);

                        match serde_json::from_str::<ServerMessage>(&text) {
                            Ok(message) => {
                                debug!("Decoded message: {:?}", message);

                                // Update state
                                state.update_state(&message).await;

                                // Update timestamp
                                let timestamp = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs_f64();
                                state.set_last_received_timestamp(timestamp).await;

                                // Call callback
                                let cb = callback.read().await;
                                if let Some(ref callback_fn) = *cb {
                                    callback_fn(message);
                                }
                            }
                            Err(e) => {
                                error!("Failed to decode message: {}", e);
                            }
                        }
                    }
                }
                Ok(Some(Err(e))) => {
                    error!("WebSocket receive error: {}", e);
                    return Err(WebSocketError::Receive(e.to_string()));
                }
                Ok(None) => {
                    warn!("WebSocket connection closed");
                    return Err(WebSocketError::Receive("Connection closed".to_string()));
                }
                Err(_) => {
                    // Timeout, continue loop
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identifier_build_uri() {
        let identifier = Identifier::new("TestMfg", "TestDevice", "TestApp", "1.0.0")
            .with_token("test-token");

        let uri = identifier.build_uri("ws://localhost:8124");

        assert!(uri.contains("protocol-version=2.0.0"));
        assert!(uri.contains("manufacturer=TestMfg"));
        assert!(uri.contains("device=TestDevice"));
        assert!(uri.contains("app=TestApp"));
        assert!(uri.contains("app-version=1.0.0"));
        assert!(uri.contains("token=test-token"));
    }
}
