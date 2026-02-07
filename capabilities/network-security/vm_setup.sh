#!/bin/bash
# Network security VM setup
# Installs mitmproxy from official binaries and generates CA certificate
#
# Note: netcat-openbsd is already installed via packages.system

set -e

echo "Setting up network security in VM..."

# Mitmproxy version to install
MITMPROXY_VERSION="12.2.1"

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
    x86_64)
        MITMPROXY_ARCH="x86_64"
        ;;
    aarch64|arm64)
        MITMPROXY_ARCH="aarch64"
        ;;
    *)
        echo "ERROR: Unsupported architecture: $ARCH"
        echo "Mitmproxy binaries are available for x86_64 and aarch64 only"
        exit 1
        ;;
esac

# Download and install mitmproxy binary
echo "Installing mitmproxy ${MITMPROXY_VERSION} (${MITMPROXY_ARCH})..."

DOWNLOAD_URL="https://downloads.mitmproxy.org/${MITMPROXY_VERSION}/mitmproxy-${MITMPROXY_VERSION}-linux-${MITMPROXY_ARCH}.tar.gz"
TEMP_DIR=$(mktemp -d)

echo "  Downloading from: $DOWNLOAD_URL"
if ! curl -fsSL "$DOWNLOAD_URL" -o "$TEMP_DIR/mitmproxy.tar.gz"; then
    echo "ERROR: Failed to download mitmproxy"
    echo "URL: $DOWNLOAD_URL"
    rm -rf "$TEMP_DIR"
    exit 1
fi

echo "  Extracting binaries..."
tar -xzf "$TEMP_DIR/mitmproxy.tar.gz" -C "$TEMP_DIR"

echo "  Installing to /usr/local/bin..."
sudo install -m 755 "$TEMP_DIR/mitmproxy" /usr/local/bin/
sudo install -m 755 "$TEMP_DIR/mitmdump" /usr/local/bin/
sudo install -m 755 "$TEMP_DIR/mitmweb" /usr/local/bin/

# Cleanup
rm -rf "$TEMP_DIR"

# Verify installation
if ! command -v mitmproxy &> /dev/null; then
    echo "ERROR: mitmproxy not found in PATH after installation"
    exit 1
fi

echo "✓ mitmproxy installed: $(mitmproxy --version | head -1)"

# Generate CA certificate
echo "Generating mitmproxy CA certificate..."
mkdir -p ~/.mitmproxy

# Generate certificates by starting mitmdump in background and killing it
# mitmdump is the non-interactive version, better for scripting
mitmdump --set confdir=~/.mitmproxy >/dev/null 2>&1 &
MITM_PID=$!

# Wait for certificate generation (usually happens within 1 second)
sleep 2

# Kill the process
kill $MITM_PID 2>/dev/null || true
wait $MITM_PID 2>/dev/null || true

# Verify certificate was generated
if [ ! -f ~/.mitmproxy/mitmproxy-ca-cert.pem ]; then
    echo "ERROR: CA certificate not generated"
    echo "Certificate directory contents:"
    ls -la ~/.mitmproxy/ || echo "Directory not found"
    exit 1
fi

# Install CA certificate in system trust store
echo "Installing CA certificate in system trust store..."
sudo cp ~/.mitmproxy/mitmproxy-ca-cert.pem \
    /usr/local/share/ca-certificates/mitmproxy-ca.crt
sudo update-ca-certificates

echo "✓ CA certificate installed in system trust store"
echo "✓ Network security VM setup complete"
