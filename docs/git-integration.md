# Git Integration

This guide covers how claude-vm integrates with git, including automatic worktree detection and conversation history mounting.

## Overview

Claude-vm provides seamless git integration for projects:

- **Automatic worktree detection**: Detects and mounts both worktree and main repository
- **Full git functionality**: All git operations work correctly in isolated VMs
- **Conversation history**: Shares Claude conversation history between host and VM
- **Project isolation**: Each project has its own conversation folder

## Architecture

The worktree management feature is built with a layered architecture that separates concerns and ensures security:

```
┌─────────────────────────────────────────────────────────────────┐
│                     CLI Commands Layer                          │
│  (commands/worktree/{create,list,remove}.rs, commands/agent.rs) │
│                                                                  │
│  • User interaction & confirmation prompts                      │
│  • Argument parsing & validation                                │
│  • Message formatting & display                                 │
└────────────────┬────────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Operations Layer                             │
│              (worktree/operations.rs)                           │
│                                                                  │
│  • create_worktree() - High-level worktree creation             │
│  • delete_worktree() - Worktree removal                         │
│  • list_merged_branches() - Branch status queries              │
│  • detect_branch_status() - Branch state detection             │
└────────────┬────────────────────────────────┬──────────────────┘
             │                                │
             │                                │
             ▼                                ▼
┌────────────────────────────┐  ┌───────────────────────────────┐
│   Validation Layer         │  │    State Management          │
│  (worktree/validation.rs)  │  │   (worktree/state.rs)        │
│                            │  │                               │
│  • Branch name validation  │  │  • Parse git worktree list   │
│  • Git version checking    │  │  • WorktreeEntry structs     │
│  • Submodule detection     │  │  • Locked worktree detection │
└────────────┬───────────────┘  └──────────┬────────────────────┘
             │                              │
             └──────────────┬───────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Template & Path System                        │
│           (worktree/template.rs)                                │
│                                                                  │
│  • TemplateContext - Variable expansion ({branch}, {repo}, etc) │
│  • compute_worktree_path() - Path computation & validation     │
│  • Path traversal prevention - Security checks                 │
│  • Path sanitization - Replace unsafe characters               │
└────────────────┬────────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                  Filter System                                  │
│              (worktree/filter.rs)                               │
│                                                                  │
│  • Composable iterator filters                                 │
│  • filter_merged() - Select merged branches                    │
│  • filter_locked() / exclude_locked() - Lock filtering        │
│  • skip_main() - Exclude main worktree                         │
└────────────────┬────────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                 Recovery & Cleanup                              │
│            (worktree/recovery.rs)                               │
│                                                                  │
│  • auto_prune() - Clean orphaned metadata (with confirmation)  │
│  • try_repair() - Repair broken worktree links                │
│  • ensure_clean_state() - Pre-operation validation            │
└────────────────┬────────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Git Command Layer                            │
│                 (utils/git.rs)                                  │
│                                                                  │
│  • run_git_command() - Execute git with 30s timeout            │
│  • run_git_query() - Query git (non-zero OK)                   │
│  • run_git_best_effort() - Cleanup operations                  │
│  • path_to_str() - UTF-8 path conversion                       │
│  • Command injection prevention                                 │
└─────────────────────────────────────────────────────────────────┘

Data Flow:
─────────
1. User invokes command (e.g., `claude-vm worktree create feature`)
2. CLI layer validates arguments and determines operation
3. Operations layer orchestrates the workflow:
   - Validates branch name (validation layer)
   - Checks current state (state layer)
   - Computes target path (template layer)
   - Executes git commands (git layer)
   - Handles errors and recovery (recovery layer)
4. Results displayed to user with appropriate messages

Security Boundaries:
───────────────────
• Branch name validation prevents command injection
• Path computation validates against traversal attacks
• All git commands use Command::args() (no shell)
• Timeouts prevent indefinite hangs
• UTF-8 validation prevents path-related panics
```

## Git Worktree Support

Git worktrees allow you to check out multiple branches in different directories from the same repository. Claude-vm provides comprehensive worktree management with dedicated commands and automatic detection.

### Worktree Management Commands

Claude-vm includes a full suite of worktree management commands:

#### Create Worktrees

