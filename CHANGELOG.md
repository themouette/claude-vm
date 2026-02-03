# Changelog

All notable changes to claude-vm will be documented in this file.

## [Unreleased]

### Added

#### Capabilities

- **Git configuration**: New capability that configures git identity (user.name, user.email) and commit signing from host configuration
  - Automatically copies git user configuration from host to VM
  - Detects and configures GPG or SSH commit signing
  - Shows contextual warnings for signing requirements (GPG capability or SSH agent forwarding)
  - Generates runtime context for Claude about git configuration
  - Gracefully handles missing git configuration on host

### Fixed

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

[Unreleased]: https://github.com/themouette/claude-vm/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/themouette/claude-vm/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/themouette/claude-vm/compare/v0.1.4...v0.2.0
[0.1.4]: https://github.com/themouette/claude-vm/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/themouette/claude-vm/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/themouette/claude-vm/compare/v0.1.0...v0.1.2
