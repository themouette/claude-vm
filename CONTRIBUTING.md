# Contributing to Claude VM

Thank you for your interest in contributing to claude-vm! This guide will help you get started.

## Development Setup

### Quick Setup

Run the automated setup script:

```bash
./bin/setup
```

This script will:
1. Detect your OS (macOS or Linux Debian/Ubuntu)
2. Install Rust toolchain if needed
3. Install Lima VM if needed
4. Install development dependencies
5. Install Rust development tools (clippy, rustfmt)
6. Build the project
7. Run all tests

### Manual Setup

If you prefer to set up manually:

1. **Install Rust** (1.70+)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Install Lima** (for testing)

   macOS:
   ```bash
   brew install lima
   ```

   Linux:
   ```bash
   # See https://lima-vm.io/docs/installation/
   ```

3. **Install development tools**
   ```bash
   rustup component add clippy rustfmt
   cargo install cargo-watch  # Optional, for auto-rebuild
   ```

4. **Build the project**
   ```bash
   cargo build
   ```

5. **Run tests**
   ```bash
   cargo test
   ```

## Development Workflow

### Building

```bash
# Debug build (fast, with debug symbols)
cargo build

# Release build (optimized)
cargo build --release

# Watch mode (auto-rebuild on changes)
cargo watch -x build
```

### Testing

```bash
# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test integration_tests

# Run a specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture
```

### Linting

```bash
# Run clippy (Rust linter)
cargo clippy

# Run clippy with warnings as errors
cargo clippy -- -D warnings

# Auto-fix some issues
cargo clippy --fix
```

### Formatting

```bash
# Check formatting
cargo fmt -- --check

# Auto-format code
cargo fmt
```

### Running Locally

```bash
# Run with cargo
cargo run -- --help
cargo run -- setup --docker

# Or build and run binary directly
cargo build
./target/debug/claude-vm --help
```

## Project Structure

```
claude-vm-rust/
├── bin/
│   └── setup              # Development setup script
├── src/
│   ├── main.rs            # Application entry point
│   ├── lib.rs             # Library interface (for testing)
│   ├── cli.rs             # CLI argument parsing
│   ├── config.rs          # Configuration system
│   ├── error.rs           # Error types
│   ├── project.rs         # Project detection
│   ├── vm/                # VM management
│   │   ├── limactl.rs     # Lima subprocess wrapper
│   │   ├── mount.rs       # Mount computation
│   │   ├── session.rs     # VM sessions with RAII
│   │   └── template.rs    # Template operations
│   ├── commands/          # Command implementations
│   │   ├── setup.rs       # Setup command
│   │   ├── run.rs         # Run command
│   │   ├── shell.rs       # Shell command
│   │   ├── list.rs        # List command
│   │   ├── clean.rs       # Clean command
│   │   └── clean_all.rs   # Clean all command
│   ├── scripts/           # Script execution
│   │   ├── mod.rs         # Embedded scripts
│   │   └── runner.rs      # Script runner
│   └── utils/             # Utilities
│       ├── git.rs         # Git operations
│       └── process.rs     # Process helpers
├── scripts/               # Installation scripts (embedded)
│   ├── install_docker.sh
│   ├── install_node.sh
│   ├── install_python.sh
│   └── install_chromium.sh
├── tests/                 # Test suite
│   ├── integration_tests.rs
│   └── integration/
│       └── cli_tests.rs
└── examples/
    └── .claude-vm.toml    # Example configuration
```

## Making Changes

### Adding a New Feature

1. **Create a branch**
   ```bash
   git checkout -b feature/my-feature
   ```

2. **Write code with tests**
   - Add unit tests in the same file (in a `#[cfg(test)]` module)
   - Add integration tests in `tests/` if needed

3. **Run the test suite**
   ```bash
   cargo test
   ```

4. **Check formatting and linting**
   ```bash
   cargo fmt
   cargo clippy -- -D warnings
   ```

5. **Commit your changes**
   ```bash
   git add .
   git commit -m "feat: add my feature"
   ```

### Fixing a Bug

1. **Write a failing test** that reproduces the bug
2. **Fix the bug**
3. **Verify the test passes**
4. **Run full test suite**
5. **Commit with clear description**

### Adding a New Command

1. Create a new file in `src/commands/`
2. Implement the command logic
3. Add the command to `src/commands/mod.rs`
4. Add CLI definition in `src/cli.rs`
5. Wire it up in `src/main.rs`
6. Add tests in `tests/integration/`

Example:
```rust
// src/commands/my_command.rs
use crate::error::Result;
use crate::project::Project;

pub fn execute(project: &Project) -> Result<()> {
    println!("Executing my command for {}", project.root().display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_command() {
        // Test logic
    }
}
```

## Code Style

### General Guidelines

- Follow Rust naming conventions (snake_case for functions/variables, CamelCase for types)
- Use meaningful variable names
- Add doc comments for public APIs
- Keep functions focused and small
- Prefer `Result<T>` over panicking

### Error Handling

```rust
// Good: Use Result and ? operator
pub fn do_something() -> Result<String> {
    let value = some_operation()?;
    Ok(value.to_string())
}

// Bad: Don't panic in library code
pub fn do_something() -> String {
    some_operation().unwrap()  // Don't do this!
}
```

### Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        let result = my_function("input");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_error_case() {
        let result = my_function("bad");
        assert!(result.is_err());
    }
}
```

## Commit Messages

Follow conventional commits:

- `feat: add new feature`
- `fix: resolve bug in X`
- `docs: update README`
- `test: add tests for Y`
- `refactor: simplify Z logic`
- `chore: update dependencies`

## Pull Request Process

1. **Ensure all tests pass**
   ```bash
   cargo test --all
   ```

2. **Ensure code is formatted**
   ```bash
   cargo fmt
   ```

3. **Ensure no clippy warnings**
   ```bash
   cargo clippy -- -D warnings
   ```

4. **Update documentation** if needed

5. **Submit PR** with clear description:
   - What does this change?
   - Why is it needed?
   - How was it tested?

## Running Integration Tests with Lima

Some tests require Lima to be installed and running:

```bash
# Start Lima default VM (if needed)
limactl start

# Run integration tests
cargo test --test integration_tests

# Run a specific integration test
cargo test --test integration_tests -- test_name
```

## Debugging

### Enable debug logging

```bash
# Set RUST_LOG environment variable
RUST_LOG=debug cargo run -- setup --docker

# Or in code
env_logger::init();  // Add to main()
```

### Use cargo-expand to see macro expansions

```bash
cargo install cargo-expand
cargo expand
```

### Use rust-analyzer for IDE support

Install rust-analyzer extension in your editor (VS Code, Vim, etc.)

## Performance Profiling

```bash
# Install flamegraph
cargo install flamegraph

# Profile the application
cargo flamegraph -- setup --docker
```

## Documentation

### Generate and view docs

```bash
# Generate documentation
cargo doc

# Open in browser
cargo doc --open

# Include private items
cargo doc --document-private-items
```

## Getting Help

- Check existing issues on GitHub
- Read the documentation in the `/docs` directory
- Review the implementation details in `IMPLEMENTATION.md`
- Ask questions by opening a GitHub issue

## Code of Conduct

Be respectful, inclusive, and constructive in all interactions.

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (MIT OR Apache-2.0).
