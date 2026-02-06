# Custom Mounts

Custom mounts allow you to share additional directories between your host machine and the VM beyond the automatic mounts (project directory, worktrees, conversations).

## Table of Contents

- [Automatic Mounts](#automatic-mounts)
- [Adding Custom Mounts](#adding-custom-mounts)
- [Runtime vs Setup Mounts](#runtime-vs-setup-mounts)
- [Mount Configuration](#mount-configuration)
- [Examples](#examples)
- [Troubleshooting](#troubleshooting)

## Automatic Mounts

Claude VM automatically mounts:

1. **Project directory** - Current working directory (writable)
2. **Git worktrees** - Both worktree and main repository (writable)
3. **Conversation history** - Claude conversation folder (writable)

These are always mounted and don't need configuration.

## Adding Custom Mounts

### CLI: Docker-Style Syntax

```bash
# Simple mount (writable, same path in VM)
claude-vm --mount /host/data shell

# Read-only mount
claude-vm --mount /host/data:ro shell

# Custom VM path (writable)
claude-vm --mount /host/data:/vm/data shell

# Custom VM path (read-only)
claude-vm --mount /host/data:/vm/data:ro shell

# Multiple mounts
claude-vm --mount /host/data1 --mount /host/data2:ro shell

# Tilde expansion
claude-vm --mount ~/Documents:/vm/docs shell
```

**Syntax:**
```
--mount <host-path>[:<vm-path>][:<mode>]

Where:
  host-path: Absolute path on host (or ~ for home)
  vm-path:   Path in VM (optional, defaults to host-path)
  mode:      ro (read-only) or rw/omit (read-write)
```

### Configuration: TOML

Define persistent mounts in `.claude-vm.toml`:

```toml
# Minimal: same path, writable
[[mounts]]
location = "/Users/me/data"
writable = true

# Custom VM path, read-only
[[mounts]]
location = "/Users/me/shared"
mount_point = "/vm/shared"
writable = false

# Tilde expansion
[[mounts]]
location = "~/Documents"
writable = true
```

**Fields:**
- `location` - Host path (required, absolute or ~)
- `mount_point` - VM path (optional, defaults to location)
- `writable` - Read-write access (default: true)

## Runtime vs Setup Mounts

### Runtime Mounts

Available during **every session** (both `claude-vm` and `claude-vm shell`):

```toml
[[mounts]]
location = "~/datasets"
mount_point = "/data"
writable = false
```

```bash
claude-vm --mount ~/datasets:/data:ro shell
```

**Use for:**
- Shared datasets
- Reference documentation
- Configuration files
- Any data needed during runtime

### Setup Mounts

Available **only during template creation** (`claude-vm setup`):

```toml
[[setup.mounts]]
location = "~/local-tools"
mount_point = "/tmp/tools"
writable = false
```

```bash
claude-vm setup --mount ~/binaries:/tmp/binaries --git
```

**Use for:**
- Copying files into template
- Installing local binaries
- Transferring assets
- One-time setup data

**Important:** Setup mounts are not available at runtime. Copy files from mount into template filesystem during setup:

```bash
# .claude-vm.setup.sh
cp /tmp/tools/my-tool /usr/local/bin/
chmod +x /usr/local/bin/my-tool
```

See [Setup Mounts Example](#setup-mount-example) below.

## Mount Configuration

### How It Works

**Accumulation:**
Mounts from all sources are combined:
```
Global config mounts
+ Project config mounts
+ CLI mounts
= Final mount list
```

**Deduplication:**
Duplicate locations are automatically filtered (last wins).

**Path Expansion:**
- `~` expands to your home directory
- Relative paths are rejected (must be absolute after expansion)

**Default Mount Points:**
If no `mount_point` specified, host path is used as VM path:
```toml
[[mounts]]
location = "/Users/me/data"
# Mounted at: /Users/me/data in VM
```

**Validation:**
- Paths must be absolute (after ~ expansion)
- Directories must exist on host
- No conflicting mount points

### Global vs Project Mounts

**Global** (`~/.claude-vm.toml`):
- Available to all projects
- Useful for shared datasets, tools

**Project** (`./.claude-vm.toml`):
- Available only to this project
- Project-specific data

**Combined:**
```bash
# Global: ~/datasets -> /data
# Project: ~/project-data -> /project-data
# Result: Both mounted
```

### Precedence

Later sources override earlier ones for the same location:

```bash
# Global config: /data -> /data (ro)
# Project config: /data -> /data (rw)
# CLI: /data -> /custom (ro)
# Result: /data -> /custom (ro) - CLI wins
```

## Examples

### Share a Dataset

Make a large dataset available to all sessions:

```toml
# ~/.claude-vm.toml (global)
[[mounts]]
location = "~/datasets"
mount_point = "/data"
writable = false
```

```bash
# In any project
claude-vm shell
$ ls /data  # Dataset available
```

### Multiple Data Sources

```toml
# .claude-vm.toml
[[mounts]]
location = "/mnt/storage/data"
mount_point = "/data"
writable = false

[[mounts]]
location = "/mnt/storage/cache"
mount_point = "/cache"
writable = true

[[mounts]]
location = "/mnt/storage/models"
mount_point = "/models"
writable = false
```

### Temporary Mount for Single Session

```bash
# One-time analysis
claude-vm --mount /tmp/experiment:/experiment "analyze this experiment data"

# Testing with different data
claude-vm --mount ~/test-data:/data shell npm test
```

### Mount Documentation

```toml
# .claude-vm.toml
[[mounts]]
location = "~/Documents/project-docs"
mount_point = "/docs"
writable = false
```

Claude can reference your docs at `/docs` in the VM.

### Setup Mount Example

Transfer local binaries to template:

```toml
# .claude-vm.toml
[[setup.mounts]]
location = "~/local-tools/bin"
mount_point = "/tmp/host-bin"
writable = false
```

```bash
# .claude-vm.setup.sh
#!/bin/bash

# Copy tools from setup mount to template
sudo cp /tmp/host-bin/* /usr/local/bin/
sudo chmod +x /usr/local/bin/*

echo "✓ Local tools installed"
```

```bash
# Create template
claude-vm setup --git

# Tools are now in template, available at runtime
claude-vm shell which my-tool
# /usr/local/bin/my-tool
```

### Mount Home Directory Subdirectories

```toml
# .claude-vm.toml
[[mounts]]
location = "~/.config/my-app"
mount_point = "/home/lima.linux/.config/my-app"
writable = false
```

Configuration files are available in VM.

## Troubleshooting

### Mount Not Appearing

**Check if directory exists:**
```bash
ls -la ~/datasets  # Host directory must exist
```

**Check config syntax:**
```bash
claude-vm config validate
```

**Check effective config:**
```bash
claude-vm config show
```

### Permission Denied

**Mount as read-only:**
```bash
# If you get permission errors, try read-only
claude-vm --mount ~/data:/data:ro shell
```

**Check host permissions:**
```bash
# Ensure you can read the host directory
ls ~/data
```

### Path Not Absolute

```toml
# ❌ Wrong: relative path
[[mounts]]
location = "./data"

# ✓ Correct: absolute path
[[mounts]]
location = "/Users/me/project/data"

# ✓ Correct: tilde expansion
[[mounts]]
location = "~/data"
```

### Conflicting Mount Points

```toml
# ❌ Wrong: both mount to /data
[[mounts]]
location = "~/datasets1"
mount_point = "/data"

[[mounts]]
location = "~/datasets2"
mount_point = "/data"  # Conflict!
```

Last one wins - use different mount points.

### Setup Mount Not Available at Runtime

This is expected behavior. Setup mounts are only for `claude-vm setup`:

```bash
# ❌ Won't work: setup mount not available at runtime
claude-vm shell ls /tmp/tools  # Not found

# ✓ Correct: copy during setup, use at runtime
# In .claude-vm.setup.sh:
#   cp /tmp/tools/my-tool /usr/local/bin/
claude-vm shell which my-tool  # Found
```

## Best Practices

### 1. Use Read-Only for Reference Data

```toml
[[mounts]]
location = "~/datasets"
writable = false  # Prevent accidental modification
```

### 2. Mount at Intuitive Paths

```toml
# Good: clear, memorable paths
location = "~/datasets"
mount_point = "/data"

# Less clear: confusing paths
location = "~/datasets"
mount_point = "/mnt/x/y/z"
```

### 3. Use Global Config for Shared Resources

```bash
# ~/.claude-vm.toml - Available to all projects
[[mounts]]
location = "~/shared-datasets"
mount_point = "/data"
```

### 4. Document Required Mounts

```markdown
# README.md
This project requires the following data:
- ~/datasets/training -> /data (read-only)

Configure in .claude-vm.toml or use:
\`\`\`bash
claude-vm --mount ~/datasets/training:/data:ro shell
\`\`\`
```

### 5. Use Setup Mounts Sparingly

Setup mounts add complexity. Consider:
- **Alternative:** Download in setup script
- **Alternative:** Install from package manager
- **Use when:** You have large local binaries or assets

## Next Steps

- **[Templates](templates.md)** - Understand template VMs
- **[Runtime Scripts](runtime-scripts.md)** - Automate environment setup
- **[Configuration](../configuration.md)** - Full configuration reference
- **[Troubleshooting](../advanced/troubleshooting.md)** - Debug mount issues
