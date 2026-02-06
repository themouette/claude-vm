# Claude VM Development Guide

This guide covers development setup, architecture, and testing for the claude-vm Rust implementation.

## Development Setup

Run the setup script to install all dependencies:

```bash
./bin/setup
```

This will install:
- Rust toolchain (rustc, cargo)
- Lima VM (macOS and Linux)
- Development tools (clippy, rustfmt)
- Build the project and run tests

## Building

```bash
cargo build --release
```

The binary will be at `target/release/claude-vm`.

## Development vs Release Builds

Claude-vm automatically distinguishes between development and release builds to enable safe parallel usage:

### Development Builds (`cargo build`)

Development builds include additional version information and use separate VM templates:

**Version String:**
- Includes git commit hash: `0.3.0-dev+a1b2c3d4`
- Shows `.dirty` suffix if working tree has uncommitted changes: `0.3.0-dev+a1b2c3d4.dirty`
- Falls back to `0.3.0-dev+unknown` if git is unavailable

**Template Names:**
- Include `-dev` suffix: `claude-tpl_project_hash-dev`
- Isolated from release templates

**Example:**
```bash
cargo build
./target/debug/claude-vm --version
# Output: claude-vm 0.3.0-dev+a1b2c3d4.dirty

./target/debug/claude-vm setup --all
# Creates: claude-tpl_my-project_12345678-dev
```

### Release Builds (`cargo build --release`)

Release builds use clean version strings and standard template names:

**Version String:**
- Clean semver version: `0.3.0`
- No git metadata

**Template Names:**
- Standard format: `claude-tpl_project_hash`
- Isolated from dev templates

**Example:**
```bash
cargo build --release
./target/release/claude-vm --version
# Output: claude-vm 0.3.0

./target/release/claude-vm setup --all
# Creates: claude-tpl_my-project_12345678
```

### Benefits

This separation allows you to:
- Test development builds without affecting release templates
- Quickly identify which version is running
- Track which commit a binary was built from
- Run both dev and release builds in parallel without conflicts

### Cleaning Up Templates

After switching between dev and release builds, you may accumulate multiple templates:

```bash
# List all templates
claude-vm list

# Clean current project's template (dev or release depending on which binary you run)
claude-vm clean

# Clean all templates (both dev and release)
claude-vm clean-all
```

## Architecture

```
claude-vm-rust/
├── src/
│   ├── main.rs           # Entry point
│   ├── lib.rs            # Library root
│   ├── cli.rs            # CLI parsing
│   ├── config.rs         # Configuration system
│   ├── error.rs          # Error types
│   ├── project.rs        # Project detection
│   ├── vm/               # VM management
│   │   ├── limactl.rs    # Lima wrapper
│   │   ├── mount.rs      # Mount computation
│   │   ├── session.rs    # RAII sessions
│   │   └── template.rs   # Template operations
│   ├── commands/         # Command implementations
│   └── scripts/          # Embedded install scripts
└── tests/                # Integration tests
```

### Core Modules

#### Project Detection (`src/project.rs`)

Detects the project root and generates a unique template name. Handles git worktrees by using `git rev-parse --git-common-dir`.

#### Configuration System (`src/config.rs`)

Implements configuration precedence: CLI > Env > Project > Global > Defaults. Supports TOML configuration files with validation.

#### VM Session Management (`src/vm/session.rs`)

RAII-based VM lifecycle management using Rust's Drop trait. Ensures cleanup even on panic or error paths.

#### Lima Wrapper (`src/vm/limactl.rs`)

Subprocess wrapper for all Lima operations. Provides type-safe interface for creating, cloning, starting, stopping, and deleting VMs.

#### Script Runner (`src/scripts/runner.rs`)

Implements Docker-like entrypoint pattern for runtime scripts. Includes shell injection protection and progress indicators.

## Continuous Integration

The project uses GitHub Actions for continuous integration. On every push and pull request:
- Tests run on Ubuntu and macOS
- Code is checked with clippy
- Formatting is verified with rustfmt
- Release binaries are built and uploaded as artifacts

See `.github/workflows/test.yml` for the full CI configuration.

## Testing

### Rust Tests

Run all Rust tests:

```bash
cargo test
```

Run integration tests:

```bash
cargo test --test integration_tests
```

Run tests with verbose output:

```bash
cargo test -- --nocapture
```

