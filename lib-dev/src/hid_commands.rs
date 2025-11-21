// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

use std::fmt;

/// Hardware types for the Macropad
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HardwareType {
    Unknown = 0x00,
    FiveButtonUsbV1 = 0x02,
    FiveButtonUsb = 0x03,
    FiveButtonBt = 0x04,
    TenButtonUsb = 0x05,
    TenButtonBt = 0x06,
}

impl From<u8> for HardwareType {
    fn from(val: u8) -> Self {
        match val {
            0x02 => HardwareType::FiveButtonUsbV1,
            0x03 => HardwareType::FiveButtonUsb,
            0x04 => HardwareType::FiveButtonBt,
            0x05 => HardwareType::TenButtonUsb,
            0x06 => HardwareType::TenButtonBt,
            _ => HardwareType::Unknown,
        }
    }
}

impl fmt::Display for HardwareType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HardwareType::Unknown => write!(f, "Unknown"),
            HardwareType::FiveButtonUsbV1 => write!(f, "Five Button USB V1"),
            HardwareType::FiveButtonUsb => write!(f, "Five Button USB"),
            HardwareType::FiveButtonBt => write!(f, "Five Button BT"),
            HardwareType::TenButtonUsb => write!(f, "Ten Button USB"),
            HardwareType::TenButtonBt => write!(f, "Ten Button BT"),
        }
    }
}

/// Identifiers for incoming HID messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HidInCommand {
    VersionInfo = 0x99,
    Status = 0x01,
    StatusRequest = 0x02,
}

/// Identifiers for outgoing HID messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HidOutCommand {
    SetLed = 0x01,
    Ping = 0xF0,
    PrepareUpdate = 0xE0,
    Reset = 0xE1,
    UpdateConfig = 0xE2,
}

/// Base trait for HID input messages
pub trait HidInputMessage: fmt::Debug {
    fn from_buffer(buffer: &[u8]) -> Result<Self, HidMessageError>
    where
        Self: Sized;
}

/// Errors that can occur when parsing HID messages
#[derive(Debug, thiserror::Error)]
pub enum HidMessageError {
    #[error("Invalid buffer length: expected at least {expected}, got {actual}")]
    InvalidLength { expected: usize, actual: usize },
    
    #[error("Unknown command: {0:#x}")]
    UnknownCommand(u8),
    
    #[error("Invalid data")]
    InvalidData,
}

/// Status message from device
#[derive(Debug, Clone)]
pub struct Status {
    buffer: [u8; 6],
}

impl Status {
    pub fn trigger_button(button: u8) -> Self {
        Self {
            buffer: [button, 1, 0, 0, 1, 0],
        }
    }

    pub fn button(&self) -> u8 {
        self.buffer[0]
    }

    pub fn triggered(&self) -> bool {
        self.buffer[1] != 0
    }

    pub fn longpressed(&self) -> bool {
        self.buffer[2] != 0
    }

    pub fn pressed(&self) -> bool {
        self.buffer[3] != 0
    }

    pub fn released(&self) -> bool {
        self.buffer[4] != 0
    }
}

impl HidInputMessage for Status {
    fn from_buffer(buffer: &[u8]) -> Result<Self, HidMessageError> {
        if buffer.len() < 6 {
            return Err(HidMessageError::InvalidLength {
                expected: 6,
                actual: buffer.len(),
            });
        }

        let mut buf = [0u8; 6];
        buf.copy_from_slice(&buffer[0..6]);

        Ok(Self { buffer: buf })
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Status {{ button: {}, triggered: {}, longpress: {}, pressed: {}, released: {} }}",
            self.button(),
            self.triggered(),
            self.longpressed(),
            self.pressed(),
            self.released()
        )
    }
}

/// Version information from device
#[derive(Debug, Clone)]
pub struct VersionInfo {
    buffer: [u8; 6],
}

impl VersionInfo {
    pub fn version(&self) -> String {
        format!("{}.{}.{}", self.buffer[0], self.buffer[1], self.buffer[2])
    }

    pub fn hardware_type(&self) -> HardwareType {
        HardwareType::from(self.buffer[3])
    }
}

impl HidInputMessage for VersionInfo {
    fn from_buffer(buffer: &[u8]) -> Result<Self, HidMessageError> {
        if buffer.len() < 6 {
            return Err(HidMessageError::InvalidLength {
                expected: 6,
                actual: buffer.len(),
            });
        }

        let mut buf = [0u8; 6];
        buf.copy_from_slice(&buffer[0..6]);

        Ok(Self { buffer: buf })
    }
}

impl fmt::Display for VersionInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Version Info: {}, type {}",
            self.version(),
            self.hardware_type()
        )
    }
}

/// Status request from device
#[derive(Debug, Clone)]
pub struct StatusRequest;

impl HidInputMessage for StatusRequest {
    fn from_buffer(_buffer: &[u8]) -> Result<Self, HidMessageError> {
        Ok(Self)
    }
}

impl fmt::Display for StatusRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Status Request")
    }
}

