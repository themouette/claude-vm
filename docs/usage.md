# Usage Guide

This guide covers all Claude VM commands with detailed examples.

## Table of Contents

- [Setup](#setup)
- [Run Claude](#run-claude)
- [Shell Access](#shell-access)
- [Project Information](#project-information)
- [Configuration Management](#configuration-management)
- [Template Management](#template-management)
- [Updates](#updates)
- [Global Options](#global-options)

## Setup

Create a template VM for your project with the tools you need.

### Basic Setup

```bash
# Minimal setup with git configuration
claude-vm setup --git

# With Docker support
claude-vm setup --git --docker

# With Node.js
claude-vm setup --git --node

# With Python
claude-vm setup --git --python
```

### Install All Tools

```bash
# Install everything
claude-vm setup --all
```

This installs:
- Docker Engine + Docker Compose
- Node.js (LTS) + npm
- Python 3 + pip
- Chromium + Chrome DevTools MCP
- GPG agent forwarding
- GitHub CLI
- Git identity configuration

### Available Tools

- `--git` - Configure git identity and signing
- `--docker` - Docker Engine + Docker Compose
- `--node` - Node.js (LTS) + npm
- `--python` - Python 3 + pip
- `--chromium` - Chromium browser + debugging tools
- `--gpg` - GPG agent forwarding for signing
- `--gh` - GitHub CLI + authentication
- `--all` - Install all tools

### Custom Setup Script

Run additional setup scripts during template creation:

```bash
claude-vm setup --git --setup-script ./my-setup.sh
```

### VM Resources

Customize VM resources:

```bash
# Set disk and memory
claude-vm setup --disk 30 --memory 16 --git --node

# Use environment variables
CLAUDE_VM_DISK=30 CLAUDE_VM_MEMORY=16 claude-vm setup --git
```

### Setup-Specific Mounts

Mount directories only during setup (for copying files):

```bash
# Mount binaries during setup
claude-vm setup --mount ~/local-tools:/tmp/tools --git

# Use in setup script (.claude-vm.setup.sh):
# cp /tmp/tools/my-tool /usr/local/bin/
```

See [Custom Mounts](features/custom-mounts.md) for more details.

## Run Claude

Run Claude in an isolated VM. The VM is automatically created from your template and destroyed when Claude exits.

### Basic Usage

```bash
# Start Claude interactively
claude-vm

# Run with a prompt
claude-vm "help me understand this code"

# Multiple-word prompts
claude-vm "analyze the database schema and suggest improvements"
```

### Auto-Setup

If no template exists, Claude VM can create one automatically:

```bash
# With prompt
claude-vm --auto-setup "help me code"

# Or enable permanently in config
[defaults]
auto_setup = true
```

### Passing Environment Variables

```bash
# Set individual variables
claude-vm --env API_KEY=secret --env DEBUG=true

# Load from file
claude-vm --env-file .env

# Inherit from host
claude-vm --inherit-env PATH --inherit-env USER

# Combine multiple sources
claude-vm --env-file .env --env API_KEY=override --inherit-env USER
```

### Custom Mounts

```bash
# Mount additional directories
claude-vm --mount ~/datasets:/data:ro "analyze this dataset"

# Multiple mounts
claude-vm --mount ~/data1 --mount ~/data2:ro "process the data"
```

### SSH Agent Forwarding

Forward your SSH agent for git operations:

```bash
# Enable SSH agent forwarding
claude-vm -A "git push to remote"

# Or use long form
claude-vm --forward-ssh-agent
```

### Other Options

```bash
# Disable conversation history mounting
claude-vm --no-conversations

# Verbose output (show Lima logs)
claude-vm --verbose "help me debug"

# Custom runtime scripts
claude-vm --runtime-script ./setup-env.sh
```

## Shell Access

Open an interactive shell or execute commands in an ephemeral VM.

### Interactive Shell

```bash
# Open bash shell in VM
claude-vm shell
```

Once in the shell, you have full access to:
- Your project directory (mounted)
- All installed tools (docker, node, etc.)
- Git repository (if in worktree, main repo is also mounted)

### Execute Single Commands

```bash
# Run tests
claude-vm shell npm test

# Check git status
claude-vm shell git status

# Run docker compose
claude-vm shell docker-compose up -d

# Execute arbitrary commands
claude-vm shell "npm install && npm test"
```

### With Environment Variables

```bash
# Set environment for command
claude-vm --env NODE_ENV=test shell npm test

# Load from file
claude-vm --env-file .env.test shell npm start

# Inherit from host
claude-vm --inherit-env PATH shell which node
```

### With Custom Mounts

```bash
# Mount additional data
claude-vm --mount ~/datasets:/data shell python process.py

# Multiple mounts
claude-vm --mount /data1 --mount /data2:ro shell ./analyze.sh
```

## Project Information

Display information about the current project's template.

```bash
claude-vm info
```

**Output includes:**
- Project path and calculated template name
- Template status (running, stopped, not created)
- VM configuration (disk, memory)
- Enabled capabilities (docker, node, etc.)
- Configured mounts
- Runtime scripts

**Example output:**
```
Project: /Users/me/my-project
Template: claude-tpl_my-project_abc123de
Status: stopped

Configuration:
  Disk: 20 GB
  Memory: 8 GB
  Capabilities: docker, node, git

Mounts:
  /Users/me/my-project -> /Users/me/my-project (writable)

Runtime Scripts:
  ./.claude-vm.runtime.sh
```

## Configuration Management

Manage and validate configuration files.

### Validate Configuration

Check configuration files for errors:

```bash
# Validate current project config
claude-vm config validate
```

This checks:
- TOML syntax
- Valid value ranges (disk: 1-1000, memory: 1-64)
- Required fields
- Type correctness

### Show Effective Configuration

Display the final merged configuration:

```bash
# Show complete configuration
claude-vm config show
```

This shows the result after applying precedence:
1. Command-line flags
2. Environment variables
3. Project config (`./.claude-vm.toml`)
4. Global config (`~/.claude-vm.toml`)
5. Built-in defaults

**Example output:**
```toml
[vm]
disk = 30      # From project config
memory = 16    # From CLI flag

[tools]
docker = true  # From global config
node = true    # From project config

[defaults]
auto_setup = false  # Built-in default
```

## Template Management

Manage VM templates for your projects.

### List Templates

```bash
# List all templates
claude-vm list
```

**Example output:**
```
claude-tpl_project1_abc123de (stopped)
claude-tpl_project2_def456ab (running)
claude-tpl_test_789xyz01 (stopped)
```

### List with Disk Usage

```bash
# Show disk usage
claude-vm list --disk-usage
```

**Example output:**
```
claude-tpl_project1_abc123de (stopped) - 5.2 GB
claude-tpl_project2_def456ab (running) - 12.8 GB
claude-tpl_test_789xyz01 (stopped) - 3.1 GB
```

### List Unused Templates

Find templates not accessed in 30+ days:

```bash
# Show only unused templates
claude-vm list --unused
```

Useful for cleaning up old project templates.

### Clean Current Template

Remove the template for the current project:

```bash
# Clean with confirmation prompt
claude-vm clean

# Clean without prompt
claude-vm clean --yes
```

This removes the template VM and frees up disk space. The template can be recreated with `claude-vm setup`.

### Clean All Templates

Remove all Claude VM templates:

```bash
# Clean all with confirmation
claude-vm clean-all

# Clean all without prompt
claude-vm clean-all --yes
```

**Warning:** This removes templates for all projects. You'll need to run `claude-vm setup` in each project to recreate them.

## Updates

Check for and install updates to Claude VM.

### Check for Updates

```bash
# Check if updates are available
claude-vm update --check
```

**Example output:**
```
Current version: 0.3.0
Latest version: 0.4.0
Update available!
```

### Update to Latest

```bash
# Update to latest version
claude-vm update
```

This will:
1. Download the latest release from GitHub
2. Verify the download
3. Replace the current binary
4. Verify the new version

### Update to Specific Version

```bash
# Install specific version
claude-vm update --version 0.4.0
```

Updates are downloaded from [GitHub Releases](https://github.com/themouette/claude-vm/releases).

## Global Options

Options that work with most commands.

### VM Resources

```bash
# Set disk size (GB)
--disk 30

# Set memory size (GB)
--memory 16

# Example
claude-vm --disk 30 --memory 16 setup --git
```

### Environment Variables

```bash
# Set variable
--env KEY=value

# Load from file
--env-file .env

# Inherit from host
--inherit-env VAR

# Example
claude-vm --env API_KEY=secret --env-file .env --inherit-env USER shell
```

### Custom Mounts

```bash
# Mount directory (writable)
--mount /host/path

# Mount with custom VM path
--mount /host/path:/vm/path

# Mount read-only
--mount /host/path:/vm/path:ro

# Example
claude-vm --mount ~/data:/data:ro shell
```

### Runtime Scripts

```bash
# Execute script before main command
--runtime-script ./setup.sh

# Multiple scripts
--runtime-script ./script1.sh --runtime-script ./script2.sh

# Example
claude-vm --runtime-script ./start-services.sh shell
```

### Agent Forwarding

```bash
# Forward SSH agent
-A, --forward-ssh-agent

# Example
claude-vm -A "git push"
```

### Other Options

```bash
# Verbose output (show Lima logs)
--verbose

# Don't mount conversation history
--no-conversations

# Auto-create template if missing
--auto-setup

# Example
claude-vm --verbose --auto-setup "help me"
```

## Examples

### Full Development Setup

```bash
# Create template with all tools
claude-vm setup --all

# Run Claude with environment
claude-vm --env-file .env "help me implement the API"

# Test in isolated environment
claude-vm shell npm test

# Check project info
claude-vm info
```

### CI/CD Usage

```bash
# Run tests in clean VM
claude-vm --auto-setup shell npm test

# Build and test
claude-vm shell "npm install && npm run build && npm test"

# Cleanup after
claude-vm clean --yes
```

### Data Analysis

```bash
# Setup with Python
claude-vm setup --python

# Mount dataset and analyze
claude-vm --mount ~/datasets:/data:ro "help me analyze the data in /data"

# Run analysis script
claude-vm shell python analyze.py
```

### Multi-Tool Project

```bash
# Setup with Docker and Node.js
claude-vm setup --docker --node --git

# Start services and run tests
claude-vm --runtime-script ./start-services.sh shell npm test

# Run with SSH forwarding for git push
claude-vm -A "implement feature and push to remote"
```

## Next Steps

- **[Configuration](configuration.md)** - Configure VM settings, tools, and scripts
- **[Runtime Scripts](features/runtime-scripts.md)** - Automate environment setup
- **[Agent Forwarding](agent-forwarding.md)** - Configure GPG, SSH, and Git
- **[Troubleshooting](advanced/troubleshooting.md)** - Debug common issues
