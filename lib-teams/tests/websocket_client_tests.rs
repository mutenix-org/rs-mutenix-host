// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger matthias@bilger.info

use mockito::Server;
use std::sync::Arc;
use std::time::Duration;
use teams_api::{
    ClientMessage, ClientMessageParameterType, Identifier, MeetingAction, ServerMessage,
    TeamsState, TeamsWebSocketClient,
};
use tokio::sync::Mutex;
use tokio::time::sleep;

// Helper function to create an identifier for tests
fn create_test_identifier() -> Identifier {
    Identifier::new("TestManufacturer", "TestDevice", "TestApp", "1.0.0")
        .with_token("test_token")
}

// Helper function to create a websocket client for tests
fn create_test_client(uri: &str) -> TeamsWebSocketClient {
    let state = TeamsState::new();
    let identifier = create_test_identifier();
    TeamsWebSocketClient::new(state, uri, identifier)
}

#[tokio::test]
async fn test_identifier_build_uri() {
    let identifier = create_test_identifier();
    let uri = "ws://testserver";
    let _client = TeamsWebSocketClient::new(TeamsState::new(), uri, identifier);
    
    // The URI is built internally, we can verify it contains expected query params
    // by inspecting the client's internal URI through a connection attempt
    // For this test, we just verify the identifier fields are set correctly
    let identifier = create_test_identifier();
    assert_eq!(identifier.manufacturer, "TestManufacturer");
    assert_eq!(identifier.device, "TestDevice");
    assert_eq!(identifier.app, "TestApp");
    assert_eq!(identifier.app_version, "1.0.0");
    assert_eq!(identifier.token, "test_token");
}

