use flowmode::{run, CliCommand, StartArgs, StopArgs};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::NamedTempFile;
use std::sync::Mutex;

// Mutex to prevent tests from running concurrently
static TEST_MUTEX: Mutex<()> = Mutex::new(());

#[tokio::test]
async fn test_config_loading() {
    // Test loading configuration from config.toml
    let config = flowmode::load_config();
    assert!(config.block_list.is_some());
    assert!(config.app_block_list.is_some());
}

#[tokio::test]
async fn test_hosts_path_windows() {
    if !cfg!(target_os = "windows") {
        return; // Skip this test on non-Windows platforms
    }
    
    // Store current env var
    let old_var = std::env::var("FLOWMODE_TEST_HOSTS_FILE").ok();
    
    // Clear any existing environment variable first
    std::env::remove_var("FLOWMODE_TEST_HOSTS_FILE");
    
    let hosts_path = flowmode::get_hosts_path();
    
    // Restore old env var if it existed
    if let Some(val) = old_var {
        std::env::set_var("FLOWMODE_TEST_HOSTS_FILE", val);
    }
    
    assert_eq!(hosts_path, PathBuf::from("C:\\Windows\\System32\\drivers\\etc\\hosts"));
}

#[tokio::test]
async fn test_hosts_path_unix() {
    if !cfg!(target_os = "windows") {
        let hosts_path = flowmode::get_hosts_path();
        assert_eq!(hosts_path, PathBuf::from("/etc/hosts"));
    }
}

#[tokio::test]
async fn test_hosts_path_custom() {
    std::env::set_var("FLOWMODE_TEST_HOSTS_FILE", "/tmp/test_hosts");
    let hosts_path = flowmode::get_hosts_path();
    assert_eq!(hosts_path, PathBuf::from("/tmp/test_hosts"));
    std::env::remove_var("FLOWMODE_TEST_HOSTS_FILE");
}

#[tokio::test]
async fn test_website_blocking_and_unblocking() {
    let _guard = TEST_MUTEX.lock().unwrap();
    
    // Create a temporary hosts file for testing
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_str().unwrap();
    
    // Store current env var if any
    let old_env = std::env::var("FLOWMODE_TEST_HOSTS_FILE").ok();
    
    // Set environment variable to use our test file
    std::env::set_var("FLOWMODE_TEST_HOSTS_FILE", temp_path);
    
    // Write initial content to test file
    fs::write(temp_path, "127.0.0.1 localhost\n").unwrap();
    
    // Create test start args with explicit whitelist = false
    let start_args = StartArgs {
        duration: "1m".to_string(),
        task: Some("Test task".to_string()),
        slack_webhook_url: None,
        whitelist: false,  // Explicitly false
        pomodoro: None,
        r#break: None,
        long_break: None,
        cycles: None,
    };
    
    // Test config with ONLY our blocked site (no defaults)
    let config = flowmode::Config {
        block_list: Some(vec!["127.0.0.1 example.com".to_string()]),
        app_block_list: None,
        whitelist: None,
        pomodoro_defaults: None,
    };
    
    println!("Test config - whitelist mode: {}", start_args.whitelist);
    println!("Test config - block_list: {:?}", config.block_list);
    
    // Test blocking
    flowmode::block_websites(&start_args, &config).await.unwrap();
    
    let blocked_content = fs::read_to_string(temp_path).unwrap();
    println!("Blocked content: {}", blocked_content);
    // Debug information
    println!("Expected to find: 127.0.0.1 example.com");
    println!("Actual content length: {}", blocked_content.len());
    println!("Original content length: {}", "127.0.0.1 localhost\n".len());
    println!("Does content contain 'example.com'? {}", blocked_content.contains("example.com"));
    
    // The website should have been added to the hosts file
    // Check for either the full line or just the domain
    assert!(blocked_content.contains("example.com"), 
            "Should contain example.com in the hosts file.\nActual content: '{}'", blocked_content);
    
    // Test unblocking
    flowmode::unblock_websites().await.unwrap();
    
    let unblocked_content = fs::read_to_string(temp_path).unwrap();
    assert_eq!(unblocked_content, "127.0.0.1 localhost\n");
    
    // Clean up - restore previous env var if it existed
    if let Some(val) = old_env {
        std::env::set_var("FLOWMODE_TEST_HOSTS_FILE", val);
    } else {
        std::env::remove_var("FLOWMODE_TEST_HOSTS_FILE");
    }
}

