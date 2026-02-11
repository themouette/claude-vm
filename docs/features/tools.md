# Tools

Claude VM supports installing various development tools during template creation. This guide covers all available tools, what they provide, and how to configure them.

## Table of Contents

- [Available Tools](#available-tools)
- [Installing Tools](#installing-tools)
- [Tool Details](#tool-details)
- [Tool Configuration](#tool-configuration)
- [Tool Context](#tool-context)
- [Examples](#examples)

## Available Tools

| Tool       | What it Provides               | Use Case                       |
| ---------- | ------------------------------ | ------------------------------ |
| `git`      | Git identity, signing config   | Any git repository             |
| `docker`   | Docker Engine, Docker Compose  | Containerized development      |
| `node`     | Node.js LTS, npm               | JavaScript/TypeScript projects |
| `python`   | Python 3, pip                  | Python development             |
| `rust`     | Rust toolchain, cargo, clippy  | Rust development               |
| `chromium` | Chromium browser, DevTools     | Web scraping, browser testing  |
| `gpg`      | GPG agent forwarding, key sync | Signed commits, encryption     |
| `gh`       | GitHub CLI, authentication     | GitHub operations              |

**Note:** Network isolation is configured separately via `[security.network]` - see [Network Isolation](#network-isolation) below.

## Installing Tools

### During Setup

Install tools when creating a template:

```bash
# Install specific tools
claude-vm setup --git --docker --node

# Install all tools
claude-vm setup --all
```

### Via Configuration

Define tools in `.claude-vm.toml`:

```toml
[tools]
git = true
docker = true
node = true
python = false  # Explicitly disabled
```

**Default:** All tools are `false` if not specified.

### Precedence

CLI flags override configuration:

```bash
# Config enables docker, CLI disables it
# .claude-vm.toml: docker = true

claude-vm setup --git --node  # Only git and node installed
```

## Tool Details

### Git

**Installs:**

- Git identity configuration (name, email)
- Commit signing setup (GPG or SSH)
- Git configuration from host

**Configuration:**

```toml
[tools]
git = true
```

**CLI:**

```bash
claude-vm setup --git
```

**What it does:**

1. Copies `user.name` and `user.email` from host
2. Detects and configures commit signing
3. Provides context about git configuration

**Commit Signing:**

- **GPG signing:** Requires `gpg` tool enabled
- **SSH signing:** Requires SSH agent forwarding (`-A` flag)

**Context provided:**

```markdown
Git configured:

- User: John Doe <john@example.com>
- Signing: GPG (requires --gpg capability)
```

**Usage:**

```bash
claude-vm shell
$ git config user.name   # Your name from host
$ git config user.email  # Your email from host
```

### Docker

**Installs:**

- Docker Engine
- Docker Compose
- Docker daemon configuration

**Configuration:**

```toml
[tools]
docker = true
```

**CLI:**

```bash
claude-vm setup --docker
```

**What it does:**

1. Installs Docker Engine
2. Installs Docker Compose v2
3. Configures Docker daemon
4. Starts Docker service
5. Adds user to docker group

**Context provided:**

```markdown
Docker available:

- Version: Docker 24.0.7
- Compose: Docker Compose v2.23.0
- Status: Running
```

**Usage:**

```bash
claude-vm shell
$ docker ps                    # List containers
$ docker-compose up -d         # Start services
$ docker build -t myapp .      # Build images
```

### Node.js

**Installs:**

- Node.js (LTS version)
- npm (latest)
- Node environment

**Configuration:**

```toml
[tools]
node = true
```

**CLI:**

```bash
claude-vm setup --node
```

**What it does:**

1. Installs Node.js LTS (currently 20.x)
2. Installs npm
3. Configures npm global directory

**Context provided:**

```markdown
Node.js available:

- Version: v20.10.0
- npm: 10.2.3
- Global packages: ~/.npm-global
```

**Usage:**

```bash
claude-vm shell
$ node --version              # Check Node version
$ npm install                 # Install dependencies
$ npm run build               # Run build scripts
```

### Python

**Installs:**

- Python 3
- pip
- Python development headers

**Configuration:**

```toml
[tools]
python = true
```

**CLI:**

```bash
claude-vm setup --python
```

**What it does:**

1. Installs Python 3 (latest from Ubuntu repos)
2. Installs pip
3. Installs python3-dev for native extensions

**Context provided:**

```markdown
Python available:

- Version: Python 3.10.12
- pip: 22.0.2
- Location: /usr/bin/python3
```

**Usage:**

```bash
claude-vm shell
$ python3 --version           # Check Python version
$ pip install -r requirements.txt  # Install packages
$ python3 app.py              # Run Python scripts
```

### Rust

**Installs:**

- Rustup (Rust toolchain manager)
- Rust stable toolchain (rustc, cargo)
- rustfmt (code formatter)
- clippy (linter)

**Configuration:**

```toml
[tools]
rust = true
```

**CLI:**

```bash
claude-vm setup --rust
```

**What it does:**

1. Installs Rustup via official installer (https://sh.rustup.rs)
2. Installs stable Rust toolchain as default
3. Adds rustfmt and clippy components
4. Configures PATH to include `$CARGO_HOME/bin`

**Context provided:**

```markdown
Rustup version: rustup 1.27.0 (2024-12-12)
Rust version: rustc 1.83.0 (90b35a623 2024-11-26)
Cargo version: cargo 1.83.0 (5ffbef321 2024-10-29)
Rustfmt version: rustfmt 1.8.0-stable (90b35a623 2024-11-26)
Clippy version: clippy 0.1.83 (90b35a62 2024-11-26)
Installed toolchains: stable-aarch64-unknown-linux-gnu (default)
```

**Usage:**

```bash
claude-vm shell
$ rustc --version              # Check Rust version
$ cargo new my-project         # Create new Rust project
$ cargo build                  # Build project
$ cargo test                   # Run tests
$ cargo fmt                    # Format code
$ cargo clippy                 # Run linter
```

**Toolchain Management:**

Rustup allows managing multiple Rust versions:

```bash
# Install nightly toolchain
$ rustup toolchain install nightly

# Use nightly for current directory
$ rustup override set nightly

# Update toolchains
$ rustup update
```

**Notes:**

- Rust is installed per-user in `~/.cargo` and `~/.rustup`
- Installation is idempotent - running setup multiple times is safe
- Stable toolchain is used by default, suitable for most development
- The capability ensures rustfmt and clippy are always available

### Chromium

**Installs:**

- Chromium browser
- Chrome DevTools Protocol support
- Browser automation tools

**Configuration:**

```toml
[tools]
chromium = true
```

**CLI:**

```bash
claude-vm setup --chromium
```

**What it does:**

1. Installs Chromium browser
2. Configures for headless operation
3. Sets up Chrome DevTools MCP server

**Context provided:**

```markdown
Chromium available:

- Version: Chromium 118.0.5993.0
- Headless: Supported
- DevTools: Available via MCP
```

**Usage:**

```bash
claude-vm shell
$ chromium --version          # Check version
$ chromium --headless --dump-dom https://example.com
```

Useful for:

- Web scraping
- Browser automation
- Screenshot generation
- Testing web applications

**Troubleshooting:**

If your project defines a chromium MCP configuration, it will be used instead of
the one provided by Claude VM, which can prevent Chromium from starting.

### GPG

**Installs:**

- GPG agent forwarding setup
- Public key synchronization
- Signing configuration

**Configuration:**

```toml
[tools]
gpg = true
```

**CLI:**

```bash
claude-vm setup --gpg
```

**What it does:**

1. Forwards GPG agent socket from host
2. Syncs public keys to VM
3. Configures GPG for git commit signing
4. Sets up agent socket paths

**Context provided:**

```markdown
GPG available:

- Agent: Forwarded from host
- Keys: [Your Key ID]
- Signing: Enabled for git commits
```

**Usage:**

```bash
claude-vm shell
$ gpg --list-keys             # List available keys
$ git commit -S -m "msg"      # Sign commit
$ gpg --sign file.txt         # Sign file
```

**Important:** Your private key stays on host - only the agent is forwarded.

### GitHub CLI

**Installs:**

- GitHub CLI (`gh`)
- GitHub authentication
- Git credential helper

**Configuration:**

```toml
[tools]
gh = true
```

**CLI:**

```bash
claude-vm setup --gh
```

**What it does:**

1. Installs GitHub CLI
2. Configures git credential helper
3. Syncs authentication from host

**Context provided:**

```markdown
GitHub CLI available:

- Version: gh 2.40.0
- Authenticated: Yes
- User: @yourusername
```

**Usage:**

```bash
claude-vm shell
$ gh repo list                # List repositories
$ gh pr create                # Create pull request
$ gh issue list               # List issues
$ gh api /user                # Make API calls
```

### Network Isolation

**Installs:**

- mitmproxy for HTTP/HTTPS filtering
- iptables rules for protocol blocking
- Domain-based policy enforcement

**Configuration:**

```toml
[security.network]
enabled = true
mode = "denylist"  # or "allowlist"
blocked_domains = ["example.com", "*.ads.com"]
```

**CLI:**

```bash
claude-vm setup --network-isolation
```

**What it does:**

1. Installs mitmproxy from official binaries
2. Generates and installs CA certificate
3. Configures transparent HTTP/HTTPS proxy
4. Sets up iptables rules for protocol blocking
5. Enforces domain filtering policies

**Context provided:**

```markdown
Network isolation is enabled with the following policies:

- HTTP/HTTPS traffic: Filtered through in-VM proxy (localhost:8080)
- Policy mode: denylist
- Blocked domains: example.com, *.ads.com (2 patterns)
- Raw TCP/UDP: Blocked
- Private networks: Blocked
- Cloud metadata: Blocked
```

**Usage:**

```bash
# Check status
claude-vm network status

# View logs
claude-vm network logs
claude-vm network logs -n 100
claude-vm network logs -f "blocked"

# Test a domain
claude-vm network test example.com
claude-vm network test api.github.com
```

**Policy Modes:**

- **Allowlist**: Block all domains except explicitly allowed
- **Denylist**: Allow all domains except explicitly blocked

**Important:** Network isolation provides policy enforcement, not security isolation. See [Network Isolation documentation](network-isolation.md) for details on security model and limitations.

**Use cases:**

- Compliance requirements
- Preventing accidental data leaks
- API access restrictions
- Internal security policies
- Auditing and logging

## Tool Configuration

### Basic Configuration

```toml
[tools]
git = true
docker = true
node = true
```

### Complete Configuration

```toml
[tools]
git = true        # Git identity and signing
docker = true     # Docker + Compose
node = true       # Node.js + npm
python = true     # Python 3 + pip
chromium = true   # Chromium browser
gpg = true        # GPG agent forwarding
gh = true         # GitHub CLI
```

### Install Everything

```bash
# CLI flag
claude-vm setup --all

# Equivalent config
[tools]
git = true
docker = true
node = true
python = true
chromium = true
gpg = true
gh = true
```

### Selective Installation

```bash
# Only what you need
claude-vm setup --git --docker  # Just git and docker
```

### Disable in Config

```toml
[tools]
docker = true
node = true
python = false    # Explicitly disabled
chromium = false  # Explicitly disabled
```

## Tool Context

Each enabled tool automatically provides context to Claude via `~/.claude/CLAUDE.md`.

### Example Context

```markdown
# Claude VM Context

## VM Configuration

- **Disk**: 20 GB
- **Memory**: 8 GB

## Enabled Capabilities

### docker

Docker engine for container management.

- **Version**: Docker 24.0.7
- **Compose**: Docker Compose v2.23.0
- **Status**: Running

### node

Node.js runtime and npm package manager.

- **Version**: v20.10.0
- **npm**: 10.2.3

### git

Git version control with identity configured.

- **User**: John Doe <john@example.com>
- **Signing**: GPG enabled
```

This context helps Claude understand:

- What tools are available
- How to use them
- Current versions
- Configuration details

## Examples

### Web Development

```bash
# Node.js + Docker for full-stack
claude-vm setup --git --node --docker
```

Tools available:

- `node` - Run JavaScript
- `npm` - Package management
- `docker` - Run databases, Redis, etc.
- `git` - Version control

### Python Data Science

```bash
# Python + Chromium for web scraping
claude-vm setup --git --python --chromium
```

Tools available:

- `python3` - Run Python scripts
- `pip` - Install packages
- `chromium` - Browser automation
- `git` - Version control

### DevOps

```bash
# Docker + GitHub CLI
claude-vm setup --git --docker --gh
```

Tools available:

- `docker` - Container management
- `gh` - GitHub operations
- `git` - Version control with signing

### Multi-Language Project

```bash
# Everything for complex projects
claude-vm setup --all
```

All tools available for maximum flexibility.

### Minimal Setup

```bash
# Just git for simple projects
claude-vm setup --git
```

Lightweight template, fast clone times.

## Tool Combinations

### Common Combinations

**JavaScript/TypeScript:**

```toml
[tools]
git = true
node = true
docker = true  # For databases, Redis, etc.
```

**Python:**

```toml
[tools]
git = true
python = true
docker = true  # For PostgreSQL, MongoDB, etc.
```

**Rust:**

```toml
[tools]
git = true
docker = true  # For testing with databases
gpg = true     # For signed releases
```

**Fullstack:**

```toml
[tools]
git = true
node = true
python = true  # Backend API
docker = true  # All services
gh = true      # CI/CD workflows
```

## Best Practices

### 1. Install Only What You Need

```bash
# Good: Minimal template
claude-vm setup --git --node

# Avoid: Installing everything unnecessarily
claude-vm setup --all  # Only if you actually need everything
```

Smaller templates = faster clones.

### 2. Use Git Tool Everywhere

```toml
[tools]
git = true  # Recommended for all projects
```

Even non-git projects benefit from git being configured.

### 3. Match Project Tech Stack

```bash
# Node.js project
claude-vm setup --git --node --docker

# Python project
claude-vm setup --git --python --docker

# Static site
claude-vm setup --git  # Minimal
```

### 4. Enable GPG for Signed Commits

```toml
[tools]
git = true
gpg = true  # Enable if you sign commits
```

### 5. Add GH for GitHub Projects

```toml
[tools]
git = true
node = true
gh = true  # For GitHub API access
```

## Troubleshooting

### Tool Not Found After Setup

```bash
# Verify tool is enabled
claude-vm info

# Recreate template if missing
claude-vm clean
claude-vm setup --git --node
```

### Wrong Version Installed

Tools install the latest version from Ubuntu repositories. For specific versions, use [custom packages](../advanced/custom-packages.md).

### Tool Conflicts

Some tools may conflict. If issues arise:

```bash
# Test with minimal setup
claude-vm clean
claude-vm setup --git

# Add tools incrementally
claude-vm clean
claude-vm setup --git --node
# Test, then add more
```

### Permission Issues

```bash
# Docker permission denied
claude-vm shell
$ docker ps  # Error: permission denied

# Fix: User should be in docker group (automatic during setup)
# Recreate template:
claude-vm clean
claude-vm setup --docker
```

## Next Steps

- **[Templates](templates.md)** - Understand template creation
- **[Configuration](../configuration.md)** - Configure tools in TOML
- **[Custom Packages](../advanced/custom-packages.md)** - Install additional packages
- **[Agent Forwarding](../agent-forwarding.md)** - Configure GPG, SSH, Git
