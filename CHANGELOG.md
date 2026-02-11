# Changelog

All notable changes to claude-vm will be documented in this file.

## [Unreleased]

### Added

- **Rust capability**: Added support for Rust toolchain installation via Rustup. Includes stable toolchain with rustc, cargo, rustfmt, and clippy components. Available through `--rust` flag or `[tools] rust = true` in configuration.

### Changed

- **Node.js capability now uses Volta**: Migrated from NodeSource repository installation to Volta for Node.js version management. Volta provides better toolchain management and allows users to easily switch Node.js versions per-project. The setup script now installs Volta and the latest stable Node.js version by default. Runtime context includes Volta version and all installed Node versions.

### Fixed

- **Template name length limit**: Enforced 50-character maximum for template names to prevent UNIX_PATH_MAX errors. Lima creates socket paths like `~/.lima/{vm-name}/ssh.sock.{random}` which must be under 104 characters. Long project names (e.g., "claude-orchestrator-themouette-add-user-authentication") could create template and session names exceeding this limit, causing errors like "instance name too long: socket path must be less than 104 characters". Template names are now truncated when necessary while preserving the MD5 hash for uniqueness.
- **Setup failure cleanup**: Template VM is now properly stopped and deleted when setup fails, preventing partially configured templates from being left behind
- **Git worktree template naming**: Fixed bug where worktrees created separate templates instead of sharing the main repository's template. All worktrees now correctly use the same template based on the main repository root. Configuration loading now checks both worktree and main repository for `.claude-vm.toml` files, with worktree config taking precedence.
- **Version check cache after update**: Fixed bug where the version check cache was not being cleared after a successful update. The cached "latest_version" could remain set to what is now the current version, leading to stale update notifications. The cache file is now properly cleared after updates to ensure fresh version checks.

## [0.5.1] - 2026-02-09

### Fixed

- **works on linux and macos**: Lima commands now work on both Linux and macOS

## [0.5.0] - 2026-02-08

### Added

- **Network Isolation Capability**: HTTP/HTTPS filtering and protocol blocking for policy enforcement
  - Transparent proxy with mitmproxy for HTTP/HTTPS traffic filtering
  - Two policy modes: Allowlist (block all except allowed) or Denylist (allow all except blocked)
  - Domain-based filtering with wildcard support (`*.example.com`)
  - Bypass domains for certificate pinning (no TLS interception)
  - Protocol blocking: raw TCP/UDP, private networks, cloud metadata services
  - Full IPv6 support with iptables and ip6tables rules
  - CLI commands: `claude-vm network status` and `claude-vm network logs`
  - Multiple ephemeral VM support: automatically detects and selects from running VMs
  - Request statistics tracking with JSON output
  - Enable with `[security.network] enabled = true` or `--network-isolation` flag
  - See `docs/features/network-isolation.md` for full documentation
  - **Important**: Provides policy enforcement, not security isolation
- **No agent install flag**: New `--no-agent-install` flag for the `setup` command skips Claude Code agent installation during template creation
  - **Only available in debug builds** (compiled with `cargo build`) via conditional compilation
  - Completely removed from release builds - not shown in help and rejected with error if used
  - Intended for development builds to speed up testing and iteration
  - Skips agent installation, authentication, and MCP server configuration
  - Documented in development guide for developer use only

## [0.4.0] - 2026-02-06

### Added

- **info command**: New `claude-vm info` command displays project information including template status, configuration, enabled capabilities, mounts, and runtime scripts
- **config commands**: New `claude-vm config` subcommands for configuration management
  - `claude-vm config validate` - Validates configuration files without creating a VM
  - `claude-vm config show` - Shows effective configuration after merging all sources (CLI > Env > Project > Global > Defaults)
- **list command improvements**: Enhanced template listing with filtering and disk usage
  - `--unused` flag filters templates not accessed in 30+ days
  - `--disk-usage` flag shows disk space usage and last access time for each template
- **Confirmation prompts**: Added safety prompts to destructive operations
  - `claude-vm clean` and `claude-vm clean-all` now prompt before deleting templates
  - Use `--yes` or `-y` flag to skip prompts in scripts or automation
