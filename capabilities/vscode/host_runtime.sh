#!/bin/bash
# VSCode Remote-SSH configuration
# Writes a per-session Lima SSH config and ensures ~/.ssh/config includes a
# glob Include that covers all active sessions.
set -e

# SESSION_ID is injected by the host executor
SESSION_DIR="$HOME/.claude-vm/sessions/$SESSION_ID"
SSH_CONFIG="$SESSION_DIR/ssh/config"
INCLUDE_LINE="Include $HOME/.claude-vm/sessions/*/ssh/config"
HOST_SSH_CONFIG="$HOME/.ssh/config"

# Ensure the per-session ssh directory exists
mkdir -p "$SESSION_DIR/ssh"

# Create an empty SSH config if it does not exist
if [ ! -f "$SSH_CONFIG" ]; then
    touch "$SSH_CONFIG"
fi

# Ensure ~/.ssh/config exists
mkdir -p "$HOME/.ssh"
chmod 700 "$HOME/.ssh"
if [ ! -f "$HOST_SSH_CONFIG" ]; then
    touch "$HOST_SSH_CONFIG"
    chmod 600 "$HOST_SSH_CONFIG"
fi

# Idempotently prepend the glob Include line to ~/.ssh/config
if ! grep -qF "$INCLUDE_LINE" "$HOST_SSH_CONFIG" 2>/dev/null; then
    # Prepend the Include line (must come before any Host blocks)
    TMPFILE=$(mktemp)
    printf '%s\n\n' "$INCLUDE_LINE" > "$TMPFILE"
    cat "$HOST_SSH_CONFIG" >> "$TMPFILE"
    mv "$TMPFILE" "$HOST_SSH_CONFIG"
    chmod 600 "$HOST_SSH_CONFIG"
fi

# Write the Lima SSH config for this VM instance
# VM_NAME is injected by the host executor
if [ -n "$VM_NAME" ]; then
    limactl show-ssh --format=config "$VM_NAME" > "$SSH_CONFIG" 2>/dev/null || true
fi
