# Network Isolation Capability - Security Model

## ⚠️ Important: Security Guarantees and Limitations

This capability provides **policy enforcement**, not **security isolation**.

## What This Protects Against

✅ **Accidental data leaks**
- Claude accidentally accessing wrong APIs
- Unintended connections to internal services
- Mistakes in generated code that call unexpected endpoints

✅ **Policy violations by well-behaved code**
- Enforcing corporate security policies
- Restricting API access to approved domains
- Preventing access to private networks

✅ **Compliance and auditing**
- Logging all HTTP/HTTPS traffic
- Recording domain access patterns
- Demonstrating security controls

✅ **Defense-in-depth layering**
- Additional security layer on top of VM isolation
- Reduces attack surface
- Makes exploitation harder (but not impossible)

## What This Does NOT Protect Against

❌ **Determined attacker with VM access**

An attacker or malicious code running inside the VM can bypass all protections:

```bash
# Easy bypass methods:
sudo iptables -F                          # Flush firewall rules
kill $(cat /tmp/mitmproxy.pid)           # Kill the proxy
unset HTTP_PROXY HTTPS_PROXY             # Remove proxy config
sudo systemctl stop mitmproxy            # Stop proxy service
echo "pass" > /tmp/mitmproxy_filter.py   # Disable filtering
```

❌ **Actively malicious code**
- Code that deliberately attempts to bypass security
- Exploits that target the proxy or firewall
- Code that uses raw socket connections

❌ **Kernel or VM escape exploits**
- Exploits that break out of the VM
- Privilege escalation to host
- Lima or QEMU vulnerabilities

## Threat Model

### Assumptions

**What we assume:**
- Claude Code or generated code may accidentally violate policies
- Most code is well-behaved but needs guardrails
- Users want to prevent mistakes, not combat active attackers
- The VM itself provides the primary security boundary

**What we DO NOT assume:**
- Active attackers trying to bypass controls
- Malicious code with explicit exploit attempts
- Nation-state level threats

### Attack Surface

**If an attacker controls code in the VM:**
1. They have full user-level access (lima user)
2. They have sudo access (can become root)
3. They can modify any file in the VM
4. They can kill any process
5. They can change network configuration
6. **They can disable all in-VM security controls**

## Domain Matching Behavior

Understanding how domain patterns work is crucial for configuring effective policies.

### Exact Domain Matching

```toml
allowed_domains = ["api.github.com"]
```

- ✅ Matches: `api.github.com` only
- ❌ Does NOT match: `gist.github.com`, `github.com`, `api.github.com.evil.org`

### Wildcard Prefix Matching

```toml
allowed_domains = ["*.github.com"]
```

**What it matches:**
- ✅ `api.github.com` - single subdomain
- ✅ `gist.github.com` - different subdomain
- ✅ `raw.githubusercontent.github.com` - nested subdomains
- ✅ `github.com` - the domain itself (implementation detail)

**What it does NOT match:**
- ❌ `notgithub.com` - different domain
- ❌ `github.com.evil.org` - suffix match attempt
- ❌ `api.githubexample.com` - partial string match

### Wildcard Rules and Limitations

**Valid patterns:**
- `*.example.com` ✅ - prefix wildcard
- `*.api.example.com` ✅ - prefix wildcard on subdomain
- `example.com` ✅ - exact match

**Invalid patterns:**
- `example.*.com` ❌ - wildcard in middle
- `example.*` ❌ - wildcard at end
- `*.*.example.com` ❌ - multiple wildcards
- `*example.com` ❌ - wildcard without dot separator
- `*.` ❌ - wildcard without domain

**Validation:** Invalid patterns are automatically detected during configuration loading with helpful error messages.

### Practical Examples

**Allow all GitHub services:**
```toml
[security.network]
mode = "allowlist"
allowed_domains = [
    "*.github.com",      # api.github.com, gist.github.com, etc.
    "*.githubusercontent.com"  # raw.githubusercontent.com, etc.
]
```

**Block all analytics:**
```toml
[security.network]
mode = "denylist"
blocked_domains = [
    "*.google-analytics.com",
    "*.mixpanel.com",
    "*.segment.com"
]
```

**Allow specific API with subdomains:**
```toml
[security.network]
mode = "allowlist"
allowed_domains = [
    "api.example.com",     # Main API
    "*.api.example.com"    # Regional endpoints: us-west.api.example.com
]
```

### Common Mistakes

❌ **Trying to match all domains:**
```toml
allowed_domains = ["*"]  # Invalid - use denylist mode instead
```

❌ **Path-based filtering:**
```toml
allowed_domains = ["api.github.com/repos"]  # Invalid - domain only, no paths
```

❌ **Port-based filtering:**
```toml
allowed_domains = ["api.example.com:443"]  # Invalid - domain only, no ports
```

✅ **Correct approach:**
```toml
# For unrestricted access, use denylist mode with specific blocks
[security.network]
mode = "denylist"
blocked_domains = ["evil.com"]
```

### Testing Your Configuration

After enabling network isolation, test your domain patterns:

```bash
# View filtering logs
claude-vm network logs

# Filter by specific domain
claude-vm network logs -f "github.com"

# Check for blocked requests
claude-vm network logs -f "blocked"
```

The logs will show which requests are allowed or blocked, helping you refine your configuration.

## When to Use This Capability

### ✅ Good Use Cases

**Preventing accidents:**
```toml
# Ensure Claude only accesses approved APIs
[security.network]
mode = "allowlist"
allowed_domains = ["api.github.com", "api.stripe.com"]
```

