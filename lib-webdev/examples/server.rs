// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

//! Example: Run the WebSocket device emulator server
//! 
//! Usage: cargo run --example server
//! 
//! Then open http://localhost:3000 in your browser

use mutenix_hid::HardwareType;
use mutenix_webdev::WebServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let server = WebServer::new(HardwareType::FiveButtonUsb, 3000);
    
    println!("Starting Mutenix device emulator on http://localhost:3000");
    println!("WebSocket endpoint: ws://localhost:3000/ws");
    
    server.run().await?;

    Ok(())
}
