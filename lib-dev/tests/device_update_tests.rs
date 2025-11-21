// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

use mutenix_hid::chunks::*;
use mutenix_hid::device_messages::ChunkAck;
use mutenix_hid::device_update::*;
use std::fs;
use std::io::Write;
use tempfile::TempDir;

fn create_test_file(dir: &TempDir, name: &str, content: &[u8]) -> std::path::PathBuf {
    let path = dir.path().join(name);
    let mut file = fs::File::create(&path).unwrap();
    file.write_all(content).unwrap();
    path
}

#[test]
fn test_transfer_file_chunks() {
        let dir = TempDir::new().unwrap();
        let content = b"fake content".repeat(10);
        let path = create_test_file(&dir, "test_file.py", &content);

        let transfer_file = TransferFile::new(1, &path).unwrap();

        assert!(transfer_file.size >= content.len());
        assert!(transfer_file.total_chunks() > 0);
    }

    #[test]
    fn test_transfer_file_get_next_chunk() {
        let dir = TempDir::new().unwrap();
        let content = b"fake content".repeat(10);
        let path = create_test_file(&dir, "test_file.py", &content);

        let transfer_file = TransferFile::new(1, &path).unwrap();
        let chunk = transfer_file.get_next_chunk();

        assert!(chunk.is_some());
        assert!(!chunk.unwrap().is_acked());
    }

    #[test]
    fn test_transfer_file_is_complete() {
        let dir = TempDir::new().unwrap();
        let content = b"fake content".repeat(10);
        let path = create_test_file(&dir, "test_file.py", &content);

        let mut transfer_file = TransferFile::new(1, &path).unwrap();

        assert!(!transfer_file.is_complete());

        // Acknowledge all chunks
        while let Some(chunk) = transfer_file.get_next_chunk() {
            let chunk_type = chunk.type_ as u8;
            let chunk_package = chunk.package;
            
            let mut data = b"AK".to_vec();
            data.extend(&chunk.id.to_le_bytes());
            data.extend(&chunk_package.to_le_bytes());
            data.push(chunk_type);
            data.extend(&[0u8; 2]);

            let ack = ChunkAck::from_bytes(&data);
            transfer_file.acknowledge_chunk(&ack);
        }

        assert!(transfer_file.is_complete());
    }

    #[test]
    fn test_transfer_file_from_path() {
        let dir = TempDir::new().unwrap();
        let content = b"fake content".repeat(10);
        let path = create_test_file(&dir, "test_file.py", &content);

        let transfer_file = TransferFile::new(1, &path).unwrap();

        assert_eq!(transfer_file.filename, "test_file.py");
        assert!(transfer_file.size >= content.len());
    }

    #[test]
    fn test_transfer_file_init() {
        let dir = TempDir::new().unwrap();
        let content = b"fake content".repeat(10);
        let path = create_test_file(&dir, "test_file.py", &content);

        let transfer_file = TransferFile::new(1, &path).unwrap();

        assert_eq!(transfer_file.id, 1);
        assert_eq!(transfer_file.filename, "test_file.py");
        assert!(transfer_file.size >= content.len());
        assert!(transfer_file.total_chunks() > 0);
    }

    #[test]
    fn test_transfer_file_init_delete() {
        let dir = TempDir::new().unwrap();
        let content = b"fake content".repeat(10);
        let path = create_test_file(&dir, "test_file.py.delete", &content);

        let transfer_file = TransferFile::new(1, &path).unwrap();

        assert_eq!(transfer_file.id, 1);
        assert_eq!(transfer_file.filename, "test_file.py");
        assert_eq!(transfer_file.total_chunks(), 1);

        // Check that the first chunk is a FileDelete
        let chunk = transfer_file.get_next_chunk().unwrap();
        assert_eq!(chunk.type_, ChunkType::FileDelete);
    }

    #[test]
    fn test_transfer_file_acknowledge_chunk() {
        let dir = TempDir::new().unwrap();
        let content = b"fake content";
        let path = create_test_file(&dir, "test.py", content);

        let mut transfer_file = TransferFile::new(1, &path).unwrap();

        let chunk = transfer_file.get_next_chunk().unwrap();
        assert!(!chunk.is_acked());

        let chunk_type = chunk.type_ as u8;
        let chunk_package = chunk.package;

        let mut data = b"AK".to_vec();
        data.extend(&chunk.id.to_le_bytes());
        data.extend(&chunk_package.to_le_bytes());
        data.push(chunk_type);
        data.extend(&[0u8; 2]);

        let ack = ChunkAck::from_bytes(&data);
        transfer_file.acknowledge_chunk(&ack);

        let chunk = transfer_file.get_next_chunk().unwrap();
        // Should move to next chunk since first is now acked
        assert!(!chunk.is_acked());
    }

    #[test]
    fn test_transfer_file_acknowledge_wrong_id() {
        let dir = TempDir::new().unwrap();
        let content = b"fake content";
        let path = create_test_file(&dir, "test.py", content);

        let mut transfer_file = TransferFile::new(1, &path).unwrap();

        let chunk = transfer_file.get_next_chunk().unwrap();
        let chunk_type = chunk.type_ as u8;
        let chunk_package = chunk.package;

        // Create ack with wrong id
        let mut data = b"AK".to_vec();
        data.extend(&(999u16).to_le_bytes()); // Wrong id
        data.extend(&chunk_package.to_le_bytes());
        data.push(chunk_type);
        data.extend(&[0u8; 2]);

        let ack = ChunkAck::from_bytes(&data);
        transfer_file.acknowledge_chunk(&ack);

        // Chunk should not be acknowledged
        let chunk = transfer_file.get_next_chunk().unwrap();
        assert_eq!(chunk.type_ as u8, chunk_type);
        assert_eq!(chunk.package, chunk_package);
    }

    #[test]
    fn test_transfer_file_total_chunks() {
        let dir = TempDir::new().unwrap();
        let content = b"x";
        let path = create_test_file(&dir, "small.py", content);

        let transfer_file = TransferFile::new(1, &path).unwrap();

        // Should have at least: FileStart, FileChunk(s), FileEnd
        assert!(transfer_file.total_chunks() >= 3);
    }
