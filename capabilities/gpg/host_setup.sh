#!/bin/bash
set -e

# Validate GPG is available on host
if ! command -v gpg &> /dev/null; then
  echo "Error: GPG not installed on host. Install it first:"
  echo "  macOS: brew install gnupg"
  echo "  Linux: sudo apt-get install gnupg"
  exit 1
fi

# Check GPG version
GPG_VERSION=$(gpg --version | head -1 | awk '{print $3}')
echo "Detected GPG version: $GPG_VERSION"

# Detect and validate extra socket
EXTRA_SOCKET=$(gpgconf --list-dir agent-extra-socket 2>/dev/null || echo "")
if [ -z "$EXTRA_SOCKET" ]; then
  echo "Error: Could not detect GPG agent extra socket"
  echo "Your GPG version may be too old (need 2.1.17+)"
  exit 1
fi

if [ ! -S "$EXTRA_SOCKET" ]; then
  echo "GPG agent extra socket not found at $EXTRA_SOCKET"
  echo "Starting GPG agent..."
  gpgconf --launch gpg-agent
  sleep 1

  if [ ! -S "$EXTRA_SOCKET" ]; then
    echo "Warning: Could not start GPG agent"
    echo "You may need to run 'gpgconf --launch gpg-agent' manually"
  fi
fi

echo "GPG extra socket: $EXTRA_SOCKET"

# Export and copy public keys to VM
# Use gpg --export to work with all GPG formats (keyboxd, old format, etc.)
echo "Exporting GPG public keys..."

# Check if there are any keys to export
if gpg --list-keys 2>/dev/null | grep -q "^pub"; then
  echo "Copying GPG public keys to VM..."

  # Wait for VM to be ready
  sleep 2

  # Export public keys to ASCII format
  if ! gpg --export --armor > /tmp/gpg-pubkeys.asc 2>&1; then
    echo "Error: Failed to export GPG public keys"
    exit 1
  fi

  # Copy public keys to VM (correct limactl syntax: copy <src> <vm>:<dest>)
  if ! limactl copy /tmp/gpg-pubkeys.asc "$LIMA_INSTANCE:/tmp/gpg-pubkeys.asc"; then
    echo "Error: Failed to copy GPG public keys to VM"
    rm -f /tmp/gpg-pubkeys.asc
    exit 1
  fi
  rm -f /tmp/gpg-pubkeys.asc

  # Export and copy trust database
  if ! gpg --export-ownertrust > /tmp/gpg-trust.txt 2>&1; then
    echo "Error: Failed to export GPG trust database"
    exit 1
  fi

  if ! limactl copy /tmp/gpg-trust.txt "$LIMA_INSTANCE:/tmp/gpg-trust.txt"; then
    echo "Error: Failed to copy GPG trust database to VM"
    rm -f /tmp/gpg-trust.txt
    exit 1
  fi
  rm -f /tmp/gpg-trust.txt

  echo "GPG keys copied to VM"
else
  echo "Warning: No GPG keys found on host"
  echo "You may need to generate a GPG key first"
fi

# Note about socket forwarding
echo ""
echo "GPG agent socket forwarding will be configured:"
echo "  Host socket: $EXTRA_SOCKET"
echo "  VM socket: /tmp/claude-vm-gpg-agent.socket"
echo ""
echo "This enables signing commits with your host's private keys and hardware tokens!"
