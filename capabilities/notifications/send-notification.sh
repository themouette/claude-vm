#!/bin/bash
# Helper script to send notifications from VM to host
#
# Usage:
#   send-notification.sh "Title" "Message" ["Subtitle"]
#   echo '{"title":"Title","message":"Message"}' | send-notification.sh

set -e

SOCKET="${CLAUDE_NOTIFICATION_SOCKET:-/tmp/claude-notifications.socket}"

# Function to send JSON to socket
send_json() {
  local json="$1"
  if [ ! -S "$SOCKET" ]; then
    echo "Error: Notification socket not found at $SOCKET" >&2
    echo "Make sure the notifications capability is enabled" >&2
    exit 1
  fi

  echo "$json" | nc -U -w1 "$SOCKET" || {
    echo "Error: Failed to send notification to $SOCKET" >&2
    exit 1
  }
}

# Check if input is piped
if [ -p /dev/stdin ]; then
  # Read JSON from stdin
  read -r json
  send_json "$json"
else
  # Build JSON from arguments
  if [ $# -lt 2 ]; then
    echo "Usage: $0 <title> <message> [subtitle]" >&2
    echo "   or: echo '{\"title\":\"...\",\"message\":\"...\"}' | $0" >&2
    exit 1
  fi

  title="$1"
  message="$2"
  subtitle="${3:-}"

  # Build JSON (simple escaping)
  title_escaped=$(echo "$title" | sed 's/"/\\"/g')
  message_escaped=$(echo "$message" | sed 's/"/\\"/g')

  if [ -n "$subtitle" ]; then
    subtitle_escaped=$(echo "$subtitle" | sed 's/"/\\"/g')
    json="{\"title\":\"$title_escaped\",\"message\":\"$message_escaped\",\"subtitle\":\"$subtitle_escaped\"}"
  else
    json="{\"title\":\"$title_escaped\",\"message\":\"$message_escaped\"}"
  fi

  send_json "$json"
fi

echo "Notification sent successfully"
