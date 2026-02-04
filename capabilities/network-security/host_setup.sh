#!/bin/bash
# Network security host setup
# Installs mitmproxy on the host machine for filtering VM traffic

set -e

echo "Setting up network security on host..."

# Detect OS
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "Installing mitmproxy on macOS..."
    if command -v brew &> /dev/null; then
        brew install mitmproxy
    else
        echo "ERROR: Homebrew is required to install mitmproxy on macOS"
        echo "Install Homebrew from https://brew.sh/"
        exit 1
    fi
elif [[ -f /etc/debian_version ]]; then
    echo "Installing mitmproxy on Debian/Ubuntu..."
    sudo apt-get update
    sudo apt-get install -y mitmproxy
elif [[ -f /etc/redhat-release ]]; then
    echo "Installing mitmproxy on RHEL/CentOS/Fedora..."
    sudo dnf install -y mitmproxy || sudo yum install -y mitmproxy
else
    echo "ERROR: Unsupported OS for automatic mitmproxy installation"
    echo "Please install mitmproxy manually: https://mitmproxy.org/"
    exit 1
fi

# Verify installation
if ! command -v mitmproxy &> /dev/null; then
    echo "ERROR: mitmproxy installation failed"
    exit 1
fi

echo "✓ mitmproxy installed successfully"

# Create config directory
MITMPROXY_DIR="$HOME/.mitmproxy"
mkdir -p "$MITMPROXY_DIR"

# Generate CA certificate if it doesn't exist
if [ ! -f "$MITMPROXY_DIR/mitmproxy-ca-cert.pem" ]; then
    echo "Generating mitmproxy CA certificate..."
    # Run mitmproxy briefly to generate certificates
    timeout 5 mitmproxy --set confdir="$MITMPROXY_DIR" 2>/dev/null || true
    sleep 1
fi

if [ -f "$MITMPROXY_DIR/mitmproxy-ca-cert.pem" ]; then
    echo "✓ CA certificate available at: $MITMPROXY_DIR/mitmproxy-ca-cert.pem"
else
    echo "WARNING: CA certificate not found. It will be generated on first proxy start."
fi

# Create network policy script directory
POLICY_DIR="$HOME/.claude-vm/network-policies"
mkdir -p "$POLICY_DIR"

echo "✓ Network security host setup complete"
echo ""
echo "Next steps:"
echo "  1. VMs will automatically use the proxy when network security is enabled"
echo "  2. Start the proxy with: claude-vm network start-proxy (automatic on VM start)"
echo "  3. Configure policies with: claude-vm network allow <domain>"
