#!/bin/bash
set -e

echo "Installing GitHub CLI..."

# Add GitHub CLI repository
sudo mkdir -p -m 755 /etc/apt/keyrings
wget -qO- https://cli.github.com/packages/githubcli-archive-keyring.gpg | sudo tee /etc/apt/keyrings/githubcli-archive-keyring.gpg > /dev/null
sudo chmod go+r /etc/apt/keyrings/githubcli-archive-keyring.gpg
echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | sudo tee /etc/apt/sources.list.d/github-cli.list > /dev/null

# Install gh
sudo DEBIAN_FRONTEND=noninteractive apt-get update
sudo DEBIAN_FRONTEND=noninteractive apt-get install -y gh

echo "GitHub CLI installed successfully."
gh --version

# Authenticate with GitHub using device flow
echo ""
echo "========================================================================="
echo "Authenticating with GitHub using device flow"
echo "========================================================================="
echo ""
echo "A code and URL will be displayed below."
echo "Copy the code, open the URL in your browser, and paste the code to authenticate."
echo ""

# Use device flow by providing empty input and setting git protocol to avoid prompts
gh auth login --git-protocol https --hostname github.com <<EOF


EOF
