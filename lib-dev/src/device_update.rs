// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

use crate::chunks::{Chunk, Completed, FileChunk, FileDelete, FileEnd, FileStart};
use crate::constants::{
    HID_COMMAND_PREPARE_UPDATE, HID_COMMAND_RESET, HID_REPORT_ID_COMMUNICATION,
    HID_REPORT_ID_TRANSFER, MAX_CHUNK_SIZE, STATE_CHANGE_SLEEP_TIME,
};
use crate::device_messages::{parse_hid_update_message, ChunkAck, HidUpdateMessage};
use hidapi::HidDevice;
use log::{debug, error, info, warn};
use std::path::Path;
use std::time::Duration;
use tokio::time::sleep;

/// Errors during device update
#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("Device not connected")]
    NotConnected,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Device error: {0}")]
    DeviceError(String),

    #[error("Write failed: {0}")]
    WriteFailed(String),

    #[error("File error: {0}")]
    FileError(String),
}

/// Represents a file to be transferred to the device
pub struct TransferFile {
    pub id: u16,
    pub filename: String,
    pub content: Vec<u8>,
    pub size: usize,
    chunks: Vec<Chunk>,
}

impl TransferFile {
    /// Create a new transfer file from a path
    pub fn new(id: u16, path: &Path) -> Result<Self, UpdateError> {
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| UpdateError::FileError("Invalid filename".to_string()))?
            .to_string();

        // Check if this is a delete marker
        if filename.ends_with(".delete") {
            let actual_filename = filename.trim_end_matches(".delete");
            let chunk = FileDelete::new(id, actual_filename);
            
            return Ok(Self {
                id,
                filename: actual_filename.to_string(),
                content: Vec::new(),
                size: 0,
                chunks: vec![chunk.inner().clone()],
            });
        }

        // Read file content
        let content = std::fs::read(path)?;
        let size = content.len();

        // Pad content to MAX_CHUNK_SIZE boundary (workaround for update issue)
        let mut padded_content = content.clone();
        let padding = MAX_CHUNK_SIZE - (size % MAX_CHUNK_SIZE);
        if padding < MAX_CHUNK_SIZE {
            padded_content.extend(vec![0x20; padding]);
        }

        let mut file = Self {
            id,
            filename,
            content: padded_content,
            size,
            chunks: Vec::new(),
        };

        file.make_chunks();
        
        debug!("File {} has {} chunks", file.filename, file.chunks.len());

        Ok(file)
    }

    fn make_chunks(&mut self) {
        let total_packages = self.calculate_total_packages();
        
        // Add file start chunk
        self.chunks.push(
            FileStart::new(self.id, 0, total_packages as u16, &self.filename, self.size as u16)
                .inner()
                .clone(),
        );

        // Add data chunks
        for i in (0..self.size).step_by(MAX_CHUNK_SIZE) {
            let end = (i + MAX_CHUNK_SIZE).min(self.content.len());
            let chunk_data = self.content[i..end].to_vec();
            
            self.chunks.push(
                FileChunk::new(
                    self.id,
                    (i / MAX_CHUNK_SIZE) as u16,
                    total_packages as u16,
                    chunk_data,
                )
                .inner()
                .clone(),
            );
        }

        // Add file end chunk
        self.chunks.push(FileEnd::new(self.id).inner().clone());
    }

    fn calculate_total_packages(&self) -> usize {
        (self.size + MAX_CHUNK_SIZE - 1) / MAX_CHUNK_SIZE
    }

    /// Get the next chunk that hasn't been acknowledged
    pub fn get_next_chunk(&self) -> Option<&Chunk> {
        self.chunks.iter().find(|c| !c.is_acked())
    }

    /// Get mutable reference to next chunk
    pub fn get_next_chunk_mut(&mut self) -> Option<&mut Chunk> {
        self.chunks.iter_mut().find(|c| !c.is_acked())
    }

    /// Acknowledge a chunk
    pub fn acknowledge_chunk(&mut self, ack: &ChunkAck) {
        if ack.id != self.id {
            return;
        }

        for chunk in &mut self.chunks {
            if chunk.type_ as u8 == ack.type_ && chunk.package == ack.package {
                chunk.set_acked(true);
                debug!("Acked chunk {}", ack);
                break;
            }
        }
    }

    /// Check if all chunks have been acknowledged
    pub fn is_complete(&self) -> bool {
        self.chunks.iter().all(|c| c.is_acked())
    }

    /// Get total number of chunks
    pub fn total_chunks(&self) -> usize {
        self.chunks.len()
    }
}

