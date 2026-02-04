# Network Security Feature - Human Test Strategy

**Version:** 1.0
**Date:** 2026-02-04
**Feature:** Network Security (HTTP/HTTPS filtering with domain policies)

## Table of Contents

1. [Test Environment Setup](#test-environment-setup)
2. [Test Categories](#test-categories)
3. [Detailed Test Cases](#detailed-test-cases)
4. [Expected Results Reference](#expected-results-reference)
5. [Known Limitations](#known-limitations)
6. [Reporting Issues](#reporting-issues)

---

## Test Environment Setup

### Prerequisites

- [ ] Lima installed and working
- [ ] `claude-vm` compiled and in PATH
- [ ] Clean test project directory
- [ ] Internet connection available
- [ ] Test domains accessible (github.com, npmjs.org, etc.)

### Setup Test Project

```bash
# Create fresh test project
mkdir -p /tmp/claude-vm-test-network
cd /tmp/claude-vm-test-network
git init

# Clean any existing VMs
claude-vm clean 2>/dev/null || true
```

---

## Test Categories

### Category 1: Installation and Setup ✓
- Basic installation
- CLI flags
- Configuration files
- Dependency installation

### Category 2: Configuration Validation ✓
- Valid configurations
- Invalid patterns
- Validation warnings
- Config precedence

### Category 3: Runtime Behavior ✓
- Proxy startup
- Domain filtering
- Wildcard matching
- Protocol blocking

### Category 4: CLI Commands ✓
- `network logs`
- `network status`
- Command options

### Category 5: Statistics ✓
- Counter accuracy
- Persistence
- Display

### Category 6: Error Handling ✓
- Graceful failures
- Helpful messages
- Recovery

### Category 7: Integration ✓
- With other capabilities
- Multiple projects
- VM lifecycle

---

## Detailed Test Cases

## Category 1: Installation and Setup

### Test 1.1: Enable via CLI flag

**Steps:**
```bash
cd /tmp/claude-vm-test-network
claude-vm setup --network-security
```

**Expected:**
- ✓ Setup completes without errors
- ✓ Installs mitmproxy
- ✓ Generates CA certificate
- ✓ Shows capability installation progress

**Verify:**
```bash
# Check VM was created
limactl list | grep claude-vm-test-network

# Check mitmproxy installed in VM
limactl shell claude-vm-test-network mitmproxy --version
```

**Pass Criteria:**
- mitmproxy version displayed (e.g., "Mitmproxy: 10.x.x")
- No error messages during setup

---

### Test 1.2: Enable via config file

**Steps:**
```bash
cd /tmp/claude-vm-test-network
cat > .claude-vm.toml << 'EOF'
[security.network]
enabled = true
mode = "denylist"
EOF

claude-vm setup
```

**Expected:**
- ✓ Setup reads config file
- ✓ Installs network security capability
- ✓ Shows network security in setup output

**Pass Criteria:**
- Same as Test 1.1

---

### Test 1.3: Enable with --all flag

**Steps:**
```bash
cd /tmp/claude-vm-test-network
claude-vm clean
claude-vm setup --all
```

**Expected:**
- ✓ Installs all capabilities including network security
- ✓ Shows installation progress for each capability

**Pass Criteria:**
- Network security capability installed
- All other capabilities also installed (docker, node, python, etc.)

---

## Category 2: Configuration Validation

### Test 2.1: Valid allowlist configuration

**Steps:**
```bash
cd /tmp/claude-vm-test-network
cat > .claude-vm.toml << 'EOF'
[security.network]
enabled = true
mode = "allowlist"
allowed_domains = ["api.github.com", "*.npmjs.org"]
EOF

# Trigger config loading (setup will validate)
claude-vm setup
```

**Expected:**
- ✓ No validation warnings
- ✓ Config accepted

**Pass Criteria:**
- Setup completes without config warnings
- No "⚠️ Network Security Configuration Warnings" message

---

### Test 2.2: Empty allowlist warning

**Steps:**
```bash
cat > .claude-vm.toml << 'EOF'
[security.network]
enabled = true
mode = "allowlist"
allowed_domains = []
EOF

claude-vm setup
```

**Expected:**
- ⚠️  Warning: "no domains are allowed. This will block ALL network access"
- ✓ Setup continues (warning, not error)

**Pass Criteria:**
- Warning message displayed
- Setup completes successfully

---

### Test 2.3: Invalid domain patterns

**Steps:**
```bash
cat > .claude-vm.toml << 'EOF'
[security.network]
enabled = true
mode = "allowlist"
allowed_domains = [
    "api.github.com",
    "example..com",           # Invalid: consecutive dots
    "*.bad.*.domain.com",     # Invalid: multiple wildcards
    "test/path.com"           # Invalid: slash not allowed
]
EOF

claude-vm setup
```

**Expected:**
- ⚠️  Warning for each invalid domain:
  - "example..com - domain cannot contain consecutive dots"
  - "*.bad.*.domain.com - only one wildcard (*) is allowed per domain"
  - "test/path.com - domain contains invalid characters"
- ✓ Setup continues

**Pass Criteria:**
- All 3 warnings displayed
- Specific error for each invalid pattern
- Setup completes

---

### Test 2.4: Conflicting domains

**Steps:**
```bash
cat > .claude-vm.toml << 'EOF'
[security.network]
enabled = true
mode = "denylist"
allowed_domains = ["api.github.com"]
blocked_domains = ["api.github.com", "evil.com"]
EOF

claude-vm setup
```

**Expected:**
- ⚠️  Warning: "api.github.com appears in both allowed_domains and blocked_domains"
- ℹ️  Note: "It will be treated as ALLOWED"

**Pass Criteria:**
- Conflict detected and reported
- Clarifies which takes precedence

---

### Test 2.5: Wildcard validation

**Steps:**
```bash
cat > .claude-vm.toml << 'EOF'
[security.network]
enabled = true
mode = "allowlist"
allowed_domains = [
    "*.github.com",        # Valid
    "example.*.com",       # Invalid: middle
    "example.*",           # Invalid: end
    "*example.com",        # Invalid: no dot
    "*."                   # Invalid: no domain
]
EOF

claude-vm setup
```

**Expected:**
- ⚠️  4 warnings for invalid wildcards
- ✓ No warning for valid `*.github.com`

**Pass Criteria:**
- Each invalid pattern gets specific error message
- Valid pattern accepted

---

## Category 3: Runtime Behavior

### Test 3.1: Proxy startup with enhanced output

**Steps:**
```bash
cd /tmp/claude-vm-test-network
cat > .claude-vm.toml << 'EOF'
[security.network]
enabled = true
mode = "allowlist"
allowed_domains = ["api.github.com", "*.npmjs.org"]
block_tcp_udp = true
block_private_networks = true
block_metadata_services = true
EOF

claude-vm shell
```

**Expected Output:**
```
Enforcing network security policies...
  Starting HTTP/HTTPS filtering proxy...
  ✓ Proxy started (PID: XXXXX) - Listening on localhost:8080

  Policy Configuration:
    Mode: allowlist
    Allowed: api.github.com, *.npmjs.org (2 patterns)
    Blocked: none
    Bypass: none

  Protocol Blocks:
    ✓ Raw TCP/UDP blocked (IPv4 and IPv6)
    ✓ Private networks blocked (10.0.0.0/8, 192.168.0.0/16, 172.16.0.0/12)
    ✓ Cloud metadata blocked (169.254.169.254)

✓ Network security active - Use 'claude-vm network logs' to monitor requests
```

**Pass Criteria:**
- All sections displayed
- PID shown
- Domain counts correct
- Protocol blocks listed
- Helpful hint at end

---

### Test 3.2: Allowlist mode - Block non-allowed domains

**Steps:**
```bash
# In VM shell (from Test 3.1)
curl -v https://api.github.com  # Should work
curl -v https://example.com     # Should be blocked
```

**Expected:**
- ✓ api.github.com: Success (200 OK)
- ✗ example.com: 403 Forbidden, "Domain blocked by allowlist policy"

**Pass Criteria:**
- Allowed domain accessible
- Non-allowed domain blocked with clear message

---

### Test 3.3: Wildcard matching

**Steps:**
```bash
# In VM shell
curl -v https://registry.npmjs.org    # Matches *.npmjs.org
curl -v https://api.npmjs.org         # Matches *.npmjs.org
curl -v https://npmjs.org             # Matches *.npmjs.org (domain itself)
curl -v https://notnpmjs.org          # Should be blocked
```

**Expected:**
- ✓ registry.npmjs.org: Success
- ✓ api.npmjs.org: Success
- ✓ npmjs.org: Success (implementation allows base domain)
- ✗ notnpmjs.org: Blocked

**Pass Criteria:**
- All subdomains match wildcard
- Base domain also matches
- Non-matching domain blocked

---

### Test 3.4: Denylist mode - Allow all except blocked

**Steps:**
```bash
# Exit VM and reconfigure
cd /tmp/claude-vm-test-network
cat > .claude-vm.toml << 'EOF'
[security.network]
enabled = true
mode = "denylist"
blocked_domains = ["evil.com", "*.tracking.com"]
EOF

claude-vm clean && claude-vm setup
claude-vm shell
```

**In VM:**
```bash
curl -v https://api.github.com      # Should work
curl -v https://example.com         # Should work
curl -v https://evil.com            # Should be blocked
curl -v https://api.tracking.com    # Should be blocked
```

**Expected:**
- ✓ Most domains work
- ✗ evil.com: Blocked
- ✗ api.tracking.com: Blocked (wildcard match)

**Pass Criteria:**
- Open internet access by default
- Specific blocks enforced
- Wildcard blocks work

---

### Test 3.5: Bypass domains (HTTPS inspection skip)

**Steps:**
```bash
cat > .claude-vm.toml << 'EOF'
[security.network]
enabled = true
mode = "allowlist"
allowed_domains = ["api.github.com"]
bypass_domains = ["localhost", "127.0.0.1"]
EOF

claude-vm clean && claude-vm setup
claude-vm shell
```

**In VM:**
```bash
curl -v http://localhost:8080       # Should work (bypass)
curl -v https://api.github.com      # Should work (allowed)
curl -v https://example.com         # Should be blocked
```

**Expected:**
- ✓ localhost: Bypassed (no inspection)
- ✓ api.github.com: Allowed
- ✗ example.com: Blocked

**Pass Criteria:**
- Bypass domains skip filtering
- Allowed domains still filtered
- Blocked domains still blocked

---

### Test 3.6: Raw TCP blocking

**Steps:**
```bash
# In VM with network security enabled
ssh github.com         # Should fail (TCP port 22)
nc -v github.com 22    # Should fail
curl https://github.com # Should work (HTTP/HTTPS allowed through proxy)
```

**Expected:**
- ✗ SSH: Connection rejected or timeout
- ✗ Netcat: Connection rejected
- ✓ curl: Works (routed through proxy)

**Pass Criteria:**
- Raw TCP connections blocked
- HTTP/HTTPS works through proxy

---

### Test 3.7: Private network blocking

**Steps:**
```bash
# In VM
curl http://192.168.1.1      # Private IP
curl http://10.0.0.1         # Private IP
curl http://172.16.0.1       # Private IP
ping 192.168.1.1             # ICMP (should also be blocked)
```

**Expected:**
- ✗ All private IPs: Connection rejected
- ✗ Ping: No response

**Pass Criteria:**
- All RFC1918 private networks blocked
- Both IPv4 and IPv6 private ranges blocked

---

### Test 3.8: Cloud metadata blocking

**Steps:**
```bash
# In VM
curl http://169.254.169.254/           # AWS/GCP metadata
curl http://169.254.169.254/latest/    # Should be blocked
```

**Expected:**
- ✗ Connection rejected or timeout

**Pass Criteria:**
- Metadata service IP blocked
- Both HTTP and direct connection blocked

---

### Test 3.9: IPv6 blocking

**Steps:**
```bash
# In VM
curl -6 https://ipv6.google.com        # If allowed by policy
nc -6 -v google.com 22                 # Raw TCP over IPv6
ping6 google.com                       # ICMPv6
```

**Expected:**
- ✓ curl: Works if domain in policy (through proxy)
- ✗ nc: Raw TCP rejected
- ✗ ping6: No response

**Pass Criteria:**
- IPv6 HTTP/HTTPS routed through proxy
- Raw IPv6 TCP/UDP blocked
- Same rules as IPv4

---

## Category 4: CLI Commands

### Test 4.1: Network logs - Basic usage

**Steps:**
```bash
# With VM running and some requests made
claude-vm network logs
```

**Expected Output:**
```
Network Security Logs
═════════════════════════════════════════════════════════════
Showing last 50 lines
═════════════════════════════════════════════════════════════

[timestamp] Request to api.github.com - ALLOWED
[timestamp] Request to example.com - BLOCKED
...

═════════════════════════════════════════════════════════════
Options:
  --all          Show all logs (no line limit)
  -n <lines>     Show last N lines (default: 50)
  -f <pattern>   Filter logs by domain pattern

Log file: /tmp/mitmproxy.log (inside VM)
```

**Pass Criteria:**
- Clean formatting
- Shows recent requests
- Helpful options footer

---

### Test 4.2: Network logs - Filter by domain

**Steps:**
```bash
claude-vm network logs -f github
claude-vm network logs -f blocked
claude-vm network logs -f ALLOWED
```

**Expected:**
- ✓ Shows only matching lines
- ✓ Case-insensitive search
- ✓ Works with patterns

**Pass Criteria:**
- Filter works correctly
- Both domain names and status keywords work

---

### Test 4.3: Network logs - Show all

**Steps:**
```bash
claude-vm network logs --all
```

**Expected:**
- ✓ Shows complete log file
- ✓ No line limit applied

**Pass Criteria:**
- All historical logs displayed
- No truncation

---

### Test 4.4: Network logs - Custom line count

**Steps:**
```bash
claude-vm network logs -n 10
claude-vm network logs -n 100
```

**Expected:**
- ✓ Shows exactly N lines
- ✓ Header reflects line count

**Pass Criteria:**
- Correct number of lines shown

---

### Test 4.5: Network logs - VM not running

**Steps:**
```bash
# Stop VM
limactl stop claude-vm-test-network
claude-vm network logs
```

**Expected:**
```
Error: VM is not running (status: Stopped)
Start the VM first with: claude-vm shell
```

**Pass Criteria:**
- Helpful error message
- Clear instructions

---

### Test 4.6: Network logs - Security not enabled

**Steps:**
```bash
# Create new project without network security
cd /tmp/claude-vm-test-no-security
git init
claude-vm setup --node  # Without network security
claude-vm shell
exit
claude-vm network logs
```

**Expected:**
```
Network security logs not found.

Network security may not be enabled for this VM.
To enable network security:
  1. Add to .claude-vm.toml:
     [security.network]
     enabled = true
  2. Recreate the VM: claude-vm clean && claude-vm setup
```

**Pass Criteria:**
- Detects feature not enabled
- Provides setup instructions

---

### Test 4.7: Network status - Active proxy

**Steps:**
```bash
cd /tmp/claude-vm-test-network
claude-vm shell  # Start VM and proxy
# In another terminal:
claude-vm network status
```

**Expected Output:**
```
Network Security Status
═══════════════════════════════════════════════

Status: ACTIVE ✓

Proxy Process:
  PID: XXXXX
  Listening: localhost:8080
  Uptime: XX:XX

Policy Configuration:
  Mode: allowlist
  Allowed domains: 2 patterns
  Blocked domains: 0 patterns
  Bypass domains: 0 patterns

Protocol Blocks:
  Raw TCP/UDP: enabled
  Private networks: enabled
  Cloud metadata: enabled

Statistics:
  Requests seen: XX
  Requests allowed: XX
  Requests blocked: XX

View logs: claude-vm network logs
```

**Pass Criteria:**
- Status shows ACTIVE
- PID displayed and valid
- Configuration matches .claude-vm.toml
- Statistics shown (if requests made)

---

### Test 4.8: Network status - Disabled

**Steps:**
```bash
cd /tmp/claude-vm-test-no-security
claude-vm network status
```

**Expected:**
```
Status: DISABLED

Network security is not enabled for this project.

To enable network security:
  1. Add to .claude-vm.toml:
     [security.network]
     enabled = true
  2. Recreate the VM:
     claude-vm clean && claude-vm setup

Or use the CLI shortcut:
  claude-vm setup --network-security
```

**Pass Criteria:**
- Clear disabled status
- Helpful instructions
- Multiple enabling options shown

---

### Test 4.9: Network status - VM not running

**Steps:**
```bash
cd /tmp/claude-vm-test-network
limactl stop claude-vm-test-network
claude-vm network status
```

**Expected:**
```
Status: INACTIVE (VM not running)

VM Status: Stopped

Start the VM first with:
  claude-vm shell
```

**Pass Criteria:**
- Detects VM not running
- Shows VM status
- Clear instructions

---

### Test 4.10: Network status - Proxy not started

**Steps:**
```bash
# Start VM but exit before proxy starts
# (This is tricky - proxy starts automatically)
# Alternative: kill proxy manually in VM
cd /tmp/claude-vm-test-network
claude-vm shell
# In VM: kill $(cat /tmp/mitmproxy.pid)
exit

# In another terminal:
claude-vm network status
```

**Expected:**
```
Status: INACTIVE (Proxy not started)

The network security proxy has not been started yet.
It will start automatically when you run:
  claude-vm        # Run Claude
  claude-vm shell  # Open shell
```

**Pass Criteria:**
- Detects proxy not running
- Helpful explanation

---

## Category 5: Statistics

### Test 5.1: Counter accuracy

**Steps:**
```bash
cd /tmp/claude-vm-test-network
claude-vm shell

# Make known number of requests
curl https://api.github.com    # 1 allowed
curl https://example.com       # 1 blocked
curl https://api.github.com    # 2 allowed
curl https://example.com       # 2 blocked
exit

claude-vm network status
```

**Expected:**
```
Statistics:
  Requests seen: 4
  Requests allowed: 2
  Requests blocked: 2
```

**Pass Criteria:**
- Counters exactly match requests made
- Total = allowed + blocked

---

### Test 5.2: Statistics persistence across sessions

**Steps:**
```bash
# Session 1
claude-vm shell
curl https://api.github.com    # 1 request
exit

# Check stats
claude-vm network status  # Should show 1 request

# Session 2
claude-vm shell
curl https://api.github.com    # 1 more request
exit

# Check stats again
claude-vm network status  # Should show 2 total requests
```

**Expected:**
- Stats persist across multiple sessions
- Counters accumulate

**Pass Criteria:**
- Second status shows cumulative count
- Stats not reset between sessions

---

### Test 5.3: Statistics reset on VM restart

**Steps:**
```bash
claude-vm network status  # Note current counts
limactl stop claude-vm-test-network
limactl start claude-vm-test-network
claude-vm shell
exit
claude-vm network status  # Should show 0 or low counts
```

**Expected:**
- Stats reset to 0 when VM restarts
- New session starts fresh counting

**Pass Criteria:**
- Stats file recreated
- Counts start from 0

---

### Test 5.4: Statistics display without requests

**Steps:**
```bash
# Fresh VM, no requests made
claude-vm clean && claude-vm setup --network-security
claude-vm shell
exit
claude-vm network status
```

**Expected:**
```
Statistics:
  Requests seen: 0
  Requests allowed: 0
  Requests blocked: 0
```

**Pass Criteria:**
- Shows zeros, not missing
- No error about missing stats

---

## Category 6: Error Handling

### Test 6.1: Proxy startup failure

**Steps:**
```bash
# In VM, manually kill proxy and prevent restart
claude-vm shell
kill $(cat /tmp/mitmproxy.pid)
# Try to make request
curl https://api.github.com
```

**Expected:**
- Request may fail or succeed (depends on timing)
- Log shows proxy not running

**Pass Criteria:**
- System handles proxy death gracefully
- No crashes or hangs

---

### Test 6.2: Invalid config recovery

**Steps:**
```bash
cat > .claude-vm.toml << 'EOF'
[security.network]
enabled = true
mode = "invalid-mode"  # Invalid value
EOF

claude-vm setup
```

**Expected:**
- Error during config parsing
- Clear error message about invalid mode

**Pass Criteria:**
- Error caught during setup
- Helpful message about valid options

---

### Test 6.3: Logs command with empty logs

**Steps:**
```bash
# Fresh VM, no requests
claude-vm clean && claude-vm setup --network-security
claude-vm shell
exit
claude-vm network logs
```

**Expected:**
```
No logs available yet.

Network security is enabled but no requests have been logged.
The proxy may still be starting up, or no network requests have been made.
```

**Pass Criteria:**
- Helpful message
- Explains why empty

---

### Test 6.4: Large domain lists

**Steps:**
```bash
# Create config with 100+ domains
cat > .claude-vm.toml << 'EOF'
[security.network]
enabled = true
mode = "allowlist"
allowed_domains = [
    "domain1.com", "domain2.com", ... (100 domains)
]
EOF

claude-vm setup
```

**Expected:**
- Setup completes successfully
- Runtime output may truncate list (OK)

**Pass Criteria:**
- No performance issues
- Filtering works correctly

---

### Test 6.5: Network failure during request

**Steps:**
```bash
claude-vm shell
# Disable network adapter in VM or disconnect internet
curl https://api.github.com
```

**Expected:**
- Standard curl error (connection failed)
- No proxy crashes

**Pass Criteria:**
- Graceful handling of network errors
- Proxy continues running

---

## Category 7: Integration

### Test 7.1: Network security + GPG

**Steps:**
```bash
claude-vm setup --network-security --gpg
claude-vm shell
# Verify both work
gpg --version
echo $HTTP_PROXY
```

**Expected:**
- Both capabilities installed
- GPG works
- Proxy configured

**Pass Criteria:**
- No conflicts
- Both features functional

---

### Test 7.2: Network security + Docker

**Steps:**
```bash
claude-vm setup --network-security --docker
claude-vm shell
# Test docker with network
docker run --rm alpine wget https://api.github.com
```

**Expected:**
- Docker works
- Network requests filtered
- May need proxy config in Docker

**Pass Criteria:**
- No capability conflicts

---

### Test 7.3: Multiple projects with different configs

**Steps:**
```bash
# Project 1: Allowlist
cd /tmp/test-project-1
git init
cat > .claude-vm.toml << 'EOF'
[security.network]
enabled = true
mode = "allowlist"
allowed_domains = ["api.github.com"]
EOF
claude-vm setup

# Project 2: Denylist
cd /tmp/test-project-2
git init
cat > .claude-vm.toml << 'EOF'
[security.network]
enabled = true
mode = "denylist"
blocked_domains = ["evil.com"]
EOF
claude-vm setup

# Verify each has own VM and config
cd /tmp/test-project-1
claude-vm shell
# Test allowlist behavior
exit

cd /tmp/test-project-2
claude-vm shell
# Test denylist behavior
exit
```

**Expected:**
- Each project has independent VM
- Different policies enforced
- No cross-contamination

**Pass Criteria:**
- VMs isolated
- Configs independent

---

### Test 7.4: Config precedence (Global vs Project)

**Steps:**
```bash
# Set global config
cat > ~/.claude-vm.toml << 'EOF'
[security.network]
enabled = true
mode = "denylist"
blocked_domains = ["global-blocked.com"]
EOF

# Set project config
cd /tmp/test-precedence
git init
cat > .claude-vm.toml << 'EOF'
[security.network]
mode = "allowlist"
allowed_domains = ["project-allowed.com"]
EOF

claude-vm setup
claude-vm shell
# Test which config wins
curl https://project-allowed.com  # Should work
curl https://global-blocked.com   # Should work (allowlist mode)
exit
```

**Expected:**
- Project config takes precedence
- Mode from project (allowlist)
- Global domains may be merged

**Pass Criteria:**
- Precedence order correct
- No conflicts

---

## Expected Results Reference

### Valid Domain Patterns

✓ `example.com` - Exact match
✓ `api.example.com` - Exact subdomain
✓ `*.example.com` - Wildcard prefix
✓ `*.api.example.com` - Wildcard on subdomain

### Invalid Domain Patterns

✗ `example..com` - Consecutive dots
✗ `*.*.example.com` - Multiple wildcards
✗ `example.*.com` - Wildcard in middle
✗ `example.*` - Wildcard at end
✗ `*example.com` - Wildcard without dot
✗ `example.com/path` - Path not allowed
✗ `example.com:443` - Port not allowed

### Policy Modes

**Allowlist:** Block all except `allowed_domains`
**Denylist:** Allow all except `blocked_domains`

### Protocol Blocks

- **Raw TCP/UDP:** Blocks SSH, raw socket connections, custom protocols
- **Private networks:** Blocks 10.0.0.0/8, 192.168.0.0/16, 172.16.0.0/12, IPv6 ULA/link-local
- **Cloud metadata:** Blocks 169.254.169.254 (AWS/GCP metadata service)

---

## Known Limitations

1. **Not Security Isolation:** Can be bypassed by malicious code with sudo access. See `capabilities/network-security/SECURITY.md` for details.

2. **Stats Reset on VM Restart:** Statistics don't persist across VM stop/start (by design).

3. **No Path-Based Filtering:** Only domain-level filtering, not URL paths.

4. **No Port-Based Filtering:** Cannot filter by port number (all HTTP/HTTPS ports treated same).

5. **HTTPS Inspection:** All HTTPS traffic goes through man-in-the-middle proxy (uses VM-generated CA cert).

6. **Performance:** Minimal overhead but every request goes through proxy (~1-5ms added latency).

---

## Reporting Issues

When reporting issues, include:

1. **Test case number** (e.g., "Test 3.2 failed")
2. **Steps to reproduce**
3. **Expected vs actual behavior**
4. **Configuration used** (`.claude-vm.toml` contents)
5. **Error messages** (complete output)
6. **Environment:**
   - Lima version: `limactl --version`
   - Claude-vm version: `claude-vm version`
   - OS: `uname -a`
7. **Logs:**
   - `claude-vm network logs`
   - `limactl shell <instance> cat /tmp/mitmproxy.log`

### Issue Template

```markdown
## Test Case
Test X.Y: [Test Name]

## Steps
1.
2.
3.

## Expected
-

## Actual
-

## Configuration
```toml
[Paste .claude-vm.toml]
```

## Environment
- Lima:
- Claude-vm:
- OS:

## Logs
```
[Paste relevant logs]
```
```

---

## Test Execution Checklist

### Pre-Test
- [ ] Clean test environment
- [ ] No existing VMs for test projects
- [ ] Internet connection verified
- [ ] All dependencies installed

### During Test
- [ ] Follow steps exactly as written
- [ ] Record actual output
- [ ] Note any deviations
- [ ] Capture screenshots if helpful

### Post-Test
- [ ] Mark tests as Pass/Fail
- [ ] Document issues found
- [ ] Clean up test VMs
- [ ] Restore original configs

### Cleanup
```bash
# Remove test projects
rm -rf /tmp/claude-vm-test-*
rm -rf /tmp/test-project-*
rm -rf /tmp/test-precedence

# Clean test VMs
limactl list | grep -E 'claude-vm-test|test-project|test-precedence' | awk '{print $1}' | xargs -I {} limactl delete {}

# Restore global config (if modified)
rm ~/.claude-vm.toml  # or restore backup
```

---

## Success Criteria

**Feature is ready for release if:**

✓ All Category 1-4 tests pass (critical path)
✓ 90%+ of Category 5-7 tests pass
✓ All validation warnings working correctly
✓ No crashes or hangs in normal usage
✓ Error messages are clear and helpful
✓ Documentation matches behavior

**Known issues documented in:**
- README.md limitations section
- capabilities/network-security/SECURITY.md

---

**End of Test Plan**
