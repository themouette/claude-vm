# Testing the Notification Capability

This guide shows how to test the notification forwarding capability.

## Prerequisites

- macOS host (for notification display)
- `socat` recommended (install via `brew install socat`)
- Terminal with notification permissions

## Step 1: Enable the Capability

Add to your `.claude-vm.toml`:

```toml
[vm]
capabilities = ["notifications"]
```

Or for testing, create a test project:

```bash
mkdir -p /tmp/notification-test
cd /tmp/notification-test

cat > .claude-vm.toml <<'EOF'
[vm]
capabilities = ["notifications"]
name = "notification-test"

[mounts]
writable = ["/tmp/notification-test"]
EOF
```

## Step 2: Setup the VM

```bash
claude-vm setup
```

This will:
1. Start the notification listener on the host
2. Create the Unix socket forwarding
3. Make the socket available in the VM at `/tmp/claude-notifications.socket`

**Verify on Host:**

```bash
# Check listener is running
ps aux | grep claude-vm-notification-listener

# Check PID file exists
cat /tmp/claude-vm-notifications.pid

# Check socket exists
ls -la /tmp/claude-vm-notifications.socket

# View logs
tail -f /tmp/claude-vm-notifications.log
```

## Step 3: Test from Host (Direct)

Before testing through the VM, verify the listener works on the host:

```bash
# Simple test
echo '{"title":"Test","message":"Hello World"}' | nc -U /tmp/claude-vm-notifications.socket

# With subtitle
echo '{"title":"Build Complete","message":"Success","subtitle":"100% tests passed"}' | nc -U /tmp/claude-vm-notifications.socket
```

You should see a macOS notification appear.

## Step 4: Test from VM

Start a shell in the VM:

```bash
claude-vm shell
```

Inside the VM:

```bash
# Check socket exists
ls -la /tmp/claude-notifications.socket

# Test notification
echo '{"title":"VM Test","message":"Hello from inside VM"}' | nc -U /tmp/claude-notifications.socket

# Using the helper script (if you copied it)
# Copy helper to VM first:
# exit
# limactl copy capabilities/notifications/send-notification.sh notification-test:/tmp/
# claude-vm shell
chmod +x /tmp/send-notification.sh
/tmp/send-notification.sh "Build Success" "Your project compiled!"
```

## Step 5: Integration Test

Test with a real build or command:

```bash
# Inside VM
(cargo build && echo '{"title":"Build Success","message":"Compilation complete"}' | nc -U /tmp/claude-notifications.socket) || \
  echo '{"title":"Build Failed","message":"Check the logs"}' | nc -U /tmp/claude-notifications.socket
```

## Troubleshooting

### Socket Not Created

If `/tmp/claude-vm-notifications.socket` doesn't exist on host:

1. Check host_setup.sh ran successfully:
   ```bash
   grep -i notification /tmp/claude-vm-setup.log
   ```

2. Manually run host setup:
   ```bash
   cd capabilities/notifications
   LIMA_INSTANCE=notification-test bash host_setup.sh
   ```

3. Check for errors:
   ```bash
   tail -f /tmp/claude-vm-notifications.log
   ```

### Socket Exists But No Notifications

1. **macOS Permissions**:
   - Go to System Settings → Notifications
   - Find Terminal (or iTerm/your terminal app)
   - Enable "Allow notifications"

2. **Test osascript directly**:
   ```bash
   osascript -e 'display notification "Test" with title "Test"'
   ```

3. **Check logs for errors**:
   ```bash
   tail -50 /tmp/claude-vm-notifications.log
   ```

### Socket Permission Denied

```bash
# Check socket ownership
ls -la /tmp/claude-vm-notifications.socket

# Should be owned by your user
# If not, restart the listener:
kill $(cat /tmp/claude-vm-notifications.pid)
rm /tmp/claude-vm-notifications.socket
claude-vm setup
```

### Listener Crashed

Restart it:

```bash
# Kill old process
if [ -f /tmp/claude-vm-notifications.pid ]; then
  kill $(cat /tmp/claude-vm-notifications.pid) 2>/dev/null || true
fi

# Clean up
rm -f /tmp/claude-vm-notifications.socket
rm -f /tmp/claude-vm-notifications.pid

# Re-run setup
claude-vm setup
```

## Performance Testing

Test handling multiple notifications:

```bash
# Inside VM
for i in {1..10}; do
  echo "{\"title\":\"Notification $i\",\"message\":\"Test message $i\"}" | nc -U /tmp/claude-notifications.socket
  sleep 0.5
done
```

All 10 notifications should appear on the host.

## Advanced: Integration with Claude Code

When Claude Code runs in the VM, you could potentially hook into it to send notifications:

```bash
# Example: Notify when Claude Code session ends
# In your shell's .bashrc or .profile in the VM:

# Function to notify on long commands
notify_on_long_command() {
  local start_time=$SECONDS
  "$@"
  local exit_code=$?
  local end_time=$SECONDS
  local duration=$((end_time - start_time))

  # Notify if command took more than 30 seconds
  if [ $duration -gt 30 ]; then
    if [ $exit_code -eq 0 ]; then
      echo "{\"title\":\"Command Complete\",\"message\":\"Finished in ${duration}s\"}" | nc -U /tmp/claude-notifications.socket
    else
      echo "{\"title\":\"Command Failed\",\"message\":\"Exited with code $exit_code after ${duration}s\"}" | nc -U /tmp/claude-notifications.socket
    fi
  fi

  return $exit_code
}

# Alias for long builds
alias nbuild="notify_on_long_command cargo build"
```

## Cleanup

To stop the notification system:

```bash
# Kill listener
kill $(cat /tmp/claude-vm-notifications.pid)

# Remove files
rm -f /tmp/claude-vm-notifications.socket
rm -f /tmp/claude-vm-notifications.pid
rm -f /tmp/claude-vm-notifications.log

# Destroy test VM
claude-vm destroy notification-test
```
