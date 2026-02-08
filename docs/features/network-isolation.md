# Network Isolation

Network isolation provides HTTP/HTTPS filtering and protocol blocking for policy enforcement. It uses mitmproxy for transparent proxying and iptables for protocol-level controls.

## Table of Contents

- [Overview](#overview)
- [Security Model](#security-model)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Policy Modes](#policy-modes)
- [Domain Patterns](#domain-patterns)
- [Protocol Blocking](#protocol-blocking)
- [CLI Commands](#cli-commands)
- [Examples](#examples)
- [Troubleshooting](#troubleshooting)
- [Limitations](#limitations)

## Overview

Network isolation filters network traffic based on configurable policies:

- **HTTP/HTTPS filtering**: Domain-based allow/block lists
- **Protocol blocking**: Block raw TCP/UDP connections
- **Network isolation**: Block private networks and cloud metadata
- **Statistics and logging**: Track all network requests
- **Full IPv6 support**: iptables and ip6tables rules

**Use cases:**

- Corporate security policies
- Compliance requirements
- Preventing accidental data leaks
- API access restrictions
- Auditing network access

## Security Model

**⚠️ IMPORTANT**: Network isolation provides **policy enforcement**, not **security isolation**.

### What It Protects Against

✅ **Accidental data leaks**

- Claude accidentally calling wrong APIs
- Unintended connections to internal services
- Mistakes in generated code

✅ **Policy violations by well-behaved code**

- Enforcing allowed domain lists
- Restricting API access
- Preventing access to private networks

✅ **Compliance and auditing**

- Logging all HTTP/HTTPS traffic
- Recording domain access patterns
- Demonstrating security controls

### What It Does NOT Protect Against

❌ **Determined attackers or malicious code**

- Can bypass proxy settings
- Can exploit vulnerabilities
- Can use alternative protocols

❌ **Complete network isolation**

- Not a firewall replacement
- Not suitable for untrusted code
- Not security sandboxing

### Threat Model

Network isolation is designed for:

- **Preventing accidents**: Stop well-intentioned code from making mistakes
- **Policy enforcement**: Ensure compliance with organizational rules
- **Defense in depth**: Additional layer on top of VM isolation

It is NOT designed for:

- **Malware analysis**: Use dedicated sandboxes
- **Untrusted code execution**: Use proper isolation
- **Security research**: Requires stronger isolation

See `capabilities/network-isolation/SECURITY.md` for detailed security analysis.

## Quick Start

### Enable During Setup

```bash
claude-vm setup --network-isolation
```

### Enable via Configuration

Add to `.claude-vm.toml`:

```toml
[tools]
network_isolation = true

[security.network]
enabled = true
mode = "denylist"
blocked_domains = ["example.com"]
```

Then recreate the VM:

```bash
claude-vm clean
claude-vm setup
```

### Check Status

```bash
claude-vm network status
```

### View Logs

```bash
claude-vm network logs
claude-vm network logs -f "github"
```

### Test a Domain

```bash
claude-vm network test api.github.com
```

## Configuration

### Basic Configuration

```toml
[security.network]
enabled = true
mode = "denylist"
blocked_domains = ["bad.com", "*.ads.com"]
```

### Complete Configuration

```toml
[security.network]
# Enable network isolation filtering
enabled = true

# Policy mode: "allowlist" or "denylist"
mode = "denylist"

# Allowed domains (for allowlist mode or denylist exceptions)
allowed_domains = [
  "api.anthropic.com", # Allow Claude API
  "github.com",
  "*.api.com",        # Wildcard: matches api.api.com, test.api.com
]

# Blocked domains (for denylist mode)
blocked_domains = [
  "example.com",      # Exact match
  "*.ads.com",        # Block all ad subdomains
]

# Bypass domains (no TLS interception, for certificate pinning)
bypass_domains = [
  "*.internal.company.com",
]

# Protocol blocking (all default to true)
block_tcp_udp = true            # Block raw TCP/UDP connections
block_private_networks = true   # Block 10.0.0.0/8, 192.168.0.0/16, etc.
block_metadata_services = true  # Block 169.254.169.254 (cloud metadata)
```

### Configuration Precedence

```
CLI flags > Environment > Project config > Global config > Defaults
```

## Policy Modes

### Allowlist Mode

Block everything except explicitly allowed domains.

```toml
[security.network]
mode = "allowlist"
allowed_domains = [
  "api.anthropic.com", # Allow Claude API
  "github.com",
  "*.api.company.com",
]
```

**Use when:**

- You know exactly which APIs are needed
- Maximum security is required
- Compliance mandates explicit allow lists

**Behavior:**

- Blocks ALL domains by default
- Only allows domains in `allowed_domains`
- Bypass domains still work

### Denylist Mode (Default)

Allow everything except explicitly blocked domains.

```toml
[security.network]
mode = "denylist"
blocked_domains = [
  "malicious.com",
  "*.ads.com",
]
```

**Use when:**

- You want to block specific problematic domains
- Most APIs should work normally
- Flexibility is important

**Behavior:**

- Allows ALL domains by default
- Blocks only domains in `blocked_domains`
- Bypass domains still work

## Domain Patterns

### Exact Match

```toml
allowed_domains = ["example.com"]
```

Matches: `example.com`
Does NOT match: `api.example.com`, `example.org`

### Wildcard Match

```toml
allowed_domains = ["*.example.com"]
```

Matches: `api.example.com`, `test.example.com`, `example.com`
Does NOT match: `example.org`, `notexample.com`

**Rules:**

- Wildcard must be at the beginning: `*.domain.com`
- Only one wildcard per pattern
- Matches the domain itself and all subdomains

### Valid Characters

Domain patterns can contain:

- Letters (a-z, A-Z)
- Numbers (0-9)
- Hyphens (-)
- Dots (.)
- Underscores (\_) - for SRV records like `_service.example.com`
- Asterisk (\*) - only as wildcard prefix

### Bypass Domains

Bypass domains pass through the proxy without TLS interception:

```toml
bypass_domains = ["*.pinned.com"]
```

**Use for:**

- Certificate pinning (domains that reject MITM certificates)
- Internal services with custom CA
- Domains that detect proxy usage

**Note:** Bypass domains still go through the proxy (iptables requires it), but mitmproxy doesn't intercept TLS.

## Protocol Blocking

### TCP/UDP Blocking

```toml
block_tcp_udp = true  # Default
```

Blocks all raw TCP and UDP connections except:

- DNS (port 53)
- Localhost
- Established connections

**Effect:**

- Only HTTP/HTTPS work
- No raw socket connections
- No custom protocols

### Private Network Blocking

```toml
block_private_networks = true  # Default
```

Blocks connections to:

- 10.0.0.0/8
- 172.16.0.0/12
- 192.168.0.0/16
- fc00::/7 (IPv6 unique local)
- fe80::/10 (IPv6 link-local)

**Effect:**

- No access to internal networks
- No access to host machine
- No access to other VMs

### Metadata Service Blocking

```toml
block_metadata_services = true  # Default
```

Blocks connections to:

- 169.254.169.254 (AWS, Azure, GCP metadata)
- fe80::a9fe:a9fe (IPv6 equivalent)

**Effect:**

- Cannot access cloud instance metadata
- Cannot retrieve instance credentials
- Cannot access instance tags

### Disable Protocol Blocking

```toml
block_tcp_udp = false
block_private_networks = false
block_metadata_services = false
```

## CLI Commands

### Status Command

Show current network isolation status:

```bash
claude-vm network status
```

**Output:**

- Proxy status (running/stopped)
- Policy configuration
- Protocol blocks enabled
- Statistics (requests allowed/blocked)

**Multiple VMs:**

- Automatically detects all running ephemeral VMs for the project
- Prompts to select a VM if multiple are running
- Shows which VM the status is for

### Logs Command

View mitmproxy logs:

```bash
# Last 50 lines (default)
claude-vm network logs

# Last 100 lines
claude-vm network logs -n 100

# All logs
claude-vm network logs --all

# Filter by pattern
claude-vm network logs -f "github"
claude-vm network logs -f "blocked"
claude-vm network logs -f "403"

# Follow logs in real-time (like tail -f)
claude-vm network logs --follow
claude-vm network logs --follow -f "github"  # Follow with filter
```

**Multiple VMs:**

- Automatically detects all running ephemeral VMs for the project
- Prompts to select a VM if multiple are running
- Displays the VM name in the log header

### Test Command

Test if a domain would be allowed:

```bash
claude-vm network test example.com
claude-vm network test api.github.com
claude-vm network test *.internal.com
```

**Output:**

- ✓ ALLOWED or ✗ BLOCKED
- Explanation of why
- Matching patterns
- Suggestions to fix

## Examples

### Example 1: Allowlist for API Project

Only allow specific APIs:

```toml
[security.network]
enabled = true
mode = "allowlist"
allowed_domains = [
  "api.github.com",
  "*.openai.com",
  "api.stripe.com",
]
```

### Example 2: Block Ads and Trackers

Allow everything except ads:

```toml
[security.network]
enabled = true
mode = "denylist"
blocked_domains = [
  "*.doubleclick.net",
  "*.google-analytics.com",
  "*.facebook.com",
  "*.ads.com",
]
```

### Example 3: Internal Corp Security

Block external APIs, allow internal only:

```toml
[security.network]
enabled = true
mode = "allowlist"
allowed_domains = [
  "*.internal.company.com",
  "*.corp.company.com",
]
bypass_domains = [
  "*.internal.company.com",  # Uses certificate pinning
]
```

### Example 4: Compliance Logging

Allow everything but log all requests:

```toml
[security.network]
enabled = true
mode = "denylist"
blocked_domains = []  # Nothing blocked, just logging
```

View logs:

```bash
claude-vm network logs --all > network-audit.log
```

## Troubleshooting

### Domain Blocked Unexpectedly

Test the domain:

```bash
claude-vm network test api.github.com
```

Check status:

```bash
claude-vm network status
```

View recent blocks:

```bash
claude-vm network logs -f "blocked"
```

### Proxy Not Starting

Check logs:

```bash
limactl shell <vm-name> cat /tmp/mitmproxy.log
```

Common issues:

- Port 8080 already in use
- Certificate generation failed
- Mitmproxy installation corrupted

### Certificate Errors

Some domains reject mitmproxy certificates. Add to bypass:

```toml
bypass_domains = ["*.github.com"]
```

### Logs Too Large

Logs rotate automatically at 10MB. To check:

```bash
limactl shell <vm-name> ls -lh /tmp/mitmproxy.log*
```

### Emergency Disable

Temporarily disable without rebuilding VM:

```bash
claude-vm shell --env CLAUDE_VM_NETWORK_ISOLATION_DISABLE=true
```

Or with --inherit-env:

```bash
CLAUDE_VM_NETWORK_ISOLATION_DISABLE=true claude-vm shell --inherit-env
```

## Limitations

### Current Limitations

1. **Port 8080 Hard-coded**: Cannot change proxy port
2. **No Rate Limiting**: No per-domain rate limits
3. **No Path Filtering**: Cannot filter by URL path, only domain
4. **No Request Body Filtering**: Cannot inspect/filter request bodies
5. **Statistics Race Condition**: Concurrent requests may have slight stat inaccuracies
6. **Log Rotation**: Simple rotation (10MB limit), not full logrotate

### Future Improvements

Planned enhancements:

- Configurable proxy port
- Rate limiting per domain
- Path-based filtering
- Request size limits
- Better statistics dashboard
- Structured JSON logging
- Learning mode (auto-generate allowlists)

## Best Practices

### 1. Start with Denylist

Begin permissive, then tighten:

```toml
mode = "denylist"
blocked_domains = []  # Start with nothing blocked
```

Monitor logs, then add blocks as needed.

### 2. Test Domains Before Deploying

```bash
claude-vm network test api.example.com
```

Verify allowlist/denylist behavior before use.

### 3. Use Bypass for Certificate Pinning

If you see TLS errors:

```toml
bypass_domains = ["*.problematic.com"]
```

### 4. Monitor Logs Regularly

```bash
claude-vm network logs -f "blocked"
```

Find domains being unexpectedly blocked.

### 5. Use Wildcards Carefully

```toml
# Too permissive
allowed_domains = ["*.com"]

# Better
allowed_domains = ["*.api.company.com"]
```

### 6. Enable All Protocol Blocks

```toml
block_tcp_udp = true
block_private_networks = true
block_metadata_services = true
```

Maximum protection unless you have specific needs.

### 7. Document Your Policies

Comment your configuration:

```toml
[security.network]
mode = "allowlist"

# Production APIs
allowed_domains = [
  "api.prod.com",
  "*.aws.company.com",
]

# Third-party integrations
allowed_domains = [
  "api.stripe.com",    # Payment processing
  "api.sendgrid.com",  # Email delivery
]
```

## Next Steps

- **[Security Model](../capabilities/network-isolation/SECURITY.md)** - Detailed security analysis
- **[Configuration](../configuration.md)** - Full configuration reference
- **[Tools](tools.md)** - Other available capabilities
- **[Troubleshooting](../advanced/troubleshooting.md)** - Advanced debugging
