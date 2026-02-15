# Capabilities

This directory contains capability definitions for claude-vm. Capabilities are modular components that extend the VM's functionality.

## Directory Structure

Each capability is organized in its own directory with colocated configuration and scripts:

```
capabilities/
├── docker/
│   ├── capability.toml    # Capability definition
│   └── setup.sh           # Setup script
├── node/
│   ├── capability.toml
│   └── setup.sh
├── python/
│   ├── capability.toml
│   └── setup.sh
├── chromium/
│   ├── capability.toml
│   └── setup.sh
├── gpg/
│   └── capability.toml    # Uses inline scripts, no separate file
└── README.md              # This file
```

## What is a Capability?

A capability is a self-contained unit that can:
- Install software during VM setup
- Run initialization code before each session
- Register MCP (Model Context Protocol) servers
- Declare resource requirements (e.g., socket forwarding)

## Capability Structure

Each capability is defined in a TOML file with this structure:

```toml
[capability]
id = "unique-id"
name = "Human Readable Name"
description = "What this capability provides"
requires = ["other-capability"]  # Optional: Dependencies on other capabilities

# Optional: Declarative package management
[packages]
system = ["package1", "package2"]  # Debian packages to install
setup_script = """
#!/bin/bash
# Optional: Add custom repositories before package installation
# Must be idempotent (safe to run multiple times)
"""

# Phase-based execution for both host and VM operations

# Host phases run on the HOST machine (not inside VM)
[[phase.host.before_setup]]
name = "export-host-config"
script = """
#!/bin/bash
# Runs on macOS/Linux host before VM setup
# Can validate prerequisites, export configuration
"""

[[phase.host.before_runtime]]
name = "refresh-credentials"
script = "aws sso login --profile dev"
when = "! aws sts get-caller-identity --profile dev"
continue_on_error = true

[[phase.host.teardown]]
name = "cleanup"
script = "echo 'Session ended at $(date)' >> ~/vm-sessions.log"

# VM setup phases run during template creation (inside VM)
[[phase.setup]]
name = "capability-setup"
script_files = ["vm_setup.sh"]  # Reference embedded script
# OR
# script = """
# #!/bin/bash
# # Inline script content
# # Note: Use [packages] for installing system packages instead
# """

# VM runtime phases run before each session (inside VM)
[[phase.runtime]]
name = "capability-init"
script = """
#!/bin/bash
# Initialize environment for session
export SOME_VAR=value
"""
# Optional phase features:
# env = { "CUSTOM_VAR" = "value" }  # Phase-specific environment variables
# continue_on_error = true          # Don't fail if this phase fails
# when = "command -v tool"          # Conditional execution
# source = true                     # Source instead of executing in subprocess

# Optional: Register MCP servers
[[mcp]]
id = "server-name"
command = "npx"
args = ["-y", "package@latest"]
enabled_when = "other-capability"  # Optional: Only if another capability is enabled

# Optional: Declare forwarding requirements
[[forwards]]
type = "unix_socket"
host = { detect = "command-to-detect-socket" }
guest = "/path/in/vm"
```

## Declarative Package Management

Capabilities can declare system packages directly in their TOML files using the `[packages]` section. This eliminates the need for manual `apt-get install` commands in setup scripts.

### Basic Package Declaration

```toml
[capability]
id = "python"
name = "Python"
description = "Python 3 with pip and development tools"

[packages]
system = ["python3", "python3-pip", "python3-venv"]
```

### Advanced Features

**Version Pinning:**
```toml
[packages]
system = [
    "python3=3.11.0-1",      # Exact version
    "nodejs=22.*",           # Wildcard version
    "libc6:amd64"            # Architecture specification
]
```

**Custom Repositories:**
```toml
[packages]
system = ["docker-ce", "docker-ce-cli", "containerd.io"]
setup_script = """
#!/bin/bash
set -e
# Add Docker's official GPG key and repository

# Ensure keyring directory exists
sudo mkdir -p /etc/apt/keyrings
sudo chmod 755 /etc/apt/keyrings

# Download GPG key only if not present
if [ ! -f /etc/apt/keyrings/docker.asc ]; then
    sudo curl -fsSL https://download.docker.com/linux/debian/gpg \\
        -o /etc/apt/keyrings/docker.asc
    sudo chmod a+r /etc/apt/keyrings/docker.asc
fi

# Add repository only if not configured
if ! grep -q "download.docker.com" /etc/apt/sources.list.d/docker.list 2>/dev/null; then
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/debian $(. /etc/os-release && echo \"$VERSION_CODENAME\") stable" | \\
        sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
fi
"""
```