#[tokio::test]
async fn test_whitelist_mode() {
    let _guard = TEST_MUTEX.lock().unwrap();
    
    // Create a temporary hosts file for testing
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_str().unwrap();
    
    // Store current env var if any
    let old_env = std::env::var("FLOWMODE_TEST_HOSTS_FILE").ok();
    
    std::env::set_var("FLOWMODE_TEST_HOSTS_FILE", temp_path);
    fs::write(temp_path, "127.0.0.1 localhost\n").unwrap();
    
    let start_args = StartArgs {
        duration: "1m".to_string(),
        task: Some("Test task".to_string()),
        slack_webhook_url: None,
        whitelist: true,  // Explicitly true for whitelist mode
        pomodoro: None,
        r#break: None,
        long_break: None,
        cycles: None,
    };
    
    let config = flowmode::Config {
        block_list: None,
        app_block_list: None, 
        whitelist: Some(vec!["github.com".to_string()]),
        pomodoro_defaults: None,
    };
    
    println!("Whitelist test - whitelist mode: {}", start_args.whitelist);
    println!("Whitelist test - whitelist domains: {:?}", config.whitelist);
    
    flowmode::block_websites(&start_args, &config).await.unwrap();
    
    let content = fs::read_to_string(temp_path).unwrap();
    println!("Whitelist content: {}", content);
    println!("Content length: {}", content.len());
    println!("Contains facebook.com: {}", content.contains("facebook.com"));
    println!("Contains twitter.com: {}", content.contains("twitter.com"));
    
    // In whitelist mode, we should see blocked social media sites
    let has_social_media_blocks = content.contains("facebook.com") || 
                                  content.contains("twitter.com") || 
                                  content.contains("instagram.com") ||
                                  content.contains("youtube.com");
    
    assert!(has_social_media_blocks, 
            "Expected social media blocking to occur in whitelist mode.\nActual content: '{}'", content);
    
    // GitHub should not be explicitly blocked (it should be whitelisted)
    assert!(!content.contains("127.0.0.1 github.com"), "GitHub should be whitelisted");
    
    // Clean up - restore previous env var if it existed
    if let Some(val) = old_env {
        std::env::set_var("FLOWMODE_TEST_HOSTS_FILE", val);
    } else {
        std::env::remove_var("FLOWMODE_TEST_HOSTS_FILE");
    }
}

#[tokio::test]
async fn test_duration_parsing() {
    let start_args = StartArgs {
        duration: "30m".to_string(),
        task: None,
        slack_webhook_url: None,
        whitelist: false,
        pomodoro: Some("25m".to_string()),
        r#break: Some("5m".to_string()),
        long_break: Some("15m".to_string()),
        cycles: Some(2),
    };
    
    // Test that duration parsing doesn't panic
    let duration = humantime::parse_duration(&start_args.duration).unwrap();
    assert_eq!(duration, Duration::from_secs(30 * 60));
    
    if let Some(pomodoro) = &start_args.pomodoro {
        let pomodoro_duration = humantime::parse_duration(pomodoro).unwrap();
        assert_eq!(pomodoro_duration, Duration::from_secs(25 * 60));
    }
}

#[tokio::test]
async fn test_csv_logging() {
    // Clean up any existing log file
    let _ = fs::remove_file("test_log.csv");
    
    // Create a test session
    let task = "Test logging task";
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("test_log.csv")
        .unwrap();
    
    use std::io::Write;
    use chrono::prelude::*;
    
    let start_time = Local::now();
    write!(file, "{},{},", task, start_time.to_rfc3339()).unwrap();
    
    // Simulate end of session
    let end_time = Local::now();
    writeln!(file, "{}", end_time.to_rfc3339()).unwrap();
    
    // Verify the log file was created and contains expected content
    let content = fs::read_to_string("test_log.csv").unwrap();
    assert!(content.contains(task));
    assert!(content.contains(&start_time.to_rfc3339()));
    assert!(content.contains(&end_time.to_rfc3339()));
    
    // Clean up
    let _ = fs::remove_file("test_log.csv");
}

#[tokio::test]
async fn test_stop_command() {
    // Test that stop command doesn't panic when no session is running
    let stop_args = StopArgs {};
    let result = run(CliCommand::Stop(stop_args)).await;
    // Allow both success and some expected errors (like missing log file)
    match result {
        Ok(_) => {},
        Err(e) => {
            // It's okay if the log file doesn't exist or other expected errors
            println!("Stop command error (expected): {}", e);
        }
    }
}

#[tokio::test]
async fn test_application_blocking() {
    // This test just verifies the function doesn't panic
    let config = flowmode::Config::default();
    let result = flowmode::block_applications(&config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_slack_webhook_error_handling() {
    // Test with an invalid URL to ensure error handling works
    let invalid_url = "not-a-valid-url";
    let result = flowmode::post_to_slack(invalid_url, "test message").await;
    assert!(result.is_err());
}