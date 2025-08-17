use clap::{Parser, Subcommand};
use reqwest;
use serde_json;
use std::fs;
use std::path::PathBuf;
use tokio::process::Command;
use std::process::Stdio;
use sysinfo::System;
use chrono::prelude::*;
use std::io::Write;
use tokio::sync::broadcast::{self, Sender};
use std::sync::OnceLock;
use serde::{Deserialize, Serialize};
use toml;

static STOP_SIGNAL_SENDER: OnceLock<Sender<()>> = OnceLock::new();

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Config {
    pub block_list: Option<Vec<String>>,
    pub app_block_list: Option<Vec<String>>,
    pub whitelist: Option<Vec<String>>,
    pub pomodoro_defaults: Option<PomodoroDefaults>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct PomodoroDefaults {
    pub pomodoro: String,
    pub r#break: String,
    pub long_break: String,
    pub cycles: u32,
}

#[derive(Parser)]
#[clap(author, version, about = "Flow Mode: A CLI tool to help you focus by blocking distractions and managing Pomodoro sessions.\n\nTo get help for a specific subcommand, use: flowmode <SUBCOMMAND> --help")]
pub struct Cli {
    #[clap(subcommand)]
    pub command: CliCommand,
}

#[derive(Subcommand)]
pub enum CliCommand {
    Start(StartArgs),
    Stop(StopArgs),
    Report,
}

#[derive(Parser)]
pub struct StartArgs {
    #[clap(short, long, help = "Session duration (e.g., 25m, 1h, 90m, 1h30m)")]
    pub duration: String,

    #[clap(short, long, help = "Task description for logging")]
    pub task: Option<String>,

    #[clap(short, long, help = "Slack webhook URL for notifications")]
    pub slack_webhook_url: Option<String>,

    #[clap(long, help = "Use whitelist mode (block all except specified sites)")]
    pub whitelist: bool,

    #[clap(long, help = "Pomodoro work session duration (e.g., 25m, 45m)")]
    pub pomodoro: Option<String>,

    #[clap(long, help = "Short break duration (e.g., 5m, 10m)")]
    pub r#break: Option<String>,

    #[clap(long, help = "Long break duration (e.g., 15m, 30m)")]
    pub long_break: Option<String>,

    #[clap(long, help = "Number of pomodoro cycles before long break")]
    pub cycles: Option<u32>,
}

#[derive(Parser)]
pub struct StopArgs {}

pub fn load_config() -> Config {
    if let Ok(content) = fs::read_to_string("config.toml") {
        if let Ok(config) = toml::from_str(&content) {
            return config;
        }
    }
    Config::default()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            block_list: Some(vec![
                "127.0.0.1 facebook.com".to_string(),
                "127.0.0.1 www.facebook.com".to_string(),
                "127.0.0.1 twitter.com".to_string(),
                "127.0.0.1 www.twitter.com".to_string(),
                "127.0.0.1 instagram.com".to_string(),
                "127.0.0.1 www.instagram.com".to_string(),
                "127.0.0.1 youtube.com".to_string(),
                "127.0.0.1 www.youtube.com".to_string(),
            ]),
            app_block_list: Some(vec![
                "slack.exe".to_string(),
                "discord.exe".to_string(),
            ]),
            whitelist: None,
            pomodoro_defaults: Some(PomodoroDefaults {
                pomodoro: "25m".to_string(),
                r#break: "5m".to_string(),
                long_break: "15m".to_string(),
                cycles: 4,
            }),
        }
    }
}

pub fn get_hosts_path() -> PathBuf {
    if let Ok(path) = std::env::var("FLOWMODE_TEST_HOSTS_FILE") {
        return PathBuf::from(path);
    }
    
    // Check if we can write to system hosts file, fallback to user hosts if not
    let system_hosts = if cfg!(target_os = "windows") {
        PathBuf::from("C:\\Windows\\System32\\drivers\\etc\\hosts")
    } else {
        PathBuf::from("/etc/hosts")
    };
    
    // Try to check write access without actually writing
    if system_hosts.exists() {
        if let Ok(metadata) = std::fs::metadata(&system_hosts) {
            if !metadata.permissions().readonly() {
                // Try to open for append to test write access
                if std::fs::OpenOptions::new().append(true).open(&system_hosts).is_ok() {
                    return system_hosts;
                }
            }
        }
    }
    
    // Fallback to user-writable hosts file
    get_user_hosts_path()
}

