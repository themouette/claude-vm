#!/bin/bash
set -e

echo "Installing Claude Code..."

curl -fsSL https://claude.ai/install.sh | bash

# Add to PATH
echo "export PATH=\$HOME/.local/bin:\$HOME/.claude/local/bin:\$PATH" >> ~/.bashrc

echo "Claude Code installed successfully"
