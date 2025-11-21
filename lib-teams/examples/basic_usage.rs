// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

use std::sync::Arc;
use teams_api::{
    ClientMessage, Identifier, MeetingAction, ServerMessage, TeamsState, TeamsWebSocketClient,
};
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Create shared state
    let state = TeamsState::new();

    // Storage for the authentication token
    // In a real application, you would load this from a file or database
    let saved_token = Arc::new(RwLock::new(load_token_from_storage()));

    // Create identifier for authentication
    // Note: Start with empty token if none is saved yet
    let identifier = Identifier::new("MyManufacturer", "MyDevice", "MyApp", "1.0.0")
        .with_token(saved_token.read().await.clone());

    // Create WebSocket client
    let client = TeamsWebSocketClient::new(state.clone(), "ws://localhost:8124", identifier);

    // Register a callback for incoming messages
    {
        let saved_token = saved_token.clone();
        client
            .register_callback(move |message| {
                println!("Received message: {:?}", message);
                
                // Handle token refresh
                handle_token_refresh(&message, saved_token.clone());
            })
            .await;
    }

    // Spawn the client processing task
    let client_clone = Arc::new(client);
    let client_handle = {
        let client = client_clone.clone();
        tokio::spawn(async move {
            if let Err(e) = client.process().await {
                eprintln!("Client error: {}", e);
            }
        })
    };

    // Wait for connection
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // To receive a token and status updates, you need to trigger an action
    // The first action will generate a token (if not already authenticated)
    println!("Triggering meeting action to authenticate and receive token...");
    let message = ClientMessage::action(MeetingAction::QueryMeetingState);
    client_clone.send_message(message)?;

    // Read the current state at any time
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            let current_state = state.state().await;
            println!("Current state: {:?}", current_state);
        }
    });

    // Wait for the client to finish (or stop it manually)
    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    client_clone.stop().await;

    client_handle.await?;

    Ok(())
}

/// Handle token refresh from server messages
/// 
/// When the server sends a new authentication token, this function:
/// 1. Detects the token_refresh field in the message
/// 2. Updates the in-memory token storage
/// 3. Persists the token for future sessions
fn handle_token_refresh(message: &ServerMessage, saved_token: Arc<RwLock<String>>) {
    if let Some(new_token) = &message.token_refresh {
        println!("Received new authentication token");
        let token_to_save = new_token.clone();
        
        tokio::spawn(async move {
            *saved_token.write().await = token_to_save.clone();
            save_token_to_storage(&token_to_save);
            println!("Token saved for future use");
        });
    }
}

/// Load the saved authentication token from storage
/// 
/// Reads the token from .teams-token file in the current directory.
/// Returns empty string if file doesn't exist or can't be read.
fn load_token_from_storage() -> String {
    match std::fs::read_to_string(".teams-token") {
        Ok(token) => {
            let token = token.trim().to_string();
            if !token.is_empty() {
                println!("Loaded existing token from .teams-token");
            }
            token
        }
        Err(_) => {
            println!("No existing token found, will request new one on first action");
            String::new()
        }
    }
}

/// Save the authentication token to storage
/// 
/// Writes the token to .teams-token file in the current directory.
fn save_token_to_storage(token: &str) {
    match std::fs::write(".teams-token", token) {
        Ok(_) => println!("Token saved to .teams-token"),
        Err(e) => eprintln!("Failed to save token: {}", e),
    }
}
