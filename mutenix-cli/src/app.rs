// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

use chrono::Local;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

const MAX_LOG_ENTRIES: usize = 100;

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    #[allow(dead_code)]
    Debug,
}

impl LogLevel {
    pub fn as_str(&self) -> &str {
        match self {
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Debug => "DEBUG",
        }
    }
}

impl LogEntry {
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        Self {
            timestamp: Local::now().format("%H:%M:%S").to_string(),
            level,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceStatus {
    pub connected: bool,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial_number: Option<String>,
}

impl Default for DeviceStatus {
    fn default() -> Self {
        Self {
            connected: false,
            manufacturer: None,
            product: None,
            serial_number: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TeamsStatus {
    pub connected: bool,
    pub in_meeting: bool,
    pub is_muted: bool,
    pub is_video_on: bool,
    pub is_hand_raised: bool,
    pub is_recording: bool,
}

impl Default for TeamsStatus {
    fn default() -> Self {
        Self {
            connected: false,
            in_meeting: false,
            is_muted: false,
            is_video_on: false,
            is_hand_raised: false,
            is_recording: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppState {
    device_status: Arc<RwLock<DeviceStatus>>,
    teams_status: Arc<RwLock<TeamsStatus>>,
    device_logs: Arc<RwLock<VecDeque<LogEntry>>>,
    teams_logs: Arc<RwLock<VecDeque<LogEntry>>>,
    version: String,
}

impl AppState {
    pub fn new(version: String) -> Self {
        Self {
            device_status: Arc::new(RwLock::new(DeviceStatus::default())),
            teams_status: Arc::new(RwLock::new(TeamsStatus::default())),
            device_logs: Arc::new(RwLock::new(VecDeque::new())),
            teams_logs: Arc::new(RwLock::new(VecDeque::new())),
            version,
        }
    }

    pub async fn get_device_status(&self) -> DeviceStatus {
        self.device_status.read().await.clone()
    }

    pub async fn update_device_status<F>(&self, f: F)
    where
        F: FnOnce(&mut DeviceStatus),
    {
        let mut status = self.device_status.write().await;
        f(&mut status);
    }

    pub async fn get_teams_status(&self) -> TeamsStatus {
        self.teams_status.read().await.clone()
    }

    pub async fn update_teams_status<F>(&self, f: F)
    where
        F: FnOnce(&mut TeamsStatus),
    {
        let mut status = self.teams_status.write().await;
        f(&mut status);
    }

    pub async fn add_device_log(&self, level: LogLevel, message: impl Into<String>) {
        let mut logs = self.device_logs.write().await;
        logs.push_back(LogEntry::new(level, message));
        if logs.len() > MAX_LOG_ENTRIES {
            logs.pop_front();
        }
    }

    pub async fn add_teams_log(&self, level: LogLevel, message: impl Into<String>) {
        let mut logs = self.teams_logs.write().await;
        logs.push_back(LogEntry::new(level, message));
        if logs.len() > MAX_LOG_ENTRIES {
            logs.pop_front();
        }
    }

    pub async fn get_device_logs(&self) -> Vec<LogEntry> {
        self.device_logs.read().await.iter().cloned().collect()
    }

    pub async fn get_teams_logs(&self) -> Vec<LogEntry> {
        self.teams_logs.read().await.iter().cloned().collect()
    }

    pub fn version(&self) -> &str {
        &self.version
    }
}
