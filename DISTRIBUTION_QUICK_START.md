# Distribution Quick Start

Quick reference for distributing claude-vm binaries.

## For End Users (How to Install)

### 🚀 Recommended: Installation Script

**Fastest and easiest:**
```bash
curl -fsSL https://raw.githubusercontent.com/themouette/claude-vm/main/install.sh | bash
```

**What it does:**
- Detects your OS and architecture
- Downloads correct binary from GitHub Releases
- Installs to `/usr/local/bin`
- Verifies installation

### 🍺 macOS Users: Homebrew

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
- Easy to update

### 📦 Manual: Download Binary

1. Go to [Releases](https://github.com/themouette/claude-vm/releases)
2. Download for your platform:
   - `claude-vm-macos-aarch64.tar.gz` (Apple Silicon)
   - `claude-vm-macos-x86_64.tar.gz` (Intel Mac)
   - `claude-vm-linux-x86_64.tar.gz` (Linux)
   - `claude-vm-linux-aarch64.tar.gz` (Linux ARM)
3. Extract and install:
   ```bash
   tar xzf claude-vm-*.tar.gz
   sudo mv claude-vm /usr/local/bin/
   ```

---

## For Maintainers (How to Release)

### 1. Quick Release Process

```bash
# 1. Update version
vim Cargo.toml              # version = "0.2.0"
vim CHANGELOG.md            # Add release notes

# 2. Test everything
cargo test --all
./tests/run-compat-tests.sh

# 3. Commit and tag
git add .
git commit -m "chore: release v0.2.0"
git tag v0.2.0
git push origin main v0.2.0

# GitHub Actions automatically:
# - Builds binaries for all platforms
# - Creates GitHub release
# - Uploads assets
```

### 2. Update Distribution Channels

**Homebrew (after GitHub release completes):**
```bash
# Get checksums
curl -L https://github.com/.../claude-vm-macos-aarch64.tar.gz -o /tmp/file.tar.gz
shasum -a 256 /tmp/file.tar.gz

# Update tap repo
cd homebrew-tap
vim Formula/claude-vm.rb  # Update version + SHA256
git commit -am "Update to v0.2.0"
git push
```

**Cargo:**
```bash
cargo publish
```

---

## Comparison Table

| Method | Speed | User Type | Requires | Updates |
|--------|-------|-----------|----------|---------|
| **Installation Script** | ⚡ Fast | All users | curl, tar | Manual re-run |
| **Homebrew** | ⚡ Fast | Mac users | brew | `brew upgrade` |
| **Cargo** | 🐌 Slow | Rust devs | Rust | `cargo install -f` |
| **Manual Download** | ⚡ Fast | All users | None | Manual download |
| **From Source** | 🐌 Slow | Developers | Rust, git | Manual rebuild |

---

## Platform Support

| Platform | Architecture | Supported | Binary Name |
|----------|--------------|-----------|-------------|
| macOS | Apple Silicon (M1/M2) | ✅ | `claude-vm-macos-aarch64` |
| macOS | Intel | ✅ | `claude-vm-macos-x86_64` |
| Linux | x86_64 | ✅ | `claude-vm-linux-x86_64` |
| Linux | ARM64 | ✅ | `claude-vm-linux-aarch64` |
| Windows | Any | ❌ | N/A (Lima not supported) |

---

## Common Issues

### "Permission denied"
```bash
# Solution: Add execute permission
chmod +x /usr/local/bin/claude-vm
```

### "command not found"
```bash
# Solution: Check PATH includes install directory
echo $PATH | grep -q "/usr/local/bin" || export PATH="/usr/local/bin:$PATH"

# Make permanent (add to ~/.bashrc or ~/.zshrc):
echo 'export PATH="/usr/local/bin:$PATH"' >> ~/.bashrc
```

### "Cannot verify developer"  (macOS)
```bash
# Solution: Allow unsigned binary
xattr -d com.apple.quarantine /usr/local/bin/claude-vm
```

### Wrong architecture
```bash
# Check your architecture
uname -m

# Download correct binary:
# - aarch64/arm64 → aarch64 version
# - x86_64/amd64 → x86_64 version
```

---

## Release Checklist

Before each release:

- [ ] Version updated in `Cargo.toml`
- [ ] `CHANGELOG.md` updated with changes
- [ ] All tests pass (`cargo test --all`)
- [ ] Compatibility tests pass (`./tests/run-compat-tests.sh`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Code formatted (`cargo fmt`)
- [ ] Tag created and pushed
- [ ] GitHub release created (automatic)
- [ ] Homebrew formula updated (manual)
- [ ] Crates.io published (manual)
- [ ] Announcement posted

---

## Quick Commands

```bash
# Create release
git tag v0.2.0 && git push origin v0.2.0

# Test installation script locally
./install.sh v0.2.0 /tmp/test-install

# Build all platforms locally (requires docker)
docker run --rm -v "$PWD":/app rust:latest cargo build --release

# Verify binary
./target/release/claude-vm --version

# Check binary size
ls -lh target/release/claude-vm

# Test installation
rm -f /tmp/claude-vm
cp target/release/claude-vm /tmp/claude-vm
/tmp/claude-vm --help
```

---

## Summary

**Best Distribution Strategy:**

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

**Workflow:**
```
Code → Tag → GitHub Actions → Release → Update Homebrew/Cargo → Announce
```

**User Choice:**
- 🏃 Want it fast? → Installation script
- 🍺 On macOS? → Homebrew
- 🦀 Rust user? → Cargo
- 🔧 Want control? → Manual download

---

See [DISTRIBUTION.md](DISTRIBUTION.md) for complete details.
