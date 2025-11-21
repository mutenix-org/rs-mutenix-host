// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

use teams_api::{
    ClientMessage, ClientMessageParameterType, MeetingAction, MeetingPermissions, MeetingState,
    MeetingUpdate, ServerMessage, TeamsState,
};

#[test]
fn test_server_message_deserialization() {
    let json = r#"{
        "requestId": 1,
        "response": "success",
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
            },
            "meetingPermissions": {
                "canToggleMute": true,
                "canToggleVideo": true,
                "canToggleHand": true,
                "canToggleBlur": false,
                "canLeave": true,
                "canReact": true,
                "canToggleShareTray": false,
                "canToggleChat": true,
                "canStopSharing": false,
                "canPair": false
            }
        }
    }"#;

    let message: ServerMessage = serde_json::from_str(json).unwrap();

    assert_eq!(message.request_id, Some(1));
    assert_eq!(message.response, Some("success".to_string()));

    let update = message.meeting_update.unwrap();
    let state = update.meeting_state.unwrap();
    assert!(state.is_muted);
    assert!(state.is_in_meeting);
    assert!(!state.is_hand_raised);
    assert!(state.has_unread_messages);

    let permissions = update.meeting_permissions.unwrap();
    assert!(permissions.can_toggle_mute);
    assert!(permissions.can_leave);
    assert!(!permissions.can_toggle_blur);
}

#[test]
fn test_client_message_serialization() {
    let message = ClientMessage::create(MeetingAction::ToggleMute, None);
    let json = serde_json::to_string(&message).unwrap();

    assert!(json.contains("\"action\":\"toggle-mute\""));
    assert!(json.contains("\"requestId\":"));
}

#[test]
fn test_client_message_with_parameters() {
    let message = ClientMessage::reaction(ClientMessageParameterType::ReactLike);
    let json = serde_json::to_string(&message).unwrap();

    assert!(json.contains("\"action\":\"send-reaction\""));
    assert!(json.contains("\"parameters\""));
    assert!(json.contains("\"type\":\"like\""));
}

#[test]
fn test_meeting_action_serialization() {
    let actions = vec![
        (MeetingAction::Mute, "\"mute\""),
        (MeetingAction::Unmute, "\"unmute\""),
        (MeetingAction::ToggleMute, "\"toggle-mute\""),
        (MeetingAction::RaiseHand, "\"raise-hand\""),
        (MeetingAction::LowerHand, "\"lower-hand\""),
        (MeetingAction::LeaveCall, "\"leave-call\""),
        (MeetingAction::QueryMeetingState, "\"query-state\""),
        (MeetingAction::ToggleBackgroundBlur, "\"toggle-background-blur\""),
    ];

    for (action, expected_json) in actions {
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(json, expected_json);
    }
}

#[tokio::test]
async fn test_state_management() {
    let state = TeamsState::new();

    // Initial state
    assert_eq!(
        state.connection_status().await,
        teams_api::ConnectionState::Disconnected
    );

    // Update state
    state
        .set_connection_status(teams_api::ConnectionState::Connected)
        .await;
    assert_eq!(
        state.connection_status().await,
        teams_api::ConnectionState::Connected
    );

    // Test message state updates
    let message = ServerMessage {
        request_id: Some(1),
        response: Some("test".to_string()),
        error_msg: None,
        token_refresh: None,
        meeting_update: Some(MeetingUpdate {
            meeting_state: Some(MeetingState {
                is_muted: true,
                is_in_meeting: true,
                ..Default::default()
            }),
            meeting_permissions: None,
        }),
    };

    state.update_state(&message).await;
    let current_state = state.state().await;

    assert_eq!(current_state.request_id, Some(1));
    assert!(current_state.meeting_update.is_some());

    if let Some(update) = current_state.meeting_update {
        if let Some(meeting_state) = update.meeting_state {
            assert!(meeting_state.is_muted);
            assert!(meeting_state.is_in_meeting);
        }
    }
}

#[test]
fn test_server_message_merge() {
    let mut msg1 = ServerMessage {
        request_id: Some(1),
        response: Some("response1".to_string()),
        error_msg: None,
        token_refresh: None,
        meeting_update: Some(MeetingUpdate {
            meeting_state: Some(MeetingState {
                is_muted: true,
                ..Default::default()
            }),
            meeting_permissions: None,
        }),
    };

    let msg2 = ServerMessage {
        request_id: Some(2),
        response: Some("response2".to_string()),
        error_msg: Some("error".to_string()),
        token_refresh: None,
        meeting_update: Some(MeetingUpdate {
            meeting_state: Some(MeetingState {
                is_hand_raised: true,
                ..Default::default()
            }),
            meeting_permissions: Some(MeetingPermissions {
                can_toggle_mute: true,
                ..Default::default()
            }),
        }),
    };

    msg1.merge(&msg2);

    assert_eq!(msg1.request_id, Some(2));
    assert_eq!(msg1.response, Some("response2".to_string()));
    assert_eq!(msg1.error_msg, Some("error".to_string()));

    if let Some(update) = msg1.meeting_update {
        assert!(update.meeting_state.is_some());
        assert!(update.meeting_permissions.is_some());
    }
}

#[test]
fn test_request_id_counter() {
    let msg1 = ClientMessage::action(MeetingAction::Mute);
    let msg2 = ClientMessage::action(MeetingAction::Unmute);
    let msg3 = ClientMessage::action(MeetingAction::RaiseHand);

    // Request IDs should be unique and incrementing
    assert!(msg2.request_id > msg1.request_id);
    assert!(msg3.request_id > msg2.request_id);
}

#[test]
fn test_default_values() {
    let permissions = MeetingPermissions::default();
    assert!(!permissions.can_toggle_mute);
    assert!(!permissions.can_leave);

    let state = MeetingState::default();
    assert!(!state.is_muted);
    assert!(!state.is_in_meeting);
    assert!(!state.is_hand_raised);
}
