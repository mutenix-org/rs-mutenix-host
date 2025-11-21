// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

use crate::types::{Action, ButtonAction};
use enigo::{
    Button as EnigoButton, Coordinate, Direction, Enigo, Key as EnigoKey, Keyboard,
    Mouse as EnigoMouse, Settings,
};
use std::process::Command;
use std::sync::Arc;
use teams_api::{ClientMessage, TeamsWebSocketClient};

/// Trait for logging from action execution
/// This allows both CLI and UI to provide their own logging implementation
pub trait ActionLogger: Send + Sync {
    fn log_device(&self, level: LogLevel, message: String) -> impl std::future::Future<Output = ()> + Send;
    fn log_teams(&self, level: LogLevel, message: String) -> impl std::future::Future<Output = ()> + Send;
}

/// Log level for actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Error,
}

/// Execute actions for a button press
pub async fn execute_button_actions<L: ActionLogger>(
    action_config: &ButtonAction,
    is_long_press: bool,
    button_id: u8,
    teams_client: Arc<TeamsWebSocketClient>,
    logger: Arc<L>,
) {
    logger
        .log_device(
            LogLevel::Info,
            format!(
                "Executing {} for button {}",
                if is_long_press {
                    "long-press action"
                } else {
                    "action"
                },
                button_id
            ),
        )
        .await;

    for action in &action_config.actions {
        execute_action(action, teams_client.clone(), logger.clone()).await;
    }
}

/// Execute a single action
async fn execute_action<L: ActionLogger>(
    action: &Action,
    teams_client: Arc<TeamsWebSocketClient>,
    logger: Arc<L>,
) {
    // Check if we should activate Teams window
    if action.should_activate_teams() {
        logger
            .log_device(LogLevel::Info, "Activating Teams window".to_string())
            .await;
        // TODO: Implement window activation on macOS
    }

    // Execute webhook action
    if let Some(webhook) = &action.webhook {
        execute_webhook(webhook, logger.clone()).await;
    }

    // Execute keyboard action
    if let Some(keyboard) = &action.keyboard {
        execute_keyboard(keyboard, logger.clone()).await;
    }

    // Execute mouse action
    if let Some(mouse) = &action.mouse {
        execute_mouse(mouse, logger.clone()).await;
    }

    // Execute Teams meeting action
    if let Some(meeting_action) = action.to_teams_action() {
        logger
            .log_teams(
                LogLevel::Info,
                format!("Sending action: {:?}", meeting_action),
            )
            .await;
        if let Err(e) = teams_client.send_message(ClientMessage::action(meeting_action)) {
            logger
                .log_teams(LogLevel::Error, format!("Failed to send action: {}", e))
                .await;
        }
    }

    // Execute Teams reaction
    if let Some(reaction) = action.to_teams_reaction() {
        logger
            .log_teams(
                LogLevel::Info,
                format!("Sending reaction: {:?}", reaction),
            )
            .await;
        if let Err(e) = teams_client.send_message(ClientMessage::reaction(reaction)) {
            logger
                .log_teams(LogLevel::Error, format!("Failed to send reaction: {}", e))
                .await;
        }
    }

    // Execute command
    if let Some(command) = &action.command {
        execute_command(command, logger.clone()).await;
    }
}

/// Execute a webhook action
async fn execute_webhook<L: ActionLogger>(webhook: &crate::types::WebhookAction, logger: Arc<L>) {
    logger
        .log_device(
            LogLevel::Info,
            format!("Executing webhook: {} {}", webhook.method, webhook.url),
        )
        .await;

    let client = reqwest::Client::new();
    let mut request = match webhook.method.to_uppercase().as_str() {
        "GET" => client.get(&webhook.url),
        "POST" => client.post(&webhook.url),
        "PUT" => client.put(&webhook.url),
        "DELETE" => client.delete(&webhook.url),
        "PATCH" => client.patch(&webhook.url),
        _ => {
            logger
                .log_device(
                    LogLevel::Error,
                    format!("Unsupported HTTP method: {}", webhook.method),
                )
                .await;
            return;
        }
    };

    // Add headers
    for (key, value) in &webhook.headers {
        request = request.header(key, value);
    }

    // Add body data if present
    if let Some(data) = &webhook.data {
        request = request.json(data);
    }

    match request.send().await {
        Ok(response) => {
            logger
                .log_device(
                    LogLevel::Info,
                    format!("Webhook response: {}", response.status()),
                )
                .await;
        }
        Err(e) => {
            logger
                .log_device(LogLevel::Error, format!("Webhook failed: {}", e))
                .await;
        }
    }
}

