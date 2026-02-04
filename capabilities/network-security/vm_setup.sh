#!/bin/bash
# Network security VM setup
# Installs mitmproxy and CA certificate inside the VM

set -e

echo "Setting up network security in VM..."

# Install mitmproxy in the VM
echo "Installing mitmproxy..."
sudo DEBIAN_FRONTEND=noninteractive apt-get update
sudo DEBIAN_FRONTEND=noninteractive apt-get install -y mitmproxy netcat-openbsd

# Generate CA certificate in the VM
echo "Generating mitmproxy CA certificate..."
mkdir -p ~/.mitmproxy

# Run mitmproxy briefly to generate certificates
# Use timeout to ensure it doesn't hang
timeout 5 mitmproxy --mode regular@8080 2>/dev/null || true
sleep 1

# Verify certificate was generated
if [ ! -f ~/.mitmproxy/mitmproxy-ca-cert.pem ]; then
    echo "ERROR: CA certificate not generated"
    exit 1
fi

# Install CA certificate in system trust store
echo "Installing CA certificate in system trust store..."
sudo cp ~/.mitmproxy/mitmproxy-ca-cert.pem \
    /usr/local/share/ca-certificates/mitmproxy-ca.crt
sudo update-ca-certificates

echo "✓ mitmproxy installed"
echo "✓ CA certificate installed in system trust store"
echo "✓ Network security VM setup complete"