### Benefits

- **Declarative**: Packages defined in data, not imperative scripts
- **Optimized**: All packages install in a single batch operation
- **Validated**: Package names are validated to prevent shell injection
- **Deduplicated**: Duplicate packages across capabilities are automatically removed
- **Ordered**: Dependency order is preserved during deduplication

### Package Installation Flow

1. Base packages install first (git, curl, wget, etc.) without `apt-get update`
2. All capability `setup_script`s run to add custom repositories
3. Single `apt-get update` executes
4. All packages from all capabilities install in one batch operation
5. Individual capability setup phases run for post-install configuration

### Migration from Shell Scripts

**Before (imperative):**
```toml
[[phase.setup]]
name = "install-python"
script = """
#!/bin/bash
set -e
sudo apt-get update
sudo apt-get install -y python3 python3-pip python3-venv
"""
```

**After (declarative):**
```toml
[packages]
system = ["python3", "python3-pip", "python3-venv"]

# phase.setup now only handles post-install configuration if needed
```

## Available Capabilities

### docker
Installs Docker engine in the VM for container management.

### node
Installs Node.js 20 LTS with npm package manager.

### python
Installs Python 3 with pip and development tools.

### chromium
Installs Chromium browser for headless automation and testing.
Registers the Chrome DevTools MCP server (requires Node.js/npm to be available).

### gpg
Enables GPG agent forwarding from host to VM (experimental).
Copies public keys and configures SSH for socket forwarding.

## Adding a New Capability

1. Create a new directory: `capabilities/my-capability/`

2. Create `capability.toml` in that directory:
   ```toml
   [capability]
   id = "my-capability"
   name = "My Capability"
   description = "What this capability provides"

   [[phase.setup]]
   name = "my-capability-setup"
   script_files = ["setup.sh"]  # Or use inline script
   ```

3. Create setup script if needed: `capabilities/my-capability/setup.sh`

4. Register the capability in `src/capabilities/registry.rs`:
   ```rust
   const CAPABILITY_FILES: &[(&str, &str)] = &[
       // ... existing capabilities ...
       ("my-capability", include_str!("../../capabilities/my-capability/capability.toml")),
   ];
   ```

5. Add script loading in `src/capabilities/executor.rs` (if using script_files):
   ```rust
   pub(crate) fn get_embedded_script(capability_id: &str, script_name: &str) -> Result<String> {
       let content = match (capability_id, script_name) {
           // ... existing cases ...
           ("my-capability", "setup.sh") => include_str!("../../capabilities/my-capability/setup.sh"),
           // ...
       };
   }
   ```

6. Add configuration field in `src/config.rs`:
   ```rust
   pub struct ToolsConfig {
       // ... existing tools ...
       #[serde(default)]
       pub my_capability: bool,
   }
   ```

7. Update the `is_enabled` method in `src/capabilities/registry.rs`:
   ```rust
   fn is_enabled(&self, id: &str, config: &Config) -> bool {
       match id {
           // ... existing cases ...
           "my-capability" => config.tools.my_capability,
           _ => false,
       }
   }
   ```

That's it! The capability system handles everything else automatically.

## Phase-Based Execution

Capabilities use a unified phase-based execution model for both host and VM operations:

### Setup Phases (`[[phase.setup]]`)
Run during template creation. Each capability can define multiple setup phases that run sequentially.

**Features:**
- Multiple phases per capability for better organization
- Named phases for easier debugging
- Optional execution with `when` conditions
- Error handling with `continue_on_error`
- Phase-specific environment variables

**Example:**
```toml
[[phase.setup]]
name = "install-tools"
script_files = ["vm_setup.sh"]

[[phase.setup]]
name = "configure-tools"
script = """
#!/bin/bash
# Post-installation configuration
"""
when = "command -v tool"  # Only run if tool is installed
```

