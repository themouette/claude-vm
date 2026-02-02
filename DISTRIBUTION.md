# Distribution Guide

This document describes how to distribute and install the claude-vm Rust binary.

## Distribution Methods

### 1. GitHub Releases (Recommended)

**Best for**: All users, especially those wanting pre-built binaries

**Setup:**
1. Create releases with pre-built binaries for multiple platforms
2. Automated via GitHub Actions (`.github/workflows/release.yml`)

**Supported Platforms:**
- macOS Intel (x86_64)
- macOS Apple Silicon (aarch64)
- Linux x86_64
- Linux ARM64

**Creating a Release:**
```bash
# Tag and push
git tag v0.1.0
git push origin v0.1.0

# GitHub Actions automatically:
# 1. Builds binaries for all platforms
# 2. Creates release
# 3. Uploads artifacts
```

**User Installation:**
```bash
# macOS Apple Silicon
curl -L https://github.com/themouette/claude-vm/releases/download/v0.1.0/claude-vm-macos-aarch64.tar.gz | tar xz
sudo mv claude-vm /usr/local/bin/

# Linux x86_64
curl -L https://github.com/themouette/claude-vm/releases/download/v0.1.0/claude-vm-linux-x86_64.tar.gz | tar xz
sudo mv claude-vm /usr/local/bin/
```

**Pros:**
- ✅ No compilation required by users
- ✅ Fast installation
- ✅ Works for non-Rust users
- ✅ Automated CI/CD
- ✅ Multiple platform support

**Cons:**
- ⚠️ Requires GitHub Actions setup
- ⚠️ Must maintain build matrix

---

### 2. Installation Script

**Best for**: Quick, one-command installation (like rustup)

**File:** `install.sh`

**Usage:**
```bash
# Install latest version
curl -fsSL https://raw.githubusercontent.com/themouette/claude-vm/main/install.sh | bash

# Install specific version
curl -fsSL https://raw.githubusercontent.com/themouette/claude-vm/main/install.sh | bash -s -- v0.1.0

# Install to custom directory
curl -fsSL https://raw.githubusercontent.com/themouette/claude-vm/main/install.sh | bash -s -- v0.1.0 ~/.local/bin
```

**Pros:**
- ✅ Single command installation
- ✅ Detects OS/architecture automatically
- ✅ Familiar pattern (like rustup, nvm)
- ✅ No manual download/extract

**Cons:**
- ⚠️ Requires trust (piping to bash)
- ⚠️ Depends on GitHub releases

---

### 3. Homebrew (macOS/Linux)

**Best for**: macOS users and Linux Homebrew users

**Formula:** `homebrew/claude-vm.rb`

**Setup Homebrew Tap:**
```bash
# Create a tap repository
# Repository: homebrew-tap (or homebrew-claude-vm)
# File: Formula/claude-vm.rb (copy from homebrew/claude-vm.rb)
```

**User Installation:**
```bash
# From tap
brew tap themouette/tap
brew install claude-vm

# Or directly
brew install themouette/tap/claude-vm
```

**Updating Formula:**
```bash
# After each release, update:
# 1. version number
# 2. SHA256 checksums (get with: shasum -a 256 <file>)
```

**Pros:**
- ✅ Most popular macOS package manager
- ✅ Easy updates (`brew upgrade`)
- ✅ Automatic dependency management (lima)
- ✅ Trusted by Mac developers

**Cons:**
- ⚠️ Requires separate tap repository
- ⚠️ Must update SHA256 for each release
- ⚠️ macOS/Linux only

---

### 4. Cargo Install

**Best for**: Rust developers

