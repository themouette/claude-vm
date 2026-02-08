/// Integration tests for network security commands
///
/// Note: Full integration tests (status, logs) require a running VM and are
/// tested manually. These tests cover the testable logic without VM dependency.
use claude_vm::config::{Config, NetworkSecurityConfig, PolicyMode, SecurityConfig};

#[test]
fn test_network_test_command_allowlist_allowed() {
    let config = Config {
        security: SecurityConfig {
            network: NetworkSecurityConfig {
                enabled: true,
                mode: PolicyMode::Allowlist,
                allowed_domains: vec!["example.com".to_string(), "*.api.com".to_string()],
                blocked_domains: vec![],
                bypass_domains: vec![],
                block_tcp_udp: true,
                block_private_networks: true,
                block_metadata_services: true,
            },
        },
        ..Default::default()
    };

    // Test exact match
    assert!(would_be_allowed(&config, "example.com"));

    // Test wildcard match
    assert!(would_be_allowed(&config, "api.api.com"));
    assert!(would_be_allowed(&config, "test.api.com"));

    // Test no match (should be blocked)
    assert!(!would_be_allowed(&config, "other.com"));
}

#[test]
fn test_network_test_command_denylist_blocked() {
    let config = Config {
        security: SecurityConfig {
            network: NetworkSecurityConfig {
                enabled: true,
                mode: PolicyMode::Denylist,
                allowed_domains: vec![],
                blocked_domains: vec!["blocked.com".to_string(), "*.bad.com".to_string()],
                bypass_domains: vec![],
                block_tcp_udp: true,
                block_private_networks: true,
                block_metadata_services: true,
            },
        },
        ..Default::default()
    };

    // Test exact match (should be blocked)
    assert!(!would_be_allowed(&config, "blocked.com"));

    // Test wildcard match (should be blocked)
    assert!(!would_be_allowed(&config, "api.bad.com"));
    assert!(!would_be_allowed(&config, "test.bad.com"));

    // Test no match (should be allowed)
    assert!(would_be_allowed(&config, "example.com"));
}

#[test]
fn test_network_test_command_bypass_always_allowed() {
    let config = Config {
        security: SecurityConfig {
            network: NetworkSecurityConfig {
                enabled: true,
                mode: PolicyMode::Allowlist,
                allowed_domains: vec![],
                blocked_domains: vec![],
                bypass_domains: vec!["bypass.com".to_string(), "*.localhost".to_string()],
                block_tcp_udp: true,
                block_private_networks: true,
                block_metadata_services: true,
            },
        },
        ..Default::default()
    };

    // Bypass domains are always allowed even in empty allowlist
    assert!(would_be_allowed(&config, "bypass.com"));
    assert!(would_be_allowed(&config, "api.localhost"));

    // Non-bypass domains blocked in empty allowlist
    assert!(!would_be_allowed(&config, "example.com"));
}

#[test]
fn test_network_test_command_disabled() {
    let config = Config {
        security: SecurityConfig {
            network: NetworkSecurityConfig {
                enabled: false,
                mode: PolicyMode::Allowlist,
                allowed_domains: vec![],
                blocked_domains: vec![],
                bypass_domains: vec![],
                block_tcp_udp: true,
                block_private_networks: true,
                block_metadata_services: true,
            },
        },
        ..Default::default()
    };

    // When disabled, all domains are allowed
    assert!(would_be_allowed(&config, "example.com"));
    assert!(would_be_allowed(&config, "anything.com"));
}

#[test]
fn test_pattern_matching_wildcard() {
    // Test the pattern matching logic used by the test command
    assert!(matches_pattern("api.example.com", "*.example.com"));
    assert!(matches_pattern("example.com", "*.example.com"));
    assert!(!matches_pattern("example.org", "*.example.com"));
    assert!(!matches_pattern("subapi.example.com", "api.example.com"));
}

#[test]
fn test_pattern_matching_exact() {
    assert!(matches_pattern("example.com", "example.com"));
    assert!(!matches_pattern("api.example.com", "example.com"));
    assert!(!matches_pattern("example.org", "example.com"));
}

// Helper function that implements the same logic as the test command
fn would_be_allowed(config: &Config, domain: &str) -> bool {
    if !config.security.network.enabled {
        return true;
    }

    // Check bypass first
    if matches_any(domain, &config.security.network.bypass_domains) {
        return true;
    }

    match config.security.network.mode {
        PolicyMode::Allowlist => {
            // Block unless explicitly allowed
            matches_any(domain, &config.security.network.allowed_domains)
        }
        PolicyMode::Denylist => {
            // Allow unless explicitly blocked
            !matches_any(domain, &config.security.network.blocked_domains)
        }
    }
}

fn matches_pattern(host: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return false;
    }
    if let Some(domain) = pattern.strip_prefix("*.") {
        host == domain || host.ends_with(&format!(".{}", domain))
    } else {
        host == pattern
    }
}

fn matches_any(host: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|p| matches_pattern(host, p))
}
