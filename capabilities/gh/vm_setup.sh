#!/bin/bash
set -e

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
