// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

use mutenix_hid::HardwareType;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Virtual LED state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedState {
    pub id: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub w: u8,
}

impl LedState {
    pub fn new(id: u8) -> Self {
        Self {
            id,
            r: 0,
            g: 0,
            b: 0,
            w: 0,
        }
    }

    pub fn set_rgbw(&mut self, rgbw: [u8; 4]) {
        self.r = rgbw[0];
        self.g = rgbw[1];
        self.b = rgbw[2];
        self.w = rgbw[3];
    }
}

/// Virtual button state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonState {
    pub button: u8,
    pub pressed: bool,
}

impl ButtonState {
    pub fn new(button: u8) -> Self {
        Self {
            button,
            pressed: false,
        }
    }
}

/// Device emulator state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmulatorState {
    pub version: String,
    pub hardware_type: u8,
    pub leds: Vec<LedState>,
    pub buttons: Vec<ButtonState>,
    pub serial_number: String,
}

impl EmulatorState {
    pub fn new(hardware_type: HardwareType, num_buttons: u8) -> Self {
        let num_leds = num_buttons;
        
        Self {
            version: "1.0.0".to_string(),
            hardware_type: hardware_type as u8,
            leds: (0..num_leds).map(|i| LedState::new(i)).collect(),
            buttons: (0..num_buttons).map(|i| ButtonState::new(i)).collect(),
            serial_number: "EMULATOR001".to_string(),
        }
    }
}

/// Device emulator
pub struct DeviceEmulator {
    state: Arc<RwLock<EmulatorState>>,
}

impl DeviceEmulator {
    /// Create a new device emulator
    pub fn new(hardware_type: HardwareType) -> Self {
        let num_buttons = match hardware_type {
            HardwareType::FiveButtonUsb | HardwareType::FiveButtonBt | HardwareType::FiveButtonUsbV1 => 5,
            HardwareType::TenButtonUsb | HardwareType::TenButtonBt => 10,
            _ => 5,
        };

        Self {
            state: Arc::new(RwLock::new(EmulatorState::new(hardware_type, num_buttons))),
        }
    }

    /// Get current state
    pub async fn get_state(&self) -> EmulatorState {
        self.state.read().await.clone()
    }

    /// Process incoming HID command
    pub async fn process_command(&self, buffer: &[u8]) -> Result<Vec<u8>, String> {
        if buffer.len() < 2 {
            return Err("Buffer too short".to_string());
        }

        let command_id = buffer[1];

        match command_id {
            0x01 => self.handle_set_led(&buffer[1..]).await,
            0xF0 => self.handle_ping(&buffer[1..]).await,
            0xE2 => self.handle_update_config(&buffer[1..]).await,
            _ => {
                log::warn!("Unknown command: {:#x}", command_id);
                Ok(Vec::new())
            }
        }
    }

    /// Handle SetLed command
    async fn handle_set_led(&self, buffer: &[u8]) -> Result<Vec<u8>, String> {
        if buffer.len() < 8 {
            return Err("SetLed buffer too short".to_string());
        }

        let led_id = buffer[1];
        let r = buffer[2];
        let g = buffer[3];
        let b = buffer[4];
        let w = buffer[5];

        let mut state = self.state.write().await;
        if let Some(led) = state.leds.iter_mut().find(|l| l.id == led_id) {
            led.set_rgbw([r, g, b, w]);
            log::debug!("LED {} set to RGBW({}, {}, {}, {})", led_id, r, g, b, w);
        }

        Ok(Vec::new())
    }

    /// Handle Ping command
    async fn handle_ping(&self, buffer: &[u8]) -> Result<Vec<u8>, String> {
        if buffer.len() < 8 {
            return Err("Ping buffer too short".to_string());
        }

        let counter = buffer[7];
        log::debug!("Ping received with counter: {}", counter);

        Ok(Vec::new())
    }

    /// Handle UpdateConfig command
    async fn handle_update_config(&self, buffer: &[u8]) -> Result<Vec<u8>, String> {
        if buffer.len() < 8 {
            return Err("UpdateConfig buffer too short".to_string());
        }

        let activate_debug = buffer[1];
        let activate_filesystem = buffer[2];

        log::debug!(
            "UpdateConfig: debug={}, filesystem={}",
            activate_debug,
            activate_filesystem
        );

        Ok(Vec::new())
    }

    /// Simulate button press
    pub async fn press_button(&self, button: u8) -> Result<Vec<u8>, String> {
        let mut state = self.state.write().await;
        if let Some(btn) = state.buttons.iter_mut().find(|b| b.button == button) {
            btn.pressed = true;
            log::debug!("Button {} pressed", button);
            
            // Create status message: [report_id, command_id, button, triggered, longpress, pressed, released, reserved]
            Ok(vec![1, 0x01, button, 1, 0, 1, 0, 0])
        } else {
            Err(format!("Button {} not found", button))
        }
    }

    /// Simulate button release
    pub async fn release_button(&self, button: u8) -> Result<Vec<u8>, String> {
        let mut state = self.state.write().await;
        if let Some(btn) = state.buttons.iter_mut().find(|b| b.button == button) {
            btn.pressed = false;
            log::debug!("Button {} released", button);
            
            // Create status message: [report_id, command_id, button, triggered, longpress, pressed, released, reserved]
            Ok(vec![1, 0x01, button, 0, 0, 0, 1, 0])
        } else {
            Err(format!("Button {} not found", button))
        }
    }

    /// Get version info message
    pub async fn get_version_info(&self) -> Vec<u8> {
        let state = self.state.read().await;
        let version_parts: Vec<&str> = state.version.split('.').collect();
        
        let major = version_parts.get(0).and_then(|v| v.parse::<u8>().ok()).unwrap_or(1);
        let minor = version_parts.get(1).and_then(|v| v.parse::<u8>().ok()).unwrap_or(0);
        let patch = version_parts.get(2).and_then(|v| v.parse::<u8>().ok()).unwrap_or(0);

        // Version info message: [report_id, command_id, major, minor, patch, hardware_type, reserved, reserved]
        vec![1, 0x99, major, minor, patch, state.hardware_type, 0, 0]
    }
}
