# Tilde Expansion Fix

## Problem

The tilde (`~`) expansion code was duplicated in two places:
- `src/config.rs` (lines 258-268)
- `src/vm/mount.rs` (lines 84-103)

Both implementations had the same limitation: they only supported `~` and `~/path` (current user's home directory) but not `~username/path` syntax (other users' home directories).

## Solution

1. **Created a centralized utility function** in `src/utils/path.rs`:
   - `expand_tilde()` - Properly expands both `~` and `~username` syntax
   - Uses the `users` crate to look up user home directories
   - Returns `Option<PathBuf>` for safe error handling

2. **Updated both locations** to use the new utility:
   - `src/config.rs` - `resolve_context_file()` method now uses `expand_tilde()`
   - `src/vm/mount.rs` - `expand_path()` function now uses `expand_tilde()` with better error messages

3. **Added comprehensive tests** to verify:
   - Current user expansion: `~` and `~/path`
   - Other user expansion: `~root/.bashrc`, `~username/file`
   - Non-existent user handling: `~nonexistentuser/path` (returns error)
   - Edge cases: paths without tilde, HOME not set, etc.

## Examples

### Before
```rust
// Only supported current user
"~/.claude-vm.toml"         // ✓ Worked
"~/Projects/my-project"     // ✓ Worked
"~otheruser/.config"        // ✗ Failed - treated as current user
```

### After
```rust
// Supports both current and other users
"~/.claude-vm.toml"         // ✓ Works - current user's home
"~/Projects/my-project"     // ✓ Works - current user's home
"~root/.bashrc"             // ✓ Works - root user's home
"~otheruser/.config"        // ✓ Works - otheruser's home (if user exists)
"~nonexistent/path"         // ✗ Error - user not found (proper error message)
```

## Changes Made

1. **New files:**
   - `src/utils/path.rs` - New utility module with `expand_tilde()` function

2. **Modified files:**
   - `Cargo.toml` - Added `users = "0.11"` dependency
   - `src/utils/mod.rs` - Added `pub mod path;`
   - `src/config.rs` - Updated `resolve_context_file()` to use new utility
   - `src/vm/mount.rs` - Updated `expand_path()` to use new utility with better errors

3. **Added tests:**
   - `src/utils/path.rs` - 8 comprehensive test cases
   - `src/vm/mount.rs` - 2 additional test cases for ~username syntax

## Benefits

1. **DRY (Don't Repeat Yourself)** - Single source of truth for tilde expansion
2. **Better functionality** - Supports `~username/` syntax
3. **Better error messages** - Clear error when user not found
4. **Maintainability** - Changes to tilde expansion only need to happen in one place
5. **Testability** - Comprehensive test coverage in one location
