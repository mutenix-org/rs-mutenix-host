// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

mod app;

use anyhow::{Context, Result};
use app::{AppState, DeviceStatus, LogLevel, TeamsStatus};
use lib_base::{Config, TeamsStateType, execute_button_actions};
use mutenix_hid::{
    ConnectionState as DeviceConnectionState, DeviceMessage, HidDevice, LedColor, SetLed,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
use tauri::Manager;
use teams_api::{
    ClientMessage, ConnectionState as TeamsConnectionState, Identifier, ServerMessage,
    TeamsState, TeamsWebSocketClient,
};
use tokio::fs;
use tokio::sync::RwLock;

const DEFAULT_CONFIG_PATH: &str = "mutenix.yaml";
const TOKEN_FILE: &str = ".mutenix_token";
const TEAMS_WS_URI: &str = "ws://localhost:8124";

#[derive(Clone, serde::Serialize)]
struct StatusPayload {
    device: DeviceStatus,
    teams: TeamsStatus,
}

struct MutenixUi {
    config: Arc<RwLock<Config>>,
    device: Arc<HidDevice>,
    teams_client: Arc<TeamsWebSocketClient>,
    teams_state: TeamsState,
    token_file: PathBuf,
    saved_token: Arc<RwLock<String>>,
    button_press_times: Arc<RwLock<HashMap<u8, std::time::Instant>>>,
    app_state: AppState,
}

impl MutenixUi {
    async fn new(config_path: PathBuf, teams_uri: String, token_file: PathBuf) -> Result<Self> {
        // Load configuration
        let config_data = Config::from_file(&config_path)
            .with_context(|| format!("Failed to load config from {:?}", config_path))?;
        let config = Arc::new(RwLock::new(config_data));

        // Create app state
        let app_state = AppState::new(env!("CARGO_PKG_VERSION").to_string());

        // Load saved token
        let saved_token = Arc::new(RwLock::new(load_token(&token_file).await));

        // Create HID device
        let device_info = config.read().await.get_device_info();
        let device = Arc::new(HidDevice::new(device_info));

        // Create Teams state and client
        let teams_state = TeamsState::new();
        let identifier =
            Identifier::new("Mutenix", "UI", "mutenix-ui", env!("CARGO_PKG_VERSION"))
                .with_token(saved_token.read().await.clone());

        let teams_client = Arc::new(TeamsWebSocketClient::new(
            teams_state.clone(),
            teams_uri,
            identifier,
        ));

        Ok(Self {
            config,
            device,
            teams_client,
            teams_state,
            token_file,
            saved_token,
            button_press_times: Arc::new(RwLock::new(HashMap::new())),
            app_state,
        })
    }

    async fn run(&self) -> Result<()> {
        self.app_state
            .add_device_log(LogLevel::Info, "Starting Mutenix UI")
            .await;

        // Setup device callbacks
        self.setup_device_callbacks().await;

        // Setup Teams callbacks
        self.setup_teams_callbacks().await;

        // Start device processing
        let device = self.device.clone();
        let app_state = self.app_state.clone();
        tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(async {
                if let Err(e) = device.process().await {
                    app_state
                        .add_device_log(LogLevel::Error, format!("Device error: {}", e))
                        .await;
                }
            })
        });

        // Start device status monitor
        self.start_device_status_monitor();

        // Start Teams status monitor
        self.start_teams_status_monitor();

        // Start Teams client
        let teams_client = self.teams_client.clone();
        tokio::spawn(async move {
            if let Err(e) = teams_client.process().await {
                eprintln!("[Teams] Client error: {}", e);
            }
        });

        // Wait a bit for Teams to connect
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // Query initial state
        self.app_state
            .add_teams_log(LogLevel::Info, "Querying initial Teams state")
            .await;
        if let Err(e) = self.teams_client.send_message(ClientMessage::action(
            teams_api::MeetingAction::QueryMeetingState,
        )) {
            self.app_state
                .add_teams_log(LogLevel::Error, format!("Failed to query state: {}", e))
                .await;
        }

        // Start LED update task
        self.start_led_update_task();

        Ok(())
    }

    async fn setup_device_callbacks(&self) {
        let config = self.config.clone();
        let teams_client = self.teams_client.clone();
        let button_times = self.button_press_times.clone();
        let app_state = self.app_state.clone();

        self.device
            .register_callback(move |message| {
                if let DeviceMessage::Status(status) = message {
                    let button_id = status.button();
                    let is_pressed = status.pressed();

                    let config = config.clone();
                    let teams_client = teams_client.clone();
                    let button_times = button_times.clone();
                    let app_state = app_state.clone();

                    tokio::spawn(async move {
                        app_state
                            .add_device_log(
                                LogLevel::Info,
                                format!(
                                    "Button {} {}",
                                    button_id,
                                    if is_pressed { "pressed" } else { "released" }
                                ),
                            )
                            .await;

                        if is_pressed {
                            button_times
                                .write()
                                .await
                                .insert(button_id, std::time::Instant::now());
                        } else {
                            let press_duration =
                                if let Some(press_time) = button_times.write().await.remove(&button_id)
                                {
                                    press_time.elapsed()
                                } else {
                                    std::time::Duration::from_millis(0)
                                };

                            let is_long_press = press_duration.as_millis() > 500;

                            let config_guard = config.read().await;
                            let button_action = if is_long_press {
                                config_guard.find_longpress_action(button_id).cloned()
                            } else {
                                config_guard.find_button_action(button_id).cloned()
                            };
                            drop(config_guard);

                            if let Some(action_config) = button_action {
                                execute_button_actions(
                                    &action_config,
                                    is_long_press,
                                    button_id,
                                    teams_client.clone(),
                                    Arc::new(app_state.clone()),
                                )
                                .await;
                            } else {
                                app_state
                                    .add_device_log(
                                        LogLevel::Warn,
                                        format!("No action configured for button {}", button_id),
                                    )
                                    .await;
                            }
                        }
                    });
                }
            })
            .await;
    }

    async fn setup_teams_callbacks(&self) {
        let saved_token = self.saved_token.clone();
        let token_file = self.token_file.clone();
        let app_state = self.app_state.clone();

        self.teams_client
            .register_callback(move |message: ServerMessage| {
                let app_state = app_state.clone();

                // Handle token refresh
                if let Some(new_token) = message.token_refresh.clone() {
                    let saved_token = saved_token.clone();
                    let token_file = token_file.clone();
                    let app_state_clone = app_state.clone();

                    tokio::spawn(async move {
                        app_state_clone
                            .add_teams_log(LogLevel::Info, "Received new authentication token")
                            .await;
                        *saved_token.write().await = new_token.clone();
                        if let Err(e) = save_token(&token_file, &new_token).await {
                            app_state_clone
                                .add_teams_log(
                                    LogLevel::Error,
                                    format!("Failed to save token: {}", e),
                                )
                                .await;
                        } else {
                            app_state_clone
                                .add_teams_log(LogLevel::Info, "Token saved successfully")
                                .await;
                        }
                    });
                }

                // Log meeting state changes
                if let Some(meeting_update) = message.meeting_update.clone() {
                    let app_state_clone = app_state.clone();
                    tokio::spawn(async move {
                        app_state_clone
                            .add_teams_log(
                                LogLevel::Info,
                                format!("Meeting state updated: {:?}", meeting_update.meeting_state),
                            )
                            .await;
                    });
                }
            })
            .await;
    }

    fn start_device_status_monitor(&self) {
        let device = self.device.clone();
        let app_state = self.app_state.clone();

        tokio::spawn(async move {
            loop {
                let hw_state = device.state().await;
                let is_connected = hw_state.connection_status == DeviceConnectionState::Connected;

                app_state
                    .update_device_status(|status| {
                        status.connected = is_connected;
                        if is_connected {
                            status.manufacturer = hw_state.manufacturer.clone();
                            status.product = hw_state.product.clone();
                            status.serial_number = hw_state.serial_number.clone();
                        } else {
                            status.manufacturer = None;
                            status.product = None;
                            status.serial_number = None;
                        }
                    })
                    .await;

                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        });
    }

    fn start_teams_status_monitor(&self) {
        let teams_state = self.teams_state.clone();
        let app_state = self.app_state.clone();

        tokio::spawn(async move {
            loop {
                let connection_status = teams_state.connection_status().await;
                let state = teams_state.state().await;
                let meeting_state = state.meeting_update.and_then(|u| u.meeting_state).unwrap_or_default();

                app_state
                    .update_teams_status(|status| {
                        status.connected = connection_status == TeamsConnectionState::Connected;
                        status.in_meeting = meeting_state.is_in_meeting;
                        status.is_muted = meeting_state.is_muted;
                        status.is_video_on = meeting_state.is_video_on;
                        status.is_hand_raised = meeting_state.is_hand_raised;
                        status.is_recording = meeting_state.is_recording_on;
                    })
                    .await;

                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        });
    }

    fn start_led_update_task(&self) {
        let config = self.config.clone();
        let device = self.device.clone();
        let teams_state = self.teams_state.clone();

        tokio::spawn(async move {
            let mut first_run = true;
            loop {
                let state = teams_state.state().await;
                let meeting_state = state.meeting_update.and_then(|u| u.meeting_state).unwrap_or_default();

                if first_run {
                    first_run = false;
                }

                let config_guard = config.read().await;
                for led_status in &config_guard.led_status {
                    if let Some(teams_config) = &led_status.teams_state {
                        let is_active = match teams_config.teams_state {
                            TeamsStateType::IsMuted => meeting_state.is_muted,
                            TeamsStateType::IsVideoOn => meeting_state.is_video_on,
                            TeamsStateType::IsHandRaised => meeting_state.is_hand_raised,
                            TeamsStateType::IsInMeeting => meeting_state.is_in_meeting,
                            TeamsStateType::IsRecordingOn => meeting_state.is_recording_on,
                            TeamsStateType::IsBackgroundBlurred => {
                                meeting_state.is_background_blurred
                            }
                            TeamsStateType::IsSharing => meeting_state.is_sharing,
                            TeamsStateType::HasUnreadMessages => {
                                meeting_state.has_unread_messages
                            }
                        };

                        let color = if is_active {
                            teams_config
                                .color_on
                                .as_ref()
                                .map(|c| c.to_led_color())
                                .unwrap_or(LedColor::Green)
                        } else {
                            teams_config
                                .color_off
                                .as_ref()
                                .map(|c| c.to_led_color())
                                .unwrap_or(LedColor::Black)
                        };

                        let set_led_cmd = SetLed::new(led_status.button_id, color);
                        match device.send_command(set_led_cmd).await {
                            Ok(_) => {},
                            Err(e) => {
                                eprintln!("[LED] Failed to set LED {}: {}", led_status.button_id, e);
                            }
                        }
                    }
                }
                drop(config_guard);

                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        });
    }
}

async fn load_token(token_file: &PathBuf) -> String {
    fs::read_to_string(token_file)
        .await
        .unwrap_or_default()
        .trim()
        .to_string()
}

async fn save_token(token_file: &PathBuf, token: &str) -> Result<()> {
    fs::write(token_file, token)
        .await
        .with_context(|| format!("Failed to write token to {:?}", token_file))
}

// Shared state for config
struct ConfigState {
    config: Arc<RwLock<Config>>,
    config_path: PathBuf,
}

// Tauri commands
#[tauri::command]
async fn get_status(state: tauri::State<'_, Arc<AppState>>) -> Result<StatusPayload, String> {
    let payload = StatusPayload {
        device: state.get_device_status().await,
        teams: state.get_teams_status().await,
    };
    Ok(payload)
}

#[tauri::command]
async fn get_device_logs(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<app::LogEntry>, String> {
    let logs = state.get_device_logs().await;
    Ok(logs)
}

#[tauri::command]
async fn get_teams_logs(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<app::LogEntry>, String> {
    let logs = state.get_teams_logs().await;
    Ok(logs)
}

#[tauri::command]
async fn get_config(
    config_state: tauri::State<'_, Arc<ConfigState>>,
) -> Result<Config, String> {
    let config = config_state.config.read().await.clone();
    Ok(config)
}

#[tauri::command]
async fn save_config(
    config_state: tauri::State<'_, Arc<ConfigState>>,
    app_state: tauri::State<'_, Arc<AppState>>,
    config: Config,
) -> Result<(), String> {
    // Update in-memory config
    *config_state.config.write().await = config.clone();
    
    // Save to file
    let yaml = serde_yaml::to_string(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    
    tokio::fs::write(&config_state.config_path, yaml)
        .await
        .map_err(|e| format!("Failed to write config file: {}", e))?;
    
    // Log successful config update
    app_state
        .add_device_log(LogLevel::Info, "Configuration updated and applied")
        .await;
    
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Create tray menu
            let show_item = MenuItem::with_id(app, "show", "Show Mutenix", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            // Setup tray icon with menu
            let tray_handle = app.handle().clone();
            if let Some(tray) = app.tray_by_id("main") {
                let _ = tray.set_menu(Some(menu));
                
                tray.on_menu_event(move |_app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = _app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            std::process::exit(0);
                        }
                        _ => {}
                    }
                });

                tray.on_tray_icon_event(move |_tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        if let Some(window) = tray_handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                });
            }

            // Setup window close handler to hide instead of exit
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_clone.hide();
                    }
                });
            }

            // Initialize the backend
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let config_path = PathBuf::from(DEFAULT_CONFIG_PATH);
                let token_file = PathBuf::from(TOKEN_FILE);
                let teams_uri = TEAMS_WS_URI.to_string();

                match MutenixUi::new(config_path.clone(), teams_uri, token_file).await {
                    Ok(ui) => {
                        
                        // Register app state with Tauri BEFORE running
                        let app_state = ui.app_state.clone();
                        handle.manage(Arc::new(app_state));
                        
                        // Register config state
                        let config_state = Arc::new(ConfigState {
                            config: ui.config.clone(),
                            config_path: config_path.clone(),
                        });
                        handle.manage(config_state);

                        if let Err(e) = ui.run().await {
                            eprintln!("[Main] Error running Mutenix UI: {}", e);
                        }

                        // Keep the app running
                        loop {
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        }
                    }
                    Err(e) => {
                        eprintln!("[Main] Error initializing Mutenix UI: {}", e);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_device_logs,
            get_teams_logs,
            get_config,
            save_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(not(mobile))]
fn main() {
    run();
}
