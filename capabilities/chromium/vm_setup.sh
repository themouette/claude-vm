#!/bin/bash
set -e

echo "Installing Chromium..."

sudo DEBIAN_FRONTEND=noninteractive apt-get install -y \
  chromium \
  fonts-liberation \
  xvfb

# Symlink so tools looking for google-chrome find Chromium
sudo ln -sf /usr/bin/chromium /usr/bin/google-chrome
sudo ln -sf /usr/bin/chromium /usr/bin/google-chrome-stable
sudo mkdir -p /opt/google/chrome
sudo ln -sf /usr/bin/chromium /opt/google/chrome/chrome

echo "Chromium installed successfully."
chromium --version || true
