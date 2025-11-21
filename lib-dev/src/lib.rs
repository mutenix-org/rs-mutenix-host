// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

//! # Mutenix HID Communication Library
//!
//! This library provides HID communication with Mutenix devices, including:
//! - Device discovery and connection management
//! - Async message sending and receiving
//! - Firmware update support
//! - Command and status message handling

pub mod chunks;
pub mod constants;
pub mod device_messages;
pub mod device_update;
pub mod hid_commands;
pub mod hid_device;

// Re-export commonly used types
pub use chunks::{Chunk, ChunkType, Completed, FileChunk, FileDelete, FileEnd, FileStart};
pub use constants::*;
pub use device_messages::{ChunkAck, HidUpdateMessage, LogLevel, LogMessage, UpdateError};
pub use device_update::{perform_hid_upgrade, TransferFile};
pub use hid_commands::{
    HardwareType, HidInputMessage, HidOutCommand, HidOutputCommand, LedColor, SetLed,
    SimpleCommand, Status, UpdateConfig, VersionInfo,
};
pub use hid_device::{ConnectionState, DeviceInfo, DeviceMessage, HardwareState, HidDevice, HidError};
