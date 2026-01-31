# Development Scripts

This directory contains scripts for development and maintenance tasks.

## Scripts

### `setup`

**Purpose**: Set up the development environment with all required dependencies.

**Platforms**: macOS and Linux (Debian/Ubuntu)

**Usage**:
```bash
./bin/setup
```

**What it does**:
1. Detects your operating system
2. Installs Homebrew (macOS only, if not present)
3. Installs development dependencies (git, jq, build tools)
4. Installs Rust toolchain (rustc, cargo, rustup)
5. Installs Lima VM
6. Installs Rust development tools (clippy, rustfmt, cargo-watch)
7. Verifies all installations
8. Builds the project
9. Runs the test suite

**Requirements**:
- macOS 10.15+ or Linux (Debian/Ubuntu)
- Internet connection
- sudo access (for package installation)

**Safe to run multiple times**: The script is idempotent and will skip already-installed tools.

**Example output**:
```
==> Setting up claude-vm development environment

==> Detected macOS
✓ Homebrew already installed
✓ Git already installed
==> Installing Rust...
✓ Rust installed
==> Installing Lima...
✓ Lima installed via Homebrew
==> Installing Rust development tools...
✓ Clippy installed
✓ Rustfmt installed
✓ Cargo-watch installed
==> Verifying installation...
✓ Rust: rustc 1.93.0
✓ Lima: limactl version 0.23.2
✓ Git: git version 2.39.0
✓ All required tools installed!
==> Building claude-vm...
✓ Project built successfully
==> Running tests...
✓ All tests passed

✓ Development environment setup complete!

Next steps:
  1. Build release binary:  cargo build --release
  2. Run tests:             cargo test
  3. Run linter:            cargo clippy
  4. Format code:           cargo fmt
  5. Install locally:       cargo install --path .
```

## Adding New Scripts

When adding new development scripts to this directory:

1. Make them executable: `chmod +x bin/script-name`
2. Add a shebang: `#!/usr/bin/env bash`
3. Document them in this README
4. Use the same logging functions for consistency:
   - `log_info` - Information messages
   - `log_success` - Success messages
   - `log_warning` - Warning messages
   - `log_error` - Error messages

### Example Template

```bash
#!/usr/bin/env bash
set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
  echo -e "${BLUE}==>${NC} $1"
}

log_success() {
  echo -e "${GREEN}✓${NC} $1"
}

log_warning() {
  echo -e "${YELLOW}⚠${NC} $1"
}

log_error() {
  echo -e "${RED}✗${NC} $1"
}

# Script logic here
main() {
  log_info "Starting script..."
  # ...
  log_success "Done!"
}

main "$@"
```

## Script Conventions

- Use `set -e` to exit on errors
- Use `set -u` for undefined variable checking (where appropriate)
- Provide clear, colored output using log functions
- Make scripts idempotent (safe to run multiple times)
- Test on both macOS and Linux if applicable
- Document any platform-specific behavior
