# Git Integration

This guide covers how claude-vm integrates with git, including automatic worktree detection and conversation history mounting.

## Overview

Claude-vm provides seamless git integration for projects:

- **Automatic worktree detection**: Detects and mounts both worktree and main repository
- **Full git functionality**: All git operations work correctly in isolated VMs
- **Conversation history**: Shares Claude conversation history between host and VM
- **Project isolation**: Each project has its own conversation folder

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

#### Delete Worktrees

```bash
# Delete specific worktree (preserves branch)
claude-vm worktree delete feature-branch

# Clean merged worktrees
claude-vm worktree clean --merged
claude-vm worktree clean --merged main
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
path_template = "{repo}-{branch}"
```

Available template variables:
- `{repo}` - Repository name
- `{branch}` - Branch name (sanitized)

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

## Related Documentation

- [Agent Forwarding](agent-forwarding.md) - Configure git identity and commit signing
- [README.md](../README.md) - Main documentation and setup guide
