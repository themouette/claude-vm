#!/bin/bash
set -e

VM_NAME="${1:-claude-vm}"

echo "Configuring git in VM..."

# Read git config from host
GIT_USER_NAME=$(git config --global user.name 2>/dev/null || echo "")
GIT_USER_EMAIL=$(git config --global user.email 2>/dev/null || echo "")

# Check if git is configured on host
if [ -z "$GIT_USER_NAME" ] || [ -z "$GIT_USER_EMAIL" ]; then
    echo ""
    echo "WARNING: Git user configuration not found on host"
    echo "Please configure git identity on your host machine:"
    echo "  git config --global user.name \"Your Name\""
    echo "  git config --global user.email \"your.email@example.com\""
    echo ""
    echo "Skipping git configuration..."
    exit 0
fi

# Check for commit signing
GPG_SIGN=$(git config --global commit.gpgsign 2>/dev/null || echo "false")
GPG_FORMAT=$(git config --global gpg.format 2>/dev/null || echo "openpgp")
SIGNING_KEY=$(git config --global user.signingkey 2>/dev/null || echo "")

# Create temporary script to run in VM
TEMP_SCRIPT=$(mktemp)
trap "rm -f $TEMP_SCRIPT" EXIT

cat > "$TEMP_SCRIPT" <<'EOF'
#!/bin/bash
set -e

# Configure git user identity
git config --global user.name "$GIT_USER_NAME"
git config --global user.email "$GIT_USER_EMAIL"

# Configure signing if enabled
if [ "$GPG_SIGN" = "true" ]; then
    git config --global commit.gpgsign true
    git config --global gpg.format "$GPG_FORMAT"

    if [ -n "$SIGNING_KEY" ]; then
        git config --global user.signingkey "$SIGNING_KEY"
    fi
fi

echo "Git configured successfully"
EOF

# Copy script to VM
TEMP_VM_SCRIPT="/tmp/git_setup_$(date +%s).sh"
limactl copy "$TEMP_SCRIPT" "$VM_NAME:$TEMP_VM_SCRIPT"

# Execute script in VM with environment variables
limactl shell "$VM_NAME" bash <<SHELL_EOF
export GIT_USER_NAME='$GIT_USER_NAME'
export GIT_USER_EMAIL='$GIT_USER_EMAIL'
export GPG_SIGN='$GPG_SIGN'
export GPG_FORMAT='$GPG_FORMAT'
export SIGNING_KEY='$SIGNING_KEY'

chmod +x "$TEMP_VM_SCRIPT"
"$TEMP_VM_SCRIPT"
rm -f "$TEMP_VM_SCRIPT"
SHELL_EOF

echo "Git user configured: $GIT_USER_NAME <$GIT_USER_EMAIL>"

# Show appropriate signing warnings
if [ "$GPG_SIGN" = "true" ]; then
    echo ""
    if [ "$GPG_FORMAT" = "ssh" ]; then
        echo "IMPORTANT: SSH commit signing detected"
        echo "  - SSH signing requires the SSH agent to be forwarded"
        echo "  - Use: claude-vm run --forward-ssh-agent"
        if [ -n "$SIGNING_KEY" ]; then
            echo "  - Your signing key: $SIGNING_KEY"
        fi
        echo "  - Ensure your SSH agent has the signing key loaded: ssh-add -l"
    else
        echo "IMPORTANT: GPG commit signing detected"
        echo "  - Enable the 'gpg' capability for signing to work in the VM"
        echo "  - Add 'gpg = true' to your .claude-vm.toml [tools] section"
        echo "  - Or use: claude-vm setup --gpg"
    fi
    echo ""
fi
