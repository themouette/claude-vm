# Changelog

All notable changes to claude-vm will be documented in this file.

## [Unreleased]

### Added

#### Agent System

- **Multi-agent support**: Claude VM now supports multiple AI coding agents beyond Claude Code. You can select which agent to use during setup with the `--agent` flag.
- **Supported agents**:
  - **Claude Code** (default): The official Claude coding agent with MCP support
  - **OpenCode**: Open-source alternative agent (requires Node.js)
- **Modular agent architecture**: New TOML-based agent definition system makes it easy to add support for additional agents. Each agent can specify:
  - Installation scripts and requirements
  - Authentication flows
  - Deployment functions for agent-specific paths (e.g., `~/.claude/CLAUDE.md` vs `~/.config/opencode/AGENTS.md`)
  - MCP configuration locations
  - Required capabilities
- **Agent selection**: Use `--agent <name>` flag during setup to choose your preferred agent. The choice is stored in the template and used consistently at runtime.
- **Agent registry**: Agents are loaded from embedded definitions in the `agents/` directory, with compile-time validation
- **Agent-specific context**: VM context and MCP configuration are automatically deployed to agent-specific locations using deployment functions
- **Agent validation**: Comprehensive validation ensures agent definitions are complete and all required scripts exist
- **Security improvements**: Added validation and shell-escaping for agent commands, proper error handling throughout the agent lifecycle

### Fixed

- **Chromium MCP server**: Removed `enabled_when = "node"` condition from chromium capability's MCP server configuration. The chrome-devtools MCP server now registers whenever chromium is enabled, allowing users who install Node.js manually in setup scripts to use the MCP functionality without enabling the node capability.
- **Resource management**: Fixed context file resource leak where temporary files on the host were not cleaned up after copying to VM
- **Error handling**: Improved error handling for agent metadata loading, deployment script sourcing, and capability runtime scripts with proper exit codes and stderr output
- **Configuration migration**: Deprecated `claude_args` config option migrated to `agent_args` with automatic migration for backward compatibility

## [0.2.2] - 2026-02-03

### Added

#### Capabilities

- **Git configuration**: New capability that configures git identity (user.name, user.email) and commit signing from host configuration
  - Automatically copies git user configuration from host to VM
  - Detects and configures GPG or SSH commit signing
  - Shows contextual warnings for signing requirements (GPG capability or SSH agent forwarding)
  - Generates runtime context for Claude about git configuration
  - Gracefully handles missing git configuration on host

### Fixed

- **GitHub CLI capability**: Fixed missing embedded script registration for gh capability and updated authentication to use device flow instead of browser-based flow for better VM compatibility
- **Tilde expansion with usernames**: Fixed limitation in path expansion to support `~username/path` syntax in addition to `~/path`. Mount specifications and config file paths can now reference other users' home directories (e.g., `~root/.ssh`)

## [0.2.1] - 2026-02-03

### Fixed

- Release documentation improvements

## [0.2.0] - 2025-01-XX

### Added

#### Context Generation System

- **Automatic VM context generation**: Claude now receives detailed information about the VM environment (disk, memory, enabled capabilities, mounted directories) at the start of each session
- **Runtime context from capabilities**: Each capability can now contribute runtime context (e.g., Docker version, Node.js version, available tools) that gets injected into Claude's context
- **Custom context files**: Support for loading additional context from `~/.claude/CLAUDE.md` (global) and project-specific `.claude-context.md` files, with proper user instruction handling
- **Context file validation**: Prompts users when context files fail to load instead of silently failing

#### File System and Mounting

- **Custom mount support**: Mount additional directories into the VM using Docker-style syntax (e.g., `/host/path:/vm/path:ro`) via CLI flags or TOML configuration
- **Setup-specific mounts**: Capabilities can define their own mount requirements (e.g., GPG socket forwarding)
- **Conversation folder mounting**: Automatically shares Claude Code conversation history with the VM at `~/.claude/projects/`, enabling conversation persistence across sessions
- **`--no-conversations` flag**: Option to disable automatic conversation folder mounting for privacy or performance reasons

