#!/bin/bash
# Network security runtime script
# Enforces iptables rules to block non-HTTP traffic

# Check if network security is enabled
if [ "${NETWORK_SECURITY_ENABLED:-false}" != "true" ]; then
    exit 0
fi

echo "Enforcing network security policies..."

# Block raw TCP/UDP if configured
if [ "${BLOCK_TCP_UDP:-true}" = "true" ]; then
    echo "  Blocking non-HTTP protocols (raw TCP/UDP)..."

    # Allow established connections
    sudo iptables -A OUTPUT -m state --state ESTABLISHED,RELATED -j ACCEPT

    # Allow DNS (required for proxy to resolve domains)
    sudo iptables -A OUTPUT -p udp --dport 53 -j ACCEPT
    sudo iptables -A OUTPUT -p tcp --dport 53 -j ACCEPT

    # Allow HTTP/HTTPS to proxy
    sudo iptables -A OUTPUT -p tcp --dport 8080 -j ACCEPT
    sudo iptables -A OUTPUT -p tcp --dport 8443 -j ACCEPT

    # Allow localhost
    sudo iptables -A OUTPUT -o lo -j ACCEPT

    # Block everything else
    sudo iptables -A OUTPUT -p tcp -j REJECT --reject-with tcp-reset
    sudo iptables -A OUTPUT -p udp -j REJECT --reject-with icmp-port-unreachable

    echo "  ✓ Non-HTTP traffic blocked"
fi

# Block private networks if configured
if [ "${BLOCK_PRIVATE_NETWORKS:-true}" = "true" ]; then
    echo "  Blocking private networks..."

    sudo iptables -A OUTPUT -d 10.0.0.0/8 -j REJECT
    sudo iptables -A OUTPUT -d 172.16.0.0/12 -j REJECT
    sudo iptables -A OUTPUT -d 192.168.0.0/16 -j REJECT

    echo "  ✓ Private networks blocked"
fi

# Block metadata services if configured
if [ "${BLOCK_METADATA_SERVICES:-true}" = "true" ]; then
    echo "  Blocking cloud metadata services..."

    sudo iptables -A OUTPUT -d 169.254.169.254 -j REJECT

    echo "  ✓ Metadata services blocked"
fi

# Write runtime context
mkdir -p ~/.claude-vm/context
cat > ~/.claude-vm/context/network-security.txt << EOF
Network security is enabled with the following policies:

- HTTP/HTTPS traffic: Filtered through proxy
- Raw TCP/UDP: $([ "${BLOCK_TCP_UDP:-true}" = "true" ] && echo "Blocked" || echo "Allowed")
- Private networks (10.0.0.0/8, etc.): $([ "${BLOCK_PRIVATE_NETWORKS:-true}" = "true" ] && echo "Blocked" || echo "Allowed")
- Cloud metadata (169.254.169.254): $([ "${BLOCK_METADATA_SERVICES:-true}" = "true" ] && echo "Blocked" || echo "Allowed")
- Proxy mode: ${POLICY_MODE:-denylist}

Allowed domains: ${ALLOWED_DOMAINS:-none configured}

You can only make HTTP/HTTPS requests to allowed domains.
Raw TCP connections and UDP traffic are blocked for security.
EOF

echo "✓ Network security policies enforced"
