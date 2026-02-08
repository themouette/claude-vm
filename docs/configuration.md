# Configuration

Claude VM can be configured through TOML files, environment variables, and command-line flags. This guide covers all configuration options and precedence rules.

## Table of Contents

- [Configuration Files](#configuration-files)
- [Configuration Precedence](#configuration-precedence)
- [VM Settings](#vm-settings)
- [Tools Configuration](#tools-configuration)
- [Custom Packages](#custom-packages)
- [Scripts](#scripts)
- [Default Arguments](#default-arguments)
- [Claude Context](#claude-context)
- [Custom Mounts](#custom-mounts)
- [Environment Variables](#environment-variables)
- [Validation](#validation)

## Configuration Files

### Locations

**Project config:** `.claude-vm.toml` in project root

```bash
my-project/
├── .claude-vm.toml          # Project-specific config
├── .claude-vm.setup.sh      # Project-specific setup script
└── .claude-vm.runtime.sh    # Auto-detected runtime script
```

**Global config:** `~/.claude-vm.toml` in home directory

```bash
~/
├── .claude-vm.toml          # Global config for all projects
└── .claude-vm.setup.sh      # Auto-detected global setup script
```

### Minimal Example

```toml
[vm]
disk = 30      # GB
memory = 16    # GB

[tools]
docker = true
node = true
git = true
```

### Complete Example

See [`examples/.claude-vm.toml`](../examples/.claude-vm.toml) for a fully commented example.

## Configuration Precedence

Configuration is merged in this order (highest to lowest priority):

1. **Command-line flags** - `--disk 30 --memory 16`
2. **Environment variables** - `CLAUDE_VM_DISK=30 CLAUDE_VM_MEMORY=16`
3. **Project config** - `./.claude-vm.toml`
4. **Global config** - `~/.claude-vm.toml`
5. **Built-in defaults** - `disk=20, memory=8`

**Example:**

```bash
# Given:
# - Global config: disk=20
# - Project config: disk=30
# - CLI flag: --disk 40

claude-vm setup --disk 40  # Uses disk=40 (CLI wins)
claude-vm setup            # Uses disk=30 (project config)
```

## VM Settings

Configure VM resources.

```toml
[vm]
disk = 20      # Disk size in GB (default: 20, range: 1-1000)
memory = 8     # Memory size in GB (default: 8, range: 1-64)
```

**Valid ranges:**

- `disk`: 1-1000 GB
- `memory`: 1-64 GB

**Override via CLI:**

```bash
claude-vm --disk 30 --memory 16 setup --git
```

**Override via environment:**

```bash
export CLAUDE_VM_DISK=30
export CLAUDE_VM_MEMORY=16
claude-vm setup --git
```

## Tools Configuration

Enable tools to be installed during template setup.

```toml
[tools]
git = true        # Git identity and signing configuration
docker = true     # Docker Engine + Docker Compose
node = true       # Node.js (LTS) + npm
python = true     # Python 3 + pip
chromium = true   # Chromium + Chrome DevTools MCP
gpg = true        # GPG agent forwarding + key sync
gh = true         # GitHub CLI + authentication
```

All tools default to `false` if not specified.

### Tool Details

| Tool       | What it installs             | Use case                       |
| ---------- | ---------------------------- | ------------------------------ |
| `git`      | Git identity, signing config | Any project with git           |
| `docker`   | Docker Engine, Compose       | Containerized development      |
| `node`     | Node.js LTS, npm             | JavaScript/TypeScript projects |
| `python`   | Python 3, pip                | Python projects                |
| `chromium` | Chromium browser, DevTools   | Web scraping, testing          |
| `gpg`      | GPG agent forwarding         | Signed commits                 |
| `gh`       | GitHub CLI                   | GitHub operations              |

### Install All Tools

```bash
# CLI flag installs everything
claude-vm setup --all
```

Or in config:

```toml
[tools]
git = true
docker = true
node = true
python = true
chromium = true
gpg = true
gh = true
```

### Tool Context

Each enabled tool automatically provides context to Claude via `~/.claude/CLAUDE.md`:

- Version information
- Configuration details
- Availability status
- Usage instructions

## Security Configuration

Configure network isolation policies for HTTP/HTTPS filtering and protocol blocking.

### Network Isolation

```toml
[security.network]
enabled = true
mode = "denylist"  # or "allowlist"
blocked_domains = ["example.com", "*.ads.com"]
```

Enable with CLI flag:

```bash
claude-vm setup --network-isolation
```

### Policy Modes

**Allowlist mode** - Block all except allowed:

```toml
[security.network]
enabled = true
mode = "allowlist"
allowed_domains = [
  "github.com",
  "*.api.company.com",
]
```

**Denylist mode** - Allow all except blocked:

```toml
[security.network]
enabled = true
mode = "denylist"
blocked_domains = [
  "malicious.com",
  "*.ads.com",
]
```

### Full Configuration

```toml
[security.network]
enabled = true
mode = "denylist"

# Domain filtering
allowed_domains = ["github.com", "*.api.com"]
blocked_domains = ["bad.com", "*.ads.com"]
bypass_domains = ["*.internal.com"]  # No TLS interception

# Protocol blocking (all default to true)
block_tcp_udp = true
block_private_networks = true
block_metadata_services = true
```

See [Network Isolation documentation](features/network-isolation.md) for detailed configuration and usage.

## Custom Packages

Install additional system packages.

### Basic Package Installation

```toml
[packages]
system = [
    "postgresql-client",
    "redis-tools",
    "jq",
    "htop",
    "curl"
]
```

### Version Pinning

```toml
[packages]
system = [
    "python3=3.11.0-1",      # Exact version
    "nodejs=22.*",           # Wildcard (any 22.x)
    "libc6:amd64"           # Specific architecture
]
```

### Custom Repositories

For packages from third-party repositories:

```toml
[packages]
system = ["terraform", "kubectl"]
setup_script = """
#!/bin/bash
set -e

# Add HashiCorp repository
curl -fsSL https://apt.releases.hashicorp.com/gpg | \
  sudo gpg --dearmor -o /usr/share/keyrings/hashicorp-archive-keyring.gpg
echo "deb [signed-by=/usr/share/keyrings/hashicorp-archive-keyring.gpg] \
  https://apt.releases.hashicorp.com $(lsb_release -cs) main" | \
  sudo tee /etc/apt/sources.list.d/hashicorp.list

# Add Kubernetes repository
curl -fsSL https://pkgs.k8s.io/core:/stable:/v1.28/deb/Release.key | \
  sudo gpg --dearmor -o /etc/apt/keyrings/kubernetes-apt-keyring.gpg
echo "deb [signed-by=/etc/apt/keyrings/kubernetes-apt-keyring.gpg] \
  https://pkgs.k8s.io/core:/stable:/v1.28/deb/ /" | \
  sudo tee /etc/apt/sources.list.d/kubernetes.list
"""
```

**Important:** Setup scripts must be idempotent (safe to run multiple times).

### Package Features

- **Batch installation**: All packages install in one operation
- **Validation**: Package names are validated to prevent injection
- **Architecture support**: Specify architecture with `:arch` syntax
- **Version constraints**: Exact versions or wildcards

See [Custom Packages](advanced/custom-packages.md) for more details.

## Scripts

Configure setup and runtime scripts.

### Setup Scripts

Run during template creation (`claude-vm setup`):

```toml
[setup]
scripts = [
    "./scripts/install-extras.sh",
    "./scripts/configure-env.sh"
]
```

**Auto-detected scripts** (no config needed):

- `~/.claude-vm.setup.sh` - Global setup script
- `./.claude-vm.setup.sh` - Project setup script

### Runtime Scripts

Run before each session (`claude-vm` or `claude-vm shell`):

```toml
[runtime]
scripts = [
    "./scripts/start-services.sh",
    "./scripts/load-env.sh"
]
```

**Auto-detected scripts** (no config needed):

- `./.claude-vm.runtime.sh` - Project runtime script

### Script Execution Order

**Setup (during `claude-vm setup`):**

1. Global setup script (`~/.claude-vm.setup.sh`)
2. Project setup script (`./.claude-vm.setup.sh`)
3. Config setup scripts (from `[setup] scripts`)
4. CLI setup scripts (from `--setup-script`)

**Runtime (before each session):**

1. Project runtime script (`./.claude-vm.runtime.sh`)
2. Config runtime scripts (from `[runtime] scripts`)
3. CLI runtime scripts (from `--runtime-script`)

See [Runtime Scripts](features/runtime-scripts.md) for detailed information.

## Default Arguments

Configure default arguments passed to Claude.

```toml
[defaults]
# Arguments passed to every Claude invocation
claude_args = [
    "--dangerously-skip-permissions",  # Enabled by default
    "--max-tokens", "4096"
]

# Auto-create template if missing
auto_setup = true
```

### Claude Arguments

By default, Claude VM passes `--dangerously-skip-permissions` to Claude since the VM provides isolation. You can add or override arguments:

```toml
[defaults]
claude_args = [
    "--dangerously-skip-permissions",
    "--max-tokens", "4096",
    "--model", "claude-opus-4"
]
```

To disable auto-permissions (not recommended):

```toml
[defaults]
claude_args = []  # Empty array = no default args
```

### Auto-Setup

Automatically create templates when missing:

```toml
[defaults]
auto_setup = true
```

Or via CLI:

```bash
claude-vm --auto-setup "help me code"
```

## Claude Context

Provide project-specific instructions to Claude.

### Inline Instructions

```toml
[context]
instructions = """
This is a Rust project using:
- Cargo for build management
- Tokio for async runtime
- Serde for serialization

Please:
- Include code examples in responses
- Follow Rust best practices
- Use proper error handling with Result types
"""
```

### Load from File

```toml
[context]
instructions_file = ".claude-context.md"
```

Create `.claude-context.md`:

```markdown
# Project Context

This is a Rust CLI project.

## Architecture

- Clap for CLI parsing
- TOML for configuration
- Modular capability system

## Standards

- Follow Rust idioms
- Use Result for errors
- Document public APIs
```

### Precedence

If both `instructions` and `instructions_file` are set, `instructions` takes precedence.

### Generated Context

Claude automatically receives context about:

- VM configuration (disk, memory)
- Enabled tools and versions
- Mounted directories
- Runtime script results
- Your custom instructions

This is merged into `~/.claude/CLAUDE.md` before each session.

## Custom Mounts

Mount additional directories in the VM.

### Configuration

```toml
# Simple mount (same path in VM, writable)
[[mounts]]
location = "/Users/me/data"
writable = true

# Mount with custom VM path
[[mounts]]
location = "/Users/me/shared"
mount_point = "/vm/shared"
writable = false

# Tilde expansion supported
[[mounts]]
location = "~/Documents"
writable = true
```

### CLI Mounts

```bash
# Docker-style syntax
claude-vm --mount /host/data shell
claude-vm --mount /host/data:/vm/data shell
claude-vm --mount /host/data:/vm/data:ro shell
```

### Setup-Only Mounts

Mounts available only during template creation:

```toml
[[setup.mounts]]
location = "~/local-tools"
mount_point = "/tmp/tools"
writable = false
```

Use in setup script to copy files into template:

```bash
# .claude-vm.setup.sh
cp /tmp/tools/my-tool /usr/local/bin/
```

See [Custom Mounts](features/custom-mounts.md) for more details.

## Environment Variables

Override configuration with environment variables.

### Available Variables

```bash
# VM resources
export CLAUDE_VM_DISK=30
export CLAUDE_VM_MEMORY=16

# Use in commands
claude-vm setup --git
```

### Passing Variables to VM

Pass environment variables into the VM:

```bash
# Set individual variables
claude-vm --env API_KEY=secret --env DEBUG=true

# Load from file
claude-vm --env-file .env

# Inherit from host
claude-vm --inherit-env PATH --inherit-env USER

# Combine
claude-vm --env-file .env --env API_KEY=override --inherit-env USER
```

Variables are available to:

- Claude process
- Runtime scripts
- Commands in `claude-vm shell`

## Validation

Validate your configuration files:

```bash
# Validate current project config
claude-vm config validate
```

### Valid Ranges

| Setting        | Type    | Range/Values        |
| -------------- | ------- | ------------------- |
| `disk`         | number  | 1-1000 (GB)         |
| `memory`       | number  | 1-64 (GB)           |
| `tools.*`      | boolean | true/false          |
| `scripts`      | array   | file paths          |
| `claude_args`  | array   | strings             |
| `instructions` | string  | multiline supported |
| `auto_setup`   | boolean | true/false          |

### Common Validation Errors

**Type error:**

```toml
[vm]
disk = "thirty"  # ❌ Must be a number
```

**Out of range:**

```toml
[vm]
memory = 128  # ❌ Must be 1-64
```

**Invalid path:**

```toml
[setup]
scripts = ["script.sh"]  # ❌ Must be absolute or start with ./
```

### Show Effective Configuration

View merged configuration after applying precedence:

```bash
claude-vm config show
```

## Complete Example

```toml
# VM resources
[vm]
disk = 30
memory = 16

# Install tools
[tools]
git = true
docker = true
node = true
gpg = true

# Custom packages
[packages]
system = ["postgresql-client", "jq", "htop"]

# Setup scripts (run during template creation)
[setup]
scripts = ["./scripts/install-extras.sh"]

# Runtime scripts (run before each session)
[runtime]
scripts = ["./scripts/start-services.sh"]

# Default options
[defaults]
claude_args = ["--dangerously-skip-permissions", "--max-tokens", "4096"]
auto_setup = true

# Project context for Claude
[context]
instructions = """
This is a Node.js API project.
Follow REST best practices.
Include error handling in examples.
"""

# Custom mounts
[[mounts]]
location = "~/datasets"
mount_point = "/data"
writable = false

# Setup-only mounts
[[setup.mounts]]
location = "~/local-tools"
mount_point = "/tmp/tools"
writable = false
```

## Next Steps

- **[Usage Guide](usage.md)** - Learn all commands
- **[Tools](features/tools.md)** - Understand available tools
- **[Runtime Scripts](features/runtime-scripts.md)** - Automate environment setup
- **[Custom Packages](advanced/custom-packages.md)** - Install additional packages
