# Capabilities

This directory contains capability definitions for claude-vm. Capabilities are modular components that extend the VM's functionality.

## Directory Structure

Each capability is organized in its own directory with colocated configuration and scripts:

```
capabilities/
├── docker/
│   ├── capability.toml    # Capability definition
│   └── setup.sh           # Setup script
├── node/
│   ├── capability.toml
│   └── setup.sh
├── python/
│   ├── capability.toml
│   └── setup.sh
├── chromium/
│   ├── capability.toml
│   └── setup.sh
├── gpg/
│   └── capability.toml    # Uses inline scripts, no separate file
└── README.md              # This file
```

## What is a Capability?

A capability is a self-contained unit that can:
- Install software during VM setup
- Run initialization code before each session
- Register MCP (Model Context Protocol) servers
- Declare resource requirements (e.g., socket forwarding)

## Capability Structure

Each capability is defined in a TOML file with this structure:

```toml
[capability]
id = "unique-id"
name = "Human Readable Name"
description = "What this capability provides"

# Optional: Declarative package management
[packages]
system = ["package1", "package2"]  # Debian packages to install
setup_script = """
#!/bin/bash
# Optional: Add custom repositories before package installation
# Must be idempotent (safe to run multiple times)
"""

# Optional: Run on host before VM creation
[host_setup]
script = """
#!/bin/bash
# Runs on macOS/Linux host
# Can validate prerequisites, copy files to VM
"""

# Optional: Run in guest VM during template setup
[vm_setup]
script_file = "setup.sh"  # Reference embedded script
# OR
script = """
#!/bin/bash
# Inline script content
# Note: Use [packages] for installing system packages instead
"""

# Optional: Run in VM before each session (vm_runtime)
[vm_runtime]
script = """
#!/bin/bash
# Initialize environment for session
export SOME_VAR=value
"""

# Optional: Register MCP servers
[[mcp]]
id = "server-name"
command = "npx"
args = ["-y", "package@latest"]
enabled_when = "other-capability"  # Optional: Only if another capability is enabled

# Optional: Declare forwarding requirements
[[forwards]]
type = "unix_socket"
host = { detect = "command-to-detect-socket" }
guest = "/path/in/vm"
```

## Declarative Package Management

Capabilities can declare system packages directly in their TOML files using the `[packages]` section. This eliminates the need for manual `apt-get install` commands in setup scripts.

### Basic Package Declaration

```toml
[capability]
id = "python"
name = "Python"
description = "Python 3 with pip and development tools"

[packages]
system = ["python3", "python3-pip", "python3-venv"]
```

### Advanced Features

**Version Pinning:**
```toml
[packages]
system = [
    "python3=3.11.0-1",      # Exact version
    "nodejs=22.*",           # Wildcard version
    "libc6:amd64"            # Architecture specification
]
```

**Custom Repositories:**
```toml
[packages]
system = ["docker-ce", "docker-ce-cli", "containerd.io"]
setup_script = """
#!/bin/bash
set -e
# Add Docker's official GPG key and repository

# Ensure keyring directory exists
sudo mkdir -p /etc/apt/keyrings
sudo chmod 755 /etc/apt/keyrings

# Download GPG key only if not present
if [ ! -f /etc/apt/keyrings/docker.asc ]; then
    sudo curl -fsSL https://download.docker.com/linux/debian/gpg \\
        -o /etc/apt/keyrings/docker.asc
    sudo chmod a+r /etc/apt/keyrings/docker.asc
fi

# Add repository only if not configured
if ! grep -q "download.docker.com" /etc/apt/sources.list.d/docker.list 2>/dev/null; then
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/debian $(. /etc/os-release && echo \"$VERSION_CODENAME\") stable" | \\
        sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
fi
"""
```

### Benefits

- **Declarative**: Packages defined in data, not imperative scripts
- **Optimized**: All packages install in a single batch operation
- **Validated**: Package names are validated to prevent shell injection
- **Deduplicated**: Duplicate packages across capabilities are automatically removed
- **Ordered**: Dependency order is preserved during deduplication

### Package Installation Flow

1. Base packages install first (git, curl, wget, etc.) without `apt-get update`
2. All capability `setup_script`s run to add custom repositories
3. Single `apt-get update` executes
4. All packages from all capabilities install in one batch operation
5. Individual capability `vm_setup` scripts run for post-install configuration

### Migration from Shell Scripts

**Before (imperative):**
```toml
[vm_setup]
script = """
#!/bin/bash
set -e
sudo apt-get update
sudo apt-get install -y python3 python3-pip python3-venv
"""
```

**After (declarative):**
```toml
[packages]
system = ["python3", "python3-pip", "python3-venv"]

# vm_setup now only handles post-install configuration if needed
```

## Available Capabilities

### docker
Installs Docker engine in the VM for container management.

### node
Installs Node.js 20 LTS with npm package manager.

### python
Installs Python 3 with pip and development tools.

### chromium
Installs Chromium browser for headless automation and testing.
Registers the Chrome DevTools MCP server (requires Node.js/npm to be available).

### gpg
Enables GPG agent forwarding from host to VM (experimental).
Copies public keys and configures SSH for socket forwarding.

## Adding a New Capability

1. Create a new directory: `capabilities/my-capability/`

2. Create `capability.toml` in that directory:
   ```toml
   [capability]
   id = "my-capability"
   name = "My Capability"
   description = "What this capability provides"

   [vm_setup]
   script_file = "setup.sh"  # Or use inline script
   ```

