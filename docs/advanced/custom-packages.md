# Custom Packages

Beyond the built-in tools, you can install custom system packages in your template VM. This guide covers package installation, version management, and custom repositories.

## Table of Contents

- [Basic Package Installation](#basic-package-installation)
- [Version Management](#version-management)
- [Custom Repositories](#custom-repositories)
- [Package Features](#package-features)
- [Examples](#examples)
- [Troubleshooting](#troubleshooting)

## Basic Package Installation

Install additional Ubuntu/Debian packages using the `[packages]` section:

```toml
[packages]
system = [
    "postgresql-client",
    "redis-tools",
    "jq",
    "htop",
    "curl"
]
```

Run `claude-vm setup` to install packages during template creation.

### How It Works

1. Packages are installed via `apt-get`
2. All packages install in a single batch operation
3. Installation happens during template creation
4. Packages are available in all cloned sessions

### Supported Package Names

- Simple names: `"jq"`, `"curl"`, `"vim"`
- With versions: `"python3=3.11.0-1"`
- With wildcards: `"nodejs=22.*"`
- With architecture: `"libc6:amd64"`

## Version Management

### Exact Version

Pin to a specific version:

```toml
[packages]
system = [
    "python3=3.11.0-1",
    "postgresql-client=14.5-1"
]
```

### Version Wildcards

Use wildcards for flexible versioning:

```toml
[packages]
system = [
    "nodejs=22.*",          # Any 22.x version
    "postgresql-client=14.*"  # Any 14.x version
]
```

### Latest Version

Omit version to get the latest:

```toml
[packages]
system = [
    "jq",      # Latest available
    "curl"     # Latest available
]
```

### Architecture-Specific

Specify architecture when needed:

```toml
[packages]
system = [
    "libc6:amd64",          # 64-bit version
    "libstdc++6:i386"       # 32-bit version
]
```

## Custom Repositories

For packages not in Debian repositories, add custom repos using a setup script:

```toml
[packages]
system = ["terraform", "kubectl"]
setup_script = """
#!/bin/bash
set -e

# Add HashiCorp repository
curl -fsSL https://apt.releases.hashicorp.com/gpg | \
  sudo gpg --dearmor -o /usr/share/keyrings/hashicorp-archive-keyring.gpg
echo "deb [signed-by=/usr/share/keyrings/hashicorp-archive-keyring.gpg] \
  https://apt.releases.hashicorp.com $(lsb_release -cs) main" | \
  sudo tee /etc/apt/sources.list.d/hashicorp.list

# Add Kubernetes repository
curl -fsSL https://pkgs.k8s.io/core:/stable:/v1.28/deb/Release.key | \
  sudo gpg --dearmor -o /etc/apt/keyrings/kubernetes-apt-keyring.gpg
echo "deb [signed-by=/etc/apt/keyrings/kubernetes-apt-keyring.gpg] \
  https://pkgs.k8s.io/core:/stable:/v1.28/deb/ /" | \
  sudo tee /etc/apt/sources.list.d/kubernetes.list
"""
```

### Setup Script Requirements

**Must be idempotent:**
```bash
# Good: Check if already added
if [ ! -f /etc/apt/sources.list.d/hashicorp.list ]; then
  # Add repository
fi

# Bad: Always tries to add (fails on reruns)
curl ... | sudo tee /etc/apt/sources.list.d/repo.list
```

**Must use `set -e`:**
```bash
#!/bin/bash
set -e  # Exit on any error
```

## Package Features

### Batch Installation

All packages install in one operation for efficiency:

```toml
[packages]
system = ["pkg1", "pkg2", "pkg3"]
# Runs: apt-get install -y pkg1 pkg2 pkg3
```

### Validation

Package names are validated to prevent injection:

```toml
[packages]
system = [
    "jq",               # ✓ Valid
    "postgresql-14",    # ✓ Valid
    "pkg; rm -rf /"     # ✗ Rejected (invalid characters)
]
```

### Automatic Dependency Resolution

Dependencies are automatically installed:

```toml
[packages]
system = ["postgresql-client"]
# Automatically installs: libpq5, postgresql-client-common, etc.
```

## Examples

### Database Tools

```toml
[packages]
system = [
    "postgresql-client",
    "mysql-client",
    "redis-tools",
    "mongodb-clients"
]
```

### Development Tools

```toml
[packages]
system = [
    "build-essential",
    "cmake",
    "pkg-config",
    "libssl-dev",
    "git-lfs"
]
```

### System Utilities

```toml
[packages]
system = [
    "htop",
    "ncdu",
    "jq",
    "curl",
    "wget",
    "vim",
    "tmux"
]
```

### Data Processing

```toml
[packages]
system = [
    "ffmpeg",
    "imagemagick",
    "pandoc",
    "graphviz"
]
```

### Cloud Tools

```toml
[packages]
system = ["awscli"]
setup_script = """
#!/bin/bash
set -e

# Install Azure CLI (installer handles repository setup)
curl -sL https://aka.ms/InstallAzureCLIDeb | sudo bash
"""
```

### HashiCorp Tools

```toml
[packages]
system = ["terraform", "packer", "vault"]
setup_script = """
#!/bin/bash
set -e

if [ ! -f /etc/apt/sources.list.d/hashicorp.list ]; then
  curl -fsSL https://apt.releases.hashicorp.com/gpg | \
    sudo gpg --dearmor -o /usr/share/keyrings/hashicorp-archive-keyring.gpg
  echo "deb [signed-by=/usr/share/keyrings/hashicorp-archive-keyring.gpg] \
    https://apt.releases.hashicorp.com $(lsb_release -cs) main" | \
    sudo tee /etc/apt/sources.list.d/hashicorp.list
fi
"""
```

### Language-Specific Packages

```toml
[packages]
system = [
    "ruby-full",
    "golang-go",
    "openjdk-17-jdk",
    "php-cli"
]
```

## Package Discovery

### Finding Package Names

**Search Ubuntu packages:**
```bash
# On host or in VM
apt-cache search <keyword>
apt-cache show <package-name>
```

**Check package versions:**
```bash
apt-cache policy <package-name>
```

**List files in package:**
```bash
dpkg -L <package-name>
```

### Common Package Names

| Tool | Package Name |
|------|--------------|
| PostgreSQL client | `postgresql-client` |
| MySQL client | `mysql-client` |
| Redis CLI | `redis-tools` |
| MongoDB tools | `mongodb-clients` |
| JSON processor | `jq` |
| YAML processor | `yq` |
| HTTP tool | `curl`, `wget`, `httpie` |
| Process viewer | `htop`, `glances` |
| Disk usage | `ncdu` |
| Text editor | `vim`, `nano`, `emacs` |
| Build tools | `build-essential`, `cmake` |

## Troubleshooting

### Package Not Found

```
Error: Package 'xyz' not found
```

**Solutions:**

1. **Check package name:**
   ```bash
   apt-cache search xyz
   ```

2. **Add repository:**
   ```toml
   [packages]
   system = ["xyz"]
   setup_script = """
   # Add the repository providing 'xyz'
   """
   ```

3. **Install from source in setup script:**
   ```bash
   # .claude-vm.setup.sh
   curl -L https://example.com/xyz.tar.gz | tar xz
   cd xyz && make install
   ```

### Version Not Available

```
Error: Version '1.2.3-4' not found for package 'xyz'
```

**Solutions:**

1. **Check available versions:**
   ```bash
   apt-cache policy xyz
   ```

2. **Use wildcard:**
   ```toml
   system = ["xyz=1.2.*"]  # Any 1.2.x version
   ```

3. **Use latest:**
   ```toml
   system = ["xyz"]  # No version constraint
   ```

### Repository Key Errors

```
Error: GPG error: repository is not signed
```

**Solution:** Add repository key in setup script:

```toml
setup_script = """
#!/bin/bash
curl -fsSL https://example.com/key.gpg | \
  sudo gpg --dearmor -o /usr/share/keyrings/example.gpg
"""
```

### Dependency Conflicts

```
Error: Unmet dependencies
```

**Solution:** Let apt resolve dependencies automatically or specify all required packages:

```toml
[packages]
system = [
    "package-a",
    "package-b",  # Required by package-a
    "package-c"   # Also required
]
```

### Setup Script Fails

```
Error: Setup script exited with code 1
```

**Debug:**

```bash
# Run setup with verbose output
claude-vm --verbose setup --git

# Check script manually
bash -x .claude-vm.setup.sh
```

**Common causes:**
- Script not idempotent (fails on rerun)
- Missing `set -e`
- Network errors (use retries)
- Permission errors (use `sudo` where needed)

### Package Install Hangs

Packages requiring interaction will hang. Ensure non-interactive installation:

```bash
# In setup script
export DEBIAN_FRONTEND=noninteractive
sudo apt-get install -y <packages>
```

Claude VM automatically sets `DEBIAN_FRONTEND=noninteractive`.

## Best Practices

### 1. Group Related Packages

```toml
# Good: Logical grouping
[packages]
system = [
    # Database tools
    "postgresql-client",
    "redis-tools",
    # Utilities
    "jq",
    "curl"
]
```

### 2. Document Package Purpose

```toml
# .claude-vm.toml
[packages]
system = [
    "imagemagick",  # For image processing in scripts
    "pandoc",       # For markdown to PDF conversion
    "jq"            # For JSON parsing in CI
]
```

### 3. Pin Critical Versions

```toml
[packages]
system = [
    "nodejs=20.*",  # Pin major version
    "jq"            # Latest is fine
]
```

### 4. Test Package Installation

```bash
# Create template
claude-vm setup --git

# Verify packages
claude-vm shell which jq
claude-vm shell postgresql-client --version
```

### 5. Use Setup Scripts for Complex Repos

For repositories requiring multiple steps, use setup scripts instead of inline:

```bash
# .claude-vm.setup.sh
#!/bin/bash
set -e
source ./scripts/add-custom-repos.sh
```

## Next Steps

- **[Tools](../features/tools.md)** - Understand built-in tools
- **[Configuration](../configuration.md)** - Configure custom packages
- **[Templates](../features/templates.md)** - Understand template creation
- **[Troubleshooting](troubleshooting.md)** - Debug package issues