fn get_user_hosts_path() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        let mut path = PathBuf::from(home);
        path.push(".flowmode");
        if !path.exists() {
            let _ = std::fs::create_dir_all(&path);
        }
        path.push("hosts");
        path
    } else {
        PathBuf::from("flowmode_hosts")
    }
}

pub async fn block_websites(args: &StartArgs, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let hosts_path = get_hosts_path();
    let is_system_hosts = hosts_path.to_string_lossy().contains("System32") || hosts_path.to_string_lossy().contains("/etc/");
    
    let original_content = if hosts_path.exists() {
        fs::read_to_string(&hosts_path)?
    } else {
        String::new()
    };

    // Create appropriate backup file
    let backup_file = if is_system_hosts {
        "hosts.backup"
    } else {
        "user_hosts.backup"
    };

    // Only create backup if it doesn't exist, to preserve the original clean state
    if !std::path::Path::new(backup_file).exists() {
        fs::write(backup_file, &original_content)?;
        println!("Created hosts file backup at {}", backup_file);
    } else {
        println!("Using existing hosts file backup");
    }

    let mut new_content = original_content.clone();
    
    // Add header for user hosts file to explain its purpose
    if !is_system_hosts && original_content.is_empty() {
        new_content.push_str("# FlowMode user-level hosts file\n");
        new_content.push_str("# This file blocks websites without requiring admin privileges\n");
        new_content.push_str("# Note: This only works if you configure your system to use this as an additional hosts source\n\n");
    }

    if args.whitelist {
        // Add a broad block for common social media and distraction sites
        let broad_blocks = vec![
            "127.0.0.1 facebook.com",
            "127.0.0.1 www.facebook.com", 
            "127.0.0.1 twitter.com",
            "127.0.0.1 www.twitter.com",
            "127.0.0.1 instagram.com", 
            "127.0.0.1 www.instagram.com",
            "127.0.0.1 youtube.com",
            "127.0.0.1 www.youtube.com",
            "127.0.0.1 reddit.com",
            "127.0.0.1 www.reddit.com",
            "127.0.0.1 tiktok.com",
            "127.0.0.1 www.tiktok.com",
        ];
        
        for block in broad_blocks {
            if !new_content.contains(block) {
                new_content.push_str("\n");
                new_content.push_str(block);
            }
        }
        
        // Remove whitelist domains from blocks if they exist
        if let Some(whitelist) = &config.whitelist {
            for domain in whitelist {
                // Remove any blocking entries for whitelisted domains
                let patterns_to_remove = vec![
                    format!("127.0.0.1 {}", domain),
                    format!("127.0.0.1 www.{}", domain),
                ];
                
                for pattern in patterns_to_remove {
                    new_content = new_content.replace(&pattern, "");
                }
                println!("Whitelisted domain: {}", domain);
            }
        }
    } else {
        if let Some(block_list) = &config.block_list {
            for site in block_list {
                if !new_content.contains(site) {
                    new_content.push_str("\n");
                    new_content.push_str(site);
                }
            }
        }
    }

    fs::write(&hosts_path, new_content)?;

    Ok(())
}

pub async fn block_applications(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(app_list) = &config.app_block_list {
        let mut system = System::new_all();
        system.refresh_all();
        
        let _current_user = std::env::var("USERNAME").or_else(|_| std::env::var("USER")).unwrap_or_else(|_| "unknown".to_string());

        for app_name in app_list {
            let mut killed_any = false;
            for (pid, process) in system.processes() {
                if process.name().to_string_lossy() == *app_name {
                    // Try to get process owner, only kill if it's the current user's process or we can't determine ownership
                    let can_kill = if let Some(_uid) = process.user_id() {
                        // On Unix-like systems, check if it's our UID
                        #[cfg(unix)]
                        {
                            use nix::unistd::getuid;
                            _uid == &getuid()
                        }
                        #[cfg(not(unix))]
                        {
                            true // On Windows, we'll try and let the OS decide
                        }
                    } else {
                        true // If we can't determine ownership, try anyway
                    };
                    
                    if can_kill {
                        if process.kill() {
                            println!("Successfully killed process: {} (PID: {})", app_name, pid);
                            killed_any = true;
                        } else {
                            eprintln!("Failed to kill process: {} (PID: {}) - may require elevated privileges", app_name, pid);
                        }
                    } else {
                        println!("Skipped process {} (PID: {}) - not owned by current user", app_name, pid);
                    }
                }
            }
            if !killed_any {
                println!("No instances of {} found running under current user", app_name);
            }
        }
    }
    Ok(())
}

