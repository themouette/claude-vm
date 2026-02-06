# Distribution and Release Guide

This guide describes how to distribute Claude VM binaries and manage releases.

## Table of Contents

- [For Users: Installation Methods](#for-users-installation-methods)
- [For Maintainers: Release Process](#for-maintainers-release-process)
- [Distribution Channels](#distribution-channels)
- [Platform Support](#platform-support)
- [Troubleshooting Installation](#troubleshooting-installation)

## For Users: Installation Methods

### 🚀 Recommended: Installation Script

**Fastest and easiest (installs to ~/.local/bin):**
```bash
curl -fsSL https://raw.githubusercontent.com/themouette/claude-vm/main/install.sh | bash
```

**What it does:**
- Detects your OS and architecture automatically
- Downloads correct binary from GitHub Releases (latest version)
- Installs to `~/.local/bin` by default (no sudo required)
- Verifies installation and checks PATH configuration

**Installation options:**
```bash
# Install specific version
curl -fsSL https://raw.githubusercontent.com/themouette/claude-vm/main/install.sh | bash -s -- --version v0.3.0

# Install system-wide to /usr/local/bin
curl -fsSL https://raw.githubusercontent.com/themouette/claude-vm/main/install.sh | bash -s -- --global

# Install to custom directory
curl -fsSL https://raw.githubusercontent.com/themouette/claude-vm/main/install.sh | bash -s -- --destination /opt/bin

# Show help
curl -fsSL https://raw.githubusercontent.com/themouette/claude-vm/main/install.sh | bash -s -- --help
```

### 🍺 macOS: Homebrew

```bash
brew tap themouette/tap
brew install claude-vm
```

**Benefits:**
- Easy updates: `brew upgrade claude-vm`
- Automatically installs Lima dependency
- Trusted by Mac developers

### 🦀 Rust Developers: Cargo

```bash
cargo install claude-vm
```

**Benefits:**
- Latest Rust features
- Builds from source
- Easy to update: `cargo install -f claude-vm`

### 📦 Manual: Download Binary

1. Visit [GitHub Releases](https://github.com/themouette/claude-vm/releases/latest)
2. Download for your platform:
   - `claude-vm-macos-aarch64.tar.gz` (Apple Silicon)
   - `claude-vm-macos-x86_64.tar.gz` (Intel Mac)
   - `claude-vm-linux-x86_64.tar.gz` (Linux x86_64)
   - `claude-vm-linux-aarch64.tar.gz` (Linux ARM64)
3. Extract and install:
   ```bash
   tar xzf claude-vm-*.tar.gz
   chmod +x claude-vm
   mv claude-vm ~/.local/bin/
   ```

### 🔨 From Source

```bash
git clone https://github.com/themouette/claude-vm
cd claude-vm
cargo build --release
cp target/release/claude-vm ~/.local/bin/
```

See [Development Guide](development.md) for details.

### Verify Installation

```bash
claude-vm --version
```

## For Maintainers: Release Process

### 1. Prepare Release

```bash
# Update version
vim Cargo.toml              # Bump version to 0.X.0
vim CHANGELOG.md            # Add release notes

# Run all tests
cargo test --all
cargo clippy -- -D warnings
cargo fmt -- --check

# Build release locally
cargo build --release
./target/release/claude-vm --version

# Commit changes
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chore: release v0.X.0"
git push origin main
```

### 2. Create Tag and Release

```bash
# Create annotated tag
git tag -a v0.X.0 -m "Release v0.X.0"

# Push tag
git push origin v0.X.0

# GitHub Actions automatically:
# - Builds binaries for all platforms
# - Creates GitHub release
# - Uploads artifacts as assets
```

### 3. Update Distribution Channels

**Homebrew (after GitHub release completes):**

```bash
# Download release artifacts
cd /tmp
curl -L https://github.com/themouette/claude-vm/releases/download/v0.X.0/claude-vm-macos-aarch64.tar.gz -o mac-arm.tar.gz
curl -L https://github.com/themouette/claude-vm/releases/download/v0.X.0/claude-vm-macos-x86_64.tar.gz -o mac-intel.tar.gz

# Get SHA256 checksums
shasum -a 256 mac-arm.tar.gz
shasum -a 256 mac-intel.tar.gz

# Update tap repository
cd ~/homebrew-tap
vim Formula/claude-vm.rb
# Update:
# - version "0.X.0"
# - sha256 "<arm-checksum>" (arm block)
# - sha256 "<intel-checksum>" (intel block)

git commit -am "Update to v0.X.0"
git push
```

**Cargo:**

```bash
cargo publish
```

### 4. Announce

- Update README badges if needed
- Post release announcement
- Update documentation

## Distribution Channels

### GitHub Releases (Primary)

**Pros:**
- ✅ Pre-built binaries for all platforms
- ✅ Fast installation
- ✅ No compilation required
- ✅ Automated CI/CD via GitHub Actions
- ✅ Works for non-Rust users

**Cons:**
- ⚠️ Requires GitHub Actions setup
- ⚠️ Must maintain build matrix

**Supported platforms:**
- macOS Intel (x86_64)
- macOS Apple Silicon (aarch64)
- Linux x86_64
- Linux ARM64

### Installation Script

**Pros:**
- ✅ Single command installation
- ✅ Auto-detects OS/architecture
- ✅ Familiar pattern (like rustup, nvm)
- ✅ No manual download/extract

**Cons:**
- ⚠️ Requires trust (piping to bash)
- ⚠️ Depends on GitHub releases

### Homebrew Tap

**Pros:**
- ✅ Most popular macOS package manager
- ✅ Easy updates via `brew upgrade`
- ✅ Automatic dependency management
- ✅ Trusted by Mac developers

**Cons:**
- ⚠️ Requires separate tap repository
- ⚠️ Must update SHA256 for each release
- ⚠️ macOS/Linux only

**Setup:**
1. Create `homebrew-tap` repository
2. Add `Formula/claude-vm.rb`
3. Update formula for each release

### Cargo/Crates.io

**Pros:**
- ✅ Easy for Rust developers
- ✅ Automatic updates
- ✅ Builds from source

**Cons:**
- ⚠️ Requires Rust installation
- ⚠️ Slower (compiles from source)
- ⚠️ Only for Rust users

**Setup:**
1. Update `Cargo.toml` metadata
2. `cargo login` (first time only)
3. `cargo publish` for each release

## Platform Support

| Platform | Architecture | Supported | Binary Name |
|----------|--------------|-----------|-------------|
| macOS | Apple Silicon (M1/M2/M3) | ✅ | `claude-vm-macos-aarch64` |
| macOS | Intel | ✅ | `claude-vm-macos-x86_64` |
| Linux | x86_64 | ✅ | `claude-vm-linux-x86_64` |
| Linux | ARM64 | ✅ | `claude-vm-linux-aarch64` |
| Windows | Any | ❌ | N/A (Lima not supported) |

## Troubleshooting Installation

### "Permission denied"

```bash
# Solution: Add execute permission
chmod +x ~/.local/bin/claude-vm
```

### "command not found"

```bash
# Check if directory is in PATH
echo $PATH | grep -q "$HOME/.local/bin" || echo "Not in PATH"

# Add to PATH (bash)
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Add to PATH (zsh)
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### "Cannot verify developer" (macOS)

```bash
# Solution: Remove quarantine attribute
xattr -d com.apple.quarantine ~/.local/bin/claude-vm
```

### Wrong Architecture

```bash
# Check your architecture
uname -m

# Download correct binary:
# - aarch64/arm64 → aarch64 version
# - x86_64/amd64 → x86_64 version
```

## Release Checklist

Before each release:

- [ ] Version updated in `Cargo.toml`
- [ ] `Cargo.lock` updated (`cargo update`)
- [ ] `CHANGELOG.md` updated with changes
- [ ] All tests pass (`cargo test --all`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Code formatted (`cargo fmt`)
- [ ] Local release build tested
- [ ] Commit and push changes
- [ ] Tag created and pushed
- [ ] GitHub release created (automatic via Actions)
- [ ] Homebrew formula updated (manual)
- [ ] Crates.io published (manual)
- [ ] Announcement posted

## Binary Optimization

Current binary size: ~965KB (release build)

**Applied optimizations:**
```toml
[profile.release]
strip = true       # Remove debug symbols
lto = true         # Link-time optimization
codegen-units = 1  # Better optimization
opt-level = "z"    # Optimize for size
```

**Further optimization (optional):**
```bash
# UPX compression (reduces to ~300KB but slower startup)
upx --best --lzma target/release/claude-vm
```

## Security Considerations

### Checksums

Generate and publish checksums for each release:

```bash
# In release process
cd releases
shasum -a 256 claude-vm-*.tar.gz > SHA256SUMS

# Optional: GPG sign checksums
gpg --sign SHA256SUMS
```

Users can verify:
```bash
sha256sum -c SHA256SUMS
```

### Code Signing (macOS)

For official releases, consider code signing:

```bash
# Sign binary
codesign --sign "Developer ID" target/release/claude-vm

# Verify signature
codesign --verify --verbose target/release/claude-vm
```

### Reproducible Builds

GitHub Actions provides reproducible builds:
- Same toolchain version
- Same dependencies
- Same build flags
- Auditable build logs

## Quick Reference

### Release Commands

```bash
# Create release
git tag v0.X.0 && git push origin v0.X.0

# Test installation script locally
./install.sh v0.X.0 /tmp/test-install

# Verify binary
./target/release/claude-vm --version

# Check binary size
ls -lh target/release/claude-vm

# Publish to crates.io
cargo publish
```

### Comparison Table

| Method | Speed | User Type | Requires | Updates |
|--------|-------|-----------|----------|---------|
| **Installation Script** | ⚡ Fast | All users | curl, tar | Manual re-run |
| **Homebrew** | ⚡ Fast | Mac users | brew | `brew upgrade` |
| **Cargo** | 🐌 Slow | Rust devs | Rust | `cargo install -f` |
| **Manual Download** | ⚡ Fast | All users | None | Manual download |
| **From Source** | 🐌 Slow | Developers | Rust, git | Manual rebuild |

## Future Distribution Channels

Not yet implemented, potential future enhancements:

### APT Repository (Debian/Ubuntu)

- Create `.deb` packages
- Host APT repository
- `sudo apt install claude-vm`

### RPM Repository (Fedora/RHEL)

- Create `.rpm` packages
- Host RPM repository
- `sudo dnf install claude-vm`

### AUR (Arch Linux)

- Create PKGBUILD
- Submit to AUR
- `yay -S claude-vm`

## Summary

**Recommended distribution strategy:**

1. **Primary**: GitHub Releases + Installation Script
   - Works for everyone
   - No dependencies
   - Fast and automated

2. **Secondary**: Homebrew Tap
   - macOS users prefer this
   - Easy updates
   - Professional

3. **Tertiary**: Cargo/Crates.io
   - Rust developers
   - Always up-to-date

**User choice guide:**
- 🏃 Want it fast? → Installation script
- 🍺 On macOS? → Homebrew
- 🦀 Rust user? → Cargo
- 🔧 Want control? → Manual download

## Next Steps

- **[Development Guide](development.md)** - Build from source
- **[Contributing](contributing.md)** - Contribute to the project
- **[Troubleshooting](advanced/troubleshooting.md)** - Debug installation issues