```bash
# Create worktree from current branch
claude-vm worktree create feature-branch

# Create worktree from specific base branch
claude-vm worktree create feature-branch main

# Create and immediately start working
claude-vm agent --worktree feature-branch
claude-vm shell --worktree feature-branch
```

#### List Worktrees

```bash
# List all worktrees with branch, path, and status
claude-vm worktree list
```

#### Remove Worktrees

```bash
# Remove specific worktree (preserves branch)
claude-vm worktree remove feature-branch
# or use the short alias
claude-vm worktree rm feature-branch

# Remove multiple worktrees at once
claude-vm worktree remove feature-1 feature-2 feature-3

# Remove merged worktrees (defaults to current branch)
claude-vm worktree remove --merged
# Or specify a branch (supports local and remote branches)
claude-vm worktree remove --merged main
claude-vm worktree remove --merged origin/main

# Preview removal without making changes
claude-vm worktree remove feature-branch --dry-run
claude-vm worktree remove --merged --dry-run

# Skip confirmation prompt
claude-vm worktree remove feature-branch --yes
claude-vm worktree remove --merged --yes
```

### Flag Integration

The `--worktree` flag on agent and shell commands provides seamless worktree integration:

```bash
# Create or resume worktree and run agent
claude-vm agent --worktree my-feature

# Specify base branch for new worktrees
claude-vm agent --worktree my-feature main

# Open shell in worktree
claude-vm shell --worktree my-feature

# Use -- to separate worktree args from Claude/shell args
claude-vm agent --worktree my-feature -- /clear
claude-vm shell --worktree my-feature -- ls -la
```

The system automatically:
- Creates the worktree if the branch doesn't exist
- Resumes the existing worktree if the branch is already checked out
- Uses current HEAD as base when base-ref not specified
- Provides clear messaging about whether it created or resumed

### Configuration

Configure worktree behavior in `.claude-vm.toml`:

```toml
[worktree]
# Default location for worktrees (default: {repo_root}-worktrees/)
location = "/path/to/worktrees"

# Path template for worktree directories
template = "{repo}-{branch}"
```

Available template variables:
- `{repo}` - Repository name
- `{branch}` - Branch name (sanitized)
- `{user}` - Username from $USER environment variable
- `{date}` - Current date in YYYY-MM-DD format
- `{short_hash}` - Short commit hash (first 8 characters)

Example templates:
- `{branch}` (default) - Simple branch name
- `{user}/{branch}` - Organize by user
- `{date}-{branch}` - Prefix with date
- `{repo}-{branch}-{short_hash}` - Include repo and commit info

### Template Variable Sanitization

