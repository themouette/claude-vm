# Claude VM

**Run Claude in `--dangerously-skip-permissions` without worrying**

Claude VM gives you safety through VM isolation, reproducibility through template VMs, and simplicity through one-command setup. Each Claude session runs in a fresh, sandboxed VM that's destroyed after use.

## Quick Start

```bash
# One-time setup: create a template VM
claude-vm setup --git --node

# Run Claude in an isolated VM
claude-vm "help me code"

# Open a shell in the VM
claude-vm shell
```

Each run starts from the same clean template and automatically cleans up when done.

## Why Claude VM?

**VM isolation is the only safe way to run Claude with `--dangerously-skip-permissions`.** Even if Claude executes unintended commands, the blast radius is limited to the disposable VM.

Claude VM runs each session in an isolated Linux VM that:

- Only mounts the current project directory
- Has its own filesystem, network stack, and process space
- Is automatically destroyed after each session
- Starts from a known-good template state every time

Think of it as Docker for AI coding assistants - isolated, reproducible, and safe.

## Key Features

- **Template VMs per Repository** - Create once per project, clone for fast startup
- **Runtime Scripts** - Automatically run setup scripts before each session
- **Configuration File Support** - Define VM resources, tools, and settings in `.claude-vm.toml`
- **Git Worktree Support** - Automatically detects and mounts both worktree and main repository
- **Comprehensive Management** - Commands for info, config validation, template cleanup, and more

## Installation

### Requirements

- [Lima](https://lima-vm.io/docs/installation/)

### Quick Install

Install to `~/.local/bin` (no sudo required):

```bash
curl -fsSL https://raw.githubusercontent.com/themouette/claude-vm/main/install.sh | bash
```

Install system-wide to `/usr/local/bin`:

```bash
curl -fsSL https://raw.githubusercontent.com/themouette/claude-vm/main/install.sh | bash -s -- --global
```

Install specific version:

```bash
curl -fsSL https://raw.githubusercontent.com/themouette/claude-vm/main/install.sh | bash -s -- --version v0.3.0
```

### Download from GitHub

Download the latest version for your platform from [GitHub Releases](https://github.com/themouette/claude-vm/releases/latest) and copy the executable to your `~/.local/bin` directory.

### From Source

See [docs/development.md](docs/development.md)

## Basic Usage

### Setup a Template

```bash
# Create template with specific tools
claude-vm setup --docker --node

# Or install everything
claude-vm setup --all
```

### Run Claude

```bash
# Run Claude in an ephemeral VM
claude-vm "help me code"

# With auto-setup if template doesn't exist
claude-vm --auto-setup "help me code"
```

### Shell Access

```bash
# Interactive shell
claude-vm shell

# Execute a command
claude-vm shell npm test
```

### Management Commands

```bash
# Show project info
claude-vm info

# Validate configuration
claude-vm config validate

# List all templates
claude-vm list

# Clean current project's template
claude-vm clean
```

## Documentation

- **[Getting Started](docs/getting-started.md)** - Installation, requirements, and first setup
- **[Usage Guide](docs/usage.md)** - All commands with examples
- **[Configuration](docs/configuration.md)** - TOML config, environment variables, precedence

### Features

- **[Templates](docs/features/templates.md)** - How template VMs work
- **[Runtime Scripts](docs/features/runtime-scripts.md)** - Runtime scripts and context contribution
- **[Custom Mounts](docs/features/custom-mounts.md)** - Mount additional directories
- **[Network Isolation](docs/features/network-isolation.md)** - Prevent access to network
- **[Tools](docs/features/tools.md)** - Available tools (Docker, Node.js, Python, etc.)

### Advanced Topics

- **[Agent Forwarding](docs/agent-forwarding.md)** - GPG, SSH, and Git configuration
- **[Git Integration](docs/git-integration.md)** - Worktrees and conversation history
- **[Custom Packages](docs/advanced/custom-packages.md)** - Install custom system packages
- **[Troubleshooting](docs/advanced/troubleshooting.md)** - Common issues and debugging

### Contributing

- **[Development Guide](docs/development.md)** - Architecture, building, and testing
- **[Contributing](docs/contributing.md)** - How to contribute to the project

## Quick Reference

### Configuration File

Create `.claude-vm.toml` in your project root:

```toml
[vm]
disk = 20      # GB
memory = 8     # GB

[tools]
docker = true
node = true
rust = true
git = true
```

See [docs/configuration.md](docs/configuration.md) for complete configuration reference.

### Environment Variables

Pass environment variables to the VM:

```bash
# Individual variables
claude-vm --env API_KEY=secret shell

# From file
claude-vm --env-file .env shell

# Inherit from host
claude-vm --inherit-env PATH shell
```

## License

MIT OR Apache-2.0

## Inspiration

Based on an [idea](https://github.com/sylvinus/agent-vm/) from [@sylvinus](https://github.com/sylvinus)

Thanks to [@babbins](https://github.com/babbins) for the catch-phrase
