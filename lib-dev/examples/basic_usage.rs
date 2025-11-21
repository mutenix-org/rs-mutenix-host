// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

use mutenix_hid::{HidDevice, LedColor, SetLed};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Create a HID device handler that auto-discovers mutenix devices
    let device = Arc::new(HidDevice::new_auto());

    // Register a callback for incoming device messages
    device
        .register_callback(|message| {
            println!("Received message: {:?}", message);
        })
        .await;

    // Clone device handle for the command task
    let device_clone = Arc::clone(&device);
    
    let sleep_time = tokio::time::Duration::from_secs(1);
    
    // Create a task to send LED commands after device connects
    tokio::spawn(async move {
        // Wait for device to be ready
        tokio::time::sleep(sleep_time).await;
        
        // Set LED to green
        println!("Setting LED to green...");
        if let Err(e) = device_clone.send_command(SetLed::new(1, LedColor::Green)).await {
            eprintln!("Failed to set LED: {}", e);
        }
        
        // Wait a bit and change to blue
        tokio::time::sleep(sleep_time).await;
        println!("Setting LED to blue...");
        if let Err(e) = device_clone.send_command(SetLed::new(2, LedColor::Blue)).await {
            eprintln!("Failed to set LED: {}", e);
        }
        
        // Wait a bit and change to red
        tokio::time::sleep(sleep_time).await;
        println!("Setting LED to red...");
        if let Err(e) = device_clone.send_command(SetLed::new(3, LedColor::Red)).await {
            eprintln!("Failed to set LED: {}", e);
        }
        
        // Turn off LED (black)
        tokio::time::sleep(sleep_time).await;
        println!("Turning LED off...");
        if let Err(e) = device_clone.send_command(SetLed::new(1, LedColor::Black)).await {
            eprintln!("Failed to set LED: {}", e);
        }
        
        // Turn off LED (black)
        tokio::time::sleep(sleep_time).await;
        println!("Turning LED off...");
        if let Err(e) = device_clone.send_command(SetLed::new(2, LedColor::Black)).await {
            eprintln!("Failed to set LED: {}", e);
        }
        
        // Turn off LED (black)
        tokio::time::sleep(sleep_time).await;
        println!("Turning LED off...");
        if let Err(e) = device_clone.send_command(SetLed::new(3, LedColor::Black)).await {
            eprintln!("Failed to set LED: {}", e);
        }
    });

    // Run device processing (this blocks until stopped)
    // Note: In a real application, you might want to run this differently
    // or restructure to allow sending commands concurrently
    if let Err(e) = device.process().await {
        eprintln!("Device error: {}", e);
    }

    Ok(())
}
