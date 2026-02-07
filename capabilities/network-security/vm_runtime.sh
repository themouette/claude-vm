#!/bin/bash
# Network security runtime script
# Starts mitmproxy in the VM and enforces iptables rules

# Check if network security is enabled
if [ "${NETWORK_SECURITY_ENABLED:-false}" != "true" ]; then
    return 0
fi

echo "Enforcing network security policies..."

# Note: No cleanup trap because this script is sourced (not executed).
# The proxy process should live for the entire VM session lifetime.
# When the VM is destroyed, the proxy process terminates naturally.
# If manual cleanup is needed, use: kill $(cat /tmp/mitmproxy.pid)

# Generate mitmproxy filter script from configuration
cat > /tmp/mitmproxy_filter.py << 'FILTER_SCRIPT_EOF'
from mitmproxy import http
import os
import json
import time
from pathlib import Path

# Read configuration from environment variables
MODE = os.environ.get("POLICY_MODE", "denylist")
ALLOWED_DOMAINS = [d.strip() for d in os.environ.get("ALLOWED_DOMAINS", "").split(",") if d.strip()]
BLOCKED_DOMAINS = [d.strip() for d in os.environ.get("BLOCKED_DOMAINS", "").split(",") if d.strip()]
BYPASS_DOMAINS = [d.strip() for d in os.environ.get("BYPASS_DOMAINS", "").split(",") if d.strip()]

# Statistics tracking
STATS_FILE = Path("/tmp/mitmproxy_stats.json")
stats = {
    "requests_total": 0,
    "requests_allowed": 0,
    "requests_blocked": 0,
    "last_update": None
}

# Load existing stats if available
if STATS_FILE.exists():
    try:
        stats = json.loads(STATS_FILE.read_text())
    except (json.JSONDecodeError, OSError, ValueError) as e:
        # Use default stats if file is corrupted or unreadable
        import sys
        print(f"Warning: Failed to load stats file: {e}", file=sys.stderr)

def update_stats():
    """Write stats to file atomically"""
    try:
        stats["last_update"] = int(time.time())
        # Write to temp file first, then rename for atomicity
        temp_file = Path("/tmp/mitmproxy_stats.json.tmp")
        temp_file.write_text(json.dumps(stats))
        temp_file.rename(STATS_FILE)
    except (OSError, IOError) as e:
        # Ignore write errors - stats are not critical for security
        pass

def matches_pattern(host, pattern):
    """Match domain with wildcard support (*.example.com)"""
    if not pattern:
        return False
    if pattern.startswith("*."):
        # *.example.com matches api.example.com and example.com
        domain = pattern[2:]
        return host == domain or host.endswith("." + domain)
    return host == pattern

def matches_any(host, patterns):
    """Check if host matches any pattern in the list"""
    return any(matches_pattern(host, p) for p in patterns)

def request(flow: http.HTTPFlow) -> None:
    """Filter requests based on domain policy"""
    stats["requests_total"] += 1

    host = flow.request.pretty_host

    # Bypass domains always allowed
    if matches_any(host, BYPASS_DOMAINS):
        stats["requests_allowed"] += 1
        update_stats()
        return

    if MODE == "allowlist":
        # Block unless explicitly allowed
        if not matches_any(host, ALLOWED_DOMAINS):
            stats["requests_blocked"] += 1
            update_stats()
            flow.response = http.Response.make(
                403,
                f"Domain blocked by allowlist policy: {host}\n".encode(),
                {"Content-Type": "text/plain"}
            )
            return
    elif MODE == "denylist":
        # Allow unless explicitly blocked
        if matches_any(host, BLOCKED_DOMAINS):
            stats["requests_blocked"] += 1
            update_stats()
            flow.response = http.Response.make(
                403,
                f"Domain blocked by denylist policy: {host}\n".encode(),
                {"Content-Type": "text/plain"}
            )
            return

    # Request allowed
    stats["requests_allowed"] += 1
    update_stats()

def response(flow: http.HTTPFlow) -> None:
    """Log allowed requests for visibility"""
    if not flow.response or flow.response.status_code < 400:
        # Request was allowed
        pass
FILTER_SCRIPT_EOF

# Export environment variables for the filter script
export POLICY_MODE="${POLICY_MODE:-denylist}"
export ALLOWED_DOMAINS="${ALLOWED_DOMAINS:-}"
export BLOCKED_DOMAINS="${BLOCKED_DOMAINS:-}"
export BYPASS_DOMAINS="${BYPASS_DOMAINS:-}"

