# Claude VM

**Run claude in --dangerously-skip-permissions without worrying**

Claude VM gives you:

- **Safety**: Each Claude session runs in a fresh, sandboxed VM that's destroyed after use
- **Reproducibility**: Template VMs ensure consistent environments across runs
- **Flexibility**: Pre-configure tools (Docker, Node.js, Python, Chromium) once, use everywhere
- **Simplicity**: One command to set up, one command to run isolated, reproducible Linux environments on macOS and Linux

## Quick Start

```bash
# One-time setup per project: create a template VM with git configuration
claude-vm setup --git

# Or customize for your tech stack
claude-vm setup --git --node      # JavaScript/TypeScript projects
claude-vm setup --git --python    # Python projects
claude-vm setup --git --docker    # Docker-based development

# Run Claude in a clean, isolated VM
claude-vm

# Spin up a VM for a single task
claude-vm "help me code"

# Open a shell in the VM
claude-vm shell
```

Each run starts from the same clean template and automatically cleans up when done.

## Why Claude VM?

**Problem**: Running AI coding assistants directly on your host machine can be risky. They have access to your entire filesystem, credentials, and running services. Using `--dangerously-skip-permissions` on your host machine is particularly dangerous.

**Solution**: Claude VM runs each session in an isolated Linux VM that:

- Only mounts the current project directory
- Has its own filesystem, network stack, and process space
- Is automatically destroyed after each session
- Starts from a known-good template state every time

**VM isolation is the only safe way to run Claude with `--dangerously-skip-permissions`.** The VM provides a security boundary - even if Claude executes unintended commands, the blast radius is limited to the disposable VM.

Think of it as Docker for AI coding assistants - isolated, reproducible, and safe.

## Key Features

**Template VMs per Repository**

- Create a template VM once per project with all required tools
- Each session clones from this template for fast startup
- Customize with global (`~/.claude-vm.setup.sh`) or project-specific (`./.claude-vm.setup.sh`) scripts

**Runtime Scripts**

- Automatically run setup scripts before each session
- Start services, set environment variables, seed databases
- Just create `.claude-vm.runtime.sh` in your project root

**Configuration File Support**

- Define VM resources, tools, and settings in `.claude-vm.toml`
- Precedence system: CLI > Env > Project > Global > Defaults
- No need to remember complex command-line flags

**Git Worktree Support**

- Automatically detects and mounts both worktree and main repository
- Full git functionality in isolated VMs

## Installation

### Requirements

