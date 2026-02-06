# Agent Forwarding

This guide covers how to forward authentication agents (GPG and SSH) from your host machine to the VM, enabling secure operations like commit signing and private repository access.

## Overview

Agent forwarding allows you to use your host machine's authentication credentials inside the VM without copying private keys. This provides:

- **Security**: Private keys never leave your host machine
- **Convenience**: Seamless authentication for git operations, signing, and SSH connections
- **Consistency**: Same authentication setup across host and VM

## GPG Agent Forwarding

Enable GPG signing in the VM by forwarding your GPG agent from the host machine.

### Setup

Configure GPG support during template creation:

```bash
# Setup with GPG support
claude-vm setup --gpg

# Or enable in config file
```

Add to `.claude-vm.toml`:

```toml
[tools]
gpg = true
```

### What It Does

When GPG forwarding is enabled:

- Forwards your GPG agent socket to the VM
- Syncs your public keys to the VM
- Enables git commit signing inside the VM
- Works automatically on every session

### Usage in VM

Once enabled, you can use GPG operations transparently:

```bash
# Sign commits (uses your host GPG key)
git commit -S -m "Signed commit"

# Sign files
gpg --sign document.txt

# List available keys (shows keys from host)
gpg --list-keys
```

### Troubleshooting

**Problem**: GPG signing fails with "No secret key" error

**Solution**: Ensure GPG agent is running on host:

```bash
# On host machine
gpg-agent --daemon
```

**Problem**: Public keys not available in VM

**Solution**: Re-run setup to sync keys:

```bash
claude-vm setup --gpg
```

## SSH Agent Forwarding

Forward your SSH agent for git operations over SSH and remote server access.

### Setup

SSH agent forwarding is available on-demand using the `-A` flag. No template configuration needed.

### Usage

Enable SSH agent forwarding at runtime:

```bash
# Run with SSH agent forwarding
claude-vm -A shell

# Or with run command
claude-vm -A "git push"

# Works with any command
claude-vm -A "ssh user@remote-server"
```

### Use Cases

SSH agent forwarding is useful for:

- **Private repositories**: Push/pull from private repositories over SSH
- **Remote servers**: SSH to remote servers for deployment or management
- **Git operations**: Any git operation requiring SSH authentication
- **SSH keys**: Operations requiring SSH key authentication

### How It Works

SSH agent forwarding uses native SSH agent forwarding (`ssh -A`). This means:

- Your private keys never leave the host machine
- The VM can only use keys for authentication, not extract them
- Standard SSH agent protocol is used for maximum compatibility

### Security Note

While SSH agent forwarding is generally safe, be aware:

- Only forward to trusted VMs (claude-vm VMs are isolated and disposable)
- The VM can use your keys while the session is active
- Keys cannot be extracted or copied from the VM

## Git Configuration

Configure git identity and commit signing in the VM from your host configuration.

### Setup

Enable git configuration during template creation:

```bash
# Setup with git support
claude-vm setup --git

# Or enable in config file
```

Add to `.claude-vm.toml`:

```toml
[tools]
git = true
```

### What It Does

When git configuration is enabled:

- Copies your git `user.name` and `user.email` from host to VM
- Automatically configures commit signing if enabled on host
- Detects GPG or SSH signing configuration
- Provides contextual warnings about signing requirements

### Commit Signing

If you have commit signing enabled on your host, you need to enable the appropriate forwarding:

**GPG signing**: Enable both `git` and `gpg` capabilities

```bash
claude-vm setup --git --gpg
```

This configures git identity and forwards GPG agent for signing.

**SSH signing**: Enable `git` capability and forward SSH agent at runtime

```bash
# Setup
claude-vm setup --git

# Run with SSH agent forwarding
claude-vm -A "make a commit"
```

This configures git identity and uses SSH agent forwarding for signing.

### Usage in VM

Once configured, git operations use your host identity:

```bash
# Your git identity is automatically configured
git config user.name    # Shows your host name
git config user.email   # Shows your host email

# Signed commits work with proper agent forwarding
git commit -m "My commit"  # Automatically signed if configured
```

### Requirements

For git configuration to work:

- Git must be configured on host: `git config --global user.name` and `user.email`
- For GPG signing: Enable `gpg` capability in template
- For SSH signing: Use `-A` flag to forward SSH agent at runtime

### Troubleshooting

**Problem**: Git identity not configured in VM

**Solution**: Check host configuration:

```bash
# On host machine
git config --global user.name
git config --global user.email
```

If not set, configure on host first, then re-run setup.

**Problem**: Signed commits fail with GPG

**Solution**: Ensure both `git` and `gpg` capabilities are enabled:

```bash
claude-vm setup --git --gpg
```

**Problem**: Signed commits fail with SSH

**Solution**: Use `-A` flag when running commands that need signing:

```bash
claude-vm -A "git commit -m 'message'"
```

## Combining Agent Forwarding

You can use multiple types of agent forwarding together:

```bash
# Setup template with git and GPG support
claude-vm setup --git --gpg

# Run with SSH agent forwarding for remote operations
claude-vm -A "git push origin main"
```

This enables:
- Git identity configuration (from git capability)
- GPG signing for commits (from gpg capability)
- SSH authentication for push/pull (from -A flag)

## Best Practices

1. **Enable what you need**: Only enable agent forwarding for the operations you need
2. **Use git capability for identity**: Always enable `git` capability if you're working with git repositories
3. **GPG for signing**: Use GPG forwarding if you sign commits with GPG keys
4. **SSH for remote access**: Use `-A` flag when you need to access remote servers or private repositories
5. **Test in VM**: After setup, test operations in VM shell to ensure everything works:

```bash
claude-vm shell
# In VM:
git config --list
gpg --list-keys
ssh -T git@github.com
```

## Security Considerations

Agent forwarding is designed to be secure, but keep these points in mind:

- **VM isolation**: Claude-vm VMs are ephemeral and isolated, making them safe for agent forwarding
- **Private keys stay on host**: Neither GPG nor SSH forwarding ever exposes your private keys to the VM
- **Temporary access**: Forwarded agents are only available during the VM session
- **Audit trail**: All operations using forwarded agents are logged on the host
- **Trust boundary**: Only forward agents to VMs you control and trust