- **shell command enhancement**: The `shell` command now accepts optional command arguments
  - Without arguments: Opens interactive shell (`claude-vm shell`)
  - With arguments: Executes command and exits (`claude-vm shell ls -la`)
  - Unified interface replaces the separate `exec` command
  - Creates ephemeral VM with full mounts and runtime scripts
  - Properly escapes arguments and propagates exit codes
- **Environment variable support**: Pass environment variables to VM commands
  - `--env KEY=VALUE` sets individual variables
  - `--env-file path` loads variables from a file
  - `--inherit-env VAR` inherits specific variables from host
  - Works with all VM commands (exec, shell, run)
- **Auto-setup on missing template**: `run` and `shell` commands now offer to create templates automatically
  - Prompts user to create template if missing (no more cryptic error messages)
  - `--auto-setup` flag automatically creates templates without prompting
  - `auto_setup = true` in config file enables auto-creation by default
  - Improves first-run experience and reduces friction for new projects
- **Development build indicators**: Development builds now clearly distinguish themselves from release builds
  - Template names include `-dev` suffix in debug builds (e.g., `claude-tpl_project_hash-dev`)
  - Version string includes git commit hash in debug builds (e.g., `0.3.0-dev+a1b2c3d4`)
  - Dirty working tree indicated with `.dirty` suffix (e.g., `0.3.0-dev+a1b2c3d4.dirty`)
  - Enables safe parallel usage of development and release builds without template conflicts
  - Automatic detection based on build profile (`cargo build` vs `cargo build --release`)

### Fixed

- **shell command shell escaping**: Command arguments are now properly escaped using single quotes to prevent word splitting and command injection. Commands with spaces, quotes, or special characters now work correctly (e.g., `claude-vm shell echo "hello world"`)
- **shell command exit code propagation**: When executing commands, the shell command now exits with the same exit code as the executed command, enabling proper error detection in scripts and automation (e.g., `claude-vm shell false && echo ok` will not print "ok")
- **shell command mounts**: The shell command now always creates an ephemeral VM with proper mounts and runtime scripts, whether in interactive or command mode
- **Environment variable consistency**: Environment variables (`--env`, `--env-file`, `--inherit-env`) now work consistently across all commands (shell, run)

### Changed

- **Template module refactoring**: Consolidated template metadata functions (`get_disk_usage`, `get_last_access_time`, `format_last_used`, `is_unused`) into `template.rs` for better code organization and reusability across commands
- **Environment variable handling**: Simplified environment variable collection using a centralized `collect_env_vars()` helper function for consistency across all commands

## [0.3.0] - 2026-02-05

### Added

- **Code safety guarantee**: Added `#![forbid(unsafe_code)]` attribute to enforce zero unsafe code in the codebase at compile time
- **Automated dependency updates**: Added Dependabot configuration for automatic weekly dependency updates
  - Cargo dependencies monitored for security updates
  - GitHub Actions automatically updated
  - PRs created with `dependencies` and `security` labels
- **Declarative system package management**: Capabilities can now declare system packages directly in TOML files via `[packages]` section, eliminating the need for manual `apt-get install` commands in setup scripts
  - Package specifications with `system` array for Debian package names
  - Optional `setup_script` for adding custom repositories (Docker, Node.js, GitHub CLI)
  - Repository setup scripts run before package installation with idempotent checks
  - Support for version pinning (e.g., `python3=3.11.0-1`), wildcards (e.g., `nodejs=22.*`), and architecture specifications (e.g., `libc6:amd64`)
  - Package name validation prevents shell injection and provides clear error messages
  - Automatic deduplication preserves dependency order across capabilities
- **User-defined package management**: Users can now add custom packages and repository setup scripts via config files
  - `[packages]` section in `.claude-vm.toml` for user-defined system packages
  - `packages.setup_script` for adding custom repositories (PPAs, third-party repos)
  - User setup scripts run after capability setups to allow extending or overriding
  - Security warnings in documentation for setup_script usage
- **Batch package installation optimization**: All system packages now install in a single `apt-get update` + `apt-get install` operation
  - Reduces from 2 to 1 `apt-get update` call during VM setup
  - Base packages (git, curl, wget, etc.) install without update using default Debian repos
  - Repository setup scripts run before the single update to add custom sources
  - Capability and user packages batch install together for maximum efficiency
  - Enhanced error messages with troubleshooting steps for installation failures