**Corporate policy enforcement:**
```toml
# Block known problematic domains
[security.network]
mode = "denylist"
blocked_domains = ["pastebin.com", "transfer.sh", "*.torrent"]
```

**Compliance requirements:**
```toml
# Log all network activity, block private networks
[security.network]
enabled = true
block_private_networks = true
```

**Development safety:**
```toml
# Prevent Claude from accessing local services during testing
[security.network]
block_private_networks = true
block_metadata_services = true
```

### ❌ Not Suitable For

**Containing malicious code:**
- Use proper container sandboxing (Docker Sandboxes)
- Use cloud-based sandboxing services
- Use hardware-isolated environments

**Processing untrusted workloads:**
- Use VM-based sandboxing with no sudo
- Use kernel-level security (SELinux, AppArmor)
- Use commercial sandboxing solutions

**High-security environments:**
- Use air-gapped systems
- Use hardware security modules
- Use formally verified isolation

## Architecture

### Current Implementation: In-VM Proxy

```
┌─────────────────────────────────────┐
│ Lima VM (Claude has full control)  │
│                                     │
│  ┌─────────────────────────────┐   │
│  │ mitmproxy (localhost:8080)  │   │
│  │ ↑ Can be killed/bypassed    │   │
│  │ iptables rules              │   │
│  │ ↑ Can be flushed with sudo  │   │
│  │                             │   │
│  │ Claude Code                 │   │
│  │ - Has sudo access           │   │
│  │ - Can modify anything       │   │
│  └─────────────────────────────┘   │
│                                     │
└─────────────────────────────────────┘
```

**Security boundary:** VM isolation (Lima/QEMU)
**Secondary layer:** In-VM proxy (bypassable)

### Why This Design?

1. **Simplicity**: No complex host networking setup
2. **Isolation**: Each VM has independent proxy (no port conflicts)
3. **Lifecycle**: Proxy automatically managed with VM
4. **Practicality**: Good enough for preventing accidents

### Comparison to Docker Sandboxes

Docker Sandboxes achieves stronger isolation by:
- Running proxy on host (cannot be killed from container)
- Using container networking (enforced at kernel level)
- No sudo in container
- Network configured before container starts

**Trade-off:** Docker Sandboxes uses MicroVMs without sudo, which limits functionality. claude-vm uses full VMs with sudo for maximum flexibility.

## Defense in Depth

Even though this can be bypassed, it's still valuable as part of layered security:

```
┌─────────────────────────────────────┐
│ Layer 1: VM Isolation (Lima)       │ ← Primary security boundary
│  ├─ Separate filesystem            │
│  ├─ Isolated processes             │
│  └─ Limited host access            │
└─────────────────────────────────────┘
           ↓
┌─────────────────────────────────────┐
│ Layer 2: Network Proxy (This)      │ ← Policy enforcement
│  ├─ Domain filtering               │
│  ├─ Protocol blocking              │
│  └─ Audit logging                  │
└─────────────────────────────────────┘
           ↓
┌─────────────────────────────────────┐
│ Layer 3: Ephemeral VMs             │ ← Limit blast radius
│  ├─ Destroyed after use            │
│  ├─ No persistent state            │
│  └─ Fresh start each time          │
└─────────────────────────────────────┘
```

## Recommendations

### For Most Users

✅ **Use this capability** for:
- Preventing accidental policy violations
- Basic security hygiene
- Compliance/auditing requirements
- Defense-in-depth

✅ **Combine with:**
- Limited VM mounts (only project directory)
- Ephemeral VMs (destroy after use)
- Code review of generated code
- Monitoring and alerting

### For High-Security Needs

If you need true isolation:

1. **Use Docker Sandboxes** instead of claude-vm
   - True container isolation
   - Host-based proxy (cannot be bypassed)
   - No sudo in container

2. **Run claude-vm in restricted environment**
   - Separate VLAN with egress filtering
   - Network firewall at infrastructure level
   - Cloud security groups

3. **Use commercial solutions**
   - Cloud-based code execution sandboxes
   - Hardware-isolated environments
   - Formally verified systems

## Monitoring and Detection

While bypass is possible, you can detect it:

```bash
# Check if proxy is running
if ! pgrep -f mitmproxy > /dev/null; then
    echo "WARNING: Proxy not running!"
fi

# Check if iptables rules are intact
if ! sudo iptables -L OUTPUT | grep -q REJECT; then
    echo "WARNING: Firewall rules modified!"
fi

# Check proxy environment variables
if [ -z "$HTTP_PROXY" ]; then
    echo "WARNING: Proxy environment not set!"
fi
```

Consider implementing these checks in your workflows if bypass detection is important.

## Future Enhancements

Potential improvements for stronger security:

1. **Lima network configuration**
   - Configure networking at Lima level (harder to bypass)
   - Enforce routing before VM boots

2. **Host-based proxy option**
   - Optional mode with proxy on host
   - Requires more complex setup
   - Provides true isolation

3. **Read-only filter script**
   - Mount filter script as read-only
   - Requires VM restart to modify

4. **Monitoring and alerts**
   - Detect proxy kills
   - Alert on iptables modifications
   - Log suspicious activity

## Summary

**This capability is:**
- ✅ Useful for preventing accidents
- ✅ Good for policy enforcement
- ✅ Valuable for compliance
- ✅ Part of defense-in-depth
- ❌ NOT isolation from malicious code
- ❌ NOT suitable for untrusted workloads

**Your security depends primarily on:**
1. VM isolation (Lima/QEMU) ← **Primary boundary**
2. Limited mounts and access
3. Ephemeral VM usage
4. This proxy capability ← **Additional layer**

If you need stronger guarantees, use purpose-built sandboxing solutions.
