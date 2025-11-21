// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

use crate::PING_LOOP_TIME_SECONDS;
use crate::hid_commands::{HidOutputCommand, SimpleCommand, Status, StatusRequest, parse_input_message};
use hidapi::{HidApi, HidDevice as RawHidDevice};
use log::{debug, error, info, warn};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::sleep;

/// Device identification information
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub vendor_id: u16,
    pub product_id: u16,
    pub serial_number: Option<String>,
}

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connected,
    Error,
}

/// Hardware state information
#[derive(Debug, Clone)]
pub struct HardwareState {
    pub connection_status: ConnectionState,
    pub serial_number: Option<String>,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
}

impl Default for HardwareState {
    fn default() -> Self {
        Self {
            connection_status: ConnectionState::Disconnected,
            serial_number: None,
            manufacturer: None,
            product: None,
        }
    }
}

/// Enum for messages that should be forwarded to subscribers
#[derive(Debug, Clone)]
pub enum DeviceMessage {
    Status(Status),
    StatusRequest(StatusRequest),
}

/// Callback type for incoming device messages
pub type MessageCallback = Arc<dyn Fn(DeviceMessage) + Send + Sync>;

/// HID Device handler with async communication
pub struct HidDevice {
    state: Arc<RwLock<HardwareState>>,
    device_info: Vec<DeviceInfo>,
    device: Arc<Mutex<Option<RawHidDevice>>>,
    callbacks: Arc<RwLock<Vec<MessageCallback>>>,
    send_buffer: mpsc::UnboundedSender<(Box<dyn HidOutputCommand + Send>, tokio::sync::oneshot::Sender<Result<usize, HidError>>)>,
    send_receiver: Arc<Mutex<mpsc::UnboundedReceiver<(Box<dyn HidOutputCommand + Send>, tokio::sync::oneshot::Sender<Result<usize, HidError>>)>>>,
    last_ping: Arc<Mutex<Instant>>,
    running: Arc<RwLock<bool>>,
}

/// Errors that can occur with HID operations
#[derive(Debug, thiserror::Error)]
pub enum HidError {
    #[error("Device not connected")]
    NotConnected,
    
    #[error("Failed to write to device: {0}")]
    WriteFailed(String),
    
    #[error("Failed to read from device: {0}")]
    ReadFailed(String),
    
    #[error("Device disconnected")]
    Disconnected,
    
    #[error("HID API error: {0}")]
    HidApiError(String),
}

