// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

use crate::constants::MAX_CHUNK_SIZE;

/// Types of chunks in the file transfer protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ChunkType {
    FileStart = 1,
    FileChunk = 2,
    FileEnd = 3,
    Complete = 4,
    FileDelete = 5,
}

impl From<ChunkType> for u16 {
    fn from(val: ChunkType) -> Self {
        val as u16
    }
}

/// Base chunk structure for file transfer protocol
#[derive(Debug, Clone)]
pub struct Chunk {
    pub type_: ChunkType,
    pub id: u16,
    pub package: u16,
    pub total_packages: u16,
    pub content: Vec<u8>,
    acked: bool,
}

impl Chunk {
    pub fn new(type_: ChunkType, id: u16, package: u16, total_packages: u16) -> Self {
        Self {
            type_,
            id,
            package,
            total_packages,
            content: Vec::new(),
            acked: false,
        }
    }

    /// Generate the packet bytes for this chunk
    pub fn packet(&self) -> Vec<u8> {
        let mut packet = self.base_packet();
        packet.extend_from_slice(&self.content);
        
        // Pad to MAX_CHUNK_SIZE
        let padding = MAX_CHUNK_SIZE.saturating_sub(self.content.len());
        packet.resize(packet.len() + padding, 0);
        
        packet
    }

    /// Generate base packet header
    fn base_packet(&self) -> Vec<u8> {
        let mut packet = Vec::with_capacity(8);
        packet.extend_from_slice(&u16::from(self.type_).to_le_bytes());
        packet.extend_from_slice(&self.id.to_le_bytes());
        packet.extend_from_slice(&self.total_packages.to_le_bytes());
        packet.extend_from_slice(&self.package.to_le_bytes());
        packet
    }

    pub fn is_acked(&self) -> bool {
        self.acked
    }

    pub fn set_acked(&mut self, acked: bool) {
        self.acked = acked;
    }
}

/// Chunk containing file data
#[derive(Debug, Clone)]
pub struct FileChunk {
    chunk: Chunk,
}

impl FileChunk {
    pub fn new(id: u16, package: u16, total_packages: u16, content: Vec<u8>) -> Self {
        let mut chunk = Chunk::new(ChunkType::FileChunk, id, package, total_packages);
        chunk.content = content;
        Self { chunk }
    }

    pub fn inner(&self) -> &Chunk {
        &self.chunk
    }

    pub fn inner_mut(&mut self) -> &mut Chunk {
        &mut self.chunk
    }
}

/// Chunk marking the start of a file transfer
#[derive(Debug, Clone)]
pub struct FileStart {
    chunk: Chunk,
}

impl FileStart {
    pub fn new(id: u16, package: u16, total_packages: u16, filename: &str, filesize: u16) -> Self {
        let mut chunk = Chunk::new(ChunkType::FileStart, id, package, total_packages);
        
        let mut content = Vec::new();
        content.push(filename.len() as u8);
        content.extend_from_slice(filename.as_bytes());
        content.push(2); // Size indicator
        content.extend_from_slice(&filesize.to_le_bytes());
        
        chunk.content = content;
        Self { chunk }
    }

    pub fn inner(&self) -> &Chunk {
        &self.chunk
    }

    pub fn inner_mut(&mut self) -> &mut Chunk {
        &mut self.chunk
    }
}

/// Chunk marking the end of a file transfer
#[derive(Debug, Clone)]
pub struct FileEnd {
    chunk: Chunk,
}

impl FileEnd {
    pub fn new(id: u16) -> Self {
        let chunk = Chunk::new(ChunkType::FileEnd, id, 0, 0);
        Self { chunk }
    }

    pub fn inner(&self) -> &Chunk {
        &self.chunk
    }

    pub fn inner_mut(&mut self) -> &mut Chunk {
        &mut self.chunk
    }
}

/// Chunk to delete a file
#[derive(Debug, Clone)]
pub struct FileDelete {
    chunk: Chunk,
}

impl FileDelete {
    pub fn new(id: u16, filename: &str) -> Self {
        let mut chunk = Chunk::new(ChunkType::FileDelete, id, 0, 0);
        
        let mut content = Vec::new();
        content.push(filename.len() as u8);
        content.extend_from_slice(filename.as_bytes());
        
        chunk.content = content;
        Self { chunk }
    }

    pub fn inner(&self) -> &Chunk {
        &self.chunk
    }

    pub fn inner_mut(&mut self) -> &mut Chunk {
        &mut self.chunk
    }
}

/// Chunk marking completion of all transfers
#[derive(Debug, Clone)]
pub struct Completed {
    chunk: Chunk,
}

impl Completed {
    pub fn new() -> Self {
        let chunk = Chunk::new(ChunkType::Complete, 0, 0, 0);
        Self { chunk }
    }

    pub fn inner(&self) -> &Chunk {
        &self.chunk
    }
}

impl Default for Completed {
    fn default() -> Self {
        Self::new()
    }
}
