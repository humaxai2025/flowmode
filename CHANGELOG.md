# Changelog

All notable changes to this project will be documented in this file.

## [1.0.0] - 2025-08-10

### Added
- 🎉 **Initial stable release of Flow Mode!**
- Complete focus session management with website and application blocking
- Pomodoro timer integration with customizable work/break cycles
- Comprehensive cross-platform support (Windows, macOS, Linux)
- Configuration system via `config.toml` file
- Session logging to CSV file for productivity tracking
- Slack webhook integration for status notifications
- Whitelist mode for selective website access during focus sessions
- Better user feedback with emojis and progress indicators
- Comprehensive test suite with unit and integration tests

### Improved
- Updated to latest sysinfo crate (0.31.0) with proper API usage
- Enhanced error handling throughout the application
- Cross-platform notification muting (Windows: nircmd, macOS: osascript, Linux: amixer)
- Better whitelist functionality that properly handles domain blocking/unblocking
- Improved CLI argument parsing and validation
- More robust file operations with proper backup/restore functionality

### Technical Details
- Built with Rust 2021 edition
- Async/await support using tokio
- Proper signal handling for graceful shutdown
- Environment variable support for testing
- Comprehensive documentation and examples

### Dependencies
- clap 4.0 for CLI parsing
- tokio 1.0 for async runtime
- reqwest 0.11 for HTTP requests (Slack integration)
- sysinfo 0.31 for system process management
- chrono 0.4 for timestamp handling
- humantime 2.1 for duration parsing
- serde & toml for configuration management

### Breaking Changes
- N/A (initial release)

### Security
- Safe process killing with proper error handling
- Secure hosts file modification with backup/restore
- No sensitive data logging or exposure
- Defensive coding practices throughout

---

## Future Releases

See the [GitHub Issues](https://github.com/cliworld/flowmode/issues) for planned features and improvements.