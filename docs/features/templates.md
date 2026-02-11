# Template VMs

Template VMs are the foundation of Claude VM's speed and reproducibility. This guide explains how templates work and how to manage them.

## What are Templates?

A template VM is a pre-configured, ready-to-clone virtual machine that serves as a starting point for all sessions in a project. Think of it as a "golden image" that contains:

- Base OS (Debian 13)
- Installed tools (Docker, Node.js, Python, etc.)
- System packages
- Configuration from setup scripts (run during template creation)
- Everything needed for your project

**Note:** Setup scripts (`.claude-vm.setup.sh`) always run during template creation. The `setup` command automatically recreates the template from scratch, running all setup scripts each time.

## Why Templates?

### Speed

Creating a VM from scratch takes 5-10 minutes. Cloning from a template takes seconds:

```bash
# First time: create template (slow)
claude-vm setup --all  # ~5 minutes

# Every session after: clone template (fast)
claude-vm shell  # ~10 seconds
```

### Reproducibility

Every session starts from the exact same state:

- Same tool versions
- Same system configuration
- Same environment setup
- No accumulated cruft or state

### Efficiency

Template VMs are stored once and cloned multiple times with copy-on-write:

```
template VM (8 GB)
├── session 1 (+ 500 MB changes)
├── session 2 (+ 200 MB changes)
└── session 3 (+ 1 GB changes)
```

Total disk usage: ~10 GB instead of 24 GB

## Template Naming

Templates are named based on the project path:

```bash
# Project: /Users/me/my-project
# Template: claude-tpl_my-project_a1b2c3d4

# Project: /Users/me/work/api
# Template: claude-tpl_api_e5f6g7h8
```

The hash ensures uniqueness even with identical project names.

### Git Worktrees

When working in git worktrees, **all worktrees share the same template**. The template name is based on the main repository root, not the worktree path:

```bash
# Main repository
cd /Users/me/project
claude-vm setup --docker
# Creates: claude-tpl_project_a1b2c3d4

# Worktree (shares same template)
cd /Users/me/project-feature
claude-vm shell
# Uses: claude-tpl_project_a1b2c3d4 (same template!)
```

This ensures:
- **Resource efficiency**: One template for all worktrees
- **Consistent environment**: Same tools across all branches
- **Shared setup**: Run setup once, use everywhere

See [Git Integration](../git-integration.md) for more details on worktree support.

## Creating Templates

### Basic Creation

```bash
# Minimal template
claude-vm setup

# With tools
claude-vm setup --git --docker --node

# Everything
claude-vm setup --all
```

### With Custom Resources

```bash
# Larger VM
claude-vm setup --disk 50 --memory 32 --all

# Or via config
[vm]
disk = 50
memory = 32
```

### With Setup Scripts

Templates can run setup scripts during creation:

**Auto-detected scripts:**

- `~/.claude-vm.setup.sh` - Global setup
- `./.claude-vm.setup.sh` - Project setup

**Config-based scripts:**

```toml
[setup]
scripts = [
    "./scripts/install-extras.sh",
    "./scripts/configure-env.sh"
]
```

**CLI scripts:**

```bash
claude-vm setup --setup-script ./my-setup.sh --git
```

## Template Lifecycle

### 1. Creation

```bash
claude-vm setup --git --docker
```

**What happens:**

1. Lima creates a new VM
2. Installs base system
3. Installs requested tools
4. Runs setup scripts
5. Stops the VM
6. Template is ready

### 2. Usage

```bash
claude-vm shell
```

**What happens:**

1. Lima clones the template VM
2. Starts the cloned VM
3. Mounts project directory
4. Runs runtime scripts
5. Opens shell
6. When you exit, VM is destroyed

### 3. Updates

```bash
# Delete old template
claude-vm clean

# Create new template with updated tools
claude-vm setup --all
```

Templates are immutable - to update, recreate them.

### 4. Cleanup

```bash
# Clean current project's template
claude-vm clean

# Clean all templates
claude-vm clean-all
```

## Managing Templates

### List Templates

```bash
# Show all templates
claude-vm list

# With disk usage
claude-vm list --disk-usage

# Only unused templates (30+ days)
claude-vm list --unused
```

**Example output:**

```
claude-tpl_project1_abc123de (stopped) - 5.2 GB
claude-tpl_project2_def456ab (running) - 12.8 GB
claude-tpl_test_789xyz01 (stopped) - 3.1 GB
```

### View Template Info

```bash
# Show info for current project
claude-vm info
```

**Example output:**

```
Project: /Users/me/my-project
Template: claude-tpl_my-project_abc123de
Status: stopped

Configuration:
  Disk: 20 GB
  Memory: 8 GB
  Capabilities: docker, node, git

Mounts:
  /Users/me/my-project -> /Users/me/my-project (writable)

Runtime Scripts:
  ./.claude-vm.runtime.sh
```

### Delete Templates

```bash
# Delete current project's template
claude-vm clean

# Skip confirmation
claude-vm clean --yes

# Delete all templates
claude-vm clean-all

# Skip confirmation
claude-vm clean-all --yes
```

## Template Best Practices

### 1. One Template Per Project

Each project should have its own template:

```bash
cd ~/project1
claude-vm setup --docker

cd ~/project2
claude-vm setup --node
```

### 2. Recreate When Tools Change

When you need different tools, recreate the template:

```bash
# Current template has docker
# Now you need node too

claude-vm setup --docker --node
```

### 3. Use Global Setup for Common Tools

For tools needed across all projects:

```bash
# ~/.claude-vm.setup.sh
#!/bin/bash
curl -sSL https://install.python-poetry.org | python3 -
```

### 4. Keep Templates Updated

Periodically recreate templates to get security updates:

```bash
# Every few months
claude-vm setup --all
```

### 5. Clean Unused Templates

Free up disk space by removing old templates:

```bash
# Find unused templates
claude-vm list --unused

# Clean specific projects
cd ~/old-project
claude-vm clean

# Or clean all unused
claude-vm clean-all
```

## Template Configuration

Templates are configured via `.claude-vm.toml`:

```toml
[vm]
disk = 30      # Template disk size
memory = 16    # Template memory

[tools]
docker = true  # Install in template
node = true
python = true

[packages]
system = ["postgresql-client", "jq"]  # Install in template

[setup]
scripts = ["./setup.sh"]  # Run during template creation
```

This configuration is "baked into" the template during creation.

## Troubleshooting

### Template Creation Fails

```bash
# Clean any partial template
claude-vm clean

# Try again with verbose output
claude-vm --verbose setup --all

# Check Lima status
limactl list
```

### Wrong Tools in Template

```bash
# Recreate template with correct tools
claude-vm setup --docker --node --git
```

## Advanced: Manual Template Management

### Access Template VM

```bash
# Get template name
claude-vm info

# Start template VM
limactl start claude-tpl_project_abc123de

# Shell into template
limactl shell claude-tpl_project_abc123de

# Stop template
limactl stop claude-tpl_project_abc123de
```

### Inspect Template

```bash
# List Lima VMs
limactl list

# Show VM info
limactl show claude-tpl_project_abc123de

# View logs
tail -f ~/.lima/claude-tpl_project_abc123de/ha.stdout.log
```

## Next Steps

- **[Runtime Scripts](runtime-scripts.md)** - Automate environment setup per session
- **[Tools](tools.md)** - Understand available tools for templates
- **[Configuration](../configuration.md)** - Full configuration reference
- **[Troubleshooting](../advanced/troubleshooting.md)** - Debug template issues