- [Lima](https://lima-vm.io/docs/installation/)

### One Liner

```bash
curl -fsSL https://raw.githubusercontent.com/themouette/claude-vm/main/install.sh | bash
```

### Download from GitHub

- Download the latest version for your platfrom from
  [GitHub](https://github.com/themouette/claude-vm/releases/latest)
- Copy executable in your `~/.local/bin` directory.

### From Source

See DEVELOPMENT.md

## Usage

### Setup a Template

Create a template VM for your project:

```bash
claude-vm setup --docker --node
```

Install all tools:

```bash
claude-vm setup --all
```

### Run Claude

Run Claude in an ephemeral VM:

```bash
claude-vm "help me code"
```

### Shell Access

Open a shell in the template VM:

```bash
claude-vm shell
```

### Project Information

Show information about the current project's template:

```bash
claude-vm info
```

This displays:
- Project path and template name
- Template status (running, stopped, not created)
- Configuration (disk, memory, capabilities)
- Configured mounts and runtime scripts

### Configuration Management

Validate configuration files:

```bash
claude-vm config validate
```

Show effective configuration after merging all sources:

```bash
claude-vm config show
```

This displays the final configuration after applying precedence rules: CLI flags > Environment variables > Project config > Global config > Defaults.

### List Templates

List all claude-vm templates:

```bash
claude-vm list
```

List templates with disk usage information:

```bash
claude-vm list --disk-usage
```

List only unused templates (not accessed in 30+ days):

```bash
claude-vm list --unused
```

### View Logs

View VM logs:

```bash
claude-vm logs              # Show last 100 lines
claude-vm logs --follow     # Follow logs in real-time
```

### Clean Templates

Clean the template for the current project:

```bash
claude-vm clean
```

Clean all templates:

```bash
claude-vm clean-all
```

Both commands will prompt for confirmation before deleting. Skip the prompt with:

```bash
claude-vm clean --yes
claude-vm clean-all --yes
```

### Update Claude VM

Check for available updates:

```bash
claude-vm update --check
```

Update to the latest version:

```bash
claude-vm update
```

Update to a specific version:

```bash
claude-vm update --version 1.2.3
```

Updates are downloaded from the [GitHub releases](https://github.com/themouette/claude-vm/releases).

## Configuration

Create a `.claude-vm.toml` file in your project root or home directory.

### Configuration File

**Minimal example:**

```toml
[vm]
disk = 30      # GB
memory = 16    # GB

[tools]
docker = true
node = true
gpg = true
```

**Complete example:**

```toml
[vm]
disk = 20      # VM disk size in GB (default: 20)
memory = 8     # VM memory size in GB (default: 8)

[tools]
docker = true     # Install Docker (default: false)
node = true       # Install Node.js (default: false)
python = false    # Install Python (default: false)
chromium = true   # Install Chromium for debugging (default: false)
gpg = true        # Enable GPG agent forwarding (default: false)

[packages]
# Custom system packages to install
system = ["postgresql-client", "redis-tools", "jq"]
# Optional: Add custom repositories
setup_script = """
#!/bin/bash
# Add third-party repositories here (must be idempotent)
"""

[setup]
# ADDITIONAL setup scripts (run during template creation)
# Standard scripts are auto-detected, no config needed:
#   - ~/.claude-vm.setup.sh (global)
#   - ./.claude-vm.setup.sh (project root)
scripts = [
    "./scripts/install-extras.sh",
]

[runtime]
# ADDITIONAL runtime scripts (run before each session)
# Standard script is auto-detected, no config needed:
#   - ./.claude-vm.runtime.sh (current git repo root)
scripts = [
    "./scripts/start-services.sh",
]

[defaults]
# Additional arguments passed to Claude (--dangerously-skip-permissions is included by default)
claude_args = ["--max-tokens", "4096"]

# Custom mounts (optional)
[[mounts]]
location = "/Users/me/data"
writable = true

[[mounts]]
location = "~/shared"
mount_point = "/vm/shared"
writable = false
```

### Configuration Locations

**Project config:** `.claude-vm.toml` in project root

```bash
my-project/
├── .claude-vm.toml          # Project-specific config
└── .claude-vm.runtime.sh    # Auto-detected runtime script
```

**Global config:** `~/.claude-vm.toml` in home directory

```bash
~/
├── .claude-vm.toml          # Global config for all projects
└── .claude-vm.setup.sh      # Auto-detected global setup script
```

### Configuration Precedence

Configuration is merged in this order (highest to lowest):

1. **Command-line flags** - `--disk 30 --memory 16`
2. **Environment variables** - `CLAUDE_VM_DISK=30 CLAUDE_VM_MEMORY=16`
3. **Project config** - `./.claude-vm.toml`
4. **Global config** - `~/.claude-vm.toml`
5. **Built-in defaults** - `disk=20, memory=8`

**Example:**

```bash
# Global config sets disk=20
# Project config sets disk=30
# CLI flag sets disk=40

claude-vm setup --disk 40  # Uses 40 (CLI wins)
claude-vm setup            # Uses 30 (project config)
```

### Environment Variables

Override config values with environment variables:

```bash
# Override VM resources
export CLAUDE_VM_DISK=30
export CLAUDE_VM_MEMORY=16

claude-vm setup  # Uses disk=30, memory=16
```

**Available variables:**

- `CLAUDE_VM_DISK` - VM disk size in GB
- `CLAUDE_VM_MEMORY` - VM memory size in GB

### Script Auto-Detection

**Setup scripts** (run during `claude-vm setup`):

1. `~/.claude-vm.setup.sh` - Global setup (always checked)
2. `./.claude-vm.setup.sh` - Project setup (always checked)
3. Config scripts - Additional custom scripts

**Runtime scripts** (run before `claude-vm` or `claude-vm shell`):

1. `./.claude-vm.runtime.sh` - Project runtime (always checked)
2. Config scripts - Additional custom scripts

**No configuration needed for standard scripts!** They're automatically detected and executed if they exist.

### Tool Installation

The `[tools]` section controls which tools are installed during setup:

```toml
[tools]
docker = true     # Docker Engine + Docker Compose
node = true       # Node.js (LTS) + npm
python = true     # Python 3 + pip
chromium = true   # Chromium + Chrome DevTools MCP
gpg = true        # GPG agent forwarding + key sync
gh = true         # GitHub CLI + authentication
git = true        # Git identity and signing configuration
```

Each enabled capability automatically provides context to Claude about its status (version, configuration, availability) via the generated `~/.claude/CLAUDE.md` file.

**Or install everything:**

```bash
claude-vm setup --all  # Installs all tools
```

### Custom Package Installation

Add custom system packages to your VM using the `[packages]` section:

```toml
[packages]
system = [
    "postgresql-client",
    "redis-tools",
    "jq",
    "htop"
]
```

**Advanced: Custom repositories**

For packages from third-party repositories, add a setup script:

```toml
[packages]
system = ["terraform", "kubectl"]
setup_script = """
#!/bin/bash
set -e
# Add HashiCorp repository
curl -fsSL https://apt.releases.hashicorp.com/gpg | sudo gpg --dearmor -o /usr/share/keyrings/hashicorp-archive-keyring.gpg
echo "deb [signed-by=/usr/share/keyrings/hashicorp-archive-keyring.gpg] https://apt.releases.hashicorp.com $(lsb_release -cs) main" | sudo tee /etc/apt/sources.list.d/hashicorp.list

# Add Kubernetes repository
curl -fsSL https://pkgs.k8s.io/core:/stable:/v1.28/deb/Release.key | sudo gpg --dearmor -o /etc/apt/keyrings/kubernetes-apt-keyring.gpg
echo "deb [signed-by=/etc/apt/keyrings/kubernetes-apt-keyring.gpg] https://pkgs.k8s.io/core:/stable:/v1.28/deb/ /" | sudo tee /etc/apt/sources.list.d/kubernetes.list
"""
```

**Package features:**
- Version pinning: `"python3=3.11.0-1"` (exact version)
- Wildcards: `"nodejs=22.*"` (any 22.x version)
- Architecture: `"libc6:amd64"` (specific architecture)
- Batch installation: All packages install in one operation for speed
- Validation: Package names are validated to prevent injection attacks

**Important:** Setup scripts must be idempotent (safe to run multiple times).

### Default Claude Arguments

Claude VM automatically passes `--dangerously-skip-permissions` to Claude by default, since the VM provides a safe isolation boundary.

You can add additional arguments or override the defaults:

```toml
[defaults]
claude_args = [
    "--dangerously-skip-permissions",  # Enabled by default
    "--max-tokens", "4096"              # Add custom args
]
```

These are added to every `claude-vm` invocation:

```bash
claude-vm "help me"
# Equivalent to: claude "help me" --dangerously-skip-permissions --max-tokens 4096
```

To disable permission bypass (not recommended), set an empty array:

```toml
[defaults]
claude_args = []
```

### Claude Context Instructions

The `[context]` section allows you to provide project-specific instructions that are automatically included in `~/.claude/CLAUDE.md` before Claude starts. This is useful for providing context about your project, coding conventions, or preferences.

**Option 1: Inline instructions**

```toml
[context]
instructions = """
This is a Rust project using:
- Cargo for build management
- Tokio for async runtime
- Serde for serialization

Please:
- Include code examples in your responses
- Follow Rust best practices
- Use proper error handling with Result types
"""
```

**Option 2: Load from file**

```toml
[context]
instructions_file = ".claude-context.md"
# Or use absolute path: instructions_file = "~/my-project-context.md"
```

Then create `.claude-context.md`:

```markdown
# Project Context

This is a Rust project for building CLI tools.

## Architecture

- Uses clap for command-line parsing
- TOML for configuration
- Modular capability system

## Coding Standards

- Include examples in responses
- Follow Rust best practices
- Use proper error handling with Result types
```

**Precedence:** If both `instructions` and `instructions_file` are set, `instructions` takes precedence. The file is only loaded if `instructions` is empty.

The context is automatically generated and includes:

- VM configuration (disk, memory)
- Enabled capabilities (docker, node, etc.)
- Mounted directories
- User-provided instructions (inline or from file)

This context is merged with any existing `~/.claude/CLAUDE.md` content using HTML comment markers, preserving your custom instructions while updating VM-specific information on each session.

### Configuration Validation

**Valid values:**

- `disk`: 1-1000 (GB)
- `memory`: 1-64 (GB)
- `tools`: true/false for each
- `scripts`: array of file paths (strings)
- `claude_args`: array of strings
- `instructions`: string (multiline supported)
- `instructions_file`: file path (string, supports ~ expansion)

**Example validation error:**

```toml
[vm]
disk = "thirty"  # ❌ Error: must be a number
```

### Complete Example

See [`examples/.claude-vm.toml`](examples/.claude-vm.toml) for a fully commented example configuration.

## Runtime Scripts

Runtime scripts are automatically executed before running Claude or opening a shell. This allows you to set up your environment, start services, or configure the session.

### Automatic Execution

Create a `.claude-vm.runtime.sh` file in your project root (or current git repository root for worktrees):

```bash
#!/bin/bash
# .claude-vm.runtime.sh

# Start background services
echo "Starting services..."
docker-compose up -d

# Set environment variables
export API_KEY="dev-key"
export DEBUG=true

# Run initialization
./scripts/init-dev-env.sh
```

This script will automatically run every time you:

- Run Claude: `claude-vm "help me code"`
- Open a shell: `claude-vm shell`

### Configuration-Based Scripts

Add additional runtime scripts in your `.claude-vm.toml`:

```toml
[runtime]
scripts = [
    "./.claude-vm.runtime.sh",    # Project script (auto-detected)
    "./scripts/start-services.sh", # Custom scripts
    "~/scripts/dev-setup.sh",      # Global scripts
]
```

### Command-Line Scripts

Pass runtime scripts via CLI flags:

```bash
claude-vm --runtime-script ./start-db.sh --runtime-script ./seed-data.sh shell
```

### Execution Order

Scripts run in this order:

1. Project runtime script (`.claude-vm.runtime.sh` if exists)
2. Config runtime scripts (from `.claude-vm.toml`)
3. CLI runtime scripts (from `--runtime-script` flags)

### Features

**Runtime Scripts**

- All runtime scripts and the main command run in a single shell invocation
- More efficient than multiple SSH connections
- Cleaner output with progress indicators

**Fail-Fast Behavior**

- If any runtime script fails (exit code ≠ 0), the main command won't run
- Ensures your environment is properly set up before Claude runs

**Shared Environment**

- Runtime scripts share the same shell environment
- Environment variables set in earlier scripts are available in later scripts and the main command
- Background processes started in runtime scripts continue running

**Interactive Support**

- Runtime scripts can prompt for user input
- Use `read` commands for configuration
- Full terminal support (colors, cursor control)

**Context Contribution**

- Runtime scripts can contribute information to Claude's context
- Write to `~/.claude-vm/context/<name>.txt` files
- Content is automatically included in `~/.claude/CLAUDE.md` before Claude starts
- Useful for providing runtime information (service status, environment details, etc.)

**Security**

- Script paths are properly escaped to prevent shell injection
- Filenames are sanitized for safe execution
- Unicode filenames are supported

### Contributing Context to Claude

Runtime scripts can optionally provide information to Claude by writing to `~/.claude-vm/context/<name>.txt` files. This content is automatically merged into `~/.claude/CLAUDE.md` before Claude starts, providing dynamic runtime information.

**Example: Service Status**

```bash
#!/bin/bash
# .claude-vm.runtime.sh

# Start services
docker-compose up -d

# Wait for services to be ready
sleep 2

# Write context for Claude
mkdir -p ~/.claude-vm/context
cat > ~/.claude-vm/context/services.txt <<EOF
Development services running:
- PostgreSQL: localhost:5432 (database: myapp_dev)
- Redis: localhost:6379
- API Server: http://localhost:3000

Database seeded with test data.
Use 'docker-compose logs' to view service logs.
EOF
```

**Example: Environment Information**

```bash
#!/bin/bash
# .claude-vm.runtime.sh

# Detect project configuration
PROJECT_TYPE="unknown"
if [ -f "package.json" ]; then
  PROJECT_TYPE="Node.js"
elif [ -f "Cargo.toml" ]; then
  PROJECT_TYPE="Rust"
elif [ -f "requirements.txt" ]; then
  PROJECT_TYPE="Python"
fi

# Write context
mkdir -p ~/.claude-vm/context
cat > ~/.claude-vm/context/environment.txt <<EOF
Project type: $PROJECT_TYPE
Git branch: $(git branch --show-current 2>/dev/null || echo "none")
Working directory: $(pwd)
Available environment: $(env | grep -E '^(API_KEY|DATABASE_URL)=' || echo "No special env vars")
EOF
```

**Context File Naming**

- Use descriptive names: `services.txt`, `environment.txt`, `database.txt`
- File basename (without .txt) becomes the section heading in CLAUDE.md
- Multiple scripts can write different context files
- Files are included in alphabetical order

**Result in CLAUDE.md**

The context files appear under "Runtime Script Results" section:

```markdown
<!-- claude-vm-context-start -->

# Claude VM Context

## VM Configuration

- **Disk**: 20 GB
- **Memory**: 8 GB

## Enabled Capabilities

- docker: Docker engine for container management
- node: Node.js runtime and npm package manager

## Runtime Script Results

### services

Development services running:

- PostgreSQL: localhost:5432 (database: myapp_dev)
- Redis: localhost:6379
- API Server: http://localhost:3000

Database seeded with test data.
Use 'docker-compose logs' to view service logs.

### environment

Project type: Node.js
Git branch: main
Working directory: /Users/user/project
Available environment: No special env vars

<!-- claude-vm-context-end -->
```

### Example: Database Setup

```bash
#!/bin/bash
# .claude-vm.runtime.sh

echo "Setting up development database..."

# Start PostgreSQL
docker-compose up -d postgres

# Wait for database to be ready
until pg_isready -h localhost -p 5432; do
  echo "Waiting for database..."
  sleep 1
done

# Run migrations
npm run db:migrate

echo "✓ Database ready"
```

### Example: Interactive Configuration

```bash
#!/bin/bash
# .claude-vm.runtime.sh

# Prompt for API key if not set
if [ -z "$API_KEY" ]; then
  read -p "Enter API key: " API_KEY
  export API_KEY
fi

# Ask if user wants to enable debug mode
read -p "Enable debug mode? (y/n): " enable_debug
if [ "$enable_debug" = "y" ]; then
  export DEBUG=true
fi
```

### Debugging

Run with `--verbose` to see detailed Lima logs:

```bash
claude-vm --verbose shell
```

This shows:

- Script copying progress with ✓/✗ indicators
- Lima VM startup logs
- Script execution output
- Detailed error messages

## Command-Line Options

### Global Options

- `--disk <GB>` - VM disk size
- `--memory <GB>` - VM memory size
- `--runtime-script <PATH>` - Runtime script to execute
- `-A, --forward-ssh-agent` - Forward SSH agent to VM
- `--no-conversations` - Don't mount Claude conversation folder in VM
- `--mount <SPEC>` - Custom mount in docker-style format (can be used multiple times)
- `-v, --verbose` - Show verbose output including Lima logs

### Setup Options

- `--docker` - Install Docker
- `--node` - Install Node.js
- `--python` - Install Python
- `--chromium` - Install Chromium
- `--gpg` - Enable GPG agent forwarding
- `--gh` - Install GitHub CLI
- `--git` - Configure git identity and signing
- `--all` - Install all tools
- `--setup-script <PATH>` - Custom setup script
- `--mount <SPEC>` - Setup-only mount (available during template creation only)

## Agent Forwarding

### GPG Agent Forwarding

Enable GPG signing in the VM by forwarding your GPG agent:

```bash
# Setup with GPG support
claude-vm setup --gpg

# Or enable in config
[tools]
gpg = true
```

**What it does:**

- Forwards your GPG agent socket to the VM
- Syncs your public keys to the VM
- Enables git commit signing inside the VM
- Works automatically on every session

**Usage in VM:**

```bash
# Sign commits (uses your host GPG key)
git commit -S -m "Signed commit"

# Sign files
gpg --sign document.txt
```

### SSH Agent Forwarding

Forward your SSH agent for git operations over SSH:

```bash
# Run with SSH agent forwarding
claude-vm -A shell

# Or with run command
claude-vm -A "git push"
```

**Use cases:**

- Push/pull from private repositories
- SSH to remote servers
- Any operation requiring SSH authentication

**Security note:** SSH agent forwarding uses native SSH agent forwarding (`ssh -A`). Your keys never leave the host machine - the VM can only use them for authentication.

### Git Configuration

Configure git identity and commit signing in the VM from your host configuration:

```bash
# Setup with git support
claude-vm setup --git

# Or enable in config
[tools]
git = true
```

**What it does:**

- Copies your git user.name and user.email from host to VM
- Automatically configures commit signing if enabled on host
- Detects GPG or SSH signing configuration
- Provides contextual warnings about signing requirements

**Commit Signing:**

If you have commit signing enabled on your host:

- **GPG signing**: Enable both `git` and `gpg` capabilities

  ```bash
  claude-vm setup --git --gpg
  ```

- **SSH signing**: Enable `git` capability and forward SSH agent
  ```bash
  claude-vm setup --git
  claude-vm -A "make a commit"  # Forward SSH agent at runtime
  ```

**Usage in VM:**

```bash
# Your git identity is automatically configured
git config user.name    # Shows your host name
git config user.email   # Shows your host email

# Signed commits work with proper agent forwarding
git commit -m "My commit"  # Automatically signed if configured
```

**Requirements:**

- Git configured on host: `git config --global user.name` and `user.email`
- For GPG signing: Enable `gpg` capability
- For SSH signing: Use `-A` flag to forward SSH agent

## Git Worktree Support

Automatically detects and handles git worktrees by:

1. Mounting the worktree directory (writable)
2. Mounting the main repository (writable, for git operations)

Git commands in worktrees require write access to the main repository's `.git` directory to update refs, create commits, and perform other operations.

## Claude Conversation History

By default, claude-vm automatically shares your Claude conversation history with the VM, allowing Claude to access context from previous conversations in the same project.

### How It Works

Claude stores conversation history in `~/.claude/projects/` with path-encoded folder names. Claude-vm automatically:

1. Detects the current project path
2. Finds or creates the corresponding conversation folder
3. Mounts it at `~/.claude/projects/` inside the VM

**Privacy note:** Only the current project's conversation folder is mounted. Conversations from other projects remain isolated and are not accessible in the VM.

This means commands running in the VM (including Claude itself) can access conversation history, artifacts, and other project-specific Claude data.

### Disabling Conversation Sharing

To run Claude in an isolated session without access to conversation history, use the `--no-conversations` flag:

```bash
# Shell without conversation history
claude-vm --no-conversations shell

# Run Claude without conversation history
claude-vm --no-conversations "help me code"
```

This is useful when:

- You want a completely isolated testing environment
- You're debugging conversation-related issues
- You need to ensure no historical context influences Claude's behavior

## Custom Mounts

Beyond the automatic mounts (project directory, worktrees, conversations), you can add custom mounts to share additional directories with the VM.

### Docker-Style Mount Syntax

Use docker-style mount specifications for the CLI:

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

# Tilde expansion supported
claude-vm --mount ~/Documents:/vm/docs shell
```

### TOML Configuration

Define persistent mounts in `.claude-vm.toml`:

```toml
[[mounts]]
location = "/Users/me/data"
writable = true

[[mounts]]
location = "/Users/me/shared"
writable = false
mount_point = "/vm/shared"  # Optional: custom path in VM

[[mounts]]
location = "~/Documents"    # Tilde expansion supported
writable = true
```

### How It Works

- **Accumulation**: Mounts from global config, project config, and CLI are all applied
- **Deduplication**: Duplicate mount locations are automatically filtered
- **Path Expansion**: `~` is expanded to your home directory
- **Validation**: Paths must be absolute (after expansion)
- **Mount Points**: By default, host paths are mounted at the same location in the VM

### Examples

**Share a dataset with the VM:**

```bash
claude-vm --mount ~/datasets:/data:ro shell
# Dataset accessible at /data in VM (read-only)
```

**Mount multiple data sources:**

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
```

**Temporary mount for a single session:**

```bash
claude-vm --mount /tmp/experiment:/experiment "analyze this data"
```

## Setup-Specific Mounts

Setup-specific mounts are directories that are available **only during template creation**, not at runtime. This allows you to transfer binaries, assets, or configuration files from your host to the template VM during setup.

### Why Use Setup Mounts?

Setup mounts are useful when you need to:

- Transfer pre-built binaries or compiled assets to the template
- Copy configuration files or credentials (that won't be in git)
- Install local packages or dependencies from your host machine
- Seed the template with data that all sessions should have

**Key difference from runtime mounts**: Setup mounts are "baked into" the template. Files copied from setup mounts become part of the template itself and are available in all cloned VMs, even though the mount itself is not present at runtime.

### CLI Usage

Add setup mounts when running `claude-vm setup`:

```bash
# Mount a directory during setup to copy files
claude-vm setup --node --mount /path/to/binaries:/tmp/binaries

# Use in setup script to copy files into the template
# In .claude-vm.setup.sh:
# cp /tmp/binaries/* /usr/local/bin/
```

### TOML Configuration

Define setup mounts in your `.claude-vm.toml`:

```toml
[[setup.mounts]]
location = "/Users/me/local-packages"
mount_point = "/tmp/packages"
writable = false

[[setup.mounts]]
location = "~/project-assets"
mount_point = "/tmp/assets"
writable = false
```

These mounts will be automatically applied every time you run `claude-vm setup`.

### Example: Installing Local Binary

Suppose you have a pre-compiled binary you want available in all VM sessions:

**1. Setup mount configuration:**

```toml
# .claude-vm.toml
[[setup.mounts]]
location = "~/my-tools/bin"
mount_point = "/tmp/host-bin"
writable = false
```

**2. Setup script to copy binary:**

```bash
#!/bin/bash
# .claude-vm.setup.sh

# Copy binary from setup mount to template
sudo cp /tmp/host-bin/my-tool /usr/local/bin/
sudo chmod +x /usr/local/bin/my-tool
```

**3. Run setup:**

```bash
claude-vm setup --node
```

Now `my-tool` is permanently installed in the template and available in every cloned VM session, even though the `/tmp/host-bin` mount doesn't exist at runtime.

### Example: Seeding Data

Transfer a dataset to the template during setup:

```bash
# Mount dataset directory during setup
claude-vm setup --mount ~/datasets:/tmp/data:ro

# In .claude-vm.setup.sh:
# mkdir -p /home/lima.linux/datasets
# cp -r /tmp/data/* /home/lima.linux/datasets/
```

The dataset is now part of the template and available in all sessions without needing to remount it.

### Important Notes

- **Setup mounts are temporary**: They're only available during `claude-vm setup`, not during `claude-vm` or `claude-vm shell`
- **Files persist**: Any files you copy from setup mounts into the template filesystem become permanent parts of the template
- **Use setup scripts**: Combine setup mounts with setup scripts (`.claude-vm.setup.sh`) to copy files from the mount into the template
- **Security**: Setup mounts use the same validation as runtime mounts (conflict detection, path validation, etc.)

## Development

For development setup, architecture details, testing instructions, and contributing guidelines, see [DEVELOPMENT.md](DEVELOPMENT.md).

## License

MIT OR Apache-2.0

## Inspiration

This tool is based on an [idea](https://github.com/sylvinus/agent-vm/) from
[@sylvinus](https://github.com/sylvinus)

Thanks to [@babbins](https://github.com/babbins) for the catch-phrase
