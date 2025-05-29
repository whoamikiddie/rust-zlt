#!/bin/bash

# Check if ZLT binary exists in current directory
if [ ! -f "zlt" ]; then
    echo "Error: ZLT binary not found in current directory"
    echo "Please make sure 'zlt' binary is in the same directory as this script"
    exit 1
fi

# Create autostart directory if it doesn't exist
mkdir -p ~/.config/autostart

# Create local bin directory if it doesn't exist
mkdir -p ~/.local/bin

# Copy the ZLT binary to user's local bin
cp ./zlt ~/.local/bin/

# Make it executable
chmod +x ~/.local/bin/zlt

# Create data directory
mkdir -p ~/.local/share/zlt

# Create desktop entry for autostart with improved configuration
cat > ~/.config/autostart/zlt.desktop << EOL
[Desktop Entry]
Type=Application
Name=ZLT
Comment=Zero Latency Transfer File Server
Exec=bash -c 'PATH=\$PATH:\$HOME/.local/bin zlt'
Terminal=false
X-GNOME-Autostart-enabled=true
StartupNotify=true
Icon=network-server
Categories=Network;FileTransfer;
StartupWMClass=zlt
EOL

# Add PATH to .profile if not already present
if ! grep -q '\.local/bin' ~/.profile; then
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.profile
fi

# Make the desktop entry executable
chmod +x ~/.config/autostart/zlt.desktop

echo "✅ ZLT has been set up for autostart on user login"
echo "✅ The application will start automatically on your next login"
echo "✅ You can find the binary in: ~/.local/bin/zlt"
echo "✅ Data directory: ~/.local/share/zlt"
echo -e "\nℹ️  To start ZLT now, run: zlt"
echo "ℹ️  To source the updated PATH without logging out, run: source ~/.profile"
echo -e "\nNote: If ZLT doesn't start automatically after login, try logging out and back in" 