#!/bin/bash
# Network security VM setup
# Installs CA certificate and configures proxy in the VM template

set -e

echo "Setting up network security in VM..."

# The CA certificate should be copied from host to VM
# This happens via a temporary mount during setup
CA_CERT_SOURCE="/tmp/mitmproxy-ca-cert.pem"

if [ ! -f "$CA_CERT_SOURCE" ]; then
    echo "WARNING: CA certificate not found at $CA_CERT_SOURCE"
    echo "The certificate will need to be installed manually or on first runtime."
    exit 0
fi

# Install CA certificate for HTTPS inspection
echo "Installing mitmproxy CA certificate..."

# Detect OS and install accordingly
if [ -f /etc/debian_version ]; then
    # Debian/Ubuntu
    sudo cp "$CA_CERT_SOURCE" /usr/local/share/ca-certificates/mitmproxy-ca.crt
    sudo update-ca-certificates
elif [ -f /etc/redhat-release ]; then
    # RHEL/CentOS/Fedora
    sudo cp "$CA_CERT_SOURCE" /etc/pki/ca-trust/source/anchors/mitmproxy-ca.crt
    sudo update-ca-trust
else
    echo "WARNING: Unknown OS, CA certificate not installed"
    exit 0
fi

echo "✓ CA certificate installed in system trust store"

# Configure environment to use proxy by default
# These will be overridden at runtime with actual proxy address
cat > /tmp/proxy-env.sh << 'EOF'
# Network security proxy configuration
# Configured by claude-vm network-security capability
export HTTP_PROXY="http://host.docker.internal:8080"
export HTTPS_PROXY="http://host.docker.internal:8080"
export http_proxy="$HTTP_PROXY"
export https_proxy="$HTTPS_PROXY"
# Don't proxy localhost and private networks
export NO_PROXY="localhost,127.0.0.1,10.0.0.0/8,172.16.0.0/12,192.168.0.0/16"
export no_proxy="$NO_PROXY"
EOF

sudo mv /tmp/proxy-env.sh /etc/profile.d/claude-vm-proxy.sh
sudo chmod 644 /etc/profile.d/claude-vm-proxy.sh

echo "✓ Proxy environment variables configured"
echo "✓ Network security VM setup complete"