### Runtime Phases (`[[phase.runtime]]`)
Run before each Claude Code session. Use these for environment initialization.

**Features:**
- Lightweight and fast (should complete in < 1 second)
- Can source scripts to persist exports
- Conditional execution
- Phase-specific environment variables

**Example:**
```toml
[[phase.runtime]]
name = "check-auth"
script = """
#!/bin/bash
if ! gh auth status &>/dev/null; then
  echo "⚠ Warning: GitHub CLI not authenticated"
fi
"""
source = false  # Run in subprocess (default)

[[phase.runtime]]
name = "export-env"
script = """
export CUSTOM_VAR=value
"""
source = true  # Source to persist exports
```

### Environment Variables Reference

All capability scripts automatically receive environment variables providing context about the VM, project, and execution phase. Here's a quick reference:

| Variable | Host Phases | VM Setup | VM Runtime | Description |
|----------|-------------|----------|------------|-------------|
| `CAPABILITY_ID` | ✓ | ✓ | ✓ | Capability identifier (e.g., "gh", "docker") |
| `TEMPLATE_NAME` | ✓ | ✓ | ✓ | VM template name |
| `LIMA_INSTANCE` | runtime+ | ✓ | ✓ | VM instance name (ephemeral for runtime) |
| `PHASE_TYPE` | ✓ | - | - | Phase type: "setup", "runtime", or "teardown" |
| `CLAUDE_VM_PHASE` | ✓ | ✓ | ✓ | Execution phase identifier |
| `CLAUDE_VM_VERSION` | - | ✓ | ✓ | Version of claude-vm tool |
| `PROJECT_ROOT` | ✓ | ✓ | ✓ | Project directory path |
| `PROJECT_NAME` | - | ✓ | ✓ | Full project name extracted from directory |
| `PROJECT_WORKTREE_ROOT` | - | ✓ | ✓ | Main project root if using git worktrees (empty otherwise) |
| `PROJECT_WORKTREE` | - | ✓ | ✓ | Current worktree path if using git worktrees (empty otherwise) |

**Note**: `runtime+` means available in `before_runtime`, `after_runtime`, and `teardown` host phases. All variables are automatically exported as environment variables.

## Lifecycle Phases

### Host Phases (run on host machine)

Host phases execute on the **HOST** machine (macOS/Linux) at specific lifecycle points:

#### `[[phase.host.before_setup]]`
- **When**: Before VM setup begins
- **Purpose**: Export host configuration, validate prerequisites, prepare data
- **Example Use Cases**: Copy host git config, export GPG keys, validate AWS credentials

#### `[[phase.host.after_setup]]`
- **When**: After VM setup completes, before template is saved
- **Purpose**: Validate setup, backup template, collect metrics
- **Example Use Cases**: Verify VM state, create template backups, run health checks

#### `[[phase.host.before_runtime]]`
- **When**: Before each session starts (after VM boots)
- **Purpose**: Refresh credentials, verify prerequisites
- **Example Use Cases**: AWS SSO login, verify GPG agent, check network connectivity

#### `[[phase.host.after_runtime]]`
- **When**: After VM runtime phases complete
- **Purpose**: Validate session readiness, collect metrics
- **Example Use Cases**: Verify services started, log session info

#### `[[phase.host.teardown]]`
- **When**: When session ends (in VmSession::Drop)
- **Purpose**: Cleanup, save logs, notify external systems
- **Example Use Cases**: Archive logs, update metrics, send notifications
- **Special**: Always runs even if session errors; errors logged as warnings

### VM Setup Phases (`[[phase.setup]]`)
- **When**: In guest VM during `claude-vm setup`
- **Where**: Inside Lima VM (guest)
- **Purpose**: Install software, configure system
- **Format**: Array of phase blocks `[[phase.setup]]`
- **Features**: Multiple phases, conditional execution, error handling
- **Environment Variables**:
  - `TEMPLATE_NAME` - VM template name
  - `LIMA_INSTANCE` - VM instance name
  - `CAPABILITY_ID` - Capability identifier
  - `CLAUDE_VM_PHASE` - Always "setup"
  - `CLAUDE_VM_VERSION` - Version of claude-vm tool
  - `PROJECT_ROOT` - Project directory path (host path for reference)
  - `PROJECT_NAME` - Full project name from host
  - `PROJECT_WORKTREE_ROOT` - Main project root (if worktree, else empty)
  - `PROJECT_WORKTREE` - Current worktree path (if worktree, else empty)