/// Execute a keyboard action
async fn execute_keyboard<L: ActionLogger>(keyboard: &crate::types::Keyboard, logger: Arc<L>) {
    let mut enigo = match Enigo::new(&Settings::default()) {
        Ok(e) => e,
        Err(e) => {
            logger
                .log_device(
                    LogLevel::Error,
                    format!("Failed to initialize keyboard control: {}", e),
                )
                .await;
            return;
        }
    };

    // Handle key press
    if let Some(press) = &keyboard.press {
        if let Some(key_str) = &press.key {
            logger
                .log_device(LogLevel::Info, format!("Pressing key: {}", key_str))
                .await;
            if let Some(key) = parse_key(key_str) {
                let _ = enigo.key(key, Direction::Press);
            } else {
                logger
                    .log_device(LogLevel::Error, format!("Unknown key: {}", key_str))
                    .await;
            }
        }
    }

    // Handle key release
    if let Some(release) = &keyboard.release {
        if let Some(key_str) = &release.key {
            logger
                .log_device(LogLevel::Info, format!("Releasing key: {}", key_str))
                .await;
            if let Some(key) = parse_key(key_str) {
                let _ = enigo.key(key, Direction::Release);
            } else {
                logger
                    .log_device(LogLevel::Error, format!("Unknown key: {}", key_str))
                    .await;
            }
        }
    }

    // Handle key tap
    if let Some(tap) = &keyboard.tap {
        if let Some(key_str) = &tap.key {
            logger
                .log_device(LogLevel::Info, format!("Tapping key: {}", key_str))
                .await;

            // Press modifiers
            if let Some(modifiers) = &tap.modifiers {
                for modifier in modifiers {
                    if let Some(key) = parse_key(modifier) {
                        let _ = enigo.key(key, Direction::Press);
                    }
                }
            }

            // Tap the main key
            if let Some(key) = parse_key(key_str) {
                let _ = enigo.key(key, Direction::Click);
            } else {
                logger
                    .log_device(LogLevel::Error, format!("Unknown key: {}", key_str))
                    .await;
            }

            // Release modifiers
            if let Some(modifiers) = &tap.modifiers {
                for modifier in modifiers.iter().rev() {
                    if let Some(key) = parse_key(modifier) {
                        let _ = enigo.key(key, Direction::Release);
                    }
                }
            }
        }
    }

    // Handle key type
    if let Some(key_type) = &keyboard.key_type {
        if let Some(string) = &key_type.string {
            logger
                .log_device(LogLevel::Info, format!("Typing string: {}", string))
                .await;
            let _ = enigo.text(string);
        }
    }
}

/// Execute a mouse action
async fn execute_mouse<L: ActionLogger>(mouse: &crate::types::Mouse, logger: Arc<L>) {
    let mut enigo = match Enigo::new(&Settings::default()) {
        Ok(e) => e,
        Err(e) => {
            logger
                .log_device(
                    LogLevel::Error,
                    format!("Failed to initialize mouse control: {}", e),
                )
                .await;
            return;
        }
    };

    // Handle mouse move (relative)
    if let Some(mov) = &mouse.mouse_move {
        logger
            .log_device(
                LogLevel::Info,
                format!("Moving mouse by: ({}, {})", mov.x, mov.y),
            )
            .await;
        let _ = enigo.move_mouse(mov.x, mov.y, Coordinate::Rel);
    }

    // Handle mouse set (absolute)
    if let Some(set) = &mouse.set {
        logger
            .log_device(
                LogLevel::Info,
                format!("Setting mouse position to: ({}, {})", set.x, set.y),
            )
            .await;
        let _ = enigo.move_mouse(set.x, set.y, Coordinate::Abs);
    }

    // Handle mouse click
    if let Some(click) = &mouse.click {
        if let Some(button) = parse_mouse_button(&click.button) {
            logger
                .log_device(
                    LogLevel::Info,
                    format!("Clicking mouse button: {}", click.button),
                )
                .await;
            let _ = enigo.button(button, Direction::Click);
        } else {
            logger
                .log_device(
                    LogLevel::Error,
                    format!("Unknown mouse button: {}", click.button),
                )
                .await;
        }
    }

    // Handle mouse press
    if let Some(press) = &mouse.press {
        if let Some(button) = parse_mouse_button(&press.button) {
            logger
                .log_device(
                    LogLevel::Info,
                    format!("Pressing mouse button: {}", press.button),
                )
                .await;
            let _ = enigo.button(button, Direction::Press);
        } else {
            logger
                .log_device(
                    LogLevel::Error,
                    format!("Unknown mouse button: {}", press.button),
                )
                .await;
        }
    }

    // Handle mouse release
    if let Some(release) = &mouse.release {
        if let Some(button) = parse_mouse_button(&release.button) {
            logger
                .log_device(
                    LogLevel::Info,
                    format!("Releasing mouse button: {}", release.button),
                )
                .await;
            let _ = enigo.button(button, Direction::Release);
        } else {
            logger
                .log_device(
                    LogLevel::Error,
                    format!("Unknown mouse button: {}", release.button),
                )
                .await;
        }
    }
}