pub async fn unblock_websites() -> Result<(), Box<dyn std::error::Error>> {
    let hosts_path = get_hosts_path();
    let is_system_hosts = hosts_path.to_string_lossy().contains("System32") || hosts_path.to_string_lossy().contains("/etc/");
    
    let backup_file = if is_system_hosts {
        "hosts.backup"
    } else {
        "user_hosts.backup"
    };

    if let Ok(backup_content) = fs::read_to_string(backup_file) {
        fs::write(&hosts_path, backup_content)?;
        if let Err(e) = fs::remove_file(backup_file) {
            eprintln!("Warning: Failed to remove backup file: {}", e);
        }
        println!("Successfully restored hosts file from backup");
    } else {
        // For user hosts, just delete the file if no backup exists
        if !is_system_hosts && hosts_path.exists() {
            if let Err(e) = fs::remove_file(&hosts_path) {
                eprintln!("Warning: Failed to remove user hosts file: {}", e);
            } else {
                println!("Removed user hosts file");
            }
        } else {
            println!("No backup file found, hosts file not modified");
        }
    }

    Ok(())
}

async fn unblock_applications() -> Result<(), Box<dyn std::error::Error>> {
    // Nothing to do here for now, as we are just killing the processes.
    Ok(())
}

async fn mute_notifications() -> Result<(), Box<dyn std::error::Error>> {
    // Try to mute without admin privileges using user-level controls
    if cfg!(target_os = "windows") {
        // Windows: Try user-level volume control first, then nircmd
        let mut success = false;
        
        // Try PowerShell user-level volume control (Windows 10+)
        match Command::new("powershell")
            .arg("-Command")
            .arg("(New-Object -ComObject WScript.Shell).SendKeys([char]173)") // Volume down key
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
        {
            Ok(_) => {
                println!("Audio muted using user-level control (Windows)");
                success = true;
            }
            Err(_) => {}
        }
        
        // Fallback to nircmd if available
        if !success {
            let nircmd_paths = vec![
                "./nircmd.exe",           // Bundled with app
                "./assets/nircmd.exe",    // In assets folder
                "nircmd",                 // System PATH
            ];
            
            for nircmd_path in nircmd_paths {
                match Command::new(nircmd_path)
                    .arg("mutesysvolume")
                    .arg("1")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await
                {
                    Ok(_) => {
                        println!("Notifications muted using nircmd (Windows)");
                        success = true;
                        break;
                    }
                    Err(_) => continue, // Try next path
                }
            }
        }
        
        if !success {
            println!("Warning: Could not mute notifications automatically. Please mute manually or install nircmd.exe.");
            println!("Download nircmd from: https://www.nirsoft.net/utils/nircmd.html");
        }
    } else if cfg!(target_os = "macos") {
        match Command::new("osascript")
            .arg("-e")
            .arg("set volume output muted true")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
        {
            Ok(_) => println!("Notifications muted (macOS)"),
            Err(e) => eprintln!("Warning: Could not mute notifications on macOS: {}", e),
        }
    } else {
        // Linux: Try user-level controls first
        let mut success = false;
        
        // Try pactl for PulseAudio (user-level)
        match Command::new("pactl")
            .arg("set-sink-mute")
            .arg("@DEFAULT_SINK@")
            .arg("1")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
        {
            Ok(_) => {
                println!("Audio muted using pactl (Linux)");
                success = true;
            }
            Err(_) => {}
        }
        
        // Fallback to amixer
        if !success {
            match Command::new("amixer")
                .arg("sset")
                .arg("Master")
                .arg("mute")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await
            {
                Ok(_) => {
                    println!("Audio muted using amixer (Linux)");
                    success = true;
                }
                Err(_) => {}
            }
        }
        
        if !success {
            println!("Warning: Could not mute notifications (neither pactl nor amixer found)");
        }
    }

    Ok(())
}

