# VSCode Remote-SSH

Some developers prefer the **Claude Code VSCode extension** over the terminal — inline diffs, editor integration, and a richer UI. The `vscode` capability makes this possible while keeping the same VM isolation guarantees.

When VSCode connects to the VM via Remote-SSH, the VSCode server — and all extensions, including Claude Code — run inside the VM. The `claude` process runs in the isolated environment, exactly as it does with `claude-vm agent`. Your project files are accessible at the same path because they are mounted from the host into the VM.

## How It Works

The `vscode` capability:

1. **Mounts** `~/.claude-vm/vscode-server/{template_name}` → `~/.vscode-server` inside the VM, so VSCode server data and extensions persist between sessions
2. **Writes** a Lima SSH config to `~/.claude-vm/ssh/config` and ensures `~/.ssh/config` includes it (idempotently), so VSCode can connect via the SSH alias

The VM is started as a **persistent session** (see [Sessions](sessions.md)) so it stays alive while you work.

## Prerequisites

- [VSCode](https://code.visualstudio.com/) with:
  - [Remote-SSH extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-ssh)
  - [Claude Code extension](https://marketplace.visualstudio.com/items?itemName=anthropic.claude-code)
- A Claude VM template for your project (see [Getting Started](../getting-started.md))

## Quick Start

For a one-command experience, add this shell function to your `~/.bashrc` or `~/.zshrc`:

```bash
function claude-vscode() {
  SESSION=$(claude-vm session start) || return 1
  ALIAS=$(grep '^Host ' ~/.claude-vm/ssh/config | awk '{print $2}')
  code --folder-uri "vscode-remote://ssh-remote+${ALIAS}/$(pwd)"
  echo "Session $SESSION is running. Stop it with: claude-vm session stop $SESSION"
}
```

Then:

```bash
claude-vscode
# ... use Claude Code in VSCode ...
claude-vm session stop <session-id>
```

## Step-by-Step Setup

### 1. Enable the vscode capability

Add to your project's `.claude-vm.toml`:

```toml
[tools]
vscode = true
```

### 2. Start a session

```bash
SESSION=$(claude-vm session start)
```

This starts the VM, writes the SSH config, and prints a session ID.

### 3. Open VSCode connected to the VM

**Option A — command line:**

```bash
ALIAS=$(grep '^Host ' ~/.claude-vm/ssh/config | awk '{print $2}')
code --folder-uri "vscode-remote://ssh-remote+${ALIAS}/$(pwd)"
```

**Option B — command palette:**

1. `Ctrl+Shift+P` / `Cmd+Shift+P`
2. **Remote-SSH: Connect to Host...**
3. Select the alias (e.g. `lima-my-project-s-a3f7c2`)
4. Open your project folder from the Remote Explorer

The Claude Code extension is now running inside the isolated VM.

### 4. Stop the session when done

```bash
claude-vm session stop "$SESSION"
```

This shuts down the VM and clears the SSH config.

## VSCode Server Persistence

Without the `vscode` capability, VSCode downloads and installs its server into the VM on every session start (because the VM is ephemeral). With the capability, server data is stored on the host at `~/.claude-vm/vscode-server/{template_name}` and mounted into every session for the same template — VSCode loads instantly on reconnect, and installed extensions are preserved.

## SSH Config Management

When a session starts, the capability:

1. Ensures `~/.ssh/config` contains `Include ~/.claude-vm/ssh/config` (prepended if missing)
2. Runs `limactl show-ssh --format=config $VM_NAME` and writes the output to `~/.claude-vm/ssh/config`

When the session stops, `~/.claude-vm/ssh/config` is cleared (the `Include` line in `~/.ssh/config` stays, but it points to an empty file).

This means:
- Only the currently running session VM appears in your SSH config
- Multiple simultaneous sessions are not supported (each overwrites the same config file)
- The original `~/.ssh/config` is never modified beyond prepending the `Include` line

## Configuration Reference

In `.claude-vm.toml`:

```toml
[tools]
vscode = true
```

No additional options are required.

## Troubleshooting

### VSCode can't connect — host not found

Check that the session is running:

```bash
claude-vm session list
```

Check that the SSH config was written:

```bash
cat ~/.claude-vm/ssh/config
grep 'Include' ~/.ssh/config
```

If empty, the session may not have started correctly. Try stopping and restarting it.

### Claude Code extension downloads on every session

Ensure `vscode = true` is in `.claude-vm.toml`. Without it, the `~/.vscode-server` directory inside the VM is ephemeral and extensions are reinstalled each time.

### Permission denied when connecting

The SSH key used by Lima is managed automatically. If you get permission errors, try:

```bash
limactl show-ssh --format=config $(claude-vm session list | awk 'NR>1 {print $2}')
```

to verify the config manually.

## Next Steps

- **[Sessions](sessions.md)** — full session lifecycle reference
- **[Tools](tools.md)** — other capabilities you can enable
- **[Configuration](../configuration.md)** — complete `.claude-vm.toml` reference