### phase.runtime (Runtime Phases)
- **When**: In VM before each `claude-vm agent`
- **Where**: Inside ephemeral VM session
- **Purpose**: Initialize environment variables, check status, start services
- **Format**: Array of phase blocks `[[phase.runtime]]`
- **Features**: Multiple phases, sourcing support, conditional execution
- **Note**: Keep lightweight (< 1 second total)
- **Environment Variables**:
  - `TEMPLATE_NAME` - VM template name
  - `LIMA_INSTANCE` - Ephemeral VM instance name (different from template)
  - `CAPABILITY_ID` - Capability identifier
  - `CLAUDE_VM_PHASE` - Always "runtime"
  - `CLAUDE_VM_VERSION` - Version of claude-vm tool
  - `PROJECT_ROOT` - Mounted project directory in VM
  - `PROJECT_NAME` - Full project name
  - `PROJECT_WORKTREE_ROOT` - Main project root (if worktree, else empty)
  - `PROJECT_WORKTREE` - Current worktree path (if worktree, else empty)

### Using Environment Variables in Scripts

All environment variables are automatically available in your capability scripts. Here's how to use them:

**Example: Basic usage in phase.setup**
```toml
[[phase.setup]]
name = "install-my-capability"
script = """
#!/bin/bash
set -e

# All environment variables are automatically provided
echo "Setting up $CAPABILITY_ID for project: $PROJECT_NAME"
echo "VM: $TEMPLATE_NAME (instance: $LIMA_INSTANCE)"
echo "Phase: $CLAUDE_VM_PHASE"

# Use PROJECT_ROOT for project-specific configuration
if [ -f "$PROJECT_ROOT/config.json" ]; then
    echo "Found project configuration"
fi
"""
```

**Example: Worktree detection**
```toml
[[phase.setup]]
name = "configure-worktree"
script = """
#!/bin/bash
set -e

if [ -n "$PROJECT_WORKTREE_ROOT" ]; then
    echo "This is a git worktree"
    echo "Main repository: $PROJECT_WORKTREE_ROOT"
    echo "Current worktree: $PROJECT_WORKTREE"
else
    echo "This is a regular project (not a worktree)"
fi
"""
```

**Example: Phase-specific behavior**
```toml
[[phase.runtime]]
name = "generate-context"
script = """
#!/bin/bash

# Different behavior based on phase
if [ "$CLAUDE_VM_PHASE" = "runtime" ]; then
    # Generate context for Claude
    mkdir -p ~/.claude-vm/context
    cat > ~/.claude-vm/context/my-capability.txt <<EOF
Capability: $CAPABILITY_ID
Project: $PROJECT_NAME
VM Instance: $LIMA_INSTANCE (ephemeral)
Version: $CLAUDE_VM_VERSION
EOF
fi
"""
```

**Example: Using in external script files**
```bash
# capabilities/my-capability/setup.sh
#!/bin/bash
set -e

# Environment variables are automatically available
echo "Setting up for project: $PROJECT_NAME"

# Use template name for VM-specific paths
CONFIG_DIR="/home/lima.linux/.config/$TEMPLATE_NAME"
mkdir -p "$CONFIG_DIR"

# Reference project files (during setup, PROJECT_ROOT is host path)
echo "Project location (host): $PROJECT_ROOT"
```

## Best Practices

1. **Keep capabilities focused**: Each capability should do one thing well
2. **Declare dependencies**: Use `requires` if your capability needs another
3. **Use script_file for complex installs**: Keep TOML clean
4. **Use inline script for simple config**: Avoid extra files for 3-line scripts
5. **Make runtime hooks fast**: They run before every session
6. **Handle errors gracefully**: Check prerequisites in host phases with proper error handling
7. **Document requirements**: Add comments explaining what the capability needs

## Examples

### Simple Capability (declarative packages)