**Setup:**
Publish to [crates.io](https://crates.io)

**User Installation:**
```bash
cargo install claude-vm
```

**Publishing:**
```bash
# First time
cargo login
cargo publish

# Updates
# 1. Bump version in Cargo.toml
# 2. cargo publish
```

**Update Cargo.toml:**
```toml
[package]
name = "claude-vm"
version = "0.1.0"
authors = ["Your Name <email@example.com>"]
edition = "2021"
description = "Run Claude Code inside sandboxed Lima VMs"
repository = "https://github.com/themouette/claude-vm"
homepage = "https://github.com/themouette/claude-vm"
license = "MIT OR Apache-2.0"
keywords = ["cli", "vm", "lima", "claude"]
categories = ["command-line-utilities", "development-tools"]
```

**Pros:**
- ✅ Easy for Rust users
- ✅ Automatic updates via cargo
- ✅ Builds from source (always latest Rust)

**Cons:**
- ⚠️ Requires Rust installation
- ⚠️ Slower (compiles from source)
- ⚠️ Only for Rust users

---

### 5. Direct Binary Download

**Best for**: Users who want manual control

**Usage:**
```bash
# Download from releases page
wget https://github.com/themouette/claude-vm/releases/download/v0.1.0/claude-vm-linux-x86_64.tar.gz

# Extract
tar xzf claude-vm-linux-x86_64.tar.gz

# Install
sudo mv claude-vm /usr/local/bin/
chmod +x /usr/local/bin/claude-vm
```

**Pros:**
- ✅ Full control
- ✅ No script execution
- ✅ Can verify checksums

**Cons:**
- ⚠️ Manual process
- ⚠️ Must choose correct platform
- ⚠️ No automatic updates

---

### 6. Package Managers (Future)

**APT (Debian/Ubuntu):**
- Create `.deb` package
- Host repository or use GitHub releases
- Users: `sudo apt install claude-vm`

**RPM (Fedora/RHEL):**
- Create `.rpm` package
- Host repository or use GitHub releases
- Users: `sudo dnf install claude-vm`

**AUR (Arch Linux):**
- Create PKGBUILD
- Submit to AUR
- Users: `yay -S claude-vm`

**Not implemented yet** - future enhancement

---

## Recommended Setup for Project

### Priority 1: GitHub Releases + Installation Script

**Why:**
- Pre-built binaries for all platforms
- One-command installation
- No dependencies required
- Fast and user-friendly

**Setup Steps:**
1. Configure GitHub Actions (`.github/workflows/release.yml`)
2. Add installation script (`install.sh`)
3. Create first release: `git tag v0.1.0 && git push origin v0.1.0`
4. Update README with installation instructions

### Priority 2: Homebrew Tap

**Why:**
- Very popular on macOS
- Easy updates
- Professional appearance

**Setup Steps:**
1. Create `homebrew-tap` repository
2. Add formula: `Formula/claude-vm.rb`
3. Update README with Homebrew instructions

### Priority 3: Cargo/Crates.io

**Why:**
- Rust developers expect it
- Easy to publish
- Automatic builds

**Setup Steps:**
1. Update Cargo.toml metadata
2. `cargo publish`
3. Update README

---

## Release Process

### 1. Prepare Release

```bash
# Update version
vim Cargo.toml  # Bump version
vim CHANGELOG.md  # Add release notes

# Test
cargo test --all
./tests/run-compat-tests.sh

# Build release locally
cargo build --release

# Commit changes
git add .
git commit -m "chore: bump version to 0.1.0"
git push
```

### 2. Create Tag and Release

```bash
# Create and push tag
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0

# GitHub Actions automatically:
# - Builds binaries for all platforms
# - Creates GitHub release
# - Uploads artifacts
```

### 3. Update Distribution Channels

**Homebrew:**
```bash
# Get SHA256 checksums
shasum -a 256 claude-vm-*.tar.gz

# Update formula in homebrew-tap repo
vim Formula/claude-vm.rb
# - Update version
# - Update SHA256 hashes
git commit -am "Update to v0.1.0"
git push
```

**Cargo:**
```bash
cargo publish
```

### 4. Announce

- Update README badges
- Post in discussions/announcements
- Update documentation

---

## Binary Size Optimization

Current: ~965KB (release build)

**Already applied:**
```toml
[profile.release]
strip = true      # Remove debug symbols
lto = true        # Link-time optimization
codegen-units = 1 # Better optimization
```

**Further optimization (if needed):**
```toml
[profile.release]
opt-level = "z"   # Optimize for size
panic = "abort"   # Smaller panic handling
```

**UPX compression (optional):**
```bash
upx --best --lzma target/release/claude-vm
# Can reduce to ~300KB, but slower startup
```

---

## Security Considerations

### Checksums

Generate and publish checksums:
```bash
# In release process
shasum -a 256 claude-vm-*.tar.gz > SHA256SUMS
gpg --sign SHA256SUMS  # Optional: sign checksums
```

### Code Signing (macOS)

```bash
# Sign the binary
codesign --sign "Developer ID" target/release/claude-vm

# Verify
codesign --verify --verbose target/release/claude-vm
```

### Reproducible Builds

Use GitHub Actions for reproducible builds:
- Same toolchain version
- Same dependencies
- Same build flags

---

## Installation Documentation Template

Add to README:

```markdown
## Installation

### Quick Install (Recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/themouette/claude-vm/main/install.sh | bash
```

### Homebrew (macOS/Linux)

```bash
brew tap themouette/tap
brew install claude-vm
```

### Cargo

```bash
cargo install claude-vm
```

### Pre-built Binaries

Download from [GitHub Releases](https://github.com/themouette/claude-vm/releases)

**macOS:**
```bash
# Apple Silicon (M1/M2)
curl -L https://github.com/.../claude-vm-macos-aarch64.tar.gz | tar xz
sudo mv claude-vm /usr/local/bin/

# Intel
curl -L https://github.com/.../claude-vm-macos-x86_64.tar.gz | tar xz
sudo mv claude-vm /usr/local/bin/
```

**Linux:**
```bash
# x86_64
curl -L https://github.com/.../claude-vm-linux-x86_64.tar.gz | tar xz
sudo mv claude-vm /usr/local/bin/

# ARM64
curl -L https://github.com/.../claude-vm-linux-aarch64.tar.gz | tar xz
sudo mv claude-vm /usr/local/bin/
```

### From Source

```bash
git clone https://github.com/themouette/claude-vm
cd claude-vm-rust
cargo build --release
sudo cp target/release/claude-vm /usr/local/bin/
```

### Verify Installation

```bash
claude-vm --version
```
```

---

## Maintenance

### Regular Tasks

- Update dependencies: `cargo update`
- Security audits: `cargo audit`
- Check for outdated deps: `cargo outdated`
- Update Homebrew formula SHA256s
- Test on all platforms before release

### Monitoring

- Track download statistics (GitHub)
- Monitor issues for installation problems
- Check platform compatibility reports

---

## Summary

**Recommended approach:**

1. ✅ **GitHub Releases** - Primary distribution
2. ✅ **Installation script** - Easy one-command install
3. ✅ **Homebrew** - macOS users' preferred method
4. ✅ **Cargo** - For Rust developers

This provides:
- Fast installation for end users
- Multiple options for different workflows
- Automated release process
- Professional distribution channels

**Next steps:**
1. Set up GitHub Actions for releases
2. Create first release (v0.1.0)
3. Test installation on all platforms
4. Create Homebrew tap
5. Publish to crates.io