/// Execute a shell command
async fn execute_command<L: ActionLogger>(command: &str, logger: Arc<L>) {
    logger
        .log_device(LogLevel::Info, format!("Executing command: {}", command))
        .await;

    let output = Command::new("sh").arg("-c").arg(command).output();

    match output {
        Ok(output) => {
            if output.status.success() {
                logger
                    .log_device(
                        LogLevel::Info,
                        format!(
                            "Command succeeded: {}",
                            String::from_utf8_lossy(&output.stdout).trim()
                        ),
                    )
                    .await;
            } else {
                logger
                    .log_device(
                        LogLevel::Error,
                        format!(
                            "Command failed with exit code {}: {}",
                            output.status.code().unwrap_or(-1),
                            String::from_utf8_lossy(&output.stderr).trim()
                        ),
                    )
                    .await;
            }
        }
        Err(e) => {
            logger
                .log_device(
                    LogLevel::Error,
                    format!("Failed to execute command: {}", e),
                )
                .await;
        }
    }
}

/// Parse a key string to Enigo key
fn parse_key(key_str: &str) -> Option<EnigoKey> {
    match key_str.to_lowercase().as_str() {
        "return" | "enter" => Some(EnigoKey::Return),
        "tab" => Some(EnigoKey::Tab),
        "space" => Some(EnigoKey::Space),
        "backspace" => Some(EnigoKey::Backspace),
        "escape" | "esc" => Some(EnigoKey::Escape),
        "delete" => Some(EnigoKey::Delete),
        "home" => Some(EnigoKey::Home),
        "end" => Some(EnigoKey::End),
        "pageup" => Some(EnigoKey::PageUp),
        "pagedown" => Some(EnigoKey::PageDown),
        "leftarrow" | "left" => Some(EnigoKey::LeftArrow),
        "rightarrow" | "right" => Some(EnigoKey::RightArrow),
        "uparrow" | "up" => Some(EnigoKey::UpArrow),
        "downarrow" | "down" => Some(EnigoKey::DownArrow),
        "f1" => Some(EnigoKey::F1),
        "f2" => Some(EnigoKey::F2),
        "f3" => Some(EnigoKey::F3),
        "f4" => Some(EnigoKey::F4),
        "f5" => Some(EnigoKey::F5),
        "f6" => Some(EnigoKey::F6),
        "f7" => Some(EnigoKey::F7),
        "f8" => Some(EnigoKey::F8),
        "f9" => Some(EnigoKey::F9),
        "f10" => Some(EnigoKey::F10),
        "f11" => Some(EnigoKey::F11),
        "f12" => Some(EnigoKey::F12),
        "shift" => Some(EnigoKey::Shift),
        "control" | "ctrl" => Some(EnigoKey::Control),
        "alt" | "option" => Some(EnigoKey::Alt),
        "meta" | "command" | "cmd" | "super" => Some(EnigoKey::Meta),
        "capslock" => Some(EnigoKey::CapsLock),
        _ => {
            // Try to parse as a single character
            if key_str.len() == 1 {
                Some(EnigoKey::Unicode(key_str.chars().next().unwrap()))
            } else {
                None
            }
        }
    }
}

/// Parse a mouse button string to Enigo button
fn parse_mouse_button(button_str: &str) -> Option<EnigoButton> {
    match button_str.to_lowercase().as_str() {
        "left" => Some(EnigoButton::Left),
        "right" => Some(EnigoButton::Right),
        "middle" => Some(EnigoButton::Middle),
        _ => None,
    }
}
