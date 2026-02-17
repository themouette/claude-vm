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

1. **Command-line flags** - `--disk 30 --memory 16 --cpus 4`
2. **Environment variables** - `CLAUDE_VM_DISK=30 CLAUDE_VM_MEMORY=16 CLAUDE_VM_CPUS=4`
3. **Project config** - `./.claude-vm.toml`
4. **Global config** - `~/.claude-vm.toml`
5. **Built-in defaults** - `disk=20, memory=8, cpus=4`

**Example:**

```bash
# Given:
# - Global config: disk=20
# - Project config: disk=30
# - CLI flag: --disk 40

claude-vm setup --disk 40  # Uses disk=40 (CLI wins)
claude-vm setup            # Uses disk=30 (project config)
```

### Git Worktree Configuration

When working in a git worktree, configuration is loaded from both the worktree and main repository:

1. **Command-line flags** - Highest priority
2. **Environment variables**
3. **Worktree config** - `./.claude-vm.toml` in worktree directory
4. **Main repo config** - `.claude-vm.toml` in main repository
5. **Global config** - `~/.claude-vm.toml`
6. **Built-in defaults** - Lowest priority

This allows you to:
- Define common settings in the main repository
- Override settings per worktree (e.g., different memory for testing)
- Share the same VM template across all worktrees

See [Git Integration](git-integration.md) for more details.

## VM Settings

Configure VM resources.

```toml
[vm]
disk = 20      # Disk size in GB (default: 20, range: 1-1000)
memory = 8     # Memory size in GB (default: 8, range: 1-64)
cpus = 4       # Number of CPUs (default: 4, range: 1-32)
```

**Valid ranges:**

- `disk`: 1-1000 GB
- `memory`: 1-64 GB
- `cpus`: 1-32

**Override via CLI:**

```bash
claude-vm --disk 30 --memory 16 --cpus 4 setup --git
```

**Override via environment:**

```bash
export CLAUDE_VM_DISK=30
export CLAUDE_VM_MEMORY=16
export CLAUDE_VM_CPUS=4
claude-vm setup --git
```

**CI Environment Constraints:**

When running in CI environments (GitHub Actions, GitLab CI, CircleCI), resource limits are automatically reduced to ensure compatibility with CI runners:

- CPUs: 1 (instead of default 4)
- Memory: 1 GB (instead of default 8 GB)

This is automatically detected via `CI`, `GITHUB_ACTIONS`, `GITLAB_CI`, or `CIRCLECI` environment variables. You can override these constraints using CLI flags or environment variables if your CI environment supports higher limits.

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

Configure setup and runtime scripts using either the legacy format or the new phase-based format.

### Phase-Based Scripts (Recommended)

The new phase-based format provides better organization and control over script execution.

#### Setup Phases

Run during template creation (`claude-vm setup`):

```toml
# Verify system requirements
[[phase.setup]]
name = "verify-system"
script = """
#!/bin/bash
echo 'Verifying system requirements'
test $(nproc) -ge 2 || exit 1
"""

# Install Docker (only if not present)
[[phase.setup]]
name = "install-docker"
when = "! command -v docker"  # Only run if docker not installed
env = { DEBIAN_FRONTEND = "noninteractive" }
script_files = ["./scripts/install-docker.sh"]

# Configure tools
[[phase.setup]]
name = "configure"
script = "echo 'Setup complete'"
```

#### Runtime Phases

Run before each session (`claude-vm` or `claude-vm shell`):

```toml
# Start services
[[phase.runtime]]
name = "start-services"
env = { DEBUG = "true", COMPOSE_PROJECT_NAME = "myapp" }
script = "docker-compose up -d"

# Wait for health check
[[phase.runtime]]
name = "wait-for-health"
continue_on_error = false
script = """
until curl -sf http://localhost:3000/health; do
  echo 'Waiting for service...'
  sleep 1
done
echo '✓ Services ready'
"""

# Optional service (won't fail if it doesn't work)
[[phase.runtime]]
name = "optional-service"
continue_on_error = true
script_files = ["./scripts/start-optional.sh"]
```

