# Getting Started

This guide will help you install Claude VM and create your first template.

## Requirements

- **Lima VM**: Required for running Linux VMs on macOS and Linux
- **macOS or Linux**: Claude VM runs on macOS (Intel and Apple Silicon) and Linux
- **Rust 1.70+**: Only needed if building from source

## Installation

### Option 1: One-Line Install (Recommended)

**Install to ~/.local/bin (no sudo required):**

```bash
curl -fsSL https://raw.githubusercontent.com/themouette/claude-vm/main/install.sh | bash
```

**Install system-wide to /usr/local/bin:**

```bash
curl -fsSL https://raw.githubusercontent.com/themouette/claude-vm/main/install.sh | bash -s -- --global
```

**Install specific version:**

```bash
curl -fsSL https://raw.githubusercontent.com/themouette/claude-vm/main/install.sh | bash -s -- --version v0.3.0
```

**Custom installation directory:**

```bash
curl -fsSL https://raw.githubusercontent.com/themouette/claude-vm/main/install.sh | bash -s -- --destination /opt/bin
```

This script will:
1. Detect your platform (macOS or Linux)
2. Download the appropriate binary for the specified version (or latest)
3. Install it to the specified directory (default: `~/.local/bin`)
4. Make it executable
5. Verify the installation and check PATH configuration

### Option 2: Download from GitHub

1. Visit the [releases page](https://github.com/themouette/claude-vm/releases/latest)
2. Download the binary for your platform:
   - `claude-vm-macos-x86_64` - macOS Intel
   - `claude-vm-macos-aarch64` - macOS Apple Silicon
   - `claude-vm-linux-x86_64` - Linux x86_64
   - `claude-vm-linux-aarch64` - Linux ARM64
3. Make it executable: `chmod +x claude-vm`
4. Move to your PATH: `mv claude-vm ~/.local/bin/`

### Option 3: Build from Source

See [Development Guide](development.md) for instructions.

## Installing Lima

Lima is required for running the VMs. Install it for your platform:

### macOS

```bash
brew install lima
```

### Linux (Debian/Ubuntu)

```bash
# Install dependencies
sudo apt-get update
sudo apt-get install -y qemu-system-x86

# Install Lima
VERSION=$(curl -fsSL https://api.github.com/repos/lima-vm/lima/releases/latest | grep tag_name | cut -d'"' -f4 | sed 's/v//')
curl -fsSL "https://github.com/lima-vm/lima/releases/download/v${VERSION}/lima-${VERSION}-Linux-x86_64.tar.gz" | sudo tar Cxzf /usr/local -
```

### Verify Installation

```bash
limactl --version
```

## First Steps

### 1. Navigate to Your Project

```bash
cd ~/Projects/my-project
```

### 2. Create a Template VM

Create a template VM with the tools you need:

```bash
# Minimal setup with git
claude-vm setup --git

# With Docker support
claude-vm setup --git --docker

# With Node.js
claude-vm setup --git --node

# With everything
claude-vm setup --all
```

This creates a template VM that will be cloned for each session. The setup takes a few minutes but only needs to be done once per project.

### 3. Run Claude

```bash
# Start Claude in an isolated VM
claude-vm "help me understand this codebase"

# Or just start Claude interactively
claude-vm
```

### 4. Try Shell Access

```bash
# Open an interactive shell in the VM
claude-vm shell

# Run a single command
claude-vm shell ls -la
```

## Auto-Setup

If you try to run `claude-vm` without creating a template first, you'll be prompted to create one:

```bash
$ claude-vm "help me"
No template found for project: /path/to/project
Template name: claude-tpl_myproject_abc123

Would you like to create it now? [Y/n]:
```

To skip the prompt and auto-create templates, use `--auto-setup`:

```bash
claude-vm --auto-setup "help me code"
```

Or enable it permanently in your config:

```toml
# .claude-vm.toml
auto_setup = true
```

## Next Steps

- **[Usage Guide](usage.md)** - Learn all available commands
- **[Configuration](configuration.md)** - Customize your VM settings
- **[Tools](features/tools.md)** - Understand available tools
- **[Runtime Scripts](features/runtime-scripts.md)** - Automate environment setup

## Troubleshooting

### Lima not found

If you get "lima not found" error:

```bash
# Verify Lima is installed
which limactl

# Install Lima
brew install lima  # macOS
```

### Permission denied

If you get permission errors:

```bash
# Make binary executable
chmod +x ~/.local/bin/claude-vm

# Ensure ~/.local/bin is in your PATH
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc  # or ~/.zshrc
source ~/.bashrc  # or ~/.zshrc
```

### Template creation fails

If template creation fails:

```bash
# Check Lima status
limactl list

# Remove failed template
claude-vm clean

# Try again with verbose output
claude-vm --verbose setup --git
```

For more troubleshooting tips, see [Troubleshooting Guide](advanced/troubleshooting.md).
