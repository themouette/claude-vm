#!/bin/bash
set -e

# Use LIMA_INSTANCE provided by executor
VM_NAME="${LIMA_INSTANCE}"

if [ -z "$VM_NAME" ]; then
    echo "Error: LIMA_INSTANCE environment variable not set"
    exit 1
fi

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

# Write config values to temp files for safe transfer
TEMP_DIR=$(mktemp -d)
trap 'rm -rf "$TEMP_DIR"' EXIT ERR INT TERM

echo -n "$GIT_USER_NAME" > "$TEMP_DIR/user.name"
echo -n "$GIT_USER_EMAIL" > "$TEMP_DIR/user.email"
echo -n "$GPG_SIGN" > "$TEMP_DIR/commit.gpgsign"
echo -n "$GPG_FORMAT" > "$TEMP_DIR/gpg.format"
echo -n "$SIGNING_KEY" > "$TEMP_DIR/user.signingkey"

# Copy config files to VM
if ! limactl copy "$TEMP_DIR/user.name" "$VM_NAME:/tmp/git-user-name"; then
    echo "Error: Failed to copy git config to VM"
    exit 1
fi

if ! limactl copy "$TEMP_DIR/user.email" "$VM_NAME:/tmp/git-user-email"; then
    echo "Error: Failed to copy git config to VM"
    exit 1
fi

if ! limactl copy "$TEMP_DIR/commit.gpgsign" "$VM_NAME:/tmp/git-commit-gpgsign"; then
    echo "Error: Failed to copy git config to VM"
    exit 1
fi

if ! limactl copy "$TEMP_DIR/gpg.format" "$VM_NAME:/tmp/git-gpg-format"; then
    echo "Error: Failed to copy git config to VM"
    exit 1
fi

if ! limactl copy "$TEMP_DIR/user.signingkey" "$VM_NAME:/tmp/git-user-signingkey"; then
    echo "Error: Failed to copy git config to VM"
    exit 1
fi

# Execute git config commands in VM, reading from temp files
if ! limactl shell "$VM_NAME" bash <<'SHELL_EOF'
set -e

# Configure git user identity
git config --global user.name "$(cat /tmp/git-user-name)"
git config --global user.email "$(cat /tmp/git-user-email)"

# Configure signing if enabled
GPG_SIGN=$(cat /tmp/git-commit-gpgsign)
if [ "$GPG_SIGN" = "true" ]; then
    git config --global commit.gpgsign true
    git config --global gpg.format "$(cat /tmp/git-gpg-format)"

    SIGNING_KEY=$(cat /tmp/git-user-signingkey)
    if [ -n "$SIGNING_KEY" ]; then
        git config --global user.signingkey "$SIGNING_KEY"
    fi
fi

# Clean up temp files
rm -f /tmp/git-user-name /tmp/git-user-email /tmp/git-commit-gpgsign /tmp/git-gpg-format /tmp/git-user-signingkey

echo "Git configured successfully"
SHELL_EOF
then
    echo "Error: Failed to configure git in VM"
    exit 1
fi

echo "Git user configured: $GIT_USER_NAME <$GIT_USER_EMAIL>"

# Show appropriate signing warnings
if [ "$GPG_SIGN" = "true" ]; then
    echo ""
    if [ "$GPG_FORMAT" = "ssh" ]; then
        echo "IMPORTANT: SSH commit signing detected"
        echo "  - SSH signing requires the SSH agent to be forwarded"
        echo "  - Use: claude-vm -A \"your command\""
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
