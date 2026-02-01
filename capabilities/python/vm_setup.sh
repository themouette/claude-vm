#!/bin/bash
set -e

echo "Installing Python 3..."

sudo DEBIAN_FRONTEND=noninteractive apt-get install -y \
  python3 python3-pip python3-venv

echo "Python installed successfully."
python3 --version
pip3 --version
