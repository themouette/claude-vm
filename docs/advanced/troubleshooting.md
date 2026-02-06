# Troubleshooting

This guide covers common issues and their solutions.

## Table of Contents

- [Installation Issues](#installation-issues)
- [Template Issues](#template-issues)
- [Runtime Issues](#runtime-issues)
- [Configuration Issues](#configuration-issues)
- [Debugging Tools](#debugging-tools)

## Installation Issues

### Lima Not Found

**Symptom:**

```
Error: lima not found. Please install Lima first.
```

**Solution:**

macOS:

```bash
brew install lima
```

Linux:

```bash
# See https://lima-vm.io/docs/installation/
```

Verify:

```bash
limactl --version
```

### Permission Denied on Binary

**Symptom:**

```
bash: ./claude-vm: Permission denied
```

**Solution:**

```bash
chmod +x ~/.local/bin/claude-vm
```

### Binary Not in PATH

**Symptom:**

```
command not found: claude-vm
```

**Solution:**

Add `~/.local/bin` to PATH:

```bash
# For bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# For zsh
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

Verify:

```bash
which claude-vm
```

## Template Issues

### Template Creation Fails

**Symptom:**

```
Error: Failed to create template
```

**Debug:**

1. Check Lima status:

   ```bash
   limactl list
   ```

2. Try with verbose output:

   ```bash
   claude-vm --verbose setup --git
   ```

3. Check Lima logs:

   ```bash
   tail -f ~/.lima/*/ha.stdout.log
   ```

4. Clean and retry:
   ```bash
   claude-vm clean
   claude-vm setup --git
   ```

### Template Disk Full

**Symptom:**

```
Error: No space left on device
```

**Solution:**

1. Increase disk size:

   ```bash
   claude-vm clean
   claude-vm setup --disk 50 --git
   ```

2. Clean unused templates:

   ```bash
   claude-vm list --unused
   claude-vm clean-all
   ```

3. Check disk usage:
   ```bash
   claude-vm list --disk-usage
   ```

### Tool Installation Fails

**Symptom:**

```
Error: Failed to install docker
```

**Debug:**

1. Check for package conflicts:

   ```bash
   claude-vm shell
   $ apt-cache policy docker
   ```

2. Try with verbose output:

   ```bash
   claude-vm --verbose setup --docker
   ```

3. Try minimal setup first:

   ```bash
   claude-vm clean
   claude-vm setup --git  # Minimal
   # If works, add tools incrementally
   ```

## Runtime Issues

### VM Won't Start

**Symptom:**

```
Error: Failed to start VM
```

**Debug:**

1. Try with verbose output:

   ```bash
   claude-vm --verbose shell
   ```

2. Check Lima VMs:

   ```bash
   limactl list
   ```

3. Check if template exists:

   ```bash
   claude-vm info
   ```

4. Stop any running VMs:
   ```bash
   limactl stop --all
   ```

### Runtime Script Fails

**Symptom:**

```
Error: Runtime script failed: .claude-vm.runtime.sh
Exit code: 1
```

**Debug:**

1. Check script locally:

   ```bash
   bash -x .claude-vm.runtime.sh
   ```

2. Check script in VM:

   ```bash
   claude-vm shell
   $ cat .claude-vm.runtime.sh
   $ bash -x .claude-vm.runtime.sh
   ```

3. Comment out failing parts:
   ```bash
   # .claude-vm.runtime.sh
   # docker-compose up -d  # Temporarily disable
   echo "Skipping docker for now"
   ```

### Command Not Found in VM

**Symptom:**

```bash
$ claude-vm shell docker ps
Error: docker: command not found
```

**Solution:**

1. Check if tool is installed:

   ```bash
   claude-vm info  # Check enabled capabilities
   ```

2. Reinstall template with tool:

   ```bash
   claude-vm clean
   claude-vm setup --docker --git
   ```

3. Check if tool is in PATH:
   ```bash
   claude-vm shell which docker
   ```

### Port Already in Use

**Symptom:**

```
Error: bind: address already in use
```

**Solution:**

1. Check what's using the port:

   ```bash
   # On host
   lsof -i :3000
   ```

2. Use different port:

   ```bash
   # In your app
   PORT=3001 npm start
   ```

3. Stop conflicting service:
   ```bash
   # Find and stop the process
   kill <PID>
   ```

### Mount Not Appearing

**Symptom:**

```bash
$ claude-vm shell ls /data
ls: /data: No such file or directory
```

**Debug:**

1. Check mount configuration:

   ```bash
   claude-vm config show
   ```

2. Verify host directory exists:

   ```bash
   ls -la ~/datasets
   ```

3. Check for typos in config:

   ```toml
   [[mounts]]
   location = "~/datasets"  # Check spelling
   mount_point = "/data"
   ```

4. Try CLI mount:
   ```bash
   claude-vm --mount ~/datasets:/data shell ls /data
   ```

## Configuration Issues

### Config Validation Fails

**Symptom:**

```
Error: Invalid configuration
```

**Debug:**

1. Run validation:

   ```bash
   claude-vm config validate
   ```

2. Check TOML syntax:

   ```toml
   # Common errors:
   [vm]
   disk = 30      # ✓ Number
   disk = "30"    # ✗ String

   [tools]
   docker = true  # ✓ Boolean
   docker = "yes" # ✗ String
   ```

3. Check value ranges:
   ```toml
   [vm]
   disk = 2000    # ✗ Out of range (max 1000)
   memory = 128   # ✗ Out of range (max 64)
   ```

### Config Not Taking Effect

**Symptom:**
Config changes don't apply.

**Debug:**

1. Check effective config:

   ```bash
   claude-vm config show
   ```

2. Check precedence:
   - CLI flags override everything
   - Then environment variables
   - Then project config
   - Then global config

3. Recreate template after config changes:
   ```bash
   claude-vm clean
   claude-vm setup --git
   ```

### Environment Variables Not Set

**Symptom:**

```bash
$ claude-vm shell echo $API_KEY
# Empty
```

**Solution:**

1. Pass via CLI:

   ```bash
   claude-vm --env API_KEY=secret shell echo $API_KEY
   ```

2. Use env file:

   ```bash
   claude-vm --env-file .env shell
   ```

3. Set in runtime script:
   ```bash
   # .claude-vm.runtime.sh
   export API_KEY="secret"
   ```

## Debugging Tools

### Verbose Mode

See detailed output:

```bash
claude-vm --verbose setup --git
claude-vm --verbose shell
claude-vm --verbose "help me"
```

Shows:

- Lima VM logs
- Script execution
- Mount operations
- Detailed errors

### Check VM Status

```bash
# List all VMs
limactl list

# Show VM details
limactl show <vm-name>

# Check VM logs
tail -f ~/.lima/<vm-name>/ha.stdout.log
```

### Access VM Directly

```bash
# Get template name
claude-vm info

# Start template VM
limactl start <template-name>

# Shell into VM
limactl shell <template-name>

# Stop VM
limactl stop <template-name>
```

### Check Configuration

```bash
# Validate config
claude-vm config validate

# Show effective config
claude-vm config show

# Show project info
claude-vm info
```

### Test Scripts

```bash
# Test runtime script locally
bash -x .claude-vm.runtime.sh

# Test in VM
claude-vm shell bash -x .claude-vm.runtime.sh
```

### Check Lima Health

```bash
# Lima version
limactl --version

# Lima status
limactl list

# Lima logs
tail -f ~/.lima/*/ha.stdout.log
```

## Common Error Messages

### "lima not found"

Install Lima:

```bash
brew install lima  # macOS
```

### "Permission denied"

Make binary executable:

```bash
chmod +x ~/.local/bin/claude-vm
```

### "Runtime script failed"

Debug script:

```bash
bash -x .claude-vm.runtime.sh
claude-vm --verbose shell
```

### "Package not found"

Check package name:

```bash
apt-cache search <package>
```

Add repository if needed:

```toml
[packages]
setup_script = """
# Add repository
"""
```

### "No space left on device"

Increase disk or clean templates:

```bash
claude-vm clean-all
claude-vm setup --disk 50 --git
```

## Getting More Help

### Check Logs

Lima logs:

```bash
tail -f ~/.lima/*/ha.stdout.log
```

VM logs:

```bash
claude-vm shell journalctl -f
```

### Report Issues

1. Run with `--verbose`:

   ```bash
   claude-vm --verbose setup --git > debug.log 2>&1
   ```

2. Include in bug report:
   - `claude-vm --version`
   - `limactl --version`
   - Operating system
   - Debug log
   - Configuration file

3. Open issue on GitHub:
   https://github.com/themouette/claude-vm/issues

## Next Steps

- **[Usage Guide](../usage.md)** - Learn all commands
- **[Configuration](../configuration.md)** - Configure Claude VM
- **[Templates](../features/templates.md)** - Understand templates
- **[Development](../development.md)** - Build from source
