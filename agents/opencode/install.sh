#!/bin/bash
set -e

echo "Installing OpenCode..."

# Check for npm
if ! command -v npm &> /dev/null; then
    echo "Error: npm is required to install OpenCode"
    echo "Please enable the 'node' capability: claude-vm setup --node"
    exit 1
fi

# Install via npm
npm install -g @opencode-ai/opencode

# Verify installation
if ! command -v opencode &> /dev/null; then
    echo "Error: OpenCode installation failed"
    exit 1
fi

echo "OpenCode installed successfully"
