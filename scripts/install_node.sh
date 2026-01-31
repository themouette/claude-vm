#!/bin/bash
set -e

echo "Installing Node.js 22..."

curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
sudo DEBIAN_FRONTEND=noninteractive apt-get install -y nodejs

echo "Node.js installed successfully."
node --version
npm --version
