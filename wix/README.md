# ZLT System Monitor MSI Installer Setup

This directory contains the configuration files needed to build an MSI installer for ZLT System Monitor using WiX Toolset.

## Files

- `main.wxs`: The main WiX source file that defines the installer structure
- `license.rtf`: The license agreement displayed during installation
- `wix.toml`: Configuration for cargo-wix integration

## Building the Installer

To build the MSI installer, follow these steps:

1. Install WiX Toolset
   - Download from: https://wixtoolset.org/releases/
   - Add the WiX tools to your PATH

2. Install cargo-wix
   ```
   cargo install cargo-wix
   ```

3. Build a release version of ZLT
   ```
   cargo build --release
   ```

4. Build the MSI installer
   ```
   cargo wix
   ```

5. The MSI installer will be generated in the `installer` directory

## Customization

To customize the installer:

- Edit `main.wxs` to change installation options, components, or features
- Replace `license.rtf` with your own license agreement
- Update `wix.toml` to configure build options

## Features

The installer includes:

- Start menu shortcuts
- Optional autostart capability
- Registry entries for installation tracking
- Proper uninstallation support
- Launch application after install option
