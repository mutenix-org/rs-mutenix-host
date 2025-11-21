// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

//! # Mutenix WebSocket Device Emulator
//!
//! This library provides a web server with WebSocket support that emulates
//! a Mutenix HID device for testing and development purposes.

pub mod server;
pub mod emulator;

pub use server::WebServer;
pub use emulator::DeviceEmulator;