async fn unmute_notifications() -> Result<(), Box<dyn std::error::Error>> {
    // Try to unmute without admin privileges using user-level controls
    if cfg!(target_os = "windows") {
        // Windows: Try user-level volume control first, then nircmd
        let mut success = false;
        
        // Try PowerShell user-level volume control (Windows 10+)
        match Command::new("powershell")
            .arg("-Command")
            .arg("(New-Object -ComObject WScript.Shell).SendKeys([char]175)") // Volume up key to unmute
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
        {
            Ok(_) => {
                println!("Audio unmuted using user-level control (Windows)");
                success = true;
            }
            Err(_) => {}
        }
        
        // Fallback to nircmd if available
        if !success {
            let nircmd_paths = vec![
                "./nircmd.exe",           // Bundled with app
                "./assets/nircmd.exe",    // In assets folder
                "nircmd",                 // System PATH
            ];
            
            for nircmd_path in nircmd_paths {
                match Command::new(nircmd_path)
                    .arg("mutesysvolume")
                    .arg("0")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await
                {
                    Ok(_) => {
                        println!("Notifications unmuted using nircmd (Windows)");
                        success = true;
                        break;
                    }
                    Err(_) => continue, // Try next path
                }
            }
        }
        
        if !success {
            println!("Warning: Could not unmute notifications automatically. Please unmute manually or install nircmd.exe.");
            println!("Download nircmd from: https://www.nirsoft.net/utils/nircmd.html");
        }
    } else if cfg!(target_os = "macos") {
        match Command::new("osascript")
            .arg("-e")
            .arg("set volume output muted false")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
        {
            Ok(_) => println!("Notifications unmuted (macOS)"),
            Err(e) => eprintln!("Warning: Could not unmute notifications on macOS: {}", e),
        }
    } else {
        // Linux: Try user-level controls first
        let mut success = false;
        
        // Try pactl for PulseAudio (user-level)
        match Command::new("pactl")
            .arg("set-sink-mute")
            .arg("@DEFAULT_SINK@")
            .arg("0")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
        {
            Ok(_) => {
                println!("Audio unmuted using pactl (Linux)");
                success = true;
            }
            Err(_) => {}
        }
        
        // Fallback to amixer
        if !success {
            match Command::new("amixer")
                .arg("sset")
                .arg("Master")
                .arg("unmute")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await
            {
                Ok(_) => {
                    println!("Audio unmuted using amixer (Linux)");
                    success = true;
                }
                Err(_) => {}
            }
        }
        
        if !success {
            println!("Warning: Could not unmute notifications (neither pactl nor amixer found)");
        }
    }

    Ok(())
}

pub async fn post_to_slack(url: &str, message: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "text": message
    });

    client.post(url)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

fn print_user_hosts_guidance(hosts_path: &std::path::Path) {
    println!("\n📋 FlowMode is using a user-level hosts file for website blocking.");
    println!("   Location: {}", hosts_path.display());
    println!("\n   For full website blocking effectiveness, you may want to:");
    
    if cfg!(target_os = "windows") {
        println!("   • Configure your DNS server to use this file as an additional hosts source");
        println!("   • Or copy the contents to C:\\Windows\\System32\\drivers\\etc\\hosts (requires admin)");
    } else if cfg!(target_os = "macos") {
        println!("   • Copy the contents to /etc/hosts (requires sudo)");
        println!("   • Or configure your DNS resolver to use this file");
    } else {
        println!("   • Copy the contents to /etc/hosts (requires sudo)");
        println!("   • Or configure your DNS resolver to use this file");
    }
    
    println!("   • Use browser extensions for additional blocking");
    println!("   • FlowMode will still provide focus tools and app blocking without admin rights\n");
}