```toml
[capability]
id = "git-lfs"
name = "Git LFS"
description = "Git Large File Storage support"

[packages]
system = ["git-lfs"]

[[phase.setup]]
name = "configure-git-lfs"
script = """
#!/bin/bash
set -e
# Post-install configuration
git lfs install
"""
```

### Complex Capability (with dependencies and MCP)

```toml
[capability]
id = "postgres-docker"
name = "PostgreSQL (Docker)"
description = "PostgreSQL database server running in Docker"
requires = ["docker"]  # Requires Docker to be enabled

[[phase.setup]]
name = "pull-postgres-image"
script = """
#!/bin/bash
set -e
# Pull PostgreSQL Docker image
docker pull postgres:16
"""

[[phase.runtime]]
name = "start-postgres"
script = """
#!/bin/bash
# Start PostgreSQL container if not running
if ! docker ps | grep -q postgres-dev; then
  docker run -d --name postgres-dev \\
    -e POSTGRES_PASSWORD=dev \\
    -p 5432:5432 \\
    postgres:16
fi

# Write context
mkdir -p ~/.claude-vm/context
cat > ~/.claude-vm/context/postgres.txt <<EOF
PostgreSQL container: $(docker ps --filter name=postgres-dev --format "{{.Status}}" 2>/dev/null || echo "not running")
Connection: postgresql://postgres:dev@localhost:5432
EOF
"""

[[mcp]]
id = "postgres-query"
command = "npx"
args = ["-y", "postgres-mcp@latest"]
enabled_when = "node"
```

### Capability with Custom Repository

```toml
[capability]
id = "postgres"
name = "PostgreSQL"
description = "PostgreSQL database server (native install)"

[packages]
system = ["postgresql-16", "postgresql-client-16"]
setup_script = """
#!/bin/bash
set -e
# Add PostgreSQL official repository
if [ ! -f /etc/apt/keyrings/postgresql.asc ]; then
    curl -fsSL https://www.postgresql.org/media/keys/ACCC4CF8.asc | \\
        sudo gpg --dearmor -o /etc/apt/keyrings/postgresql.asc
    echo "deb [signed-by=/etc/apt/keyrings/postgresql.asc] http://apt.postgresql.org/pub/repos/apt $(lsb_release -cs)-pgdg main" | \\
        sudo tee /etc/apt/sources.list.d/pgdg.list
fi
"""

[[phase.setup]]
name = "configure-postgresql"
script = """
#!/bin/bash
set -e
# Configure PostgreSQL
sudo systemctl enable postgresql
sudo systemctl start postgresql
"""

[[phase.runtime]]
name = "postgres-context"
script = """
#!/bin/bash
# Write PostgreSQL context
mkdir -p ~/.claude-vm/context
cat > ~/.claude-vm/context/postgres.txt <<EOF
PostgreSQL version: $(psql --version 2>/dev/null || echo "not available")
Service status: $(systemctl is-active postgresql 2>/dev/null || echo "unknown")
EOF
"""

[[mcp]]
id = "postgres-query"
command = "npx"
args = ["-y", "postgres-mcp@latest"]
enabled_when = "node"
```

### Host Phase Example

```toml
[capability]
id = "aws-credentials"
name = "AWS Credentials"
description = "Forward AWS credentials to VM"

[[phase.host.before_setup]]
name = "copy-aws-credentials"
script = """
#!/bin/bash
# Check if AWS credentials exist
if [ ! -f ~/.aws/credentials ]; then
  echo "Error: AWS credentials not found at ~/.aws/credentials"
  exit 1
fi

# Copy credentials to VM
limactl copy "$LIMA_INSTANCE" ~/.aws/credentials /tmp/aws-credentials
"""

[[phase.setup]]
name = "setup-aws-credentials"
script = """
#!/bin/bash
mkdir -p ~/.aws
mv /tmp/aws-credentials ~/.aws/credentials
chmod 600 ~/.aws/credentials
"""
```

## Testing Capabilities

Test your capability by:

1. Setting up a clean template:
   ```bash
   claude-vm clean
   claude-vm setup --my-capability
   ```

2. Checking logs for errors during setup

3. Verifying the capability works:
   ```bash
   claude-vm shell
   # Test your installed software
   ```

4. Testing runtime hooks:
   ```bash
   claude-vm "echo test"
   # Check that runtime initialization worked
   ```
