# ZLT - Zero Latency Transfer

A secure and feature-rich file server implementation in Rust, designed for fast and secure file transfers with zero latency. ZLT provides a modern web interface for easy file management, automatic tunnel creation for public access, and secure communication features.

## Features

- Web-based file browser with modern UI
- Directory listing and navigation
- File upload via drag-and-drop or file picker
- File download and preview (images, videos, audio, text)
- Folder zipping and download
- Ngrok integration for public URL tunneling
- Telegram notifications for server status
- AES-GCM encryption utilities
- Stealth functions for obfuscation

## Requirements

- Rust (2021 edition)
- Cargo package manager

## Building and Running

1. Clone the repository:

```bash
git clone https://github.com/yourusername/zlt.git
cd zlt
```

2. Build the application:

```bash
cargo build --release
```

3. Run the application:

```bash
./target/release/zlt
```

You can also use cargo to run the application directly:

```bash
cargo run --release
```

Access the file server at:
- Local: http://localhost:8000
- Public: The ngrok URL will be displayed in the console and sent via Telegram notifications

## Configuration

Edit the `src/config.rs` file to configure:

### Telegram Integration
1. Create a Telegram bot using [@BotFather](https://t.me/botfather)
2. Get your bot token and chat ID
3. Update the token and chat ID in the configuration

<<<<<<< Updated upstream
### Ngrok Integration
=======
### Tunneling Integration
#### Ngrok
>>>>>>> Stashed changes
1. Create a free account at [ngrok.com](https://ngrok.com)
2. Get your authentication token
3. Update the token in the configuration

### Other Settings
- Server port
- Security options
- File handling preferences

## Troubleshooting

### Common Issues
<<<<<<< Updated upstream
- If ngrok fails to start, ensure your authentication token is valid
=======
- If tunneling fails to start:
  - Ensure your ngrok authentication token is valid
  - Check your network connectivity
>>>>>>> Stashed changes
- If Telegram notifications aren't received, check your bot token and chat ID
- For upload issues, verify file permissions in the target directory

## Development

### Project Structure
- `src/main.rs` - Application entry point
- `src/file_server.rs` - File server implementation
- `src/ngrok.rs` - Ngrok tunnel handling
- `src/notification.rs` - Telegram notification system
- `src/encryption.rs` - Encryption utilities
- `src/config.rs` - Configuration management
- `src/stealth.rs` - Stealth functions
- `src/utils.rs` - Utility functions

### Building for Development

```bash
cargo build
cargo run
```