# Build mitmproxy ignore_hosts option for true bypass (no TLS interception)
IGNORE_HOSTS_ARG=""
if [ -n "${BYPASS_DOMAINS:-}" ]; then
    # Convert comma-separated domains to Python list format for mitmproxy
    # "a.com,b.com" -> "['a.com','b.com']"
    IGNORE_LIST=$(echo "${BYPASS_DOMAINS}" | awk -F',' '{
        printf "["
        for (i=1; i<=NF; i++) {
            gsub(/^[ \t]+|[ \t]+$/, "", $i)  # trim whitespace
            if ($i != "") {
                if (i > 1) printf ","
                printf "'\''%s'\''", $i
            }
        }
        printf "]"
    }')
    IGNORE_HOSTS_ARG="--set ignore_hosts=${IGNORE_LIST}"
fi

# Start mitmproxy in background
echo "  Starting HTTP/HTTPS filtering proxy..."
mitmproxy \
  --mode regular@8080 \
  --set confdir=~/.mitmproxy \
  --set block_global=false \
  $IGNORE_HOSTS_ARG \
  -s /tmp/mitmproxy_filter.py \
  > /tmp/mitmproxy.log 2>&1 &

PROXY_PID=$!
echo $PROXY_PID > /tmp/mitmproxy.pid

# Wait for proxy to be ready
for i in {1..20}; do
  if nc -z localhost 8080 2>/dev/null; then
    echo "  ✓ Proxy started (PID: $PROXY_PID) - Listening on localhost:8080"
    break
  fi
  if [ $i -eq 20 ]; then
    echo "  ERROR: Proxy failed to start"
    if [ -f /tmp/mitmproxy.log ]; then
      echo "  Proxy log:"
      tail -20 /tmp/mitmproxy.log
    fi
    exit 1
  fi
  sleep 0.5
done

# Display policy configuration
echo ""
echo "  Policy Configuration:"
echo "    Mode: ${POLICY_MODE:-denylist}"

# Count and display domain patterns
ALLOWED_COUNT=$(echo "${ALLOWED_DOMAINS:-}" | awk -F',' '{print NF}')
if [ -z "${ALLOWED_DOMAINS:-}" ]; then
    ALLOWED_COUNT=0
fi
BLOCKED_COUNT=$(echo "${BLOCKED_DOMAINS:-}" | awk -F',' '{print NF}')
if [ -z "${BLOCKED_DOMAINS:-}" ]; then
    BLOCKED_COUNT=0
fi
BYPASS_COUNT=$(echo "${BYPASS_DOMAINS:-}" | awk -F',' '{print NF}')
if [ -z "${BYPASS_DOMAINS:-}" ]; then
    BYPASS_COUNT=0
fi

if [ "$ALLOWED_COUNT" -gt 0 ]; then
    echo "    Allowed: ${ALLOWED_DOMAINS} ($ALLOWED_COUNT pattern$([ "$ALLOWED_COUNT" -ne 1 ] && echo "s" || echo ""))"
else
    echo "    Allowed: none"
fi

if [ "$BLOCKED_COUNT" -gt 0 ]; then
    echo "    Blocked: ${BLOCKED_DOMAINS} ($BLOCKED_COUNT pattern$([ "$BLOCKED_COUNT" -ne 1 ] && echo "s" || echo ""))"
else
    echo "    Blocked: none"
fi

if [ "$BYPASS_COUNT" -gt 0 ]; then
    echo "    Bypass: ${BYPASS_DOMAINS} ($BYPASS_COUNT pattern$([ "$BYPASS_COUNT" -ne 1 ] && echo "s" || echo ""))"
else
    echo "    Bypass: none"
fi

echo ""
echo "  Protocol Blocks:"

# Set proxy environment variables for the session
export HTTP_PROXY="http://localhost:8080"
export HTTPS_PROXY="http://localhost:8080"
export http_proxy="$HTTP_PROXY"
export https_proxy="$HTTPS_PROXY"

# Build NO_PROXY list: localhost variants only
# Note: Cannot add bypass_domains here because iptables blocks direct connections.
# Bypass domains must still go through proxy, but mitmproxy will pass them through.
NO_PROXY_LIST="127.0.0.1,localhost,::1,[::1]"
export NO_PROXY="$NO_PROXY_LIST"
export no_proxy="$NO_PROXY"

# Helper function to add iptables rule only if it doesn't exist
add_iptables_rule() {
    local cmd="$1"
    shift
    # Check if rule exists (using -C instead of -A/-I)
    if ! sudo "$cmd" -C OUTPUT "$@" 2>/dev/null; then
        # Rule doesn't exist, add it
        sudo "$cmd" -A OUTPUT "$@"
    fi
}

