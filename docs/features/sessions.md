# Persistent Sessions

By default, Claude VM creates an ephemeral VM per invocation and destroys it on exit. Persistent sessions let you start a VM once and reuse it across multiple commands — no re-creation overhead, no state loss between commands.

## Why Persistent Sessions?

**Ephemeral VMs** are perfect for isolated one-shot tasks:

```bash
claude-vm "implement this feature"   # VM created, task done, VM destroyed
```

**Persistent sessions** are better when an external tool needs to control VM lifetime:

- **VSCode Remote-SSH** — connect your editor to a running VM; the VM must stay alive while you edit
- **agentree / worktree orchestrators** — start one VM per workspace, run multiple `agent` and `shell` operations in it
- **Long-running services** — start a database or dev server once, run multiple agent passes against it

## Quick Start

```bash
# 1. Start a persistent session (prints a session ID)
SESSION=$(claude-vm session start)

# 2. Run agent and shell commands in the same VM
claude-vm agent --session "$SESSION" "add unit tests"
claude-vm shell --session "$SESSION" npm test

# 3. Stop the session when done
claude-vm session stop "$SESSION"
```

## Session Lifecycle

### `session start`

Creates a persistent VM and returns a session ID:

```bash
SESSION=$(claude-vm session start)
echo $SESSION   # e.g. "a3f7c2"
```

What happens:
1. Loads and merges configuration (capabilities baked in)
2. Creates a persistent VM (named `{template}-s{id}`, not pruned by auto-cleanup)
3. Runs `before_runtime` host phases (SSH config, etc.)
4. Saves a session record to `~/.claude-vm/sessions/{id}.json`
5. Prints the session ID and returns — **no blocking**

The session record stores a frozen snapshot of the configuration at start time, so later `agent`/`shell` invocations use the exact same config without reloading.

### `session list`

Show all sessions and their status:

```bash
claude-vm session list
```

Example output:

```
ID       VM name                              Template          Status   Created
a3f7c2   my-project-s-a3f7c2                 my-project        running  2 minutes ago
b8e1d4   other-project-s-b8e1d4              other-project     stopped  3 hours ago
```

### `session stop <id>`

Stops the VM and removes the session record:

```bash
claude-vm session stop a3f7c2
```

What happens:
1. Runs `host.teardown` phases (SSH cleanup, etc.)
2. Stops and deletes the Lima VM
3. Removes the session record from `~/.claude-vm/sessions/`

### `--session` flag on `agent` and `shell`

Reuse a running session:

```bash
claude-vm agent --session "$SESSION" "implement the feature"
claude-vm shell --session "$SESSION" npm test
claude-vm shell --session "$SESSION" -- cat README.md
```

When `--session` is set:
- Skips VM creation and resource checks
- Uses the frozen config from the session record
- No `CleanupGuard` — the VM lifetime is owned by the session, not the command
- Errors with a clear message if the session's VM is not running

## Configuration

Sessions inherit the configuration from `.claude-vm.toml` at the time `session start` is run. This includes:

- VM resources (disk, memory, CPUs)
- Enabled capabilities (tools, mounts)
- Runtime scripts

Changes to `.claude-vm.toml` after `session start` do **not** affect the running session.

### Enabling Capabilities for Sessions

Any capability enabled in `.claude-vm.toml` is automatically included when the session starts:

```toml
[tools]
git = true
node = true
vscode = true   # SSH config written on session start
```

## Session Storage

Session records are stored at `~/.claude-vm/sessions/{id}.json`. Each record contains:

- Session ID
- VM name
- Template name
- Project root path
- Creation time
- Frozen configuration snapshot

## Cleanup

**Manual cleanup:**

```bash
claude-vm session stop "$SESSION"
```

**Automatic orphan cleanup:**

The `prune` command removes orphaned session records (records whose VM no longer exists in Lima):

```bash
claude-vm prune
```

**Template cleanup:**

`clean` and `clean-all` also remove session records for the affected templates:

```bash
claude-vm clean       # Removes session records for current project's template
claude-vm clean-all   # Removes all session records
```

## Session Naming

Persistent session VMs use the naming convention `{template_name}-s{random_hex6}` (e.g. `my-project-s-a3f7c2`). The `s` prefix ensures they are **not** touched by the auto-prune logic, which only removes VMs with an all-digit suffix (ephemeral VMs named `{template}-{pid}`).

## Examples

### Parallel Agent Passes

```bash
SESSION=$(claude-vm session start)

claude-vm agent --session "$SESSION" "add docstrings to all public functions"
claude-vm agent --session "$SESSION" "run the test suite and fix any failures"
claude-vm shell --session "$SESSION" cargo fmt

claude-vm session stop "$SESSION"
```

### VSCode Remote-SSH

See the dedicated [VSCode guide](vscode.md) for a full walkthrough.

```bash
SESSION=$(claude-vm session start)
# Connect VSCode to the session VM, then:
claude-vm session stop "$SESSION"
```

### Scripted Orchestration

```bash
#!/bin/bash
SESSION=$(claude-vm session start)
trap "claude-vm session stop '$SESSION'" EXIT

for worktree in feature-1 feature-2 bugfix-3; do
  claude-vm agent --session "$SESSION" --worktree "$worktree" "run tests"
done
```

## Next Steps

- **[VSCode Remote-SSH](vscode.md)** — use persistent sessions with Visual Studio Code
- **[Tools](tools.md)** — capabilities you can enable for sessions
- **[Usage Guide](../usage.md)** — all commands and flags