async fn start_flow_mode(args: StartArgs) -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config();

    println!("🚀 Starting Flow Mode session...");
    
    // Validate duration early to catch errors before any setup
    let _duration_check = humantime::parse_duration(&args.duration)
        .map_err(|e| format!("Invalid duration '{}': {}. Use format like '25m', '1h', '30s', etc.", args.duration, e))?;
    
    println!("📵 Blocking distracting websites...");
    let hosts_path = get_hosts_path();
    let is_system_hosts = hosts_path.to_string_lossy().contains("System32") || hosts_path.to_string_lossy().contains("/etc/");
    
    block_websites(&args, &config).await?;
    
    // Show guidance if using user-level hosts
    if !is_system_hosts {
        print_user_hosts_guidance(&hosts_path);
    }
    
    println!("🔪 Closing distracting applications...");
    block_applications(&config).await?;
    
    println!("🔇 Muting notifications...");
    mute_notifications().await?;

    if let Some(url) = &args.slack_webhook_url {
        if let Err(e) = post_to_slack(url, "In flow mode, will reply later.").await {
            eprintln!("Warning: Failed to post to Slack: {}", e);
        }
    }

    println!("✅ Flow mode activated! Focus time begins now.");
    if let Some(ref task) = args.task {
        println!("📝 Working on: {}", task);
    }

    let pid = std::process::id();
    fs::write("flowmode.pid", pid.to_string())?;

    // Always log session start, with task name or "No task specified"
    let task_name = args.task.as_deref().unwrap_or("No task specified");
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("log.csv")?;
    let start_time = Local::now();
    write!(file, "{},{},", task_name, start_time.to_rfc3339())?;
    println!("Logging task: {}", task_name);

    let (tx, mut rx) = broadcast::channel(1);
    STOP_SIGNAL_SENDER.set(tx).unwrap();

    let pomodoro_duration = if let Some(ref d) = args.pomodoro {
        humantime::parse_duration(d).map_err(|e| format!("Invalid pomodoro duration '{}': {}. Use format like '25m', '1h', etc.", d, e))?
    } else if let Some(defaults) = &config.pomodoro_defaults {
        humantime::parse_duration(&defaults.pomodoro).map_err(|e| format!("Invalid pomodoro duration in config '{}': {}. Use format like '25m', '1h', etc.", defaults.pomodoro, e))?
    } else {
        humantime::parse_duration("25m")?
    };

    let break_duration = if let Some(ref d) = args.r#break {
        humantime::parse_duration(d).map_err(|e| format!("Invalid break duration '{}': {}. Use format like '5m', '10m', etc.", d, e))?
    } else if let Some(defaults) = &config.pomodoro_defaults {
        humantime::parse_duration(&defaults.r#break).map_err(|e| format!("Invalid break duration in config '{}': {}. Use format like '5m', '10m', etc.", defaults.r#break, e))?
    } else {
        humantime::parse_duration("5m")?
    };

    let long_break_duration = if let Some(ref d) = args.long_break {
        humantime::parse_duration(d).map_err(|e| format!("Invalid long break duration '{}': {}. Use format like '15m', '30m', etc.", d, e))?
    } else if let Some(defaults) = &config.pomodoro_defaults {
        humantime::parse_duration(&defaults.long_break).map_err(|e| format!("Invalid long break duration in config '{}': {}. Use format like '15m', '30m', etc.", defaults.long_break, e))?
    } else {
        humantime::parse_duration("15m")?
    };

    let cycles = if let Some(c) = args.cycles {
        c
    } else if let Some(defaults) = &config.pomodoro_defaults {
        defaults.cycles
    } else {
        4
    };

    if args.pomodoro.is_some() || config.pomodoro_defaults.is_some() {
        // If duration is specified, calculate how many complete sessions can fit within that duration
        let session_duration = humantime::parse_duration(&args.duration)
            .map_err(|e| format!("Invalid duration '{}': {}. Use format like '25m', '1h', '30s', etc.", args.duration, e))?;
        
        let single_cycle_duration = pomodoro_duration + break_duration;
        let max_sessions = (session_duration.as_secs() / single_cycle_duration.as_secs()).max(1) as u32;
        let actual_cycles = cycles.min(max_sessions);
        
        for i in 1..=actual_cycles {
            println!("🍅 Starting Pomodoro Work Session {}/{}", i, actual_cycles);
            tokio::select! {
                _ = tokio::time::sleep(pomodoro_duration) => {},
                _ = rx.recv() => { println!("Pomodoro interrupted."); break; }
            }
            println!("✅ Work session {} completed!", i);

            if i == actual_cycles {
                // Only do long break if we completed all originally planned cycles, not just duration-limited cycles
                if actual_cycles == cycles {
                    println!("☕ Starting Long Break ({} minutes)", long_break_duration.as_secs() / 60);
                    tokio::select! {
                        _ = tokio::time::sleep(long_break_duration) => {},
                        _ = rx.recv() => { println!("Pomodoro interrupted."); break; }
                    }
                    println!("✅ Long Break finished! Great work completing all cycles!");
                } else {
                    println!("✅ Duration limit reached! Session completed.");
                }
                break;
            } else {
                println!("☕ Starting Short Break ({} minutes)", break_duration.as_secs() / 60);
                tokio::select! {
                    _ = tokio::time::sleep(break_duration) => {},
                    _ = rx.recv() => { println!("Pomodoro interrupted."); break; }
                }
                println!("✅ Short Break finished! Back to work.");
            }
        }
    } else {
        // If no pomodoro args, just sleep for the main duration
        let duration = humantime::parse_duration(&args.duration).map_err(|e| format!("Invalid duration '{}': {}. Use format like '25m', '1h', '30s', etc.", args.duration, e))?;
        tokio::select! {
            _ = tokio::time::sleep(duration) => {},
            _ = rx.recv() => { println!("Flow mode interrupted."); }
        }
    }

    stop_flow_mode(StopArgs {}).await?;

    Ok(())
}

