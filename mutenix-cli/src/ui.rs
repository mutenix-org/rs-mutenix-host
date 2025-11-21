// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

use crate::app::{AppState, LogLevel};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::time::Duration;

pub struct Ui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl Ui {
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self { terminal })
    }

    pub fn cleanup(&mut self) -> io::Result<()> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    pub async fn run(&mut self, state: AppState) -> io::Result<()> {
        loop {
            self.draw(&state).await?;

            // Check for key events with timeout
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            return Ok(());
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    async fn draw(&mut self, state: &AppState) -> io::Result<()> {
        let device_status = state.get_device_status().await;
        let teams_status = state.get_teams_status().await;
        let device_logs = state.get_device_logs().await;
        let teams_logs = state.get_teams_logs().await;
        let version = state.version().to_string();

        self.terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(8),  // Status section
                    Constraint::Min(10),    // Logs section
                    Constraint::Length(3),  // Version footer
                ])
                .split(f.area());

            // Status section (top)
            render_status_section(f, chunks[0], &device_status, &teams_status);

            // Logs section (middle) - split into device and teams
            let log_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[1]);

            render_device_logs(f, log_chunks[0], &device_logs);
            render_teams_logs(f, log_chunks[1], &teams_logs);

            // Version footer (bottom)
            render_footer(f, chunks[2], &version);
        })?;

        Ok(())
    }
}

fn render_status_section(
    f: &mut Frame,
    area: Rect,
    device_status: &crate::app::DeviceStatus,
    teams_status: &crate::app::TeamsStatus,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Device status
    let device_color = if device_status.connected {
        Color::Green
    } else {
        Color::Red
    };

    let device_info = vec![
        Line::from(vec![
            Span::raw("Status: "),
            Span::styled(
                if device_status.connected {
                    "Connected"
                } else {
                    "Disconnected"
                },
                Style::default().fg(device_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(format!(
            "Product: {}",
            device_status.product.as_deref().unwrap_or("N/A")
        )),
        Line::from(format!(
            "Manufacturer: {}",
            device_status.manufacturer.as_deref().unwrap_or("N/A")
        )),
        Line::from(format!(
            "Serial: {}",
            device_status.serial_number.as_deref().unwrap_or("N/A")
        )),
    ];

    let device_block = Block::default()
        .title(" Device Status ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let device_paragraph = Paragraph::new(device_info)
        .block(device_block)
        .wrap(Wrap { trim: true });

    f.render_widget(device_paragraph, chunks[0]);

    // Teams status
    let teams_color = if teams_status.connected {
        Color::Green
    } else {
        Color::Red
    };

    let teams_info = vec![
        Line::from(vec![
            Span::raw("Status: "),
            Span::styled(
                if teams_status.connected {
                    "Connected"
                } else {
                    "Disconnected"
                },
                Style::default().fg(teams_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("In Meeting: "),
            Span::styled(
                if teams_status.in_meeting { "Yes" } else { "No" },
                Style::default().fg(if teams_status.in_meeting {
                    Color::Green
                } else {
                    Color::Gray
                }),
            ),
        ]),
        Line::from(vec![
            Span::raw("Muted: "),
            Span::styled(
                if teams_status.is_muted { "Yes" } else { "No" },
                Style::default().fg(if teams_status.is_muted {
                    Color::Red
                } else {
                    Color::Green
                }),
            ),
            Span::raw(" | Video: "),
            Span::styled(
                if teams_status.is_video_on { "On" } else { "Off" },
                Style::default().fg(if teams_status.is_video_on {
                    Color::Green
                } else {
                    Color::Gray
                }),
            ),
        ]),
        Line::from(vec![
            Span::raw("Hand Raised: "),
            Span::styled(
                if teams_status.is_hand_raised { "Yes" } else { "No" },
                Style::default().fg(if teams_status.is_hand_raised {
                    Color::Yellow
                } else {
                    Color::Gray
                }),
            ),
            Span::raw(" | Recording: "),
            Span::styled(
                if teams_status.is_recording { "Yes" } else { "No" },
                Style::default().fg(if teams_status.is_recording {
                    Color::Red
                } else {
                    Color::Gray
                }),
            ),
        ]),
    ];

    let teams_block = Block::default()
        .title(" Teams Status ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let teams_paragraph = Paragraph::new(teams_info)
        .block(teams_block)
        .wrap(Wrap { trim: true });

    f.render_widget(teams_paragraph, chunks[1]);
}

fn render_device_logs(f: &mut Frame, area: Rect, logs: &[crate::app::LogEntry]) {
    let items: Vec<ListItem> = logs
        .iter()
        .rev()
        .take(area.height as usize - 2)
        .rev()
        .map(|entry| {
            let level_color = match entry.level {
                LogLevel::Info => Color::White,
                LogLevel::Warn => Color::Yellow,
                LogLevel::Error => Color::Red,
                LogLevel::Debug => Color::Gray,
            };

            let content = Line::from(vec![
                Span::styled(
                    format!("[{}] ", entry.timestamp),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{:5} ", entry.level.as_str()),
                    Style::default().fg(level_color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(&entry.message),
            ]);

            ListItem::new(content)
        })
        .collect();

    let block = Block::default()
        .title(" Device / HID Messages ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let list = List::new(items).block(block);

    f.render_widget(list, area);
}

fn render_teams_logs(f: &mut Frame, area: Rect, logs: &[crate::app::LogEntry]) {
    let items: Vec<ListItem> = logs
        .iter()
        .rev()
        .take(area.height as usize - 2)
        .rev()
        .map(|entry| {
            let level_color = match entry.level {
                LogLevel::Info => Color::White,
                LogLevel::Warn => Color::Yellow,
                LogLevel::Error => Color::Red,
                LogLevel::Debug => Color::Gray,
            };

            let content = Line::from(vec![
                Span::styled(
                    format!("[{}] ", entry.timestamp),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{:5} ", entry.level.as_str()),
                    Style::default().fg(level_color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(&entry.message),
            ]);

            ListItem::new(content)
        })
        .collect();

    let block = Block::default()
        .title(" Teams Messages ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let list = List::new(items).block(block);

    f.render_widget(list, area);
}

fn render_footer(f: &mut Frame, area: Rect, version: &str) {
    let info = vec![Line::from(vec![
        Span::styled(
            format!("Mutenix CLI v{}", version),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::styled(
            "Press 'q' or ESC to quit",
            Style::default().fg(Color::Yellow),
        ),
    ])];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(info).block(block);

    f.render_widget(paragraph, area);
}