#### Host Phases

Host phases run on the **HOST machine** (not inside the VM) at specific lifecycle points. They enable operations that need to execute outside the VM, such as exporting host configuration, refreshing credentials, or collecting logs.

**Available Host Phase Types:**

| Phase | When | Use Cases |
|-------|------|-----------|
| `[[phase.host.before_setup]]` | Before VM setup scripts | Export host config, prepare data files |
| `[[phase.host.after_setup]]` | After VM setup, before template save | Validate setup, backup template |
| `[[phase.host.before_runtime]]` | Before each session starts | Refresh tokens, verify prerequisites |
| `[[phase.host.after_runtime]]` | After runtime scripts complete | Collect metrics, validate session |
| `[[phase.host.teardown]]` | When session ends | Cleanup, save logs, notify systems |

**Example: AWS Token Refresh**

```toml
[[phase.host.before_runtime]]
name = "aws-sso-login"
script = "aws sso login --profile dev"
when = "! aws sts get-caller-identity --profile dev"
continue_on_error = true
```

**Example: Template Backup**

```toml
[[phase.host.after_setup]]
name = "backup-template"
script = """
#!/bin/bash
limactl shell $TEMPLATE_NAME -- tar czf /tmp/backup.tar.gz /home
limactl copy $TEMPLATE_NAME:/tmp/backup.tar.gz ./template-backup.tar.gz
"""
```

**Example: Session Logging**

```toml
[[phase.host.teardown]]
name = "save-session-logs"
script = "limactl shell $LIMA_INSTANCE -- journalctl > session-$(date +%s).log"
continue_on_error = true
```

**Environment Variables:**

Host phases receive these standard environment variables:

- `PROJECT_ROOT` - Project directory path
- `TEMPLATE_NAME` - VM template name
- `LIMA_INSTANCE` - VM instance name (runtime and teardown phases only)
- `PHASE_TYPE` - Phase type (setup, runtime, or teardown)

Host phases support all the same features as VM phases: `name`, `script`, `script_files`, `env`, `when`, `continue_on_error`.

#### Sourcing Scripts for Persistent Exports

When you need exports (like PATH modifications) to persist across phases, use `source = true`:

```toml
# Add custom tools to PATH (exports persist)
[[phase.runtime]]
name = "setup-custom-path"
source = true
script = """
export PATH="$HOME/.local/bin:$PATH"
export CUSTOM_VAR="value"
"""

# This phase can now use the modified PATH
[[phase.runtime]]
name = "use-custom-tools"
script = """
#!/bin/bash
# Can now use tools from ~/.local/bin
custom-tool --version
echo "CUSTOM_VAR=$CUSTOM_VAR"  # Variable is available
"""
```

**When to use `source`:**
- ✅ Adding directories to PATH
- ✅ Exporting environment variables that should persist
- ✅ Setting up shell functions or aliases (for runtime phases)

**When NOT to use `source`:**
- ❌ Default behavior (subprocess isolation is safer)
- ❌ Scripts that modify the filesystem (no benefit from sourcing)
- ❌ Long-running processes or background tasks

#### Phase Fields

| Field              | Type              | Required | Description                                          |
| ------------------ | ----------------- | -------- | ---------------------------------------------------- |
| `name`             | string            | Yes      | Phase name (for logging/debugging)                   |
| `script`           | string            | No       | Inline script content                                |
| `script_files`     | array of strings  | No       | File paths to execute (in order)                     |
| `env`              | map               | No       | Phase-specific environment variables                 |
| `continue_on_error`| boolean           | No       | Don't fail if phase fails (default: false)           |
| `when` / `if`      | string            | No       | Conditional - only run if command succeeds (exit 0)  |
| `source`           | boolean           | No       | Source script instead of running in subprocess (default: false). When true, exports persist to subsequent phases. |

**Note:** At least one of `script` or `script_files` must be provided.

#### Features

