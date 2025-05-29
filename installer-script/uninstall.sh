#!/bin/bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print with color
print_status() {
    echo -e "${GREEN}[*]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
}

print_error() {
    echo -e "${RED}[x]${NC} $1"
}

# Function to check if running as root
check_root() {
    if [ "$EUID" -ne 0 ]; then
        print_error "Please run as root (use sudo)"
        exit 1
    fi
}

# Function to uninstall from Linux
uninstall_linux() {
    print_status "Uninstalling ZLT from Linux..."

    # Stop and disable the service
    if systemctl is-active --quiet zlt@zlt.service; then
        print_status "Stopping ZLT service..."
        systemctl stop zlt@zlt.service
    fi
    
    if systemctl is-enabled --quiet zlt@zlt.service; then
        print_status "Disabling ZLT service..."
        systemctl disable zlt@zlt.service
    fi

    # Remove service file
    if [ -f "/etc/systemd/system/zlt@.service" ]; then
        print_status "Removing service file..."
        rm /etc/systemd/system/zlt@.service
        systemctl daemon-reload
    fi

    # Remove binary
    if [ -f "/usr/local/bin/zlt" ]; then
        print_status "Removing ZLT binary..."
        rm /usr/local/bin/zlt
    fi

    # Remove ZLT user (optional)
    if id "zlt" &>/dev/null; then
        print_warning "Do you want to remove the 'zlt' user? (y/N)"
        read -r response
        if [[ "$response" =~ ^([yY][eE][sS]|[yY])+$ ]]; then
            print_status "Removing 'zlt' user..."
            userdel zlt
        fi
    fi

    print_status "ZLT has been successfully uninstalled from Linux"
}

# Function to uninstall from macOS
uninstall_macos() {
    print_status "Uninstalling ZLT from macOS..."

    # Unload and remove LaunchDaemon
    if [ -f "/Library/LaunchDaemons/com.zlt.service.plist" ]; then
        print_status "Unloading and removing LaunchDaemon..."
        launchctl unload /Library/LaunchDaemons/com.zlt.service.plist
        rm /Library/LaunchDaemons/com.zlt.service.plist
    fi

    # Remove binary
    if [ -f "/usr/local/bin/zlt" ]; then
        print_status "Removing ZLT binary..."
        rm /usr/local/bin/zlt
    fi

    # Remove data directory
    if [ -d "/usr/local/var/zlt" ]; then
        print_status "Removing ZLT data directory..."
        rm -rf /usr/local/var/zlt
    fi

    # Handle log files
    if [ -f "/usr/local/var/log/zlt.err" ] || [ -f "/usr/local/var/log/zlt.out" ]; then
        print_warning "Do you want to remove ZLT log files? (y/N)"
        read -r response
        if [[ "$response" =~ ^([yY][eE][sS]|[yY])+$ ]]; then
            print_status "Removing log files..."
            rm -f /usr/local/var/log/zlt.err /usr/local/var/log/zlt.out
        fi
    fi

    print_status "ZLT has been successfully uninstalled from macOS"
}

# Show help if requested
if [ "$1" == "--help" ] || [ "$1" == "-h" ]; then
    echo "ZLT Uninstaller"
    echo "Usage:"
    echo "  sudo ./uninstall.sh  # Uninstall ZLT"
    echo "  ./uninstall.sh --help  # Show this help"
    exit 0
fi

# Main uninstallation logic
main() {
    # Detect OS and uninstall accordingly
    if [[ "$OSTYPE" == "darwin"* ]]; then
        check_root
        uninstall_macos
    else
        check_root
        uninstall_linux
    fi
}

main "$@" 