// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

use crate::teams_messages::ServerMessage;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Connected,
    Disconnected,
}

/// Shared state for the Teams connection
#[derive(Debug, Clone)]
pub struct TeamsState {
    inner: Arc<TeamsStateInner>,
}

#[derive(Debug)]
struct TeamsStateInner {
    connection_status: RwLock<ConnectionState>,
    state: RwLock<ServerMessage>,
    last_received_timestamp: RwLock<Option<f64>>,
}

impl TeamsState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(TeamsStateInner {
                connection_status: RwLock::new(ConnectionState::Disconnected),
                state: RwLock::new(ServerMessage::default()),
                last_received_timestamp: RwLock::new(None),
            }),
        }
    }

    pub async fn connection_status(&self) -> ConnectionState {
        *self.inner.connection_status.read().await
    }

    pub async fn set_connection_status(&self, status: ConnectionState) {
        *self.inner.connection_status.write().await = status;
    }

    pub async fn state(&self) -> ServerMessage {
        self.inner.state.read().await.clone()
    }

    pub async fn update_state(&self, message: &ServerMessage) {
        let mut state = self.inner.state.write().await;
        state.merge(message);
    }

    pub async fn last_received_timestamp(&self) -> Option<f64> {
        *self.inner.last_received_timestamp.read().await
    }

    pub async fn set_last_received_timestamp(&self, timestamp: f64) {
        *self.inner.last_received_timestamp.write().await = Some(timestamp);
    }
}

impl Default for TeamsState {
    fn default() -> Self {
        Self::new()
    }
}