- **Repository GPG verification**: Repository setup scripts follow official vendor documentation
  - Removed manual GPG fingerprint verification not present in official docs
  - APT automatically verifies package signatures via `signed-by` parameter
  - Downloads GPG keys over HTTPS for authenticity via TLS certificate validation

### Security

- **GitHub Actions pinning**: All GitHub Actions now pinned to specific commit SHAs instead of version tags
  - Prevents supply chain attacks from compromised action updates
  - Each action includes version comment for reference (e.g., `# v6.0.2`)
  - Provides immutable action versions that cannot be modified post-audit
  - Updated actions/checkout to v6.0.2 for latest improvements

### Changed

- **Capability migrations**: All capabilities migrated from imperative shell scripts to declarative package specifications
  - Docker: Declarative packages with idempotent repository setup
  - Node.js: Uses official NodeSource setup script for repository configuration
  - Python: Simple package list (python3, python3-pip, python3-venv)
  - Chromium: Packages plus post-install symlink configuration
  - GPG: Added gnupg package declaration, removed apt-get from vm_setup
  - GitHub CLI: Declarative package with repository setup

### Fixed

- **Mount configuration not applied**: Fixed bug where mount configurations from TOML files were not being merged and applied to VMs. Both `mounts` and `setup.mounts` fields were being ignored when loading from global or project config files. Only CLI `--mount` arguments were working. The `Config::merge` function now properly extends mount arrays when merging configurations.
- **packages.setup_script not merged**: Fixed bug where `packages.setup_script` configuration in TOML files was not being merged. Only the first loaded config's setup_script was used, ignoring overrides from project configs.
- **Comprehensive merge test coverage**: Added tests for all config merge behaviors to prevent future regressions.
- **Test race condition with HOME environment variable**: Fixed intermittent test failures in `test_from_spec_tilde_expansion` and `test_from_spec_tilde_expansion_both_paths`. These tests read the HOME environment variable but weren't marked with `#[serial_test::serial]`, causing failures when other tests modified HOME concurrently. All tests that access HOME (even just reading) are now properly serialized to prevent race conditions.
- **Interactive test in release builds**: Fixed `test_context_file_not_found` that would hang during release builds by prompting for user input. The test now uses `#[cfg(test)]` to skip the interactive prompt and fail immediately when a context file is not found, making tests fully non-interactive.

## [0.2.3] - 2026-02-04

### Added

- Check for available updates regularly

### Fixed

- **Git capability config merge**: Fixed bug where git capability was not being merged when loading from `.claude-vm.toml` files. The config merge function was missing the line to merge the git field, causing git configuration to be ignored when specified in project or global config files.
- **Git capability host setup**: Fixed syntax error and permission issues in git capability host setup script that prevented git from being installed in the VM. Added proper sudo usage and fixed bash syntax for git installation check.
- **Worktree support**: Fixed project detection to properly work with git worktrees. Previously, the tool would always use the main repository's root even when run from a worktree, causing it to miss worktree-specific `.claude-vm.toml` files. Now uses `git rev-parse --show-toplevel` to correctly detect the worktree root.
- **Git capability CLI flag**: Added missing `--git` flag to `claude-vm setup` command. The git capability was added in v0.2.2 but the CLI flag was not included, preventing users from enabling git configuration via command line and causing it to be excluded from `--all`. Users can now use `claude-vm setup --git` or `claude-vm setup --all` to enable git configuration.
- **Chromium MCP server**: Removed `enabled_when = "node"` condition from chromium capability's MCP server configuration. The chrome-devtools MCP server now registers whenever chromium is enabled, allowing users who install Node.js manually in setup scripts to use the MCP functionality without enabling the node capability.

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

[Unreleased]: https://github.com/themouette/claude-vm/compare/v0.5.1...HEAD
[0.5.1]: https://github.com/themouette/claude-vm/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/themouette/claude-vm/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/themouette/claude-vm/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/themouette/claude-vm/compare/v0.2.3...v0.3.0
[0.2.3]: https://github.com/themouette/claude-vm/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/themouette/claude-vm/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/themouette/claude-vm/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/themouette/claude-vm/compare/v0.1.4...v0.2.0
[0.1.4]: https://github.com/themouette/claude-vm/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/themouette/claude-vm/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/themouette/claude-vm/compare/v0.1.0...v0.1.2
