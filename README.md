# claude-vm

Run Claude Code inside a sandboxed Lima VM for secure, isolated AI-assisted development.

## What is this?

`claude-vm` is a command-line tool that runs [Claude Code](https://claude.ai/code) inside a fresh virtual machine for each session. This provides:

- **Security**: Each Claude session runs in an isolated VM that's deleted after use
- **Clean environment**: No risk of polluting your host system
- **Sandboxing**: Claude can safely execute commands without affecting your main machine
- **Per-project templates**: Each project gets its own template with project-specific dependencies
- **Reproducibility**: Each session starts from a known template state

Based on [agent-vm](https://github.com/sylvinus/agent-vm) by Sylvain Zimmer.

## How it works

`claude-vm` creates a **persistent template VM per project**, then clones it for each Claude session:

1. **Project detection**: Identifies your project by git root (or current directory if not in git)
2. **Template per project**: Each project gets its own template VM with project-specific setup
3. **Ephemeral sessions**: Each `claude-vm` run clones the template, runs Claude, then deletes the clone
4. **Fast startup**: Cloning is much faster than creating a new VM from scratch
5. **Isolated changes**: Template stays clean; install dependencies there, not in sessions

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

### Project setup

Navigate to your project directory and create its VM template:

```bash
cd /path/to/your/project
claude-vm setup
```

This will:
- Install Lima (if needed)
- Detect your project (via git root or current directory)
- Create a Debian 13 VM template for this project
- Install base tools (git, curl, jq, wget, build-essential, ripgrep, etc.)
- Install Claude Code
- Authenticate Claude (you'll need your API key)
- Optionally install additional tools based on flags

**Setup options:**

By default, only base tools and Claude Code are installed. Add optional tools with:

- `--docker`: Install Docker CE with compose plugin
- `--node`: Install Node.js 22
- `--python`: Install Python 3 with pip and venv
- `--chromium`: Install Chromium browser (headless capable)
  - **Note**: Chrome MCP server requires Node.js. Use `--node` flag or install Node.js later via runtime scripts.
- `--all`: Install all optional tools (docker, node, python, chromium)

VM configuration:
- `--disk GB`: Set VM disk size (default: 20GB)
- `--memory GB`: Set VM memory (default: 8GB)

Examples:
```bash
cd ~/my-project
claude-vm setup                    # Minimal install (base tools + Claude)
claude-vm setup --all              # Install everything
claude-vm setup --docker --node    # Install Docker and Node.js only
claude-vm setup --python --disk 15 # Python with 15GB disk
```

You need to run `setup` **once per project**. Different projects get different templates.

### Running Claude

Navigate to your project directory and run:

```bash
cd /path/to/your/project
claude-vm "help me with my code"
```

This will:
1. Detect your project and find its template
2. Clone the project template VM
3. Mount your current directory into the VM
4. Run Claude Code with your prompt
5. Clean up and delete the VM when done

The template VM is reused across sessions, but each session gets a fresh clone.

### Debug shell

To explore the VM environment or debug issues:

```bash
claude-vm shell
```

This opens a bash shell in a fresh VM with your current directory mounted.

### Managing templates

List all project templates:
```bash
claude-vm list
```

Delete the current project's template:
```bash
cd /path/to/your/project
claude-vm clean
```

Delete all claude-vm templates:
```bash
claude-vm clean-all
```

### Custom setup scripts

#### Global template customization

Create `~/.claude-vm.setup.sh` to add custom setup steps that run during **every** template creation:

```bash
# Example: Install additional tools for all projects
sudo apt-get install -y vim neovim
npm install -g pnpm
```

#### Project template customization

Create `.claude-vm.setup.sh` in your project directory to run setup steps during **this project's** template creation:

```bash
# Example: Install project-specific tools in the template
sudo apt-get install -y postgresql-client
npm install -g typescript
```

This runs once during `claude-vm setup` and installs into the project's template.

#### Ad-hoc setup scripts

Pass custom setup scripts via the `--setup-script` flag during setup:

```bash
# Run one or more custom setup scripts
claude-vm setup --setup-script ./custom-tools.sh
claude-vm setup --setup-script ~/shared-setup.sh --setup-script ./project-deps.sh
```

These scripts run during template creation, after the global and project-specific scripts.

**Execution order:**
1. `~/.claude-vm.setup.sh` (global, if exists)
2. `.claude-vm.setup.sh` (project-specific, if exists)
3. Scripts passed via `--setup-script` flags (in order specified)

#### Runtime customization

Create `.claude-vm.runtime.sh` in your project directory to run setup steps for **each VM session**:

```bash
# Example: Install project dependencies (runs every session)
npm install
pip install -r requirements.txt
docker compose up -d
```

This runs every time you call `claude-vm` or `claude-vm shell`.

## Commands

```bash
claude-vm setup [options]   # Create VM template for current project (run once per project)
claude-vm [args...]         # Run Claude in a fresh VM
claude-vm shell             # Open debug shell in a fresh VM
claude-vm list              # List all project templates
claude-vm clean             # Delete current project's template
claude-vm clean-all         # Delete all claude-vm templates
claude-vm --help            # Show help
```

## Project detection

Templates are tied to projects using this logic (first match wins):

1. **Git repository root**: If you're in a git repo, uses `git rev-parse --show-toplevel`
2. **Current directory**: If not in a git repo, uses `pwd`

Template names use the format: `claude-tpl--<project-name>--<hash>`
- `<project-name>`: Sanitized basename of the project path (lowercase, alphanumeric + dashes)
- `<hash>`: 8-character hash for uniqueness

Examples:
- `/Users/you/Projects/my-app` → `claude-tpl--my-app--a1b2c3d4`
- `/home/user/web_site` → `claude-tpl--web-site--e5f6a7b8`

This means:
- All subdirectories of a git repo share the same template
- Non-git projects get a template per directory
- Moving a project changes its identity (you'd need to run setup again)
- Template names are human-readable while remaining unique

## Development

### Testing

The project includes a test suite using [bats-core](https://github.com/bats-core/bats-core).

**Install bats:**
```bash
# macOS
brew install bats-core

# Ubuntu/Debian
sudo apt-get install bats
```

**Run tests:**
```bash
# Run all unit tests (fast, no VMs created)
bats test/unit/

# Run specific test file
bats test/unit/test_project_functions.bats

# Run integration tests (slow, creates real VMs)
INTEGRATION=1 bats test/integration/
```

See [test/README.md](test/README.md) for detailed testing documentation.

### Contributing

1. Write tests for new features
2. Run `bats test/unit/` before committing
3. Run `shellcheck claude-vm` to lint the script
4. Update documentation as needed

## Security considerations

- Each VM session is completely isolated from your host system
- VMs are ephemeral - deleted after each use
- Your project directory is mounted with write access, so Claude can modify files
- The template VM stores your Claude authentication token (inherited by clones)

## Troubleshooting

**"Template VM not found" error:**
Run `claude-vm setup` in your project directory to create a template for that project.

**Wrong template being used:**
Check your project detection with:
```bash
git rev-parse --show-toplevel  # Shows git root if in a repo
pwd                            # Shows current directory otherwise
```

**Lima not installed:**
On macOS with Homebrew, Lima will be installed automatically. Otherwise, install from [https://lima-vm.io/docs/installation/](https://lima-vm.io/docs/installation/)

**VM creation is slow:**
The first `setup` per project takes several minutes to download and configure the VM. Subsequent Claude sessions are much faster as they clone the template.

**Templates taking up disk space:**
Each template uses ~20GB by default. Use `claude-vm list` to see all templates, and `claude-vm clean` or `claude-vm clean-all` to remove them.

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Credits

Based on [agent-vm](https://github.com/sylvinus/agent-vm) by Sylvain Zimmer.

Modified to work as a standalone command without requiring shell sourcing.
