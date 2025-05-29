#!/bin/bash

# Set error handling
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Starting ZLT uninstallation...${NC}"

# Stop the ZLT service if running
echo -e "${YELLOW}Stopping ZLT service...${NC}"
if pgrep zlt > /dev/null; then
    killall zlt 2>/dev/null || true
    sleep 1
    # Force kill if still running
    if pgrep zlt > /dev/null; then
        killall -9 zlt 2>/dev/null || true
    fi
    echo -e "${GREEN}ZLT process stopped${NC}"
else
    echo -e "${YELLOW}No running ZLT process found${NC}"
fi

# Remove autostart entry
echo -e "${YELLOW}Removing autostart configuration...${NC}"
if [ -f ~/.config/autostart/zlt.desktop ]; then
    rm -f ~/.config/autostart/zlt.desktop
    echo -e "${GREEN}Autostart configuration removed${NC}"
else
    echo -e "${YELLOW}No autostart configuration found${NC}"
fi

# Remove binary
echo -e "${YELLOW}Removing ZLT binary...${NC}"
if [ -f ~/.local/bin/zlt ]; then
    rm -f ~/.local/bin/zlt
    echo -e "${GREEN}ZLT binary removed${NC}"
else
    echo -e "${YELLOW}ZLT binary not found in ~/.local/bin${NC}"
fi

# Check for system-wide installation
if [ -f /usr/local/bin/zlt ]; then
    echo -e "${YELLOW}Found system-wide installation. Removing...${NC}"
    sudo rm -f /usr/local/bin/zlt
    echo -e "${GREEN}System-wide binary removed${NC}"
fi

echo -e "${YELLOW}Note: Data directory ~/.local/share/zlt has been preserved${NC}"
echo -e "${YELLOW}To remove all data, run: rm -rf ~/.local/share/zlt${NC}"

echo -e "${GREEN}ZLT has been successfully uninstalled!${NC}" 