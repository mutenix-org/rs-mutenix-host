// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

/// Size of the header in bytes for chunk packets
pub const HEADER_SIZE: usize = 8;

/// Maximum size of data in a single chunk (60 bytes total - header)
pub const MAX_CHUNK_SIZE: usize = 60 - HEADER_SIZE;

/// Sleep time between data transfers in seconds
pub const DATA_TRANSFER_SLEEP_TIME: f64 = 1.0;

/// Sleep time for state changes in seconds
pub const STATE_CHANGE_SLEEP_TIME: f64 = 0.5;

/// Sleep time while waiting for requests in seconds
pub const WAIT_FOR_REQUESTS_SLEEP_TIME: f64 = STATE_CHANGE_SLEEP_TIME;

/// HID Report ID for communication commands
pub const HID_REPORT_ID_COMMUNICATION: u8 = 1;

/// HID Report ID for data transfer
pub const HID_REPORT_ID_TRANSFER: u8 = 2;

/// HID command to prepare device for update
pub const HID_COMMAND_PREPARE_UPDATE: u8 = 0xE0;

/// HID command to reset device
pub const HID_COMMAND_RESET: u8 = 0xE1;

pub const PING_LOOP_TIME_SECONDS: u64 = 4;