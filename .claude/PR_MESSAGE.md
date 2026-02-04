# Add Git Capability for Identity and Signing Configuration

## Overview

Adds a new `git` capability that automatically configures git identity (user.name, user.email) and commit signing settings in the VM from the host machine's configuration. This enables Claude to make properly attributed commits and supports both GPG and SSH signing workflows.

## Motivation

Users running Claude Code in the VM need git properly configured to:
- Make commits with correct authorship attribution
- Sign commits when required by repository policies
- Maintain consistent git identity across host and VM environments
- Avoid manual configuration of git in each session

## What's Included

### New Capability
- **`capabilities/git/`** - New git capability with host setup script and runtime context
- Configures git user identity from host to VM
- Detects and configures commit signing (GPG or SSH)
- Provides contextual warnings about signing requirements

### Configuration Support
- New `git` field in `ToolsConfig` struct
- CLI flag: `--git` for setup command
- TOML config: `[tools] git = true`

### Documentation
- CHANGELOG.md updated with new capability details
- README.md with comprehensive git capability documentation
- Examples for GPG and SSH signing workflows

## Key Features

### Smart Configuration Detection
- Reads **local git config first**, then falls back to global (matches git's native behavior)
- Respects project-specific git identities
- Shows which config scope is being used (local vs global)

### Security
- **No shell injection vulnerability** - uses temp files for safe value transfer
- Only copies specific config values (name, email, signing settings)
- No private key material copied to VM
- Proper escaping and error handling throughout

### Graceful Degradation
- If git not configured on host, shows highly visible warning and skips configuration
- Doesn't block setup of other capabilities
- Clear instructions for configuring git (both global and local options)

### Signing Support
- Detects GPG signing and shows warning to enable `gpg` capability
- Detects SSH signing and reminds user to forward SSH agent with `-A` flag
- Copies signing configuration (format, key) when present
- Works seamlessly with existing GPG capability

### Runtime Context
- Generates `~/.claude-vm/context/git.txt` with:
  - Git version
  - User name and email
  - Signing status and format
  - Signing key (if configured)

## Usage Examples

### Basic Setup
```bash
# Enable git capability
claude-vm setup --git

# Or via config
# .claude-vm.toml
[tools]
git = true
```

### With GPG Signing
```bash
# Setup both git and GPG capabilities
claude-vm setup --git --gpg

# Git commits in VM will be signed with your GPG key
```

### With SSH Signing
```bash
# Setup git capability
claude-vm setup --git

# Forward SSH agent at runtime for signing
claude-vm -A "fix the bug"
```

### Project-Specific Identity
```bash
# If you have local git config in your project:
cd ~/work-project
git config user.email "work@company.com"  # Local config

# claude-vm setup --git will use work email (local config takes precedence)
```

## Technical Implementation

### Architecture
Follows the established capability pattern:
- `capability.toml` - Capability definition with host_setup and vm_runtime scripts
- `host_setup.sh` - Reads host git config and copies to VM
- `vm_runtime` - Generates context file for Claude

### Host Setup Process
1. Changes to `PROJECT_ROOT` if available to pick up local git config
2. Reads git config (local first, then global)
3. Validates user.name and user.email are configured
4. Writes config values to temp files (prevents shell injection)
5. Copies temp files to VM via `limactl copy` (with error checking)
6. Executes git config commands in VM reading from temp files
7. Shows appropriate warnings for signing configuration

### Security Measures
- Uses `LIMA_INSTANCE` environment variable (matches GPG pattern)
- Temp files for data transfer (no variable expansion in heredoc)
- Quoted heredoc delimiters prevent expansion: `<<'SHELL_EOF'`
- Error handling on all `limactl` operations
- Comprehensive signal handling: `trap EXIT ERR INT TERM`

## Testing

### Automated Tests
- ✅ All 87 tests pass (unit, integration, capability)
- ✅ `cargo clippy` passes with `-D warnings`
- ✅ `cargo fmt` applied

### Manual Testing Scenarios
- [x] Basic config (name + email) copied successfully
- [x] Local config takes precedence over global
- [x] Warning displayed when git not configured
- [x] GPG signing detection and warning
- [x] SSH signing detection and warning
- [x] Runtime context generated correctly
- [x] Special characters in name/email handled safely
- [x] Integration with GPG capability

## Breaking Changes

None - this is a new optional capability.

## Future Enhancements

Potential improvements for future PRs:
- Add optional `copy_signing` flag to control signing behavior separately
- Support for git credential configuration
- Verification step to test git config after setup
- Integration tests for git signing workflows

## Checklist

- [x] Code follows project style and patterns
- [x] All tests pass
- [x] Documentation updated (CHANGELOG, README)
- [x] Security considerations addressed
- [x] Error handling implemented
- [x] Matches existing capability patterns (GPG, etc.)
- [x] cargo fmt and clippy clean

## Related

This capability works alongside:
- **GPG capability** - For GPG-based commit signing
- **SSH agent forwarding** - For SSH-based commit signing

---

**Ready to merge** - All functionality implemented, tested, and documented.
