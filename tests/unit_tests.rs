use flowmode::*;

#[test]
fn test_config_default() {
    let config = Config::default();
    
    assert!(config.block_list.is_some());
    assert!(config.app_block_list.is_some());
    assert!(config.pomodoro_defaults.is_some());
    
    let block_list = config.block_list.unwrap();
    assert!(block_list.contains(&"127.0.0.1 facebook.com".to_string()));
    assert!(block_list.contains(&"127.0.0.1 www.facebook.com".to_string()));
    
    let app_list = config.app_block_list.unwrap();
    assert!(app_list.contains(&"slack.exe".to_string()));
    assert!(app_list.contains(&"discord.exe".to_string()));
    
    let pomodoro = config.pomodoro_defaults.unwrap();
    assert_eq!(pomodoro.pomodoro, "25m");
    assert_eq!(pomodoro.r#break, "5m");
    assert_eq!(pomodoro.long_break, "15m");
    assert_eq!(pomodoro.cycles, 4);
}

#[test]
fn test_hosts_path_determination() {
    // Test environment variable override
    std::env::set_var("FLOWMODE_TEST_HOSTS_FILE", "/tmp/custom_hosts");
    let path = get_hosts_path();
    assert_eq!(path.to_str().unwrap(), "/tmp/custom_hosts");
    std::env::remove_var("FLOWMODE_TEST_HOSTS_FILE");
    
    // Test platform-specific paths
    let path = get_hosts_path();
    if cfg!(target_os = "windows") {
        assert_eq!(path.to_str().unwrap(), "C:\\Windows\\System32\\drivers\\etc\\hosts");
    } else {
        assert_eq!(path.to_str().unwrap(), "/etc/hosts");
    }
}

#[test]
fn test_cli_parsing() {
    use clap::Parser;
    
    // Test start command parsing
    let args = vec!["flowmode", "start", "--duration", "30m", "--task", "Test task"];
    let cli = Cli::try_parse_from(args).unwrap();
    
    match cli.command {
        CliCommand::Start(start_args) => {
            assert_eq!(start_args.duration, "30m");
            assert_eq!(start_args.task, Some("Test task".to_string()));
            assert_eq!(start_args.whitelist, false);
        }
        _ => panic!("Expected Start command"),
    }
    
    // Test stop command parsing
    let args = vec!["flowmode", "stop"];
    let cli = Cli::try_parse_from(args).unwrap();
    match cli.command {
        CliCommand::Stop(_) => {} // Success
        _ => panic!("Expected Stop command"),
    }
    
    // Test report command parsing
    let args = vec!["flowmode", "report"];
    let cli = Cli::try_parse_from(args).unwrap();
    match cli.command {
        CliCommand::Report => {} // Success
        _ => panic!("Expected Report command"),
    }
}

#[test]
fn test_start_args_with_pomodoro() {
    use clap::Parser;
    
    let args = vec![
        "flowmode", "start", 
        "--duration", "2h",
        "--task", "Deep work session",
        "--pomodoro", "45m",
        "--break", "10m", 
        "--long-break", "30m",
        "--cycles", "3",
        "--whitelist"
    ];
    
    let cli = Cli::try_parse_from(args).unwrap();
    
    match cli.command {
        CliCommand::Start(start_args) => {
            assert_eq!(start_args.duration, "2h");
            assert_eq!(start_args.task, Some("Deep work session".to_string()));
            assert_eq!(start_args.pomodoro, Some("45m".to_string()));
            assert_eq!(start_args.r#break, Some("10m".to_string()));
            assert_eq!(start_args.long_break, Some("30m".to_string()));
            assert_eq!(start_args.cycles, Some(3));
            assert_eq!(start_args.whitelist, true);
        }
        _ => panic!("Expected Start command"),
    }
}

#[test]
fn test_duration_validation() {
    // Test valid durations
    let valid_durations = vec!["1m", "30m", "1h", "2h 30m", "90s", "1h 30m 45s"];
    
    for duration in valid_durations {
        let result = humantime::parse_duration(duration);
        assert!(result.is_ok(), "Duration '{}' should be valid", duration);
    }
    
    // Test invalid durations
    let invalid_durations = vec!["invalid", "1x", "", "30"];
    
    for duration in invalid_durations {
        let result = humantime::parse_duration(duration);
        assert!(result.is_err(), "Duration '{}' should be invalid", duration);
    }
}

#[test] 
fn test_config_serialization() {
    use serde_json;
    
    let config = Config::default();
    
    // Test that config can be serialized and deserialized
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: Config = serde_json::from_str(&json).unwrap();
    
    assert_eq!(config.block_list, deserialized.block_list);
    assert_eq!(config.app_block_list, deserialized.app_block_list);
}

#[test]
fn test_pomodoro_defaults_serialization() {
    let defaults = PomodoroDefaults {
        pomodoro: "25m".to_string(),
        r#break: "5m".to_string(),
        long_break: "15m".to_string(),
        cycles: 4,
    };
    
    let json = serde_json::to_string(&defaults).unwrap();
    let deserialized: PomodoroDefaults = serde_json::from_str(&json).unwrap();
    
    assert_eq!(defaults.pomodoro, deserialized.pomodoro);
    assert_eq!(defaults.r#break, deserialized.r#break);
    assert_eq!(defaults.long_break, deserialized.long_break);
    assert_eq!(defaults.cycles, deserialized.cycles);
}

#[test]
fn test_empty_config_loading() {
    // Test loading config when no file exists
    // Should return default config
    let config = load_config();
    assert!(config.block_list.is_some());
    assert!(config.app_block_list.is_some());
}

#[cfg(test)]
mod integration {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[test]
    fn test_config_file_loading() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let config_content = r#"
block_list = ["127.0.0.1 test.com"]
app_block_list = ["test.exe"]

[pomodoro_defaults]
pomodoro = "30m"
break = "10m"
long_break = "20m"
cycles = 2
"#;
        
        fs::write(&config_path, config_content).unwrap();
        
        // Change to temp directory to test config loading
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        let config = load_config();
        
        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
        
        assert_eq!(config.block_list.unwrap(), vec!["127.0.0.1 test.com"]);
        assert_eq!(config.app_block_list.unwrap(), vec!["test.exe"]);
        
        let pomodoro = config.pomodoro_defaults.unwrap();
        assert_eq!(pomodoro.pomodoro, "30m");
        assert_eq!(pomodoro.r#break, "10m");
        assert_eq!(pomodoro.long_break, "20m");
        assert_eq!(pomodoro.cycles, 2);
    }
}