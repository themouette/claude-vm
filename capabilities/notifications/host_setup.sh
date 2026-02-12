#!/bin/bash
set -e

# Check if we're on macOS
if [[ "$OSTYPE" != "darwin"* ]]; then
  echo "Warning: Notification forwarding is currently only supported on macOS"
  echo "Skipping notification setup..."
  exit 0
fi

SOCKET_PATH="/tmp/claude-vm-notifications.socket"
PID_FILE="/tmp/claude-vm-notifications.pid"
LOG_FILE="/tmp/claude-vm-notifications.log"

# Stop any existing notification listener
if [ -f "$PID_FILE" ]; then
  OLD_PID=$(cat "$PID_FILE")
  if kill -0 "$OLD_PID" 2>/dev/null; then
    echo "Stopping existing notification listener (PID: $OLD_PID)..."
    kill "$OLD_PID" 2>/dev/null || true
    sleep 1
  fi
  rm -f "$PID_FILE"
fi

# Clean up old socket
rm -f "$SOCKET_PATH"

echo "Starting notification listener on $SOCKET_PATH..."

# Create the notification listener script
cat > /tmp/claude-vm-notification-listener.sh <<'LISTENER_EOF'
#!/bin/bash

SOCKET_PATH="/tmp/claude-vm-notifications.socket"
LOG_FILE="/tmp/claude-vm-notifications.log"

# Function to send macOS notification
send_notification() {
  local json="$1"

  # Parse JSON using jq if available, otherwise use basic grep/sed
  if command -v jq &> /dev/null; then
    title=$(echo "$json" | jq -r '.title // "Claude VM"')
    message=$(echo "$json" | jq -r '.message // ""')
    subtitle=$(echo "$json" | jq -r '.subtitle // ""')
  else
    # Fallback: basic parsing (not robust but works for simple cases)
    title=$(echo "$json" | grep -o '"title"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/.*"\([^"]*\)".*/\1/' || echo "Claude VM")
    message=$(echo "$json" | grep -o '"message"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/.*"\([^"]*\)".*/\1/' || echo "")
    subtitle=$(echo "$json" | grep -o '"subtitle"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/.*"\([^"]*\)".*/\1/' || echo "")
  fi

  # Build AppleScript notification
  local script="display notification \"$message\" with title \"$title\""
  if [ -n "$subtitle" ]; then
    script="display notification \"$message\" with title \"$title\" subtitle \"$subtitle\""
  fi

  # Send notification
  osascript -e "$script" 2>&1 | tee -a "$LOG_FILE"
  echo "[$(date)] Sent notification: $title - $message" >> "$LOG_FILE"
}

# Listen on Unix socket
echo "[$(date)] Starting notification listener on $SOCKET_PATH" >> "$LOG_FILE"

# Use socat if available (more reliable), otherwise fall back to nc
if command -v socat &> /dev/null; then
  echo "[$(date)] Using socat for socket listening" >> "$LOG_FILE"
  # socat is more reliable for Unix socket servers
  while true; do
    socat UNIX-LISTEN:"$SOCKET_PATH",fork EXEC:'/bin/bash -c "
      read -r line
      if [ -n \"\$line\" ]; then
        echo \"[\$(date)] Received: \$line\" >> \"$LOG_FILE\"
        # Parse and send notification
        if command -v jq &> /dev/null; then
          title=\$(echo \"\$line\" | jq -r \".title // \\\"Claude VM\\\"\")
          message=\$(echo \"\$line\" | jq -r \".message // \\\"\\\"\")
          subtitle=\$(echo \"\$line\" | jq -r \".subtitle // \\\"\\\"\")
        else
          title=\"Claude VM\"
          message=\$(echo \"\$line\" | grep -o \"\\\"message\\\"[[:space:]]*:[[:space:]]*\\\"[^\\\"]*\\\"\" | sed \"s/.*\\\"\\([^\\\"]*\\)\\\".*/\\1/\" || echo \"\")
        fi
        if [ -n \"\$subtitle\" ]; then
          osascript -e \"display notification \\\"\$message\\\" with title \\\"\$title\\\" subtitle \\\"\$subtitle\\\"\" 2>&1 | tee -a \"$LOG_FILE\"
        else
          osascript -e \"display notification \\\"\$message\\\" with title \\\"\$title\\\"\" 2>&1 | tee -a \"$LOG_FILE\"
        fi
        echo \"[\$(date)] Sent notification: \$title - \$message\" >> \"$LOG_FILE\"
      fi
    "'
    echo "[$(date)] socat exited, restarting..." >> "$LOG_FILE"
    sleep 1
  done
else
  echo "[$(date)] Using nc for socket listening (socat not available)" >> "$LOG_FILE"
  # Fall back to nc (less reliable but works)
  while true; do
    # Listen for one connection, handle it, then loop
    nc -U -l "$SOCKET_PATH" | while IFS= read -r line; do
      if [ -n "$line" ]; then
        echo "[$(date)] Received: $line" >> "$LOG_FILE"
        send_notification "$line"
      fi
    done
    sleep 0.1
  done
fi
LISTENER_EOF

chmod +x /tmp/claude-vm-notification-listener.sh

# Start the listener in the background
nohup /tmp/claude-vm-notification-listener.sh > /dev/null 2>&1 &
LISTENER_PID=$!

# Save the PID
echo "$LISTENER_PID" > "$PID_FILE"

echo "Notification listener started (PID: $LISTENER_PID)"
echo "Logs: $LOG_FILE"
echo ""
echo "Test notification from host:"
echo '  echo '"'"'{"title":"Test","message":"Hello from VM"}'"'"' | nc -U /tmp/claude-vm-notifications.socket'
echo ""

# Give it a moment to start
sleep 1

# Verify the socket was created
if [ ! -S "$SOCKET_PATH" ]; then
  echo "Warning: Socket not created. Check logs at $LOG_FILE"
else
  echo "✓ Notification socket ready at $SOCKET_PATH"
fi