impl HidDevice {
    /// Create a new HID device handler
    pub fn new(device_info: Vec<DeviceInfo>) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        
        Self {
            state: Arc::new(RwLock::new(HardwareState::default())),
            device_info,
            device: Arc::new(Mutex::new(None)),
            callbacks: Arc::new(RwLock::new(Vec::new())),
            send_buffer: tx,
            send_receiver: Arc::new(Mutex::new(rx)),
            last_ping: Arc::new(Mutex::new(Instant::now())),
            running: Arc::new(RwLock::new(true)),
        }
    }

    /// Create a new HID device handler that searches for any mutenix device
    pub fn new_auto() -> Self {
        Self::new(Vec::new())
    }

    /// Get the current hardware state
    pub async fn state(&self) -> HardwareState {
        self.state.read().await.clone()
    }

    /// Register a callback for incoming device messages (Status and StatusRequest only)
    pub async fn register_callback<F>(&self, callback: F)
    where
        F: Fn(DeviceMessage) + Send + Sync + 'static,
    {
        let mut callbacks = self.callbacks.write().await;
        callbacks.push(Arc::new(callback));
    }

    /// Send a HID command
    pub async fn send_command<C: HidOutputCommand + Send + 'static>(&self, command: C) -> Result<usize, HidError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.send_buffer.send((Box::new(command), tx))
            .map_err(|_| HidError::NotConnected)?;
        rx.await.map_err(|_| HidError::NotConnected)?
    }

    /// Get raw device access (for firmware updates)
    pub async fn raw_device(&self) -> Option<Arc<Mutex<Option<RawHidDevice>>>> {
        Some(self.device.clone())
    }

    /// Wait for device connection
    async fn wait_for_device(&self) -> Result<(), HidError> {
        info!("Looking for device...");
        
        let mut state = self.state.write().await;
        state.connection_status = ConnectionState::Disconnected;
        drop(state);

        loop {
            match self.search_for_device().await {
                Ok(device) => {
                    let mut dev = self.device.lock().await;
                    *dev = Some(device);
                    drop(dev);
                    
                    self.set_hardware_info().await;
                    return Ok(());
                }
                Err(_) => {
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    /// Search for a compatible device
    async fn search_for_device(&self) -> Result<RawHidDevice, HidError> {
        let api = HidApi::new().map_err(|e| HidError::HidApiError(e.to_string()))?;

        // If no specific device info, search for mutenix devices
        if self.device_info.is_empty() {
            for device_info in api.device_list() {
                if let Some(product) = device_info.product_string() {
                    if product.to_lowercase().contains("mutenix") {
                        debug!("Found mutenix device: {:?}", device_info);
                        return device_info
                            .open_device(&api)
                            .map_err(|e| HidError::HidApiError(e.to_string()));
                    }
                }
            }
        } else {
            // Try to open specific devices
            for info in &self.device_info {
                let result = if info.vendor_id == 0 && info.product_id == 0 {
                    // Try to open by serial number only
                    if let Some(serial) = &info.serial_number {
                        api.device_list()
                            .find(|d| d.serial_number() == Some(serial.as_str()))
                            .and_then(|d| d.open_device(&api).ok())
                    } else {
                        None
                    }
                } else {
                    // Try to open by VID/PID and optional serial
                    if let Some(serial) = &info.serial_number {
                        api.open_serial(info.vendor_id, info.product_id, serial).ok()
                    } else {
                        api.open(info.vendor_id, info.product_id).ok()
                    }
                };

                if let Some(device) = result {
                    info!("Device opened successfully");
                    return Ok(device);
                }
            }
        }

        Err(HidError::NotConnected)
    }

    /// Set hardware information from connected device
    async fn set_hardware_info(&self) {
        let device = self.device.lock().await;
        
        if let Some(dev) = device.as_ref() {
            let mut state = self.state.write().await;
            state.serial_number = dev.get_serial_number_string().ok().flatten();
            state.manufacturer = dev.get_manufacturer_string().ok().flatten();
            state.product = dev.get_product_string().ok().flatten();
            state.connection_status = ConnectionState::Connected;
            
            info!("Connected to device: {:?}", state);
        }
    }

    /// Send a report to the device
    async fn send_report(&self, command: &dyn HidOutputCommand) -> Result<usize, HidError> {
        let device = self.device.lock().await;
        
        let dev = device.as_ref().ok_or(HidError::NotConnected)?;
        
        let mut buffer = vec![command.report_id()];
        buffer.extend_from_slice(&command.to_buffer());
        
        debug!("HID TX: {:02x?}", buffer);
        
        dev.write(&buffer)
            .map_err(|e| HidError::WriteFailed(e.to_string()))
    }

    /// Read loop
    async fn read_loop(&self) {
        loop {
            if !*self.running.read().await {
                break;
            }

            match self.read_once().await {
                Ok(_) => {
                    // Small yield to prevent busy loop
                    tokio::task::yield_now().await;
                }
                Err(HidError::NotConnected) => {
                    sleep(Duration::from_millis(100)).await;
                }
                Err(e) => {
                    error!("Read error: {}", e);
                    if let Err(e) = self.wait_for_device().await {
                        error!("Failed to reconnect: {}", e);
                    }
                }
            }
        }
    }

    /// Read once from device
    async fn read_once(&self) -> Result<(), HidError> {
        let device = self.device.lock().await;
        
        let dev = device.as_ref().ok_or(HidError::NotConnected)?;
        
        let mut buffer = [0u8; 64];
        match dev.read_timeout(&mut buffer, 100) {
            Ok(0) => {
                // No data, this is normal
                Ok(())
            }
            Ok(size) => {
                debug!("HID RX: {:02x?}", &buffer[..size]);
                
                // Parse and filter messages - only forward Status and StatusRequest to subscribers
                if let Ok(parsed) = parse_input_message(&buffer[..size]) {
                    // Try to downcast to Status
                    if let Some(status) = parsed.downcast_ref::<Status>() {
                        let callbacks = self.callbacks.read().await;
                        let msg = DeviceMessage::Status(status.clone());
                        for callback in callbacks.iter() {
                            callback(msg.clone());
                        }
                    }
                    // Try to downcast to StatusRequest
                    else if let Some(_status_req) = parsed.downcast_ref::<StatusRequest>() {
                        let callbacks = self.callbacks.read().await;
                        let msg = DeviceMessage::StatusRequest(StatusRequest);
                        for callback in callbacks.iter() {
                            callback(msg.clone());
                        }
                    }
                    // Ping responses, VersionInfo, and other messages are not forwarded
                    else {
                        debug!("Message received but not forwarded to subscribers (ping/version/etc)");
                    }
                } else {
                    debug!("Failed to parse message or unknown message type");
                }
                Ok(())
            }
            Err(e) => Err(HidError::ReadFailed(e.to_string())),
        }
    }

    /// Write loop
    async fn write_loop(&self) {
        let mut receiver = self.send_receiver.lock().await;
        
        loop {
            if !*self.running.read().await {
                break;
            }
            
            match receiver.recv().await {
                Some((command, response)) => {
                    match self.send_report(command.as_ref()).await {
                        Ok(size) => {
                            let _ = response.send(Ok(size));
                        }
                        Err(e) => {
                            error!("Failed to send command: {}", e);
                            let _ = response.send(Err(e));
                        }
                    }
                }
                None => {
                    // Channel closed, exit loop
                    break;
                }
            }
        }
    }

    /// Ping loop to keep connection alive
    async fn ping_loop(&self) {
        // Use interval for more precise timing
        let mut interval = tokio::time::interval(Duration::from_secs(PING_LOOP_TIME_SECONDS));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut ping_counter = 0;
        
        loop {
            interval.tick().await;
            
            if !*self.running.read().await {
                break;
            }

            let last_ping = *self.last_ping.lock().await;
            let elapsed = last_ping.elapsed();
            
            debug!("Sending ping after {:?} since last ping", elapsed);
            
            match self.send_command(SimpleCommand::ping(ping_counter)).await {
                Ok(_) => {
                    debug!("Ping sent successfully");
                    *self.last_ping.lock().await = Instant::now();
                }
                Err(e) => {
                    warn!("Failed to send ping: {}", e);
                }
            }
            ping_counter = ping_counter.wrapping_add(1);
        }
    }

    /// Main processing loop
    pub async fn process(&self) -> Result<(), HidError> {
        self.wait_for_device().await?;
        
        tokio::select! {
            _ = self.read_loop() => {},
            _ = self.write_loop() => {},
            _ = self.ping_loop() => {},
        }
        
        Ok(())
    }

    /// Stop the device processing
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
    }
}
