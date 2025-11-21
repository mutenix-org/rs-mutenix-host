// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

use log::info;
use std::fmt;

/// Error message received from device
#[derive(Debug, Clone)]
pub struct UpdateError {
    pub identifier: String,
    pub info: String,
}

impl UpdateError {
    pub fn from_bytes(data: &[u8]) -> Self {
        if data.len() < 2 {
            return Self {
                identifier: String::new(),
                info: String::new(),
            };
        }

        let identifier = String::from_utf8_lossy(&data[0..2]).to_string();
        
        if !Self::is_valid_identifier(&identifier) {
            return Self {
                identifier,
                info: String::new(),
            };
        }

        let length = if data.len() > 2 {
            data[2].max(33) as usize
        } else {
            0
        };

        let info = if data.len() > 3 {
            let end = (3 + length).min(data.len());
            String::from_utf8_lossy(&data[3..end]).to_string()
        } else {
            String::new()
        };

        info!("Error received: {}", info);

        Self { identifier, info }
    }

    pub fn is_valid(&self) -> bool {
        Self::is_valid_identifier(&self.identifier)
    }

    fn is_valid_identifier(id: &str) -> bool {
        id == "ER"
    }
}

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_valid() {
            write!(f, "Error: {}", self.info)
        } else {
            write!(f, "Invalid Request")
        }
    }
}

/// Acknowledgment of a received chunk
#[derive(Debug, Clone)]
pub struct ChunkAck {
    pub identifier: String,
    pub id: u16,
    pub package: u16,
    pub type_: u8,
}

impl ChunkAck {
    pub fn from_bytes(data: &[u8]) -> Self {
        if data.len() < 2 {
            return Self {
                identifier: String::new(),
                id: 0,
                package: 0,
                type_: 0,
            };
        }

        let identifier = String::from_utf8_lossy(&data[0..2]).to_string();

        if !Self::is_valid_identifier(&identifier) {
            return Self {
                identifier,
                id: 0,
                package: 0,
                type_: 0,
            };
        }

        let id = if data.len() >= 4 {
            u16::from_le_bytes([data[2], data[3]])
        } else {
            0
        };

        let package = if data.len() >= 6 {
            u16::from_le_bytes([data[4], data[5]])
        } else {
            0
        };

        let type_ = if data.len() >= 7 { data[6] } else { 0 };

        Self {
            identifier,
            id,
            package,
            type_,
        }
    }

    pub fn is_valid(&self) -> bool {
        Self::is_valid_identifier(&self.identifier)
    }

    fn is_valid_identifier(id: &str) -> bool {
        id == "AK"
    }
}

impl fmt::Display for ChunkAck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_valid() {
            write!(
                f,
                "File: {}, Type: {}, Package: {}",
                self.id, self.type_, self.package
            )
        } else {
            write!(f, "Invalid Request")
        }
    }
}

/// Log message from device
#[derive(Debug, Clone)]
pub struct LogMessage {
    pub identifier: String,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Error,
}

impl LogMessage {
    pub fn from_bytes(data: &[u8]) -> Self {
        if data.len() < 2 {
            return Self {
                identifier: String::new(),
                level: LogLevel::Debug,
                message: String::new(),
            };
        }

        let identifier = String::from_utf8_lossy(&data[0..2]).to_string();

        if !Self::is_valid_identifier(&identifier) {
            return Self {
                identifier,
                level: LogLevel::Debug,
                message: String::new(),
            };
        }

        let level = if identifier == "LD" {
            LogLevel::Debug
        } else {
            LogLevel::Error
        };

        // Find null terminator or use entire remaining buffer
        let end_pos = data[2..]
            .iter()
            .position(|&b| b == 0)
            .map(|p| p + 2)
            .unwrap_or(data.len());

        let message = String::from_utf8_lossy(&data[2..end_pos]).to_string();

        Self {
            identifier,
            level,
            message,
        }
    }

    pub fn is_valid(&self) -> bool {
        Self::is_valid_identifier(&self.identifier)
    }

    fn is_valid_identifier(id: &str) -> bool {
        id == "LE" || id == "LD"
    }
}

impl fmt::Display for LogMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_valid() {
            write!(
                f,
                "{}: {}",
                match self.level {
                    LogLevel::Debug => "debug",
                    LogLevel::Error => "error",
                },
                self.message
            )
        } else {
            write!(f, "Invalid Request")
        }
    }
}

/// Parsed HID update message
#[derive(Debug, Clone)]
pub enum HidUpdateMessage {
    ChunkAck(ChunkAck),
    Error(UpdateError),
    Log(LogMessage),
}

/// Parse a HID update message from raw bytes
pub fn parse_hid_update_message(data: &[u8]) -> Option<HidUpdateMessage> {
    if data.len() < 2 {
        return None;
    }

    let identifier = String::from_utf8_lossy(&data[0..2]);

    match identifier.as_ref() {
        "AK" => Some(HidUpdateMessage::ChunkAck(ChunkAck::from_bytes(data))),
        "ER" => Some(HidUpdateMessage::Error(UpdateError::from_bytes(data))),
        "LD" | "LE" => Some(HidUpdateMessage::Log(LogMessage::from_bytes(data))),
        _ => None,
    }
}
