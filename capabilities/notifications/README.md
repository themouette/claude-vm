# Notification Forwarding Capability

Forward notifications from the VM to the host system's notification center.

## Features

- ✅ Forward notifications from VM to macOS Notification Center
- ✅ JSON-based notification protocol
- ✅ Support for title, message, and subtitle
- ✅ Unix socket communication (secure and efficient)
- ✅ Helper script for easy notification sending

## Requirements

- **Host**: macOS (uses `osascript` for native notifications)
- **VM**: None (uses standard Unix tools)

## How It Works

1. **Host Setup**: Starts a background listener on `/tmp/claude-vm-notifications.socket`
2. **Socket Forwarding**: Lima forwards the host socket to `/tmp/claude-notifications.socket` in the VM
3. **Sending Notifications**: Any process in the VM can write JSON to the socket
4. **Display**: Host receives messages and displays them using macOS Notification Center

## Usage

### From Shell Scripts

Use the helper script (located in this directory):

```bash
# Simple notification
./send-notification.sh "Build Complete" "Your project compiled successfully"

# With subtitle
./send-notification.sh "Test Results" "All tests passed" "100% coverage"

# From stdin (JSON)
echo '{"title":"Git Push","message":"Successfully pushed to main"}' | ./send-notification.sh
```

### Direct Socket Communication

Send JSON directly to the socket:

```bash
echo '{"title":"Hello","message":"World"}' | nc -U /tmp/claude-notifications.socket
```

### JSON Format

```json
{
  "title": "Notification Title",
  "message": "Notification body text",
  "subtitle": "Optional subtitle"
}
```

- `title`: Required - main notification heading
- `message`: Required - notification body
- `subtitle`: Optional - additional context

### Environment Variables

- `CLAUDE_NOTIFICATION_SOCKET`: Socket path (default: `/tmp/claude-notifications.socket`)

### Integration Examples

**Git Hook** (notify on successful push):
```bash
#!/bin/bash
# .git/hooks/post-receive
echo '{"title":"Git Push","message":"Changes received"}' | nc -U /tmp/claude-notifications.socket
```

**Build Script**:
```bash
#!/bin/bash
if cargo build; then
  echo '{"title":"Build Success","message":"Compilation completed"}' | nc -U /tmp/claude-notifications.socket
else
  echo '{"title":"Build Failed","message":"Check logs for errors"}' | nc -U /tmp/claude-notifications.socket
fi
```

**Long-Running Process**:
```bash
#!/bin/bash
./long-task.sh && \
  echo '{"title":"Task Complete","message":"Processing finished"}' | nc -U /tmp/claude-notifications.socket
```

## Troubleshooting

### Socket Not Found

If `/tmp/claude-notifications.socket` doesn't exist:

1. Check that the capability is enabled in `.claude-vm.toml`:
   ```toml
   [vm]
   capabilities = ["notifications"]
   ```

2. Check host listener logs:
   ```bash
   # On host (outside VM)
   tail -f /tmp/claude-vm-notifications.log
   ```

3. Verify host listener is running:
   ```bash
   # On host
   ps aux | grep claude-vm-notification-listener
   cat /tmp/claude-vm-notifications.pid
   ```

### Notifications Not Appearing

1. **Test on Host**: Verify the listener works on the host first:
   ```bash
   # On host (outside VM)
   echo '{"title":"Test","message":"Direct test"}' | nc -U /tmp/claude-vm-notifications.socket
   ```

2. **Check Logs**: Look for errors in the listener log:
   ```bash
   # On host
   tail -f /tmp/claude-vm-notifications.log
   ```

3. **macOS Permissions**: Ensure Terminal/iTerm has notification permissions:
   - System Settings → Notifications → Terminal/iTerm → Allow notifications

### Restart Listener

If the listener becomes unresponsive:

```bash
# On host (outside VM)
# Stop existing listener
if [ -f /tmp/claude-vm-notifications.pid ]; then
  kill $(cat /tmp/claude-vm-notifications.pid)
fi

# Remove socket
rm -f /tmp/claude-vm-notifications.socket

# Restart VM to re-run host_setup.sh
claude-vm setup
```

## Architecture

```
┌─────────────────────┐
│   VM Process        │
│                     │
│  echo JSON →        │
└──────────┬──────────┘
           │
           ↓ Unix Socket
    /tmp/claude-notifications.socket
           │
           ↓ Lima Port Forward
           │
           ↓ Unix Socket
 /tmp/claude-vm-notifications.socket
           │
           ↓
┌──────────┴──────────┐
│  Host Listener      │
│  (notification-     │
│   listener.sh)      │
└──────────┬──────────┘
           │
           ↓ osascript
┌──────────┴──────────┐
│  macOS Notification │
│  Center             │
└─────────────────────┘
```

## Security

- Unix sockets are only accessible within the VM (not exposed to network)
- Communication is local to the host machine
- No authentication needed (trusted VM environment)
- JSON parsing is defensive (escapes shell metacharacters)

## Future Enhancements

- [ ] Support for Linux notification systems (notify-send, dunstify)
- [ ] Support for notification actions/buttons
- [ ] Support for icons/images
- [ ] Rate limiting to prevent spam
- [ ] Notification history/logging
