// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

use serde::{Deserialize, Serialize};
use teams_api::{ClientMessageParameterType, MeetingAction};

/// Actions associated with a button
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonAction {
    pub button_id: u8,
    pub actions: Vec<Action>,
}

/// Individual action that can be triggered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook: Option<WebhookAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyboard: Option<Keyboard>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mouse: Option<Mouse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub teams_reaction: Option<TeamsReact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_action: Option<MeetingActionType>,
    #[serde(default)]
    pub activate_teams: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

impl Action {
    pub fn should_activate_teams(&self) -> bool {
        self.activate_teams
    }

    pub fn to_teams_action(&self) -> Option<MeetingAction> {
        self.meeting_action.as_ref().map(|a| a.to_meeting_action())
    }

    pub fn to_teams_reaction(&self) -> Option<ClientMessageParameterType> {
        self.teams_reaction.as_ref().map(|r| r.reaction.to_parameter_type())
    }
}

/// Webhook action configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookAction {
    pub url: String,
    #[serde(default = "default_http_method")]
    pub method: String,
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

fn default_http_method() -> String {
    "GET".to_string()
}

/// Keyboard action configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyboard {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub press: Option<Key>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release: Option<Key>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tap: Option<KeyTap>,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_type: Option<KeyType>,
}

/// Key press/release configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Key {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}

/// Key tap configuration with modifiers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyTap {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modifiers: Option<Vec<String>>,
}

/// Key type configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyType {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub string: Option<String>,
}

/// Mouse action configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mouse {
    #[serde(rename = "move")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mouse_move: Option<MousePosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set: Option<MousePosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub click: Option<MouseButton>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub press: Option<MouseButton>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release: Option<MouseButton>,
}

/// Mouse position configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MousePosition {
    pub x: i32,
    pub y: i32,
}

/// Mouse button configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseButton {
    pub button: String,
}

/// Teams reaction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsReact {
    pub reaction: ReactionType,
}

/// Meeting action types from config
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MeetingActionType {
    None,
    QueryState,
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
    SendReaction,
    ToggleUi,
    StopSharing,
}

impl MeetingActionType {
    pub fn to_meeting_action(&self) -> MeetingAction {
        match self {
            MeetingActionType::None => MeetingAction::NoneAction,
            MeetingActionType::QueryState => MeetingAction::QueryMeetingState,
            MeetingActionType::Mute => MeetingAction::Mute,
            MeetingActionType::Unmute => MeetingAction::Unmute,
            MeetingActionType::ToggleMute => MeetingAction::ToggleMute,
            MeetingActionType::HideVideo => MeetingAction::HideVideo,
            MeetingActionType::ShowVideo => MeetingAction::ShowVideo,
            MeetingActionType::ToggleVideo => MeetingAction::ToggleVideo,
            MeetingActionType::UnblurBackground => MeetingAction::UnblurBackground,
            MeetingActionType::BlurBackground => MeetingAction::BlurBackground,
            MeetingActionType::ToggleBackgroundBlur => MeetingAction::ToggleBackgroundBlur,
            MeetingActionType::LowerHand => MeetingAction::LowerHand,
            MeetingActionType::RaiseHand => MeetingAction::RaiseHand,
            MeetingActionType::ToggleHand => MeetingAction::ToggleHand,
            MeetingActionType::LeaveCall => MeetingAction::LeaveCall,
            MeetingActionType::SendReaction => MeetingAction::React,
            MeetingActionType::ToggleUi => MeetingAction::ToggleUi,
            MeetingActionType::StopSharing => MeetingAction::StopSharing,
        }
    }
}

/// Reaction types from config
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReactionType {
    Applause,
    Laugh,
    Like,
    Love,
    Wow,
    Chat,
    #[serde(rename = "sharing-tray")]
    SharingTray,
}

impl ReactionType {
    pub fn to_parameter_type(&self) -> ClientMessageParameterType {
        match self {
            ReactionType::Like => ClientMessageParameterType::ReactLike,
            ReactionType::Love => ClientMessageParameterType::ReactLove,
            ReactionType::Applause => ClientMessageParameterType::ReactApplause,
            ReactionType::Laugh => ClientMessageParameterType::ReactLaugh,
            ReactionType::Wow => ClientMessageParameterType::ReactWow,
            ReactionType::Chat => ClientMessageParameterType::ToggleUiChat,
            ReactionType::SharingTray => ClientMessageParameterType::ToggleUiSharing,
        }
    }
}
