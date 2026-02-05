#!/bin/bash
set -e

echo "Configuring GPG in VM..."

# Disable GPG agent autostart in VM (we'll use the forwarded agent)
mkdir -p ~/.gnupg
cat > ~/.gnupg/gpg-agent.conf << 'EOF'
# Disable autostart - we use the forwarded agent from host
no-autostart
EOF
chmod 600 ~/.gnupg/gpg-agent.conf

# Mask the gpg-agent systemd services to prevent conflicts
if command -v systemctl &> /dev/null; then
  systemctl --user mask gpg-agent.service gpg-agent.socket gpg-agent-ssh.socket gpg-agent-extra.socket gpg-agent-browser.socket 2>/dev/null || true
fi

# Configure SSH for socket forwarding
sudo mkdir -p /etc/ssh/sshd_config.d
echo 'StreamLocalBindUnlink yes' | sudo tee /etc/ssh/sshd_config.d/gpg-forward.conf > /dev/null
sudo chmod 644 /etc/ssh/sshd_config.d/gpg-forward.conf

# Reload SSH daemon
sudo systemctl reload sshd 2>/dev/null || sudo systemctl reload ssh 2>/dev/null || true

# Create directory for forwarded socket
mkdir -p /run/user/$(id -u)/gnupg
chmod 700 /run/user/$(id -u)/gnupg

# Setup GPG directory with correct permissions first
echo "Setting up GPG directory..."
mkdir -p ~/.gnupg
chmod 700 ~/.gnupg

# Import public keys if available
if [ -f /tmp/gpg-pubkeys.asc ]; then
  echo "Importing GPG public keys..."

  # Import the keys
  gpg --import /tmp/gpg-pubkeys.asc 2>&1 || {
    echo "Warning: Failed to import GPG keys"
  }

  # Import trust
  if [ -f /tmp/gpg-trust.txt ]; then
    gpg --import-ownertrust < /tmp/gpg-trust.txt 2>&1 || true
    rm /tmp/gpg-trust.txt
  fi

  rm /tmp/gpg-pubkeys.asc

  # Fix permissions on all GPG files
  chmod 600 ~/.gnupg/* 2>/dev/null || true

  echo "GPG public keys imported successfully"
  echo ""
  echo "Available keys:"
  gpg --list-keys 2>/dev/null || echo "  (none yet)"
else
  echo "No GPG public keys to import (file not found at /tmp/gpg-pubkeys.asc)"
  echo "This might mean:"
  echo "  - No GPG keys exist on the host"
  echo "  - The host_setup script failed to export keys"
fi

echo ""
echo "GPG is installed in the VM."
echo "GPG agent socket will be forwarded from host to VM."
echo "This enables:"
echo "  • Signing with host's private keys"
echo "  • Using hardware tokens (Yubikey, etc.)"
echo "  • Accessing host's gpg-agent from VM"
