Of course. Here is a comprehensive `README.md` file for your `flowmode` tool.

-----

# Flow Mode

**A CLI tool to help you focus by blocking distractions and managing Pomodoro sessions. Built for developers, sysadmins, and anyone who lives in the terminal.**

`flowmode` is a lightweight, powerful focus tool written in Rust. It helps you enter a state of deep work by programmatically blocking distracting websites and applications, managing your work/break cycles with a Pomodoro timer, and logging your sessions for review.

-----

## Key Features ✨

  * **Website Blocker**: Block distracting websites by adding them to your system's `hosts` file.
  * **Application Blocker**: Automatically kill distracting applications (like Slack or Discord) at the start of a session.
  * **Pomodoro Timer**: Use the built-in Pomodoro timer to manage work and break cycles.
  * **Session Logging**: Log your focused work sessions to a `log.csv` file for productivity analysis.
  * **Highly Configurable**: Customize everything from the block lists to Pomodoro durations using a simple `config.toml` file.
  * **CLI-Native**: Control everything from your terminal, allowing for easy integration with scripts and aliases.

-----

## Installation

Ensure you have the Rust toolchain installed. You can install it from [rustup.rs](https://rustup.rs/).

1.  **Clone the repository:**

    ```sh
    git clone https://github.com/cliworld/flowmode.git
    cd flowmode
    ```

2.  **Build and install the binary:**

    ```sh
    cargo install --path .
    ```

    This will compile the `flowmode` binary and place it in your Cargo binary path (`~/.cargo/bin/`), making it available system-wide.

-----

## Usage

`flowmode` works **without administrator/root privileges** by using user-level alternatives:

- **Website blocking**: Uses a user-writable hosts file (with guidance for full effectiveness)
- **Application blocking**: Only terminates processes owned by the current user
- **Audio control**: Uses user-level audio controls when possible

For enhanced effectiveness, you can optionally run with elevated privileges to access system-level hosts file and audio controls.

### Start a Focus Session

The `start` command begins a focus session.

```sh
flowmode start --duration "1h 30m" --task "Working on the new feature"
```

**Arguments:**

| Flag | Shorthand | Description | Example |
| :--- | :--- | :--- | :--- |
| `--duration` | `-d` | **Required.** The total duration for the focus session. Accepts human-readable formats (e.g., "1h", "30m", "2h 15m"). | `--duration "45m"` |
| `--task` | `-t` | **Optional.** A description of the task for this session. This will be logged. | `--task "Refactoring the auth module"` |
| `--pomodoro` | | **Optional.** The duration of a single Pomodoro work session. | `--pomodoro "25m"` |
| `--break` | | **Optional.** The duration of a short break. | `--break "5m"` |
| `--long-break`| | **Optional.** The duration of a long break after a set number of cycles. | `--long-break "20m"` |
| `--cycles` | | **Optional.** The number of Pomodoro work sessions before a long break. | `--cycles 4` |

### Stop a Focus Session

The `stop` command immediately terminates the current focus session, unblocks all websites/apps, and logs the session end time.

```sh
flowmode stop
```

### Report on Past Sessions

The `report` command reads the `log.csv` file and displays a summary of your past focus sessions.

```sh
flowmode report
```

-----

## Configuration

You can create a `config.toml` file in the same directory where you run the command to customize `flowmode`'s behavior.

Here is an example `config.toml`:

```toml
# A list of websites to block. The format should be "IP FQDN".
block_list = [
    "127.0.0.1 reddit.com",
    "127.0.0.1 www.reddit.com",
    "127.0.0.1 news.ycombinator.com"
]

# A list of application executable names to kill at the start of a session.
app_block_list = [
    "slack.exe",    # For Windows
    "discord.exe",
    "Slack",        # For macOS
    "Discord"
]

# Default settings for the Pomodoro timer.
# These will be used if you don't provide command-line arguments.
[pomodoro_defaults]
pomodoro = "25m"
break = "5m"
long_break = "15m"
cycles = 4
```

-----

## Building from Source

If you want to contribute or build the project manually:

1.  Clone the repository: `git clone https://github.com/cliworld/flowmode.git`
2.  Navigate to the directory: `cd flowmode`
3.  Build the project: `cargo build --release`
4.  The executable will be located at `target/release/flowmode`.

### Running Tests

To run the test suite:

```sh
# Run all tests (use single thread to avoid environment variable conflicts)
cargo test -- --test-threads=1

# Run only unit tests
cargo test --test unit_tests

# Run only integration tests  
cargo test --test integration_tests -- --test-threads=1
```

The test suite includes:
- **Unit tests** (9 tests): Core functionality, configuration, CLI parsing
- **Integration tests** (11 tests): File operations, website blocking, cross-platform compatibility

## License

This project is licensed under the MIT License.