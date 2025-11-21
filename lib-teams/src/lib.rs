// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

pub mod state;
pub mod teams_messages;
pub mod websocket_client;

// Re-export commonly used types
pub use state::{ConnectionState, TeamsState};
pub use teams_messages::{
    ClientMessage, ClientMessageParameter, ClientMessageParameterType, MeetingAction,
    MeetingPermissions, MeetingState, MeetingUpdate, ServerMessage,
};
pub use websocket_client::{Identifier, TeamsWebSocketClient, WebSocketError};