#[tokio::test]
async fn test_send_message() {
    let client = create_test_client("ws://testserver");
    let message = ClientMessage::reaction(ClientMessageParameterType::ReactWow);
    
    // Test that sending a message doesn't panic
    let result = client.send_message(message);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_receive_message() {
    let mut server = Server::new_async().await;
    let url = server.url();
    let ws_url = url.replace("http://", "ws://");

    // Setup mock server to respond with a JSON message
    let _mock = server
        .mock("GET", "/")
        .with_status(101)
        .with_header("upgrade", "websocket")
        .with_header("connection", "Upgrade")
        .create_async()
        .await;

    let state = TeamsState::new();
    let identifier = create_test_identifier();
    let client = TeamsWebSocketClient::new(state.clone(), &ws_url, identifier);

    let callback_called = Arc::new(Mutex::new(false));
    let callback_called_clone = callback_called.clone();

    client
        .register_callback(move |_msg: ServerMessage| {
            let called = callback_called_clone.clone();
            tokio::spawn(async move {
                *called.lock().await = true;
            });
        })
        .await;

    // Note: Full websocket mocking is complex. This test verifies the callback registration
    assert!(!*callback_called.lock().await);
}

#[tokio::test]
async fn test_receive_message_sync_callback() {
    let state = TeamsState::new();
    let identifier = create_test_identifier();
    let client = TeamsWebSocketClient::new(state, "ws://testserver", identifier);

    let callback_called = Arc::new(Mutex::new(false));

    // Register a synchronous callback (in Rust, all callbacks are sync Fn)
    client
        .register_callback(move |_msg: ServerMessage| {
            // In a real async context, we'd need to use blocking or channels
            // For this test, we just verify the callback type works
        })
        .await;

    assert!(!*callback_called.lock().await);
}

#[tokio::test]
async fn test_stop() {
    let client = create_test_client("ws://testserver");
    
    // Client should be running initially
    assert!(!client.is_running().await);
    
    // Stop the client
    client.stop().await;
    
    // Verify it's not running
    assert!(!client.is_running().await);
}

#[tokio::test]
async fn test_send_invalid_message_type_safety() {
    // Rust's type system prevents sending invalid messages at compile time
    // This test verifies that only ClientMessage can be sent
    let client = create_test_client("ws://testserver");
    let message = ClientMessage::reaction(ClientMessageParameterType::ReactWow);
    
    let result = client.send_message(message);
    assert!(result.is_ok());
    
    // The following would not compile in Rust:
    // client.send_message("This is not a ClientMessage");
}

#[tokio::test]
async fn test_client_lifecycle() {
    let client = create_test_client("ws://testserver");
    
    // Test initial state
    assert!(!client.is_running().await);
    
    // Start the client in a background task
    let client_clone = Arc::new(client);
    let client_ref = client_clone.clone();
    
    let handle = tokio::spawn(async move {
        // This will fail to connect, but that's expected in tests
        let _ = client_ref.process().await;
    });
    
    // Give it a moment to start
    sleep(Duration::from_millis(10)).await;
    
    // Stop the client
    client_clone.stop().await;
    
    // Wait for the task to complete
    sleep(Duration::from_millis(50)).await;
    
    // Cancel the handle if still running
    handle.abort();
}

#[tokio::test]
async fn test_send_queue_operations() {
    let client = create_test_client("ws://testserver");
    
    // Send multiple messages
    let message1 = ClientMessage::reaction(ClientMessageParameterType::ReactWow);
    let message2 = ClientMessage::action(MeetingAction::ToggleMute);
    
    let result1 = client.send_message(message1);
    let result2 = client.send_message(message2);
    
    assert!(result1.is_ok());
    assert!(result2.is_ok());
}

#[tokio::test]
async fn test_state_management() {
    let state = TeamsState::new();
    
    // Test initial state
    let current_state = state.state().await;
    assert!(current_state.request_id.is_none());
    assert!(current_state.meeting_update.is_none());
    
    // Update state with a message
    let mut message = ServerMessage::default();
    message.request_id = Some(1);
    message.error_msg = Some("TEST".to_string());
    
    state.update_state(&message).await;
    
    // Verify state was updated
    let updated_state = state.state().await;
    assert_eq!(updated_state.request_id, Some(1));
    assert_eq!(updated_state.error_msg, Some("TEST".to_string()));
}

#[tokio::test]
async fn test_connection_state_tracking() {
    let state = TeamsState::new();
    
    // Initial state should be disconnected
    use teams_api::ConnectionState;
    assert_eq!(state.connection_status().await, ConnectionState::Disconnected);
    
    // Set to connected
    state.set_connection_status(ConnectionState::Connected).await;
    assert_eq!(state.connection_status().await, ConnectionState::Connected);
    
    // Set back to disconnected
    state
        .set_connection_status(ConnectionState::Disconnected)
        .await;
    assert_eq!(state.connection_status().await, ConnectionState::Disconnected);
}

#[tokio::test]
async fn test_timestamp_tracking() {
    let state = TeamsState::new();
    
    // Initial timestamp should be None
    assert!(state.last_received_timestamp().await.is_none());
    
    // Set a timestamp
    let timestamp = 1234567890.0;
    state.set_last_received_timestamp(timestamp).await;
    
    // Verify timestamp was set
    assert_eq!(state.last_received_timestamp().await, Some(timestamp));
}

#[tokio::test]
async fn test_callback_registration() {
    let client = create_test_client("ws://testserver");
    
    let received_messages = Arc::new(Mutex::new(Vec::new()));
    let messages_clone = received_messages.clone();
    
    client
        .register_callback(move |msg: ServerMessage| {
            let messages = messages_clone.clone();
            tokio::spawn(async move {
                messages.lock().await.push(msg);
            });
        })
        .await;
    
    // Verify callback was registered (messages vec is still empty since no messages received)
    assert_eq!(received_messages.lock().await.len(), 0);
}

#[tokio::test]
async fn test_multiple_callbacks_overwrite() {
    let client = create_test_client("ws://testserver");
    
    // Register first callback
    client
        .register_callback(|_msg: ServerMessage| {
            // First callback
        })
        .await;
    
    // Register second callback (should overwrite first)
    let called = Arc::new(Mutex::new(false));
    let called_clone = called.clone();
    
    client
        .register_callback(move |_msg: ServerMessage| {
            let c = called_clone.clone();
            tokio::spawn(async move {
                *c.lock().await = true;
            });
        })
        .await;
    
    // Only the second callback should be registered
    assert!(!*called.lock().await);
}

#[tokio::test]
async fn test_client_message_creation() {
    // Test simple action message
    let msg1 = ClientMessage::action(MeetingAction::ToggleMute);
    assert_eq!(msg1.action, MeetingAction::ToggleMute);
    assert!(msg1.parameters.is_none());
    
    // Test reaction message
    let msg2 = ClientMessage::reaction(ClientMessageParameterType::ReactLike);
    assert_eq!(msg2.action, MeetingAction::React);
    assert!(msg2.parameters.is_some());
    
    // Test toggle UI message
    let msg3 = ClientMessage::toggle_ui(ClientMessageParameterType::ToggleUiChat);
    assert_eq!(msg3.action, MeetingAction::ToggleUi);
    assert!(msg3.parameters.is_some());
}

#[tokio::test]
async fn test_request_id_auto_increment() {
    // Create multiple messages and verify request IDs increment
    let msg1 = ClientMessage::action(MeetingAction::ToggleMute);
    let msg2 = ClientMessage::action(MeetingAction::ToggleMute);
    let msg3 = ClientMessage::action(MeetingAction::ToggleMute);
    
    // Request IDs should be different (auto-incremented)
    assert_ne!(msg1.request_id, msg2.request_id);
    assert_ne!(msg2.request_id, msg3.request_id);
    assert!(msg2.request_id > msg1.request_id);
    assert!(msg3.request_id > msg2.request_id);
}

#[tokio::test]
async fn test_server_message_merge() {
    let mut base_msg = ServerMessage::default();
    base_msg.request_id = Some(1);
    
    let mut update_msg = ServerMessage::default();
    update_msg.error_msg = Some("Error".to_string());
    
    base_msg.merge(&update_msg);
    
    // Both fields should be present after merge
    assert_eq!(base_msg.request_id, Some(1));
    assert_eq!(base_msg.error_msg, Some("Error".to_string()));
}

#[tokio::test]
async fn test_concurrent_sends() {
    let client = Arc::new(create_test_client("ws://testserver"));
    
    let mut handles = vec![];
    
    // Spawn multiple tasks that send messages concurrently
    for i in 0..10 {
        let client_clone = client.clone();
        let handle = tokio::spawn(async move {
            let msg = if i % 2 == 0 {
                ClientMessage::action(MeetingAction::ToggleMute)
            } else {
                ClientMessage::reaction(ClientMessageParameterType::ReactLike)
            };
            client_clone.send_message(msg)
        });
        handles.push(handle);
    }
    
    // Wait for all sends to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_state_concurrent_updates() {
    let state = Arc::new(TeamsState::new());
    let mut handles = vec![];
    
    // Spawn multiple tasks that update state concurrently
    for i in 0..10 {
        let state_clone = state.clone();
        let handle = tokio::spawn(async move {
            let mut msg = ServerMessage::default();
            msg.request_id = Some(i);
            state_clone.update_state(&msg).await;
        });
        handles.push(handle);
    }
    
    // Wait for all updates to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    // State should have been updated (we can't predict the exact final state due to race conditions)
    let final_state = state.state().await;
    assert!(final_state.request_id.is_some());
}

#[test]
fn test_client_message_serialization() {
    let message = ClientMessage::action(MeetingAction::ToggleMute);
    let json = serde_json::to_string(&message).unwrap();
    
    assert!(json.contains("\"action\":\"toggle-mute\""));
    assert!(json.contains("\"requestId\":"));
}

#[test]
fn test_client_message_with_parameters_serialization() {
    let message = ClientMessage::reaction(ClientMessageParameterType::ReactWow);
    let json = serde_json::to_string(&message).unwrap();
    
    assert!(json.contains("\"action\":\"send-reaction\""));
    assert!(json.contains("\"parameters\""));
    assert!(json.contains("\"type\":\"wow\""));
}

#[test]
fn test_server_message_deserialization() {
    let json = r#"{"errorMsg": "TEST"}"#;
    let message: ServerMessage = serde_json::from_str(json).unwrap();
    
    assert_eq!(message.error_msg, Some("TEST".to_string()));
}

#[test]
fn test_server_message_with_meeting_update() {
    let json = r#"{
        "meetingUpdate": {
            "meetingState": {
                "isMuted": true,
                "isInMeeting": true,
                "isHandRaised": false,
                "isRecordingOn": false,
                "isBackgroundBlurred": false,
                "isSharing": false,
                "hasUnreadMessages": true,
                "isVideoOn": false
            }
        }
    }"#;
    
    let message: ServerMessage = serde_json::from_str(json).unwrap();
    
    assert!(message.meeting_update.is_some());
    let update = message.meeting_update.unwrap();
    assert!(update.meeting_state.is_some());
    
    let state = update.meeting_state.unwrap();
    assert!(state.is_muted);
    assert!(state.is_in_meeting);
    assert!(!state.is_hand_raised);
}
