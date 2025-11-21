// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};

static REQUEST_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MeetingPermissions {
    #[serde(default)]
    pub can_toggle_mute: bool,
    #[serde(default)]
    pub can_toggle_video: bool,
    #[serde(default)]
    pub can_toggle_hand: bool,
    #[serde(default)]
    pub can_toggle_blur: bool,
    #[serde(default)]
    pub can_leave: bool,
    #[serde(default)]
    pub can_react: bool,
    #[serde(default)]
    pub can_toggle_share_tray: bool,
    #[serde(default)]
    pub can_toggle_chat: bool,
    #[serde(default)]
    pub can_stop_sharing: bool,
    #[serde(default)]
    pub can_pair: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MeetingState {
    #[serde(default)]
    pub is_muted: bool,
    #[serde(default)]
    pub is_hand_raised: bool,
    #[serde(default)]
    pub is_in_meeting: bool,
    #[serde(default)]
    pub is_recording_on: bool,
    #[serde(default)]
    pub is_background_blurred: bool,
    #[serde(default)]
    pub is_sharing: bool,
    #[serde(default)]
    pub has_unread_messages: bool,
    #[serde(default)]
    pub is_video_on: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MeetingUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_permissions: Option<MeetingPermissions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_state: Option<MeetingState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServerMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_msg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_refresh: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_update: Option<MeetingUpdate>,
}

impl ServerMessage {
    /// Merge this message with another, keeping non-None values from both
    pub fn merge(&mut self, other: &ServerMessage) {
        if other.request_id.is_some() {
            self.request_id = other.request_id;
        }
        if other.response.is_some() {
            self.response = other.response.clone();
        }
        if other.error_msg.is_some() {
            self.error_msg = other.error_msg.clone();
        }
        if other.token_refresh.is_some() {
            self.token_refresh = other.token_refresh.clone();
        }
        if let Some(ref update) = other.meeting_update {
            if let Some(ref mut self_update) = self.meeting_update {
                if update.meeting_permissions.is_some() {
                    self_update.meeting_permissions = update.meeting_permissions.clone();
                }
                if update.meeting_state.is_some() {
                    self_update.meeting_state = update.meeting_state.clone();
                }
            } else {
                self.meeting_update = other.meeting_update.clone();
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ClientMessageParameterType {
    #[serde(rename = "applause")]
    ReactApplause,
    #[serde(rename = "laugh")]
    ReactLaugh,
    #[serde(rename = "like")]
    ReactLike,
    #[serde(rename = "love")]
    ReactLove,
    #[serde(rename = "wow")]
    ReactWow,
    #[serde(rename = "chat")]
    ToggleUiChat,
    #[serde(rename = "sharing-tray")]
    ToggleUiSharing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientMessageParameter {
    #[serde(rename = "type")]
    pub type_: ClientMessageParameterType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MeetingAction {
    #[serde(rename = "none")]
    NoneAction,
    #[serde(rename = "query-state")]
    QueryMeetingState,
    Mute,
    Unmute,
    ToggleMute,
    HideVideo,
    ShowVideo,
    ToggleVideo,
    UnblurBackground,
    BlurBackground,
    ToggleBackgroundBlur,
    LowerHand,
    RaiseHand,
    ToggleHand,
    LeaveCall,
    #[serde(rename = "send-reaction")]
    React,
    ToggleUi,
    StopSharing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientMessage {
    pub action: MeetingAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<ClientMessageParameter>,
    pub request_id: u32,
}

impl ClientMessage {
    /// Create a new client message with an auto-incremented request ID
    pub fn create(action: MeetingAction, parameters: Option<ClientMessageParameter>) -> Self {
        let request_id = REQUEST_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
        Self {
            action,
            parameters,
            request_id,
        }
    }

    /// Create a simple action message without parameters
    pub fn action(action: MeetingAction) -> Self {
        Self::create(action, None)
    }

    /// Create a reaction message
    pub fn reaction(reaction_type: ClientMessageParameterType) -> Self {
        Self::create(
            MeetingAction::React,
            Some(ClientMessageParameter {
                type_: reaction_type,
            }),
        )
    }

    /// Create a toggle UI message
    pub fn toggle_ui(ui_type: ClientMessageParameterType) -> Self {
        Self::create(
            MeetingAction::ToggleUi,
            Some(ClientMessageParameter { type_: ui_type }),
        )
    }
}
