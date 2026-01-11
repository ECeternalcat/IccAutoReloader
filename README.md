# Icc Auto Reloader

A lightweight, high-performance ICC Profile Loader for Windows, written in Rust. Inspired by the DisplayCAL Profile Loader, this utility ensures your display calibration remains active by automatically reloading it whenever Windows or other applications (like the Settings app or Action Center) reset the Video LUT (Gamma Ramp).

## Features

- **Automatic Reloading**: Detects display configuration changes, system wake-up events, and manual resets.
- **Robust Polling**: Continuously monitors the system Gamma Ramp (default every 3 seconds). If it detects the calibration has been lost (ramp becomes linear), it automatically re-applies your profile.
- **Silent Operation**: Runs entirely in the background. All command-line calls to `dispwin` are suppressed to prevent console window popups.
- **Multi-language Support**: Supports English, Chinese (简体中文), and Japanese (日本語).
- **Settings GUI**: Configurable polling interval (1–10 seconds) via a native Windows settings panel.
- **Small Footprint**: Written in Rust with minimal dependencies and no runtime overhead.
- **Broad Compatibility**: Supports both 32-bit (x86) and 64-bit (x64) Windows, from Windows 7 to Windows 11.

## Requirements

The application requires `dispwin.exe` (part of the ArgyllCMS suite) to function.
1. Create a folder named `bin` in the same directory as the executable.
2. Place `dispwin.exe` inside the `bin` folder.

Structure:
```
IccAutoReloader/
├── icc_auto_reloader.exe
└── bin/
    └── dispwin.exe
```

## Usage

1. **First Run**: Upon launching, the app will ask if you'd like to import an ICC profile.
2. **Tray Icon**: Look for the Icc Auto Reloader icon in the system tray.
   - **Right-click**: Access the menu to reload manually, import a new profile, change settings, or toggle startup/polling.
   - **Double-click**: Instantly reload the current calibration.
3. **Settings**: Open the "Settings..." menu to change the UI language or the background check frequency.

## Registry Usage (for Installers)

The application uses the following registry keys to store user preferences and ensure persistence:

- **App Settings**: `HKEY_CURRENT_USER\Software\IccAutoReloader`
  - Values: `Configured`, `PollingEnabled`, `PollingInterval`, `Language`.
- **Startup**: `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run`
  - Value: `IccAutoReloader` (path to executable).

*Note: The app also checks `HKEY_LOCAL_MACHINE\Software\IccAutoReloader` for shared defaults if user settings are not found.*

## Building from Source

To compile the project yourself, ensure you have the Rust toolchain installed.

### Build 64-bit
```bash
cargo build --release
```

### Build 32-bit
```bash
rustup target add i686-pc-windows-msvc
cargo build --release --target i686-pc-windows-msvc
```

The output will be in `target/release/` or `target/i686-pc-windows-msvc/release/`.

## License

This project is released under the same terms as the associated ArgyllCMS tools (GPL where applicable). Refer to `LICENSE` for details.