### Test Coverage

Run with coverage reporting (requires cargo-tarpaulin):

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

## Code Quality

### Formatting

Format code with rustfmt:

```bash
cargo fmt
```

Check formatting without making changes:

```bash
cargo fmt -- --check
```

### Linting

Run clippy for lint checks:

```bash
cargo clippy -- -D warnings
```

Run clippy on all targets:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

## Dependencies

- **clap**: CLI parsing
- **toml**: Configuration file parsing
- **serde**: Serialization/deserialization
- **anyhow**: Error handling
- **thiserror**: Error type definitions
- **md5**: Project ID hashing
- **which**: Executable detection

## Requirements

- Rust 1.70+ (edition 2021)
- Lima VM installed
- macOS with Apple Silicon (for Rosetta support)

## Contributing

### Adding New Features

1. Add tests first (TDD approach)
2. Implement the feature
3. Update documentation
4. Run all tests and linting
5. Submit a pull request

### Code Review Checklist

- [ ] All tests pass
- [ ] Code formatted with rustfmt
- [ ] No clippy warnings
- [ ] Documentation updated
- [ ] Error handling implemented
- [ ] Security considerations addressed

## Debugging

### Verbose Mode

Run with `--verbose` to see detailed Lima logs:

```bash
claude-vm --verbose shell
```

### Rust Backtrace

Enable backtrace for panics:

```bash
RUST_BACKTRACE=1 claude-vm setup
```

Full backtrace with all frames:

```bash
RUST_BACKTRACE=full claude-vm setup
```

### Lima Debugging

Check Lima logs directly:

```bash
limactl list
limactl shell vm-name
tail -f ~/.lima/vm-name/ha.stdout.log
```

## Release Process

The project includes an automated release script that handles version bumping, testing, and GitHub release creation.

### Using the Release Script

```bash
# Show help and usage information
./bin/release --help

# Bump patch version (0.1.0 -> 0.1.1) - for bug fixes
./bin/release patch

# Bump minor version (0.1.0 -> 0.2.0) - for new features
./bin/release minor

# Bump major version (0.1.0 -> 1.0.0) - for breaking changes
./bin/release major

# Set specific version
./bin/release 0.2.0

# Interactive mode with prompts
./bin/release
```

### What the Release Script Does

1. Validates the version format (semantic versioning)
2. Checks that the git working tree is clean
3. Verifies you're on the main branch (with confirmation if not)
4. Checks that the version tag doesn't already exist
5. Runs all tests: `cargo test --all`
6. Runs clippy: `cargo clippy -- -D warnings`
7. Updates the version in `Cargo.toml`
8. Updates `Cargo.lock`
9. Creates a git commit with the version bump
10. Creates an annotated git tag (e.g., `v0.2.0`)
11. Pushes the commit and tag to the remote repository
12. Triggers the GitHub Actions release workflow

### Version Bumping

- **patch**: Increments the patch version (0.1.0 -> 0.1.1) - for bug fixes
- **minor**: Increments the minor version and resets patch (0.1.0 -> 0.2.0) - for new features
- **major**: Increments the major version and resets minor/patch (0.1.0 -> 1.0.0) - for breaking changes

### GitHub Actions Workflow

When the release tag is pushed, `.github/workflows/release.yml` automatically:

1. Builds binaries for all supported platforms:
   - macOS x86_64 (Intel)
   - macOS aarch64 (Apple Silicon)
   - Linux x86_64
   - Linux aarch64 (ARM64)
2. Creates a GitHub release with the version tag
3. Uploads all platform binaries as release assets
4. Generates installation instructions in the release notes

### Requirements

- Clean git working tree (no uncommitted changes)
- All tests passing
- No clippy warnings
- Git remote configured
- Version must follow semantic versioning (e.g., 0.2.0, 1.0.0, 2.1.3)

### Manual Release Process

If you need to release manually without the script:

1. Update version in `Cargo.toml`
2. Update CHANGELOG.md
3. Run all tests: `cargo test --all`
4. Run clippy: `cargo clippy -- -D warnings`
5. Build release binary: `cargo build --release`
6. Commit changes: `git commit -am "Release version X.Y.Z"`
7. Tag release: `git tag -a vX.Y.Z -m "Release version X.Y.Z"`
8. Push commit and tag: `git push origin main && git push origin vX.Y.Z`
