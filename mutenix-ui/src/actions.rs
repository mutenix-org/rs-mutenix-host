// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

use crate::app::{AppState, LogLevel};
use lib_base::{ActionLogger, execute_button_actions as lib_execute_button_actions, ButtonAction};
use std::sync::Arc;
use teams_api::TeamsWebSocketClient;

// Implement ActionLogger for AppState
impl ActionLogger for AppState {
    async fn log_device(&self, level: lib_base::LogLevel, message: String) {
        let app_level = match level {
            lib_base::LogLevel::Info => LogLevel::Info,
            lib_base::LogLevel::Error => LogLevel::Error,
        };
        self.add_device_log(app_level, message).await;
    }

    async fn log_teams(&self, level: lib_base::LogLevel, message: String) {
        let app_level = match level {
            lib_base::LogLevel::Info => LogLevel::Info,
            lib_base::LogLevel::Error => LogLevel::Error,
        };
        self.add_teams_log(app_level, message).await;
    }
}

/// Execute actions for a button press
pub async fn execute_button_actions(
    action_config: &ButtonAction,
    is_long_press: bool,
    button_id: u8,
    teams_client: Arc<TeamsWebSocketClient>,
    app_state: AppState,
) {
    lib_execute_button_actions(
        action_config,
        is_long_press,
        button_id,
        teams_client,
        Arc::new(app_state),
    )
    .await;
}
