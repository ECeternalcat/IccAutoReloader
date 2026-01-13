<p align="center">
  <img src="https://github.com/ECeternalcat/IccAutoReloader/raw/master/icon.ico" alt="Icc Auto Reloader Icon" width="128" height="128">
</p>

<h1 align="center">Icc Auto Reloader</h1>

<p align="center">
  <a href="https://github.com/ECeternalcat/IccAutoReloader/releases/tag/Release">
    <img alt="Download Release" src="https://img.shields.io/badge/Download-Release-brightgreen?logo=windows" style="max-width: 100%;">
  </a>
  <img alt="License" src="https://img.shields.io/badge/license-GPL%20v3-blue">
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows-lightgrey?logo=windows">
  <img alt="Rust" src="https://img.shields.io/badge/rust-1.70%2B-orange?logo=rust">
</p>

<p align="center">
  A lightweight, high-performance ICC Profile Loader for Windows, written in Rust.
</p>

<p align="center">
  Inspired by the DisplayCAL Profile Loader, this utility ensures your display calibration remains active by automatically reloading it whenever Windows or other applications (like the Settings app or Action Center) reset the Video LUT (Gamma Ramp).
</p>

## Features

- **Automatic Reloading**: Detects display configuration changes, system wake-up events, and manual resets.
- **Robust Polling**: Continuously monitors the system Gamma Ramp (default every 3 seconds). If it detects the calibration has been lost (ramp becomes linear), it automatically re-applies your profile.
- **Multi-language Support**: Supports English, Chinese (简体中文), and Japanese (日本語).
- **Settings GUI**: Configurable polling interval (1–5 seconds) via a settings panel.
- **Small Footprint**: Written in Rust with minimal dependencies and no runtime overhead.
- **Broad Compatibility**: Supports Arm64, x86 and x64 Windows, from Windows 7 to Windows 11.

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

1. **First Run**: Upon launching, a setup wizard will appear to guide you through the initial configuration:
   - Select your preferred language (English / 中文 / 日本語)
   - Import an ICC profile or skip to use Windows default
   - Choose whether to run at startup

2. **Tray Icon**: Look for the Icc Auto Reloader icon in the system tray.
   - **Right-click**: Access the menu to:
     - **Reload Calibration**: Manually re-apply the current profile
     - **Import ICC Profile**: Load a new profile
     - **Enable Auto-Reload**: Toggle automatic polling on/off
     - **Run at Startup**: Toggle startup with Windows
     - **Settings**: Change polling interval and language
     - **Exit**: Quit the application (with confirmation dialog)
   - **Double-click**: Instantly reload the current calibration

3. **Exit Confirmation**: When clicking "Exit", a warning dialog will appear:
   - Message: *"Warning: After exiting, the system will no longer automatically load color calibration. Continue?"*
   - Click **Yes** to exit, or **No** to cancel and keep the app running

4. **Settings**: Open the "Settings..." menu to change:
   - Polling interval (1-5 seconds)
   - UI language (changes take effect immediately)

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
### Build Arm64
```bash
rustup target add aarch64-pc-windows-msvc
cargo build --release --target aarch64-pc-windows-msvc
```


The output will be in `target/release/` or `target/i686-pc-windows-msvc/release/`.

## License

This project is released under the same terms as the associated ArgyllCMS tools (GPL where applicable). Refer to `LICENSE` for details.
