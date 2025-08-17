# FlowMode v1.0.0 - Distribution Package

## What's Included

- `flowmode.exe` - The main FlowMode application (optimized release build)
- `nircmd.exe` - Bundled NirCmd utility for Windows notification muting
- `assets/` - Additional NirCmd files and documentation
- `NIRCMD_LICENSE.txt` - NirCmd licensing information

## Quick Start

The application is now fully self-contained! Simply run:

```bash
# Basic focus session
.\flowmode.exe start --duration 25m --task "Deep work"

# Pomodoro session
.\flowmode.exe start --duration 2h --pomodoro 25m --break 5m --cycles 4

# View your productivity report
.\flowmode.exe report
```

## Key Features

✅ **Works out of the box** - No additional downloads required
✅ **Notification muting** - Automatically mutes Windows notifications using bundled NirCmd
✅ **Clear error messages** - Helpful guidance when you make mistakes
✅ **Comprehensive help** - Run `.\flowmode.exe start --help` for examples

## Duration Format Examples

- `25m` - 25 minutes
- `1h` - 1 hour  
- `90m` - 90 minutes
- `1h30m` - 1 hour 30 minutes
- `2h` - 2 hours

## Correct Command Examples

```bash
# Your original command now works perfectly:
.\flowmode.exe start --duration 15m --pomodoro 25m

# With all options:
.\flowmode.exe start --duration 2h --pomodoro 25m --break 5m --long-break 15m --cycles 4 --task "Deep work session"
```

## Changes in This Version

1. **Fixed NumberExpected(0) error** - Now shows helpful error messages
2. **Bundled NirCmd** - No more "nircmd not found" warnings  
3. **Improved help** - All options now show format examples
4. **Better CSV logging** - Handles missing tasks correctly
5. **Enhanced error handling** - All edge cases properly handled

## File Structure

```
flowmode/
├── flowmode.exe          # Main application
├── nircmd.exe           # Notification muting utility
├── assets/
│   ├── nircmd.exe       # Backup copy
│   ├── nircmdc.exe      # Console version
│   └── NirCmd.chm       # Documentation
├── NIRCMD_LICENSE.txt   # NirCmd license
└── DISTRIBUTION_README.md # This file
```

Enjoy focused productivity with FlowMode! 🚀