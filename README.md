# claude-vm

Run Claude Code inside a sandboxed Lima VM for secure, isolated AI-assisted development.

## What is this?

`claude-vm` is a command-line tool that runs [Claude Code](https://claude.ai/code) inside a fresh virtual machine for each session. This provides:

- **Security**: Each Claude session runs in an isolated VM that's deleted after use
- **Clean environment**: No risk of polluting your host system
- **Sandboxing**: Claude can safely execute commands without affecting your main machine
- **Reproducibility**: Each session starts from a known template state

Based on [agent-vm](https://github.com/sylvinus/agent-vm) by Sylvain Zimmer.

## Requirements

- macOS or Linux
- [Lima](https://lima-vm.io/) - Will be automatically installed via Homebrew if not present (macOS only)
- [Claude Code](https://claude.ai/code) - Will be installed in the VM template during setup

## Installation

1. Clone or download this repository:
   ```bash
   git clone <your-repo-url>
   cd claude-vm
   ```

2. Make the script executable (if not already):
   ```bash
   chmod +x claude-vm
   ```

3. Optionally, symlink it to your PATH for system-wide access:
   ```bash
   ln -s "$(pwd)/claude-vm" /usr/local/bin/claude-vm
   ```

## Usage

### First-time setup

Create the VM template with Claude Code pre-installed:

```bash
claude-vm setup
```

This will:
- Install Lima (if needed)
- Create a Debian 13 VM template
- Install development tools (git, curl, Docker, Node.js, Python, etc.)
- Install Claude Code
- Authenticate Claude (you'll need your API key)
- Configure optional MCP servers (like Chrome DevTools)

**Setup options:**
- `--minimal`: Install only git, curl, jq, and Claude Code (no Docker, Node.js, etc.)
- `--disk GB`: Set VM disk size (default: 20GB)
- `--memory GB`: Set VM memory (default: 8GB)

Example:
```bash
claude-vm setup --minimal --disk 10 --memory 4
```

### Running Claude

Navigate to any project directory and run:

```bash
claude-vm "help me with my code"
```

This will:
1. Clone the template VM
2. Mount your current directory into the VM
3. Run Claude Code with your prompt
4. Clean up and delete the VM when done

### Debug shell

To explore the VM environment or debug issues:

```bash
claude-vm shell
```

This opens a bash shell in a fresh VM with your current directory mounted.

### Custom setup scripts

#### Template-level customization

Create `~/.claude-vm.setup.sh` to add custom setup steps that run once during template creation:

```bash
# Example: Install additional tools
sudo apt-get install -y vim neovim
npm install -g pnpm
```

#### Project-level customization

Create `.claude-vm.runtime.sh` in your project directory to run setup steps for each VM session:

```bash
# Example: Install project dependencies
npm install
pip install -r requirements.txt
```

## How it works

1. **Template creation** (`claude-vm setup`): Creates a reusable VM template with Claude Code and tools pre-installed
2. **VM cloning**: Each session clones the template for a fresh, isolated environment
3. **Directory mounting**: Your current directory is mounted into the VM
4. **Session execution**: Claude Code runs with full access to the mounted directory
5. **Cleanup**: The VM is automatically deleted when Claude exits

## Commands

```bash
claude-vm setup [options]   # Create VM template (run once)
claude-vm shell             # Open debug shell in a fresh VM
claude-vm [args...]         # Run Claude in a fresh VM
claude-vm --help            # Show help
```

## Security considerations

- Each VM session is completely isolated from your host system
- VMs are ephemeral - deleted after each use
- Your project directory is mounted with write access, so Claude can modify files
- The template VM stores your Claude authentication token (inherited by clones)

## Troubleshooting

**"Template VM not found" error:**
Run `claude-vm setup` first to create the template.

**Lima not installed:**
On macOS with Homebrew, Lima will be installed automatically. Otherwise, install from [https://lima-vm.io/docs/installation/](https://lima-vm.io/docs/installation/)

**VM creation is slow:**
The first `setup` takes several minutes to download and configure the VM. Subsequent sessions are much faster as they clone the template.

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Credits

Based on [agent-vm](https://github.com/sylvinus/agent-vm) by Sylvain Zimmer.

Modified to work as a standalone command without requiring shell sourcing.
