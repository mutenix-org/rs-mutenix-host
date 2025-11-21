// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Matthias Bilger <matthias@bilger.info>

use mutenix_hid::hid_commands::*;

    #[test]
    fn test_status() {
        let buffer = [1, 1, 0, 0, 1, 0];
        let status = Status::from_buffer(&buffer).unwrap();
        
        assert_eq!(status.button(), 1);
        assert_eq!(status.triggered(), true);
        assert_eq!(status.longpressed(), false);
        assert_eq!(status.pressed(), false);
        assert_eq!(status.released(), true);
    }

    #[test]
    fn test_version_info() {
        let buffer = [1, 0, 0, 3, 0, 0];
        let version_info = VersionInfo::from_buffer(&buffer).unwrap();
        
        assert_eq!(version_info.version(), "1.0.0");
        assert_eq!(version_info.hardware_type(), HardwareType::FiveButtonUsb);
    }

    #[test]
    fn test_reset() {
        let reset = SimpleCommand::reset();
        let buffer = reset.to_buffer();
        
        assert_eq!(&buffer[..7], &[HidOutCommand::Reset as u8, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_set_led() {
        let led = SetLed::new(1, LedColor::Red);
        let buffer = led.to_buffer();
        
        assert_eq!(
            &buffer[..7],
            &[HidOutCommand::SetLed as u8, 1, 0x0A, 0x00, 0x00, 0x00, 0]
        );
    }

    #[test]
    fn test_from_buffer_version_info() {
        let buffer = [0, HidInCommand::VersionInfo as u8, 1, 2, 3, 4, 5, 6];
        let message = parse_input_message(&buffer).unwrap();
        
        let version_info = message.downcast_ref::<VersionInfo>().unwrap();
        assert_eq!(version_info.version(), "1.2.3");
        assert_eq!(version_info.hardware_type(), HardwareType::from(4));
    }

    #[test]
    fn test_from_buffer_status() {
        let buffer = [0, HidInCommand::Status as u8, 1, 0, 0, 1, 0, 0];
        let message = parse_input_message(&buffer).unwrap();
        
        let status = message.downcast_ref::<Status>().unwrap();
        assert_eq!(status.button(), 1);
        assert_eq!(status.triggered(), false);
        assert_eq!(status.longpressed(), false);
        assert_eq!(status.pressed(), true);
        assert_eq!(status.released(), false);
    }

    #[test]
    fn test_from_buffer_status_request() {
        let buffer = [0, HidInCommand::StatusRequest as u8, 0, 0, 0, 0, 0, 0];
        let message = parse_input_message(&buffer).unwrap();
        
        assert!(message.downcast_ref::<StatusRequest>().is_some());
    }

    #[test]
    fn test_from_buffer_not_implemented() {
        let buffer = [0, 0xFF, 0, 0, 0, 0, 0, 0];
        let result = parse_input_message(&buffer);
        
        assert!(result.is_err());
        match result.unwrap_err() {
            HidMessageError::UnknownCommand(cmd) => assert_eq!(cmd, 0xFF),
            _ => panic!("Expected UnknownCommand error"),
        }
    }

    #[test]
    fn test_update_config_to_buffer_default() {
        let update_config = UpdateConfig::new();
        let buffer = update_config.to_buffer();
        
        assert_eq!(
            &buffer[..7],
            &[HidOutCommand::UpdateConfig as u8, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn test_update_config_to_buffer_debug() {
        let update_config = UpdateConfig::new().activate_serial_console(true);
        let buffer = update_config.to_buffer();
        
        assert_eq!(
            &buffer[..7],
            &[HidOutCommand::UpdateConfig as u8, 2, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn test_update_config_to_buffer_filesystem() {
        let update_config = UpdateConfig::new().activate_filesystem(true);
        let buffer = update_config.to_buffer();
        
        assert_eq!(
            &buffer[..7],
            &[HidOutCommand::UpdateConfig as u8, 0, 2, 0, 0, 0, 0]
        );
    }

    #[test]
    fn test_update_config_to_buffer_debug_and_filesystem() {
        let update_config = UpdateConfig::new()
            .activate_serial_console(true)
            .activate_filesystem(true);
        let buffer = update_config.to_buffer();
        
        assert_eq!(
            &buffer[..7],
            &[HidOutCommand::UpdateConfig as u8, 2, 2, 0, 0, 0, 0]
        );
    }

    #[test]
    fn test_update_config_str_default() {
        let update_config = UpdateConfig::new();
        assert_eq!(
            format!("{}", update_config),
            "UpdateConfig { debug: 0, filesystem: 0 }"
        );
    }

    #[test]
    fn test_update_config_str_debug() {
        let update_config = UpdateConfig::new().activate_serial_console(true);
        assert_eq!(
            format!("{}", update_config),
            "UpdateConfig { debug: 2, filesystem: 0 }"
        );
    }

    #[test]
    fn test_update_config_str_filesystem() {
        let update_config = UpdateConfig::new().activate_filesystem(true);
        assert_eq!(
            format!("{}", update_config),
            "UpdateConfig { debug: 0, filesystem: 2 }"
        );
    }

    #[test]
    fn test_update_config_str_debug_and_filesystem() {
        let update_config = UpdateConfig::new()
            .activate_serial_console(true)
            .activate_filesystem(true);
        assert_eq!(
            format!("{}", update_config),
            "UpdateConfig { debug: 2, filesystem: 2 }"
        );
    }