/// Parse incoming HID message
pub fn parse_input_message(buffer: &[u8]) -> Result<Box<dyn std::any::Any>, HidMessageError> {
    if buffer.len() < 2 {
        return Err(HidMessageError::InvalidLength {
            expected: 2,
            actual: buffer.len(),
        });
    }

    match buffer[1] {
        0x99 => Ok(Box::new(VersionInfo::from_buffer(&buffer[2..])?)),
        0x01 => Ok(Box::new(Status::from_buffer(&buffer[2..])?)),
        0x02 => Ok(Box::new(StatusRequest::from_buffer(&buffer[2..])?)),
        cmd => Err(HidMessageError::UnknownCommand(cmd)),
    }
}

/// LED colors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedColor {
    Red,
    Green,
    Blue,
    White,
    Black,
    Yellow,
    Cyan,
    Magenta,
    Orange,
    Purple,
}

impl LedColor {
    /// Returns RGBW values
    pub fn to_rgbw(&self) -> [u8; 4] {
        match self {
            LedColor::Red => [0x0A, 0x00, 0x00, 0x00],
            LedColor::Green => [0x00, 0x0A, 0x00, 0x00],
            LedColor::Blue => [0x00, 0x00, 0x0A, 0x00],
            LedColor::White => [0x00, 0x00, 0x00, 0x0A],
            LedColor::Black => [0x00, 0x00, 0x00, 0x00],
            LedColor::Yellow => [0x0A, 0x0A, 0x00, 0x00],
            LedColor::Cyan => [0x00, 0x0A, 0x0A, 0x00],
            LedColor::Magenta => [0x0A, 0x00, 0x0A, 0x00],
            LedColor::Orange => [0x0A, 0x08, 0x00, 0x00],
            LedColor::Purple => [0x09, 0x00, 0x09, 0x00],
        }
    }
}

/// Base trait for HID output commands
pub trait HidOutputCommand: fmt::Debug + Send + Sync {
    fn to_buffer(&self) -> Vec<u8>;
    fn report_id(&self) -> u8 {
        1
    }
}

/// Set LED command
#[derive(Debug, Clone)]
pub struct SetLed {
    id: u8,
    color: LedColor,
    counter: u8,
}

impl SetLed {
    pub fn new(id: u8, color: LedColor) -> Self {
        Self { id, color, counter: 0 }
    }

    pub fn with_counter(mut self, counter: u8) -> Self {
        self.counter = counter;
        self
    }
}

impl HidOutputCommand for SetLed {
    fn to_buffer(&self) -> Vec<u8> {
        let color = self.color.to_rgbw();
        vec![
            HidOutCommand::SetLed as u8,
            self.id,
            color[0],
            color[1],
            color[2],
            color[3],
            0,
            self.counter,
        ]
    }
}

impl fmt::Display for SetLed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SetLed {{ id: {}, color: {:?} }}", self.id, self.color)
    }
}

/// Update config command
#[derive(Debug, Clone)]
pub struct UpdateConfig {
    activate_debug: u8,
    activate_filesystem: u8,
    counter: u8,
}

impl UpdateConfig {
    pub fn new() -> Self {
        Self {
            activate_debug: 0,
            activate_filesystem: 0,
            counter: 0,
        }
    }

    pub fn activate_serial_console(mut self, activate: bool) -> Self {
        self.activate_debug = if activate { 2 } else { 1 };
        self
    }

    pub fn activate_filesystem(mut self, activate: bool) -> Self {
        self.activate_filesystem = if activate { 2 } else { 1 };
        self
    }

    pub fn with_counter(mut self, counter: u8) -> Self {
        self.counter = counter;
        self
    }
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl HidOutputCommand for UpdateConfig {
    fn to_buffer(&self) -> Vec<u8> {
        vec![
            HidOutCommand::UpdateConfig as u8,
            self.activate_debug,
            self.activate_filesystem,
            0,
            0,
            0,
            0,
            self.counter,
        ]
    }
}

impl fmt::Display for UpdateConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "UpdateConfig {{ debug: {}, filesystem: {} }}",
            self.activate_debug, self.activate_filesystem
        )
    }
}

/// Simple HID command (Ping, PrepareUpdate, Reset)
#[derive(Debug, Clone)]
pub struct SimpleCommand {
    command: HidOutCommand,
    counter: u8,
}

impl SimpleCommand {
    pub fn new(command: HidOutCommand) -> Self {
        Self { command, counter: 0 }
    }

    pub fn with_counter(mut self, counter: u8) -> Self {
        self.counter = counter;
        self
    }

    pub fn ping(ping_counter: u8) -> Self {
        Self::new(HidOutCommand::Ping).with_counter(ping_counter)
    }

    pub fn prepare_update() -> Self {
        Self::new(HidOutCommand::PrepareUpdate)
    }

    pub fn reset() -> Self {
        Self::new(HidOutCommand::Reset)
    }
}

impl HidOutputCommand for SimpleCommand {
    fn to_buffer(&self) -> Vec<u8> {
        vec![self.command as u8, 0, 0, 0, 0, 0, 0, self.counter]
    }
}

impl fmt::Display for SimpleCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.command)
    }
}