# Helper function to insert iptables rule at beginning only if it doesn't exist
insert_iptables_rule() {
    local cmd="$1"
    shift
    # Check if rule exists
    if ! sudo "$cmd" -C OUTPUT "$@" 2>/dev/null; then
        # Rule doesn't exist, insert at beginning
        sudo "$cmd" -I OUTPUT "$@"
    fi
}

# Block raw TCP/UDP if configured
if [ "${BLOCK_TCP_UDP:-true}" = "true" ]; then
    # IPv4 rules
    # Allow established connections
    add_iptables_rule iptables -m state --state ESTABLISHED,RELATED -j ACCEPT

    # Allow DNS (required for proxy to resolve domains)
    add_iptables_rule iptables -p udp --dport 53 -j ACCEPT
    add_iptables_rule iptables -p tcp --dport 53 -j ACCEPT

    # Allow localhost (proxy runs here)
    add_iptables_rule iptables -o lo -j ACCEPT

    # Block everything else
    add_iptables_rule iptables -p tcp -j REJECT --reject-with tcp-reset
    add_iptables_rule iptables -p udp -j REJECT --reject-with icmp-port-unreachable

    # IPv6 rules (same logic)
    # Allow established connections
    add_iptables_rule ip6tables -m state --state ESTABLISHED,RELATED -j ACCEPT

    # Allow DNS
    add_iptables_rule ip6tables -p udp --dport 53 -j ACCEPT
    add_iptables_rule ip6tables -p tcp --dport 53 -j ACCEPT

    # Allow localhost
    add_iptables_rule ip6tables -o lo -j ACCEPT

    # Block everything else
    add_iptables_rule ip6tables -p tcp -j REJECT --reject-with tcp-reset
    add_iptables_rule ip6tables -p udp -j REJECT --reject-with icmp6-port-unreachable

    echo "    ✓ Raw TCP/UDP blocked (IPv4 and IPv6)"
fi

# Block private networks if configured
if [ "${BLOCK_PRIVATE_NETWORKS:-true}" = "true" ]; then
    # IPv4 private networks
    # Insert at beginning to override later rules
    insert_iptables_rule iptables -d 10.0.0.0/8 -j REJECT
    insert_iptables_rule iptables -d 172.16.0.0/12 -j REJECT
    insert_iptables_rule iptables -d 192.168.0.0/16 -j REJECT

    # IPv6 private networks
    insert_iptables_rule ip6tables -d fc00::/7 -j REJECT      # Unique local addresses
    insert_iptables_rule ip6tables -d fe80::/10 -j REJECT     # Link-local addresses

    echo "    ✓ Private networks blocked (10.0.0.0/8, 192.168.0.0/16, 172.16.0.0/12)"
fi

# Block metadata services if configured
if [ "${BLOCK_METADATA_SERVICES:-true}" = "true" ]; then
    # IPv4 metadata
    insert_iptables_rule iptables -d 169.254.169.254 -j REJECT

    # IPv6 metadata (fe80::)
    insert_iptables_rule ip6tables -d fe80::a9fe:a9fe -j REJECT  # IPv6-mapped 169.254.169.254

    echo "    ✓ Cloud metadata blocked (169.254.169.254)"
fi

# Write runtime context for Claude
mkdir -p ~/.claude-vm/context
cat > ~/.claude-vm/context/network-security.txt << EOF
Network security is enabled with the following policies:

- HTTP/HTTPS traffic: Filtered through in-VM proxy (localhost:8080)
- Policy mode: ${POLICY_MODE:-denylist}
- Allowed domains: ${ALLOWED_DOMAINS:-none configured}
- Blocked domains: ${BLOCKED_DOMAINS:-none configured}
- Bypass domains: ${BYPASS_DOMAINS:-none configured}
- Raw TCP/UDP: $([ "${BLOCK_TCP_UDP:-true}" = "true" ] && echo "Blocked" || echo "Allowed")
- Private networks (10.0.0.0/8, etc.): $([ "${BLOCK_PRIVATE_NETWORKS:-true}" = "true" ] && echo "Blocked" || echo "Allowed")
- Cloud metadata (169.254.169.254): $([ "${BLOCK_METADATA_SERVICES:-true}" = "true" ] && echo "Blocked" || echo "Allowed")

You can only make HTTP/HTTPS requests. The proxy filters domains according to the policy.
Raw TCP connections and UDP traffic are blocked for security.
EOF

echo ""
echo "✓ Network security active - Use 'claude-vm network logs' to monitor requests"