#### Capabilities

- **GitHub CLI (gh)**: New capability that installs and configures the GitHub CLI tool for repository management

### Fixed

- **Conversation folder encoding**: Fixed path encoding to match Claude Code's expected format, ensuring conversations sync correctly
- **Mount validation**: Improved mount path validation and error handling to catch configuration issues early
- **Git worktree writable access**: Main repository is now mounted as writable in git worktrees, fixing permission issues
- **Context generation race conditions**: Serialized context generation to prevent concurrent file access issues
- **Test isolation**: Fixed race conditions in HOME environment tests by properly serializing test execution

### Changed

- Added comprehensive test suite for context generation feature
- Improved error messages and user feedback for context-related operations

## [0.1.4] - 2025-01-XX

### Added

- **Capabilities framework**: Introduced a modular, TOML-based system for defining VM capabilities (docker, node, python, etc.). Each capability can specify:
  - Required packages and installation commands
  - Runtime scripts for environment verification
  - Custom mount points and socket forwarding
  - Dependencies on other capabilities
- **SSH agent forwarding**: New `--ssh-agent` runtime flag to forward your SSH agent into the VM, enabling git operations with SSH keys
- **GPG agent documentation**: Comprehensive documentation for GPG agent forwarding, allowing secure git commit signing inside the VM

## [0.1.3] - 2025-01-XX

### Added

- **Self-update command**: New `claude-vm update` command automatically downloads and installs the latest release from GitHub, keeping your installation up to date without manual intervention

### Fixed

- GitHub Actions release workflow configuration issues

## [0.1.2] - 2025-01-XX

### Added

#### Core Rewrite

- **Rust implementation**: Complete rewrite from bash to Rust, providing better performance, type safety, and maintainability. The new codebase includes proper error handling, configuration management, and extensibility

#### Custom Scripts

- **Runtime scripts** (`--runtime-script`): Execute custom scripts at the start of each Claude session for environment initialization, tool verification, or dynamic configuration
- **Setup scripts** (`--setup-script`): Run custom scripts during VM creation for one-time setup tasks like installing additional tools or configuring services
- **Systemd service support**: Runtime scripts can be configured as systemd services for persistent background processes

#### Git Integration

- **Git worktree support**: Automatically detects and handles git worktrees, sharing VM templates across worktrees while mounting the main repository for full git access
- **Worktree template sharing**: VM templates are stored in the main repository's `.git` directory, accessible to all worktrees

#### Development

- **Comprehensive test suite**: Added unit and integration tests covering configuration loading, VM lifecycle, and git worktree handling
- **CI/CD pipeline**: GitHub Actions workflow for automated testing and releases

### Changed

- **Minimal install default**: VMs now start with a minimal Debian installation by default. Tools like Docker, Node.js, or Python must be explicitly requested via flags (e.g., `--docker`, `--node`)
- **Human-readable VM names**: VM templates now use project basenames instead of cryptic identifiers (e.g., `claude-vm_my-project` instead of `claude-vm_abc123`)
- **Lima naming compatibility**: Changed from double dashes to underscores in VM names to comply with Lima's naming requirements

### Fixed

- **Worktree git access**: Fixed permission issues preventing git operations in worktrees by properly mounting the main repository as writable

## [0.1.0] - Initial Release

### Added

- **Initial bash implementation**: First version of claude-vm as a bash script
- **Per-project isolation**: Each project gets its own dedicated Lima VM instance, providing complete isolation between projects
- **VM lifecycle management**: Basic commands for creating, starting, stopping, and destroying VMs
- **Lima integration**: Leverages Lima (Linux virtual machines) for macOS to provide lightweight, fast VM creation

[Unreleased]: https://github.com/themouette/claude-vm/compare/v0.2.2...HEAD
[0.2.2]: https://github.com/themouette/claude-vm/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/themouette/claude-vm/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/themouette/claude-vm/compare/v0.1.4...v0.2.0
[0.1.4]: https://github.com/themouette/claude-vm/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/themouette/claude-vm/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/themouette/claude-vm/compare/v0.1.0...v0.1.2