- **Inline scripts**: Write scripts directly in the TOML file
- **File scripts**: Reference external script files
- **Mixed mode**: Combine inline and file scripts in the same phase
- **Environment variables**: Set phase-specific env vars
- **Conditional execution**: Run phases only when conditions are met
- **Error handling**: Control whether failures stop execution
- **Named phases**: Better logging and debugging output
- **Script sourcing**: Source scripts to persist exports (like PATH modifications) across phases

### Legacy Format (Deprecated)

> ⚠️  **Deprecated**: The `[setup]` and `[runtime]` scripts arrays are deprecated. Please migrate to `[[phase.setup]]` and `[[phase.runtime]]`. The legacy format continues to work with deprecation warnings.

#### Setup Scripts

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

#### Runtime Scripts

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
3. Legacy config setup scripts (from `[setup] scripts`)
4. Phase-based setup scripts (from `[[phase.setup]]`)
5. CLI setup scripts (from `--setup-script`)

**Runtime (before each session):**

1. Project runtime script (`./.claude-vm.runtime.sh`)
2. Legacy config runtime scripts (from `[runtime] scripts`)
3. Phase-based runtime scripts (from `[[phase.runtime]]`)
4. CLI runtime scripts (from `--runtime-script`)

See [Runtime Scripts](features/runtime-scripts.md) for detailed information.

### Troubleshooting Phase Scripts

#### Exports Don't Persist Across Phases

**Problem**: Environment variables exported in one phase aren't available in the next phase.

```toml
[[phase.runtime]]
name = "setup-path"
script = "export PATH=$HOME/.local/bin:$PATH"

[[phase.runtime]]
name = "use-tool"
script = "my-custom-tool"  # Command not found!
```

**Solution**: Add `source = true` to make exports persist:

```toml
[[phase.runtime]]
name = "setup-path"
source = true  # ← Exports now persist!
script = "export PATH=$HOME/.local/bin:$PATH"
```

**Why**: By default, scripts run in a subprocess (subprocess isolation). Use `source = true` to run the script in the current shell context.

#### Runtime Script Runs Multiple Times

**Problem**: Runtime phases execute on **every** `claude-vm` command, causing duplicates:

```bash
# ❌ BAD: Appends every time
echo "export PATH=..." >> ~/.profile  # ~/.profile grows infinitely!
```

**Solution**: Make runtime scripts idempotent (safe to run multiple times):

```bash
# ✅ GOOD: Check before adding
if ! grep -q "my-custom-path" ~/.profile; then
    echo "export PATH=..." >> ~/.profile
fi

# ✅ GOOD: Use absolute PATH (doesn't grow)
export PATH="$HOME/.local/bin:$PATH"  # Safe to repeat
```

**Best Practice**: Runtime phases should be **fast** (<1 second) since they run on every command.

#### Phase Fails Silently

**Problem**: A phase fails but setup/runtime continues without error.

**Solution**: Check the `continue_on_error` setting:

```toml
[[phase.setup]]
name = "optional-tools"
continue_on_error = true  # ← Phase failure won't stop execution
script = "install-optional-tool"
```

Remove `continue_on_error` (or set to `false`) to fail-fast:

```toml
[[phase.setup]]
name = "required-tools"
# continue_on_error defaults to false
script = "install-required-tool"  # Failure stops execution
```

#### Conditional Phase Doesn't Run

**Problem**: Phase with `when` condition never executes.

```toml
[[phase.setup]]
name = "install-docker"
when = "! command -v docker"  # Never runs!
script = "brew install docker"
```

**Solution**: Check the condition manually in a shell:

```bash
# In VM shell:
claude-vm shell

# Test condition:
! command -v docker && echo "Would run" || echo "Would skip"
```

**Common Issues**:
- Condition has wrong exit code (remember: 0 = true, non-zero = false)
- Tool is already installed but in different location
- Syntax error in condition

#### Script Has Shebang But Uses Wrong Interpreter

**Problem**: Script has `#!/usr/bin/python` but runs with bash when `source = true`.

```toml
[[phase.runtime]]
name = "python-script"
source = true  # ← Sourcing ignores shebang!
script = """
#!/usr/bin/python
print("Hello")
"""
```

**Solution**: When using `source = true`, the script runs in the current bash shell. The shebang is **ignored**.