3. Create setup script if needed: `capabilities/my-capability/setup.sh`

4. Register the capability in `src/capabilities/registry.rs`:
   ```rust
   const CAPABILITY_FILES: &[(&str, &str)] = &[
       // ... existing capabilities ...
       ("my-capability", include_str!("../../capabilities/my-capability/capability.toml")),
   ];
   ```

5. Add script loading in `src/capabilities/executor.rs` (if using script_file):
   ```rust
   fn get_embedded_script(capability_id: &str, script_name: &str) -> Result<String> {
       let content = match (capability_id, script_name) {
           // ... existing cases ...
           ("my-capability", "setup.sh") => include_str!("../../capabilities/my-capability/setup.sh"),
           // ...
       };
   }
   ```

6. Add configuration field in `src/config.rs`:
   ```rust
   pub struct ToolsConfig {
       // ... existing tools ...
       #[serde(default)]
       pub my_capability: bool,
   }
   ```

7. Update the `is_enabled` method in `src/capabilities/registry.rs`:
   ```rust
   fn is_enabled(&self, id: &str, config: &Config) -> bool {
       match id {
           // ... existing cases ...
           "my-capability" => config.tools.my_capability,
           _ => false,
       }
   }
   ```

That's it! The capability system handles everything else automatically.

## Lifecycle Hooks

### host_setup
- **When**: On host before VM is created
- **Where**: macOS/Linux host machine
- **Purpose**: Validate prerequisites, detect resources, copy files to VM
- **Environment**: `$PROJECT_ROOT`, `$TEMPLATE_NAME`, `$LIMA_INSTANCE`

### vm_setup
- **When**: In guest VM during `claude-vm setup`
- **Where**: Inside Lima VM (guest)
- **Purpose**: Install software, configure system
- **Environment**: Standard Lima environment

### vm_runtime
- **When**: In VM before each `claude-vm run`
- **Where**: Inside ephemeral VM session
- **Purpose**: Initialize environment variables, start services
- **Note**: Runs silently (only errors shown)

## Best Practices

1. **Keep capabilities focused**: Each capability should do one thing well
2. **Declare dependencies**: Use `requires` if your capability needs another
3. **Use script_file for complex installs**: Keep TOML clean
4. **Use inline script for simple config**: Avoid extra files for 3-line scripts
5. **Make runtime hooks fast**: They run before every session
6. **Handle errors gracefully**: Check prerequisites in host_setup
7. **Document requirements**: Add comments explaining what the capability needs

## Examples

### Simple Capability (declarative packages)

```toml
[capability]
id = "git-lfs"
name = "Git LFS"
description = "Git Large File Storage support"

[packages]
system = ["git-lfs"]

[vm_setup]
script = """
#!/bin/bash
set -e
# Post-install configuration
git lfs install
"""
```

### Complex Capability (with custom repository and MCP)

```toml
[capability]
id = "postgres"
name = "PostgreSQL"
description = "PostgreSQL database server"

[packages]
system = ["postgresql-16", "postgresql-client-16"]
setup_script = """
#!/bin/bash
set -e
# Add PostgreSQL official repository
if [ ! -f /etc/apt/keyrings/postgresql.asc ]; then
    curl -fsSL https://www.postgresql.org/media/keys/ACCC4CF8.asc | \\
        sudo gpg --dearmor -o /etc/apt/keyrings/postgresql.asc
    echo "deb [signed-by=/etc/apt/keyrings/postgresql.asc] http://apt.postgresql.org/pub/repos/apt $(lsb_release -cs)-pgdg main" | \\
        sudo tee /etc/apt/sources.list.d/pgdg.list
fi
"""

[vm_setup]
script = """
#!/bin/bash
set -e
# Configure PostgreSQL
sudo systemctl enable postgresql
sudo systemctl start postgresql
"""

[vm_runtime]
script = """
#!/bin/bash
# Write PostgreSQL context
mkdir -p ~/.claude-vm/context
cat > ~/.claude-vm/context/postgres.txt <<EOF
PostgreSQL version: $(psql --version 2>/dev/null || echo "not available")
Service status: $(systemctl is-active postgresql 2>/dev/null || echo "unknown")
EOF
"""

[[mcp]]
id = "postgres-query"
command = "npx"
args = ["-y", "postgres-mcp@latest"]
enabled_when = "node"
```

### Host Setup Example

```toml
[capability]
id = "aws-credentials"
name = "AWS Credentials"
description = "Forward AWS credentials to VM"

[host_setup]
script = """
#!/bin/bash
# Check if AWS credentials exist
if [ ! -f ~/.aws/credentials ]; then
  echo "Error: AWS credentials not found at ~/.aws/credentials"
  exit 1
fi

# Copy credentials to VM
limactl copy "$LIMA_INSTANCE" ~/.aws/credentials /tmp/aws-credentials
"""

[vm_setup]
script = """
#!/bin/bash
mkdir -p ~/.aws
mv /tmp/aws-credentials ~/.aws/credentials
chmod 600 ~/.aws/credentials
"""
```

## Testing Capabilities

Test your capability by:

1. Setting up a clean template:
   ```bash
   claude-vm clean
   claude-vm setup --my-capability
   ```

2. Checking logs for errors during setup

3. Verifying the capability works:
   ```bash
   claude-vm shell
   # Test your installed software
   ```

4. Testing runtime hooks:
   ```bash
   claude-vm "echo test"
   # Check that runtime initialization worked
   ```
