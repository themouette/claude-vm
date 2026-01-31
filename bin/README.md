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

### `release`

**Purpose**: Create a new release with automated version bumping, tagging, and GitHub Actions integration.

**Platforms**: macOS and Linux (requires git and cargo)

**Usage**:
```bash
# Show help
./bin/release --help

# Interactive mode (prompts for version bump type)
./bin/release

# Bump patch version (e.g., 0.1.0 -> 0.1.1)
./bin/release patch

# Bump minor version (e.g., 0.1.0 -> 0.2.0)
./bin/release minor

# Bump major version (e.g., 0.1.0 -> 1.0.0)
./bin/release major

# Set specific version
./bin/release 0.2.0
```

**What it does**:
1. Validates the version format (semantic versioning: MAJOR.MINOR.PATCH)
2. Checks that the git working tree is clean
3. Verifies you're on the main branch (with confirmation prompt if not)
4. Checks that the version tag doesn't already exist
5. Runs all tests to ensure they pass
6. Runs clippy to ensure code quality
7. Updates the version in Cargo.toml
8. Updates Cargo.lock
9. Creates a git commit with the version bump
10. Creates an annotated git tag (e.g., v0.2.0)
11. Pushes the commit and tag to the remote repository
12. Triggers the GitHub Actions release workflow

**Requirements**:
- Clean git working tree (no uncommitted changes)
- All tests passing
- No clippy warnings
- Git remote configured
- Version must follow semantic versioning (e.g., 0.2.0, 1.0.0, 2.1.3)

**Version Bumping**:
- `patch`: Increments the patch version (0.1.0 -> 0.1.1) - for bug fixes
- `minor`: Increments the minor version and resets patch (0.1.0 -> 0.2.0) - for new features
- `major`: Increments the major version and resets minor/patch (0.1.0 -> 1.0.0) - for breaking changes
- Specific version: Set any valid semver version directly

**Safe release process**: The script includes multiple confirmation prompts:
1. Confirm the release version
2. Confirm pushing to remote

You can cancel at any point by answering 'n' to the prompts.

**Example output**:
```
==> Claude VM Release Script

==> Current version: 0.1.0
Enter version bump type or specific version:
  - patch: 0.1.0 -> 0.1.1
  - minor: 0.1.0 -> 0.2.0
  - major: 0.1.0 -> 1.0.0
  - Or enter a specific version (e.g., 1.2.3)

Version: minor
==> Bumping minor version
==> New version: 0.2.0

⚠ This will create release v0.2.0
Continue? (y/N): y
==> Checking git working tree...
✓ Working tree is clean
==> Running tests...
✓ All tests passed
==> Running clippy...
✓ Clippy passed
==> Updating version in Cargo.toml to 0.2.0...
✓ Version updated to 0.2.0
==> Creating release commit and tag...
✓ Created tag v0.2.0
⚠ Ready to push to remote. This will trigger the release workflow.
Push now? (y/N): y
==> Pushing to remote...
✓ Pushed to origin

✓ Release 0.2.0 created successfully!

Next steps:
  1. GitHub Actions will automatically build binaries for all platforms
  2. A new release will be created at: https://github.com/themouette/claude-vm/releases/tag/v0.2.0
  3. Binaries will be available for download once the workflow completes

Monitor the build progress at:
  https://github.com/themouette/claude-vm/actions
```

**GitHub Actions Integration**: When the tag is pushed, the `.github/workflows/release.yml` workflow automatically:
- Builds binaries for all supported platforms (macOS x86_64/ARM64, Linux x86_64/ARM64)
- Creates a GitHub release with the version tag
- Uploads all platform binaries as release assets
- Generates installation instructions in the release notes

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
