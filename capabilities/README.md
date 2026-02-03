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

### Simple Capability (inline script)

```toml
[capability]
id = "git-lfs"
name = "Git LFS"
description = "Git Large File Storage support"

[vm_setup]
script = """
#!/bin/bash
set -e
sudo apt-get update
sudo apt-get install -y git-lfs
git lfs install
"""
```

### Complex Capability (with MCP)

```toml
[capability]
id = "postgres"
name = "PostgreSQL"
description = "PostgreSQL database server"
requires = ["docker"]

[vm_setup]
script = """
#!/bin/bash
set -e
# Pull PostgreSQL Docker image
docker pull postgres:16
"""

[vm_runtime]
script = """
#!/bin/bash
# Start PostgreSQL container if not running
if ! docker ps | grep -q postgres-dev; then
  docker run -d --name postgres-dev \
    -e POSTGRES_PASSWORD=dev \
    -p 5432:5432 \
    postgres:16
fi
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