**Options**:
1. Remove `source = true` to execute with shebang interpreter:
```toml
[[phase.runtime]]
name = "python-script"
# source = false (default) - respects shebang
script = """
#!/usr/bin/python
print("Hello")
"""
```

2. Use bash syntax if sourcing is required:
```toml
[[phase.runtime]]
name = "setup-env"
source = true
script = """
#!/bin/bash
export MY_VAR="value"
"""
```

#### Phase Environment Variables Not Available

**Problem**: Env vars set in `env` field not available in subsequent phases.

```toml
[[phase.runtime]]
name = "set-env"
env = { DEBUG = "true" }
script = "echo $DEBUG"  # Works here

[[phase.runtime]]
name = "use-env"
script = "echo $DEBUG"  # Empty!
```

**Explanation**: Phase-specific env vars are isolated by default (subprocess).

**Solution A**: Export from script with `source = true`:
```toml
[[phase.runtime]]
name = "set-env"
source = true
script = "export DEBUG=true"
```

**Solution B**: Set env vars in each phase that needs them:
```toml
[[phase.runtime]]
name = "use-env"
env = { DEBUG = "true" }
script = "echo $DEBUG"
```

#### Script Works Locally But Fails in VM

**Problem**: Script succeeds when run directly but fails in phase.

**Common Causes**:
1. **Missing tools**: VM doesn't have same tools as host
   ```bash
   # Check what's available:
   claude-vm shell
   which my-tool
   ```

2. **Different PATH**: VM has different PATH
   ```bash
   # Debug in VM:
   echo $PATH
   ```

3. **Working directory**: Script assumes specific directory
   ```toml
   # Solution: Use absolute paths
   [[phase.setup]]
   name = "install"
   script = "cd /full/path && ./install.sh"
   ```

#### Performance: Runtime Phases Are Slow

**Problem**: `claude-vm shell` takes 10+ seconds to start.

**Cause**: Runtime phases execute on **every** command.

**Solution**: Optimize runtime phases:

```toml
# ❌ BAD: Network calls in runtime
[[phase.runtime]]
name = "check-api"
script = "curl -s https://api.example.com/status"  # Slow!

# ✅ GOOD: Fast local checks only
[[phase.runtime]]
name = "check-local"
script = "test -f ~/.configured && echo 'Ready'"

# ✅ GOOD: Cache expensive operations
[[phase.runtime]]
name = "check-cached"
script = """
if [ ! -f /tmp/api-status ] || [ $(($(date +%s) - $(stat -f %m /tmp/api-status))) -gt 300 ]; then
  curl -s https://api.example.com/status > /tmp/api-status
fi
cat /tmp/api-status
"""
```

**Best Practices**:
- Keep runtime phases under 1 second
- Use setup phases for slow operations
- Cache results when possible
- Avoid network calls

#### Getting More Debug Information

Enable verbose output to see what phases are running:

```bash
# Setup with debug output:
claude-vm setup --no-agent-install 2>&1 | tee setup.log

# Runtime with debug:
claude-vm shell bash -c 'set -x; env'
```

Check the generated entrypoint script:
```bash
# The entrypoint shows all phases
claude-vm shell cat /tmp/entrypoint.sh
```

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
export CLAUDE_VM_CPUS=4

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

# Setup phases (run during template creation)
[[phase.setup]]
name = "verify-requirements"
script = """
#!/bin/bash
echo 'Verifying system requirements'
test $(nproc) -ge 2 || exit 1
"""

[[phase.setup]]
name = "install-extras"
when = "test -f ./scripts/install-extras.sh"
script_files = ["./scripts/install-extras.sh"]

# Runtime phases (run before each session)
[[phase.runtime]]
name = "start-services"
env = { COMPOSE_PROJECT_NAME = "myapp", DEBUG = "true" }
script = "docker-compose up -d"

[[phase.runtime]]
name = "wait-for-db"
continue_on_error = false
script = """
until pg_isready -h localhost -p 5432; do
  echo 'Waiting for database...'
  sleep 1
done
echo '✓ Database ready'
"""

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