Branch names and other variables with special characters are automatically sanitized for filesystem safety:
- Slashes (`/`, `\`) are replaced with dashes (`-`)
- Spaces and control characters are replaced with underscores (`_`)
- Other characters are preserved

Examples:
- `feature/user-auth` → `feature-user-auth`
- `my branch` → `my_branch`
- `fix bug #123` → `fix_bug_#123`

### How It Works

When you run claude-vm from a git worktree directory, it automatically:

1. **Shares the same template** - All worktrees of a repository use the same VM template (based on the main repository root)
2. **Mounts the worktree directory** (writable) - Your current working directory
3. **Mounts the main repository** (writable) - The `.git` directory and parent repository
4. **Merges configurations** - Loads `.claude-vm.toml` from both worktree and main repo (worktree takes precedence)

Both mounts are writable because git commands in worktrees require write access to the main repository's `.git` directory to:

- Update refs (branches, tags)
- Create commits
- Perform git operations that modify repository state

#### Template Naming

All worktrees of the same repository share a single VM template. The template name is based on the main repository root, not the worktree path. This means:

- **Efficient resource usage**: One template serves all worktrees
- **Consistent environment**: Same VM configuration across all branches
- **Shared setup**: Run `claude-vm setup` once, use in all worktrees

#### Configuration Precedence

When in a worktree, configuration files are loaded with this precedence (highest to lowest):

1. **Worktree config**: `.claude-vm.toml` in the worktree directory
2. **Main repo config**: `.claude-vm.toml` in the main repository
3. **Global config**: `~/.claude-vm.toml` in your home directory
4. **Built-in defaults**: Default VM settings

This allows you to:
- Define common settings in the main repository
- Override specific settings per worktree (e.g., different memory limits, capabilities)
- Test configuration changes in a worktree before merging to main

### Example

Consider this git worktree setup:

```bash
# Main repository
/home/user/project/.git

# Worktree for feature branch
/home/user/project-feature/
```

When you run `claude-vm` from `/home/user/project-feature/`:

```bash
cd /home/user/project-feature
claude-vm shell
```

Claude-vm automatically mounts:

- `/home/user/project-feature/` (writable) - The worktree
- `/home/user/project/` (writable) - The main repository

### Git Operations in Worktrees

All standard git operations work correctly:

```bash
# Inside the VM
git status           # Works
git add .           # Works
git commit -m "msg" # Works
git push            # Works (with SSH agent forwarding if needed)
git pull            # Works
git branch          # Shows all branches
git worktree list   # Shows all worktrees
```

### Troubleshooting

**Problem**: Git operations fail with "unable to write" errors

**Solution**: This should not happen as both directories are mounted writable. If it does:

1. Check file permissions on host
2. Ensure the main repository directory is accessible
3. Verify git worktree is properly configured: `git worktree list`

**Problem**: Worktree not detected

**Solution**: Claude-vm detects worktrees automatically. If detection fails:

1. Verify you're in a valid worktree: `git rev-parse --is-inside-work-tree`
2. Check worktree configuration: `git worktree list`
3. Ensure `.git` file in worktree points to correct location

### Best Practices

1. **Run from worktree directory**: Always run claude-vm from within the worktree directory, not the main repository
2. **Commit in worktree**: Make commits from the worktree directory for clarity
3. **Shared operations**: Operations affecting the repository as a whole (like `git gc`) work from any worktree

## Combining Git Features

You can use worktrees and conversation history together seamlessly:

```bash
# Main repository
cd /home/user/project
claude-vm "help me refactor the auth module"

# Switch to worktree for feature branch
cd /home/user/project-feature
claude-vm "continue working on new feature"
```

Both share the same conversation history because they're part of the same project, but Claude-vm correctly mounts both the worktree and main repository.

## Best Practices

### For Git Worktrees

1. **Work in worktree directories**: Run claude-vm from the worktree, not the main repo
2. **Use descriptive names**: Name worktrees clearly for easier navigation
3. **Clean up old worktrees**: Remove worktrees when done to avoid confusion

### For Conversation History

1. **Use default mounting**: Keep conversations enabled for continuity
2. **Disable when needed**: Use `--no-conversations` for isolated testing
3. **One project, one conversation**: Keep related work in the same project directory
4. **Review history**: Check `~/.claude/projects/` to see available conversations

### For Combined Usage

1. **Consistent project structure**: Use the same project root for all worktrees
2. **Shared conversation context**: All worktrees of the same project share conversation history
3. **Test in isolation**: Use `--no-conversations` when testing across different branches
4. **Document branch context**: Help Claude understand which branch you're working on

## Advanced Configuration

### Worktree-Specific Behavior

Worktree detection is automatic and cannot be disabled. If you need to prevent main repository mounting:

1. This is not currently supported
2. Workaround: Run from a non-worktree directory
3. File an issue if you need this feature

## Security Considerations

### Git Worktrees

- **Full write access**: Both worktree and main repository are writable
- **Shared state**: Changes in VM affect host git state
- **Commit signing**: Use agent forwarding for signed commits (see agent-forwarding.md)
- **Isolated changes**: VM modifications are contained to the project directory

## Troubleshooting

### Common Issues and Solutions

#### "Branch name cannot start with a dash"

**Problem**: Attempting to create a worktree with a branch name starting with `-`.

**Cause**: Branch names starting with `-` are rejected to prevent command injection and flag confusion.

**Solution**: Use a valid branch name without leading dash:
```bash
# ✗ Won't work
claude-vm worktree create -feature

# ✓ Use instead
claude-vm worktree create feature
```

#### "Path contains invalid UTF-8"

**Problem**: Error message about invalid UTF-8 in path.

**Cause**: The worktree path contains characters that cannot be represented in UTF-8.

**Solution**: This is rare but can occur with unusual filesystem configurations. Check your worktree configuration template and ensure it uses standard characters.

#### "git worktree timed out after 30 seconds"

**Problem**: Git command hangs and times out.

**Cause**: Network issues (remote branches), repository problems, or very large repositories.

**Solutions**:
1. Check network connectivity if using remote branches
2. Try with a local branch: `claude-vm worktree create feature main`
3. Verify repository health: `git fsck`
4. For large repos, consider using shallow clones

#### "No worktree found for branch 'feature'"

**Problem**: Trying to remove a worktree that doesn't exist.

**Cause**: The branch exists but isn't checked out in any worktree, or the worktree was already removed.

**Solutions**:
1. List current worktrees: `claude-vm worktree list`
2. Check if branch exists: `git branch -a`
3. Create the worktree first: `claude-vm worktree create feature`

#### Orphaned Worktree Metadata

**Problem**: Git tracks worktrees that no longer exist on disk.

**Symptoms**:
- `git worktree list` shows paths that don't exist
- Errors about missing worktree directories

**Solution**: The system automatically prompts to prune orphaned metadata:
```bash
# Automatic on most operations
claude-vm worktree list
# Prompts: "Found orphaned worktree metadata. Prune? [y/N]"

# Manual prune
git worktree prune
```

#### Locked Worktrees

**Problem**: Worktree is locked and cannot be removed.

**Symptoms**:
- `--merged` flag skips certain worktrees
- Cannot remove specific worktree

**Check if locked**:
```bash
claude-vm worktree list --locked
```

**Solutions**:
1. Include locked worktrees in removal:
   ```bash
   claude-vm worktree remove --merged --locked --yes
   ```

2. Manually unlock:
   ```bash
   git worktree unlock /path/to/worktree
   ```

3. Understand why it's locked:
   ```bash
   # Check lock reason
   cat .git/worktrees/<branch>/locked
   ```

#### "This repository contains submodules"

**Problem**: Warning message when using worktrees with submodules.

**Cause**: Git's worktree support for submodules is experimental.

**Solutions**:
- The warning is informational only
- Worktrees will work but submodules may behave unexpectedly
- Test carefully if using submodules with worktrees
- See: https://git-scm.com/docs/git-worktree#_bugs

#### Worktree Directory Exists but Git Doesn't Know About It

**Problem**: The worktree directory exists but `claude-vm worktree list` doesn't show it.

**Cause**: Directory was created manually or git metadata is out of sync.

**Solutions**:
1. Remove the directory manually:
   ```bash
   rm -rf /path/to/worktree
   ```

2. Try repair:
   ```bash
   git worktree repair
   ```

3. Create fresh worktree with correct setup:
   ```bash
   claude-vm worktree create branch-name
   ```

#### "Git version X.Y.Z is too old"

**Problem**: Git version doesn't support worktrees.

**Cause**: Worktrees require Git 2.5+.

**Solution**: Update git:
```bash
# macOS
brew upgrade git

# Ubuntu/Debian
sudo apt update && sudo apt upgrade git

# Check version
git --version
```

#### Branch Creation Fails with "Invalid reference"

**Problem**: Creating worktree with base branch fails.

**Cause**: Base branch doesn't exist or isn't accessible.

**Solutions**:
1. Check branch exists:
   ```bash
   git branch -a | grep base-branch
   ```

2. Fetch remote branches:
   ```bash
   git fetch origin
   ```

3. Use correct branch reference:
   ```bash
   # For local branch
   claude-vm worktree create feature main

   # For remote branch
   claude-vm worktree create feature origin/main
   ```

#### Worktree Path Contains Spaces

**Problem**: Spaces in worktree paths cause issues.

**Cause**: Branch names with spaces get sanitized to underscores in paths.

**Expected Behavior**: This is intentional. Branch names with spaces like "my feature" become "my_feature" in the filesystem path. The branch name itself remains unchanged.

### Getting Help

If you encounter issues not covered here:

1. **Check git status**: Run `git worktree list` and `git status` to understand current state
2. **Review logs**: Look for error messages in command output
3. **Try repair**: Run `git worktree repair` for metadata issues
4. **File an issue**: Report bugs at https://github.com/anthropics/claude-vm/issues with:
   - Full error message
   - Git version (`git --version`)
   - claude-vm version (`claude-vm --version`)
   - Steps to reproduce

## Related Documentation

- [Agent Forwarding](agent-forwarding.md) - Configure git identity and commit signing
- [README.md](../README.md) - Main documentation and setup guide
