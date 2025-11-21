// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

use mutenix_hid::chunks::*;
use mutenix_hid::constants::MAX_CHUNK_SIZE;

    #[test]
    fn test_file_chunk_packet() {
        let chunk = FileChunk::new(1, 2, 3, b"content".to_vec());
        let packet = chunk.inner().packet();
        
        // Check type (FileChunk = 2)
        assert_eq!(&packet[0..2], &(2u16).to_le_bytes());
        // Check id
        assert_eq!(&packet[2..4], &(1u16).to_le_bytes());
        // Check total_packages
        assert_eq!(&packet[4..6], &(3u16).to_le_bytes());
        // Check package
        assert_eq!(&packet[6..8], &(2u16).to_le_bytes());
        // Check content starts correctly
        assert_eq!(&packet[8..15], b"content");
    }

    #[test]
    fn test_file_start_packet() {
        let start = FileStart::new(1, 0, 3, "test.py", 100);
        let packet = start.inner().packet();
        
        // Check type (FileStart = 1)
        assert_eq!(&packet[0..2], &(1u16).to_le_bytes());
        // Check id
        assert_eq!(&packet[2..4], &(1u16).to_le_bytes());
        // Check total_packages
        assert_eq!(&packet[4..6], &(3u16).to_le_bytes());
        // Check package
        assert_eq!(&packet[6..8], &(0u16).to_le_bytes());
        // Check filename length
        assert_eq!(packet[8], 7);
        // Check filename and size
        assert_eq!(
            &packet[9..19],
            &[b't', b'e', b's', b't', b'.', b'p', b'y', 2, 100, 0]
        );
    }

    #[test]
    fn test_file_end_packet() {
        let end = FileEnd::new(1);
        let packet = end.inner().packet();
        
        // Check type (FileEnd = 3)
        assert_eq!(&packet[0..2], &(3u16).to_le_bytes());
        // Check id
        assert_eq!(&packet[2..4], &(1u16).to_le_bytes());
        // Rest should be zeros
        assert_eq!(&packet[4..], &vec![0u8; MAX_CHUNK_SIZE + 4][..]);
    }

    #[test]
    fn test_file_delete_packet() {
        let delete = FileDelete::new(1, "test.py");
        let packet = delete.inner().packet();
        
        // Check type (FileDelete = 5)
        assert_eq!(&packet[0..2], &(5u16).to_le_bytes());
        // Check id
        assert_eq!(&packet[2..4], &(1u16).to_le_bytes());
        // Check filename length
        assert_eq!(packet[8], 7);
        // Check filename
        assert_eq!(&packet[9..16], b"test.py");
    }

    #[test]
    fn test_chunk_acked() {
        let mut chunk = Chunk::new(ChunkType::FileChunk, 1, 2, 3);
        
        assert_eq!(chunk.is_acked(), false);
        
        chunk.set_acked(true);
        assert_eq!(chunk.is_acked(), true);
        
        chunk.set_acked(false);
        assert_eq!(chunk.is_acked(), false);
    }

    #[test]
    fn test_completed_packet() {
        let completed = Completed::new();
        let packet = completed.inner().packet();
        
        // Check type (Complete = 4)
        assert_eq!(&packet[0..2], &(4u16).to_le_bytes());
        // Check id is 0
        assert_eq!(&packet[2..4], &(0u16).to_le_bytes());
    }
