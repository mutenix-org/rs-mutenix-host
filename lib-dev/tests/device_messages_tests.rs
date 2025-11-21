// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

use mutenix_hid::device_messages::*;

    #[test]
    fn test_chunk_ack_str_valid() {
        let mut data = b"AK".to_vec();
        data.extend(&(1u16).to_le_bytes()); // id
        data.extend(&(2u16).to_le_bytes()); // package
        data.push(3); // type
        
        let chunk_ack = ChunkAck::from_bytes(&data);
        
        assert!(chunk_ack.is_valid());
        assert_eq!(format!("{}", chunk_ack), "File: 1, Type: 3, Package: 2");
    }

    #[test]
    fn test_chunk_ack_str_invalid() {
        let mut data = b"XX".to_vec();
        data.extend(&(1u16).to_le_bytes());
        data.extend(&(2u16).to_le_bytes());
        data.push(3);
        
        let chunk_ack = ChunkAck::from_bytes(&data);
        
        assert!(!chunk_ack.is_valid());
        assert_eq!(format!("{}", chunk_ack), "Invalid Request");
    }

    #[test]
    fn test_chunk_ack_fields() {
        let mut data = b"AK".to_vec();
        data.extend(&(1u16).to_le_bytes()); // id
        data.extend(&(2u16).to_le_bytes()); // package
        data.push(3); // type
        
        let chunk_ack = ChunkAck::from_bytes(&data);
        
        assert_eq!(chunk_ack.id, 1);
        assert_eq!(chunk_ack.package, 2);
        assert_eq!(chunk_ack.type_, 3);
    }

    #[test]
    fn test_update_error_str_valid() {
        let mut data = b"ER".to_vec();
        data.push(5); // length
        data.extend(b"Error");
        
        let update_error = UpdateError::from_bytes(&data);
        
        assert!(update_error.is_valid());
        assert_eq!(format!("{}", update_error), "Error: Error");
    }

    #[test]
    fn test_update_error_str_invalid() {
        let mut data = b"XX".to_vec();
        data.push(5);
        data.extend(b"Error");
        
        let update_error = UpdateError::from_bytes(&data);
        
        assert!(!update_error.is_valid());
        assert_eq!(format!("{}", update_error), "Invalid Request");
    }

    #[test]
    fn test_log_message_debug() {
        let mut data = b"LD".to_vec();
        data.extend(b"Debug message\0");
        
        let log_msg = LogMessage::from_bytes(&data);
        
        assert!(log_msg.is_valid());
        assert_eq!(log_msg.level, LogLevel::Debug);
        assert_eq!(log_msg.message, "Debug message");
        assert_eq!(format!("{}", log_msg), "debug: Debug message");
    }

    #[test]
    fn test_log_message_error() {
        let mut data = b"LE".to_vec();
        data.extend(b"Error message\0");
        
        let log_msg = LogMessage::from_bytes(&data);
        
        assert!(log_msg.is_valid());
        assert_eq!(log_msg.level, LogLevel::Error);
        assert_eq!(log_msg.message, "Error message");
        assert_eq!(format!("{}", log_msg), "error: Error message");
    }

    #[test]
    fn test_parse_hid_update_message_chunk_ack() {
        let mut data = b"AK".to_vec();
        data.extend(&(1u16).to_le_bytes());
        data.extend(&(2u16).to_le_bytes());
        data.push(3);
        
        let msg = parse_hid_update_message(&data);
        
        assert!(msg.is_some());
        match msg.unwrap() {
            HidUpdateMessage::ChunkAck(ack) => {
                assert_eq!(ack.id, 1);
                assert_eq!(ack.package, 2);
                assert_eq!(ack.type_, 3);
            }
            _ => panic!("Expected ChunkAck"),
        }
    }

    #[test]
    fn test_parse_hid_update_message_error() {
        let mut data = b"ER".to_vec();
        data.push(5);
        data.extend(b"Error");
        
        let msg = parse_hid_update_message(&data);
        
        assert!(msg.is_some());
        match msg.unwrap() {
            HidUpdateMessage::Error(err) => {
                assert_eq!(err.info, "Error");
            }
            _ => panic!("Expected Error"),
        }
    }

    #[test]
    fn test_parse_hid_update_message_log() {
        let mut data = b"LD".to_vec();
        data.extend(b"Debug message\0");
        
        let msg = parse_hid_update_message(&data);
        
        assert!(msg.is_some());
        match msg.unwrap() {
            HidUpdateMessage::Log(log) => {
                assert_eq!(log.level, LogLevel::Debug);
                assert_eq!(log.message, "Debug message");
            }
            _ => panic!("Expected Log"),
        }
    }

    #[test]
    fn test_parse_hid_update_message_invalid() {
        let data = b"XX";
        
        let msg = parse_hid_update_message(data);
        
        assert!(msg.is_none());
    }

    #[test]
    fn test_parse_hid_update_message_too_short() {
        let data = b"A";
        
        let msg = parse_hid_update_message(data);
        
        assert!(msg.is_none());
    }
