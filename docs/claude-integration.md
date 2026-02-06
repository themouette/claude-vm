# Claude integration

This guide covers how claude-vm integrates with claude, which features are available in the sandbox.

## Automatic context

Claude-vm make claude aware that it runs in constrainted environments, such as a
VM and which tools are at its disposal.

It also adds you custom context.

### How to add context from scripts

Refer to [Runtime Scripts doc](./features/runtime-scripts.md)

## Claude Conversation History

By default, claude-vm automatically shares your Claude conversation history with the VM, allowing Claude to access context from previous conversations in the same project.

### How It Works

Claude stores conversation history in `~/.claude/projects/` using path-encoded folder names. Claude-vm automatically:

1. **Detects the current project path** - Based on your working directory
2. **Finds or creates the conversation folder** - Matches the path encoding
3. **Mounts it at `~/.claude/projects/`** - Inside the VM at the same location

This means commands running in the VM (including Claude itself) can access:

- Conversation history
- Artifacts
- Other project-specific Claude data

### Privacy and Isolation

**Important**: Only the current project's conversation folder is mounted.

- Conversations from other projects remain isolated
- Other projects' data is not accessible in the VM
- Each project has its own conversation space

This ensures:

- **Privacy**: Sensitive data from other projects stays isolated
- **Security**: VM compromise doesn't expose all conversations
- **Clarity**: Claude only sees relevant project context

### Path Encoding

Claude uses path encoding to create unique folder names for each project:

```bash
# Project path
/Users/user/my-project

# Encoded conversation folder
~/.claude/projects/-Users-user-my-project/
```

Claude-vm uses the same encoding scheme to ensure correct mapping.

### What Gets Mounted

The mounted conversation folder contains:

- **Conversation transcripts**: Full conversation history
- **Artifacts**: Generated files and code
- **Context**: Project-specific settings and data
- **Metadata**: Conversation timestamps and markers

### Disabling Conversation Sharing

To run Claude in an isolated session without access to conversation history, use the `--no-conversations` flag:

```bash
# Shell without conversation history
claude-vm --no-conversations shell

# Run Claude without conversation history
claude-vm --no-conversations "help me code"
```

This completely isolates the VM from conversation history.

### Use Cases for Disabled Conversations

Disable conversation sharing when:

- **Isolated testing**: You want a completely fresh environment
- **Debugging**: You're debugging conversation-related issues
- **No context needed**: You need to ensure no historical context influences Claude's behavior
- **Security testing**: You want to test behavior without historical data
- **Clean slate**: Starting a completely new context for experiments

### Example Usage

**With conversation history (default):**

```bash
cd /home/user/project
claude-vm "continue working on the authentication feature"
# Claude has access to previous conversations about the project
```

**Without conversation history:**

```bash
cd /home/user/project
claude-vm --no-conversations "analyze this codebase"
# Claude starts fresh with no historical context
```

### Configuration

Currently, conversation mounting cannot be disabled in configuration files. Use the `--no-conversations` flag when needed.

### Troubleshooting

**Problem**: Claude doesn't remember previous conversations

**Solution**: Ensure conversation history is being mounted:

1. Check that `--no-conversations` flag is not set
2. Verify conversation folder exists: `ls ~/.claude/projects/`
3. Check folder naming matches project path

**Problem**: Conversation folder not found

**Solution**: Claude creates conversation folders automatically:

1. Run Claude directly on host first to create folder
2. Then run claude-vm to mount the existing folder

**Problem**: Wrong conversation history appears in VM

**Solution**: This should not happen due to path encoding. If it does:

1. Check current working directory
2. Verify project path is correctly detected
3. Compare folder names manually

### Custom Conversation Location

Currently not supported. Conversation mounting always uses `~/.claude/projects/` with automatic path detection.

## Security Considerations

### Conversation History

- **Project isolation**: Only current project's conversations are accessible
- **No cross-project leakage**: Other projects' conversations stay isolated
- **Writable mount**: VM can modify conversation history (by design for Claude)
- **Host backup**: Keep backups of `~/.claude/projects/` for important conversations