/// Send a HID command to the device
pub fn send_hid_command(device: &HidDevice, command: u8) -> Result<(), UpdateError> {
    let mut buffer = vec![HID_REPORT_ID_COMMUNICATION, command];
    buffer.extend(vec![0; 7]);
    
    device
        .write(&buffer)
        .map_err(|e| UpdateError::WriteFailed(e.to_string()))?;
    
    Ok(())
}

/// Perform HID upgrade with multiple files
pub async fn perform_hid_upgrade(
    device: &HidDevice,
    files: Vec<&Path>,
) -> Result<(), UpdateError> {
    info!("Starting device update");
    
    // Send prepare update command
    send_hid_command(device, HID_COMMAND_PREPARE_UPDATE)?;
    sleep(Duration::from_secs_f64(STATE_CHANGE_SLEEP_TIME)).await;

    // Create transfer files
    let mut transfer_files: Vec<TransferFile> = files
        .iter()
        .enumerate()
        .map(|(i, path)| TransferFile::new(i as u16, path))
        .collect::<Result<Vec<_>, _>>()?;

    info!("Prepared {} files for update", transfer_files.len());

    let cancelled = false;
    let total_files = transfer_files.len();
    for (i, file) in transfer_files.iter_mut().enumerate() {
        if cancelled {
            break;
        }

        info!(
            "Sending file {} ({}/{})",
            file.filename,
            i + 1,
            total_files
        );

        let mut chunks_sent = 0;
        let total_chunks = file.total_chunks();

        while !file.is_complete() {
            // Check for device responses
            let mut buffer = [0u8; 100];
            match device.read_timeout(&mut buffer, 1000) {
                Ok(size) if size > 0 => {
                    if let Some(msg) = parse_hid_update_message(&buffer[1..]) {
                        match msg {
                            HidUpdateMessage::ChunkAck(ack) => {
                                file.acknowledge_chunk(&ack);
                                chunks_sent += 1;
                                debug!("Progress: {}/{}", chunks_sent, total_chunks);
                            }
                            HidUpdateMessage::Error(err) => {
                                error!("Device error: {}", err);
                                return Err(UpdateError::DeviceError(err.info));
                            }
                            HidUpdateMessage::Log(log) => {
                                match log.level {
                                    crate::device_messages::LogLevel::Debug => {
                                        debug!("Device: {}", log.message)
                                    }
                                    crate::device_messages::LogLevel::Error => {
                                        error!("Device: {}", log.message)
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(_) => {
                    // No data, continue
                }
                Err(e) => {
                    warn!("Read timeout: {}", e);
                }
            }

            // Send next chunk
            if let Some(chunk) = file.get_next_chunk() {
                let mut packet = vec![HID_REPORT_ID_TRANSFER];
                packet.extend_from_slice(&chunk.packet());

                debug!(
                    "Sending chunk ({}...) of file {}",
                    &packet[..10.min(packet.len())]
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<String>(),
                    file.filename
                );

                device
                    .write(&packet)
                    .map_err(|e| UpdateError::WriteFailed(e.to_string()))?;
            }
        }

        info!("File {} transfer complete", file.filename);
    }

    if !cancelled {
        // Send completion packet
        sleep(Duration::from_secs_f64(STATE_CHANGE_SLEEP_TIME)).await;
        
        let completed = Completed::new();
        let mut packet = vec![HID_REPORT_ID_TRANSFER];
        packet.extend_from_slice(&completed.inner().packet());
        
        device
            .write(&packet)
            .map_err(|e| UpdateError::WriteFailed(e.to_string()))?;

        sleep(Duration::from_secs_f64(STATE_CHANGE_SLEEP_TIME)).await;

        // Reset device
        info!("Resetting device");
        send_hid_command(device, HID_COMMAND_RESET)?;

        info!("Device update complete");
    }

    Ok(())
}
