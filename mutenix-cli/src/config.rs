// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

use anyhow::{Context, Result};
use lib_actions::ButtonAction;
use mutenix_hid::LedColor;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Root configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub version: u32,
    pub device_identifications: Vec<DeviceIdentification>,
    pub actions: Vec<ButtonAction>,
    #[serde(default)]
    pub longpress_action: Vec<ButtonAction>,
    pub led_status: Vec<LedStatus>,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub virtual_keypad: VirtualKeypadConfig,
}

/// Device identification (vendor ID, product ID)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceIdentification {
    pub vendor_id: u16,
    pub product_id: u16,
}

/// LED status configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedStatus {
    pub button_id: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub teams_state: Option<TeamsStateConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_command: Option<LedStatusResultCommand>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_command: Option<LedStatusColorCommand>,
    #[serde(default)]
    pub webhook: bool,
}

/// LED status result command configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedStatusResultCommand {
    pub command: String,
    #[serde(default = "default_interval")]
    pub interval: f64,
    #[serde(default = "default_timeout")]
    pub timeout: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_on: Option<LedColorConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_off: Option<LedColorConfig>,
}

/// LED status color command configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedStatusColorCommand {
    pub command: String,
    #[serde(default = "default_interval")]
    pub interval: f64,
    #[serde(default = "default_timeout")]
    pub timeout: f64,
}

fn default_interval() -> f64 {
    5.0
}

fn default_timeout() -> f64 {
    0.5
}

/// Teams state configuration for LED
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsStateConfig {
    pub teams_state: TeamsStateType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_on: Option<LedColorConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_off: Option<LedColorConfig>,
}

/// Teams state types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TeamsStateType {
    IsMuted,
    IsHandRaised,
    IsVideoOn,
    IsInMeeting,
    IsRecordingOn,
    IsBackgroundBlurred,
    IsSharing,
    HasUnreadMessages,
}

/// LED color configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LedColorConfig {
    Black,
    Red,
    Green,
    Blue,
    Yellow,
    Cyan,
    Magenta,
    White,
    Orange,
    Purple,
}

impl LedColorConfig {
    pub fn to_led_color(&self) -> LedColor {
        match self {
            LedColorConfig::Black => LedColor::Black,
            LedColorConfig::Red => LedColor::Red,
            LedColorConfig::Green => LedColor::Green,
            LedColorConfig::Blue => LedColor::Blue,
            LedColorConfig::Yellow => LedColor::Yellow,
            LedColorConfig::Cyan => LedColor::Cyan,
            LedColorConfig::Magenta => LedColor::Magenta,
            LedColorConfig::White => LedColor::White,
            LedColorConfig::Orange => LedColor::Orange,
            LedColorConfig::Purple => LedColor::Purple,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_true")]
    pub console_enabled: bool,
    #[serde(default = "default_log_level")]
    pub console_level: String,
    #[serde(default = "default_true")]
    pub file_enabled: bool,
    #[serde(default = "default_log_level")]
    pub file_level: String,
    #[serde(default = "default_file_max_size")]
    pub file_max_size: usize,
    #[serde(default = "default_file_backup_count")]
    pub file_backup_count: u32,
    #[serde(default)]
    pub submodules: std::collections::HashMap<String, String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            console_enabled: default_true(),
            console_level: default_log_level(),
            file_enabled: default_true(),
            file_level: default_log_level(),
            file_max_size: default_file_max_size(),
            file_backup_count: default_file_backup_count(),
            submodules: std::collections::HashMap::new(),
        }
    }
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_true() -> bool {
    true
}

fn default_file_max_size() -> usize {
    3145728
}

fn default_file_backup_count() -> u32 {
    5
}

/// Virtual keypad configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualKeypadConfig {
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    #[serde(default = "default_bind_port")]
    pub bind_port: u16,
}

impl Default for VirtualKeypadConfig {
    fn default() -> Self {
        Self {
            bind_address: default_bind_address(),
            bind_port: default_bind_port(),
        }
    }
}

fn default_bind_address() -> String {
    "127.0.0.1".to_string()
}

fn default_bind_port() -> u16 {
    12909
}

impl Config {
    /// Load configuration from a YAML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;
        let config: Config = serde_yaml::from_str(&content)
            .with_context(|| "Failed to parse YAML config")?;
        Ok(config)
    }

    /// Find button action by button ID
    pub fn find_button_action(&self, button_id: u8) -> Option<&ButtonAction> {
        self.actions.iter().find(|a| a.button_id == button_id)
    }

    /// Find longpress action by button ID
    pub fn find_longpress_action(&self, button_id: u8) -> Option<&ButtonAction> {
        self.longpress_action.iter().find(|a| a.button_id == button_id)
    }

    /// Find LED status by button ID
    pub fn find_led_status(&self, button_id: u8) -> Option<&LedStatus> {
        self.led_status.iter().find(|l| l.button_id == button_id)
    }

    /// Get device info for HID connection
    pub fn get_device_info(&self) -> Vec<mutenix_hid::DeviceInfo> {
        self.device_identifications
            .iter()
            .map(|d| mutenix_hid::DeviceInfo {
                vendor_id: d.vendor_id,
                product_id: d.product_id,
                serial_number: None,
            })
            .collect()
    }
}
