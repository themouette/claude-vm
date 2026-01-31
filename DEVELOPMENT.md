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

1. Update version in `Cargo.toml`
2. Update CHANGELOG.md
3. Run all tests: `cargo test --all`
4. Build release binary: `cargo build --release`
5. Tag release: `git tag v0.x.0`
6. Push tag: `git push origin v0.x.0`