async fn stop_flow_mode(_args: StopArgs) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(tx) = STOP_SIGNAL_SENDER.get() {
        let _ = tx.send(()); // Send stop signal
    }
    unblock_websites().await?;
    unblock_applications().await?;
    unmute_notifications().await?;
    if fs::metadata("flowmode.pid").is_ok() {
        fs::remove_file("flowmode.pid")?;
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("log.csv")?;
    let end_time = Local::now();
    writeln!(file, "{}", end_time.to_rfc3339())?;

    println!("🎉 Flow mode session completed and logged successfully!");

    Ok(())
}

async fn report_flow_sessions() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n--- Flow Mode Session Report ---");
    
    let content = match fs::read_to_string("log.csv") {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading log file: {}. Make sure you have completed at least one session.", e);
            return Ok(());
        }
    };

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() == 3 {
            match (DateTime::parse_from_rfc3339(parts[1]), DateTime::parse_from_rfc3339(parts[2])) {
                (Ok(start_time), Ok(end_time)) => {
                    let start_local = start_time.with_timezone(&Local);
                    let end_local = end_time.with_timezone(&Local);
                    let duration = end_local.signed_duration_since(start_local);

                    println!("Task: {}", parts[0]);
                    println!("  Start: {}", start_local.format("%Y-%m-%d %H:%M:%S"));
                    println!("  End:   {}", end_local.format("%Y-%m-%d %H:%M:%S"));
                    println!("  Duration: {} minutes", duration.num_minutes());
                    println!("--------------------------------");
                }
                _ => {
                    eprintln!("Warning: Skipping malformed entry on line {}: {}", line_num + 1, line);
                }
            }
        } else {
            eprintln!("Warning: Skipping incomplete entry on line {}: {}", line_num + 1, line);
        }
    }

    Ok(())
}

pub async fn run(command: CliCommand) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        CliCommand::Start(args) => start_flow_mode(args).await?,
        CliCommand::Stop(args) => stop_flow_mode(args).await?,
        CliCommand::Report => report_flow_sessions().await?,
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_duration_parsing_error_handling() {
        // Test that we get a helpful error message for invalid duration
        let result = humantime::parse_duration("5").map_err(|e| format!("Invalid duration '{}': {}. Use format like '25m', '1h', '30s', etc.", "5", e));
        
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("Invalid duration '5'"));
        assert!(error_msg.contains("Use format like"));
    }

    #[tokio::test]
    async fn test_pomodoro_duration_parsing_error() {
        let result = humantime::parse_duration("25").map_err(|e| format!("Invalid pomodoro duration '{}': {}. Use format like '25m', '1h', etc.", "25", e));
        
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("Invalid pomodoro duration '25'"));
        assert!(error_msg.contains("Use format like"));
    }
}
