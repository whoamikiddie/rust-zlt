#!/bin/bash

# Function to check if running as root
check_root() {
    if [ "$EUID" -ne 0 ]; then
        echo "Please run as root (use sudo)"
        exit 1
    fi
}

# Function to install on Linux
install_linux() {
    echo "Installing ZLT for Linux..."
    
    # Create zlt user if it doesn't exist
    if ! id "zlt" &>/dev/null; then
        useradd -r -s /bin/false zlt
    fi

    # Create necessary directories
    mkdir -p /var/lib/zlt
    chown zlt:zlt /var/lib/zlt
    chmod 755 /var/lib/zlt

    # Copy binary
    cp target/release/zlt /usr/local/bin/
    chmod +x /usr/local/bin/zlt
    
    # Setup systemd service
    cp installer-script/zlt.service /etc/systemd/system/zlt@.service
    
    # Set proper permissions for the binary
    chown root:root /usr/local/bin/zlt
    chmod 755 /usr/local/bin/zlt
    
    # Reload systemd and enable service
    systemctl daemon-reload
    systemctl enable zlt@zlt.service
    systemctl start zlt@zlt.service
    
    echo "ZLT has been installed and started. Service status:"
    systemctl status zlt@zlt.service
}

# Function to install on macOS
install_macos() {
    echo "Installing ZLT for macOS..."
    
    # Create necessary directories
    mkdir -p /usr/local/var/zlt
    mkdir -p /usr/local/var/log
    
    # Copy binary
    cp target/release/zlt /usr/local/bin/
    chmod +x /usr/local/bin/zlt
    
    # Setup LaunchDaemon (system-wide)
    cp installer-script/com.zlt.service.plist /Library/LaunchDaemons/
    
    # Set proper ownership and permissions
    chown root:wheel /Library/LaunchDaemons/com.zlt.service.plist
    chmod 644 /Library/LaunchDaemons/com.zlt.service.plist
    chown -R root:wheel /usr/local/var/zlt
    chmod 755 /usr/local/var/zlt
    
    # Load the service
    launchctl load /Library/LaunchDaemons/com.zlt.service.plist
    
    echo "ZLT has been installed and started as a system service."
}

# Function to uninstall
uninstall() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        echo "Uninstalling ZLT from macOS..."
        launchctl unload /Library/LaunchDaemons/com.zlt.service.plist
        rm /Library/LaunchDaemons/com.zlt.service.plist
        rm /usr/local/bin/zlt
        rm -rf /usr/local/var/zlt
        echo "Note: Log files in /usr/local/var/log/zlt.* are preserved."
    else
        echo "Uninstalling ZLT from Linux..."
        systemctl stop zlt@zlt.service
        systemctl disable zlt@zlt.service
        rm /etc/systemd/system/zlt@.service
        rm /usr/local/bin/zlt
        systemctl daemon-reload
    fi
    echo "ZLT has been uninstalled."
}

# Main installation logic
main() {
    # Check if uninstall flag is provided
    if [ "$1" == "--uninstall" ]; then
        check_root
        uninstall
        exit 0
    fi
    
    # Check if binary exists
    if [ ! -f "target/release/zlt" ]; then
        echo "Error: ZLT binary not found. Please build the project first with 'cargo build --release'"
        exit 1
    fi
    
    # Detect OS and install accordingly
    if [[ "$OSTYPE" == "darwin"* ]]; then
        install_macos
    else
        check_root
        install_linux
    fi
}

# Show help if requested
if [ "$1" == "--help" ] || [ "$1" == "-h" ]; then
    echo "ZLT Installer"
    echo "Usage:"
    echo "  sudo ./install.sh         # Install ZLT"
    echo "  sudo ./install.sh --uninstall  # Uninstall ZLT"
    echo "  ./install.sh --help       # Show this help"
    exit 0
fi

main "$@" 