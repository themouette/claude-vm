#!/bin/bash
# Network security runtime script
# Starts mitmproxy in the VM and enforces iptables rules

# Check if network security is enabled
if [ "${NETWORK_SECURITY_ENABLED:-false}" != "true" ]; then
    exit 0
fi

echo "Enforcing network security policies..."

# Generate mitmproxy filter script from configuration
cat > /tmp/mitmproxy_filter.py << 'FILTER_SCRIPT_EOF'
from mitmproxy import http
import os

# Read configuration from environment variables
MODE = os.environ.get("POLICY_MODE", "denylist")
ALLOWED_DOMAINS = [d.strip() for d in os.environ.get("ALLOWED_DOMAINS", "").split(",") if d.strip()]
BLOCKED_DOMAINS = [d.strip() for d in os.environ.get("BLOCKED_DOMAINS", "").split(",") if d.strip()]
BYPASS_DOMAINS = [d.strip() for d in os.environ.get("BYPASS_DOMAINS", "").split(",") if d.strip()]

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
    host = flow.request.pretty_host

    # Bypass domains always allowed
    if matches_any(host, BYPASS_DOMAINS):
        return

    if MODE == "allowlist":
        # Block unless explicitly allowed
        if not matches_any(host, ALLOWED_DOMAINS):
            flow.response = http.Response.make(
                403,
                f"Domain blocked by allowlist policy: {host}\n".encode(),
                {"Content-Type": "text/plain"}
            )
            return
    elif MODE == "denylist":
        # Allow unless explicitly blocked
        if matches_any(host, BLOCKED_DOMAINS):
            flow.response = http.Response.make(
                403,
                f"Domain blocked by denylist policy: {host}\n".encode(),
                {"Content-Type": "text/plain"}
            )
            return

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

# Start mitmproxy in background
echo "  Starting HTTP/HTTPS filtering proxy..."
mitmproxy \
  --mode regular@8080 \
  --set confdir=~/.mitmproxy \
  --set block_global=false \
  -s /tmp/mitmproxy_filter.py \
  > /tmp/mitmproxy.log 2>&1 &

PROXY_PID=$!
echo $PROXY_PID > /tmp/mitmproxy.pid

# Wait for proxy to be ready
for i in {1..20}; do
  if nc -z localhost 8080 2>/dev/null; then
    echo "  ✓ Proxy started (PID: $PROXY_PID)"
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

# Set proxy environment variables for the session
export HTTP_PROXY="http://localhost:8080"
export HTTPS_PROXY="http://localhost:8080"
export http_proxy="$HTTP_PROXY"
export https_proxy="$HTTPS_PROXY"
export NO_PROXY="127.0.0.1"
export no_proxy="$NO_PROXY"

# Block raw TCP/UDP if configured
if [ "${BLOCK_TCP_UDP:-true}" = "true" ]; then
    echo "  Blocking non-HTTP protocols (raw TCP/UDP)..."

    # Allow established connections
    sudo iptables -A OUTPUT -m state --state ESTABLISHED,RELATED -j ACCEPT

    # Allow DNS (required for proxy to resolve domains)
    sudo iptables -A OUTPUT -p udp --dport 53 -j ACCEPT
    sudo iptables -A OUTPUT -p tcp --dport 53 -j ACCEPT

    # Allow localhost (proxy runs here)
    sudo iptables -A OUTPUT -o lo -j ACCEPT

    # Block everything else
    sudo iptables -A OUTPUT -p tcp -j REJECT --reject-with tcp-reset
    sudo iptables -A OUTPUT -p udp -j REJECT --reject-with icmp-port-unreachable

    echo "  ✓ Non-HTTP traffic blocked"
fi

# Block private networks if configured
if [ "${BLOCK_PRIVATE_NETWORKS:-true}" = "true" ]; then
    echo "  Blocking private networks..."

    # Insert at beginning to override later rules
    sudo iptables -I OUTPUT -d 10.0.0.0/8 -j REJECT
    sudo iptables -I OUTPUT -d 172.16.0.0/12 -j REJECT
    sudo iptables -I OUTPUT -d 192.168.0.0/16 -j REJECT

    echo "  ✓ Private networks blocked"
fi

# Block metadata services if configured
if [ "${BLOCK_METADATA_SERVICES:-true}" = "true" ]; then
    echo "  Blocking cloud metadata services..."

    sudo iptables -I OUTPUT -d 169.254.169.254 -j REJECT

    echo "  ✓ Metadata services blocked"
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

echo "✓ Network security policies enforced"
