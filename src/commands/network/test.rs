use crate::config::{Config, PolicyMode};
use crate::error::Result;

/// Test if a domain would be allowed or blocked by network security policies
pub fn execute(config: &Config, domain: &str) -> Result<()> {
    println!("Testing domain: {}", domain);
    println!("════════════════════════════════════════════════════════════");
    println!();

    // Check if network security is enabled
    if !config.security.network.enabled {
        println!("Status: Network security is DISABLED");
        println!();
        println!("Network security is not enabled for this project.");
        println!("The domain would be allowed (no filtering active).");
        println!();
        println!("To enable network security:");
        println!("  1. Add to .claude-vm.toml:");
        println!("     [security.network]");
        println!("     enabled = true");
        println!("  2. Recreate the VM: claude-vm clean && claude-vm setup");
        return Ok(());
    }

    // Check bypass domains first
    if matches_any(domain, &config.security.network.bypass_domains) {
        println!("Result: ✓ ALLOWED (bypass)");
        println!();
        println!("This domain matches a bypass pattern:");
        for pattern in &config.security.network.bypass_domains {
            if matches_pattern(domain, pattern) {
                println!("  • {}", pattern);
            }
        }
        println!();
        println!("Bypass domains:");
        println!("  - Pass through proxy without TLS interception");
        println!("  - Useful for certificate pinning");
        println!("  - Always allowed regardless of policy mode");
        return Ok(());
    }

    // Check policy mode
    match config.security.network.mode {
        PolicyMode::Allowlist => {
            // In allowlist mode, block unless explicitly allowed
            if matches_any(domain, &config.security.network.allowed_domains) {
                println!("Result: ✓ ALLOWED");
                println!();
                println!("Policy mode: Allowlist (block all except allowed)");
                println!();
                println!("This domain matches an allowed pattern:");
                for pattern in &config.security.network.allowed_domains {
                    if matches_pattern(domain, pattern) {
                        println!("  • {}", pattern);
                    }
                }
            } else {
                println!("Result: ✗ BLOCKED");
                println!();
                println!("Policy mode: Allowlist (block all except allowed)");
                println!();
                println!("This domain does NOT match any allowed patterns.");
                if config.security.network.allowed_domains.is_empty() {
                    println!("No domains are configured as allowed.");
                } else {
                    println!("Allowed patterns:");
                    for pattern in &config.security.network.allowed_domains {
                        println!("  • {}", pattern);
                    }
                }
                println!();
                println!("To allow this domain, add to .claude-vm.toml:");
                println!("  [security.network]");
                println!("  allowed_domains = [\"{}\"]", domain);
                println!();
                println!("Or use a wildcard pattern:");
                let parts: Vec<&str> = domain.split('.').collect();
                if parts.len() >= 2 {
                    println!(
                        "  allowed_domains = [\"*.{}\"]",
                        parts[parts.len() - 2..].join(".")
                    );
                }
            }
        }
        PolicyMode::Denylist => {
            // In denylist mode, allow unless explicitly blocked
            if matches_any(domain, &config.security.network.blocked_domains) {
                println!("Result: ✗ BLOCKED");
                println!();
                println!("Policy mode: Denylist (allow all except blocked)");
                println!();
                println!("This domain matches a blocked pattern:");
                for pattern in &config.security.network.blocked_domains {
                    if matches_pattern(domain, pattern) {
                        println!("  • {}", pattern);
                    }
                }
                println!();
                println!("To unblock this domain, remove it from .claude-vm.toml:");
                println!("  [security.network]");
                println!("  blocked_domains = [...]  # Remove matching pattern");
            } else {
                println!("Result: ✓ ALLOWED");
                println!();
                println!("Policy mode: Denylist (allow all except blocked)");
                println!();
                println!("This domain does NOT match any blocked patterns.");
                if !config.security.network.blocked_domains.is_empty() {
                    println!("Blocked patterns:");
                    for pattern in &config.security.network.blocked_domains {
                        println!("  • {}", pattern);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Check if host matches a pattern (with wildcard support)
fn matches_pattern(host: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return false;
    }
    if pattern.starts_with("*.") {
        // *.example.com matches api.example.com and example.com
        let domain = &pattern[2..];
        host == domain || host.ends_with(&format!(".{}", domain))
    } else {
        host == pattern
    }
}

/// Check if host matches any pattern in the list
fn matches_any(host: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|p| matches_pattern(host, p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_pattern_exact() {
        assert!(matches_pattern("example.com", "example.com"));
        assert!(!matches_pattern("example.com", "other.com"));
    }

    #[test]
    fn test_matches_pattern_wildcard() {
        assert!(matches_pattern("api.example.com", "*.example.com"));
        assert!(matches_pattern("example.com", "*.example.com"));
        assert!(!matches_pattern("example.org", "*.example.com"));
    }

    #[test]
    fn test_matches_any() {
        let patterns = vec!["example.com".to_string(), "*.test.com".to_string()];
        assert!(matches_any("example.com", &patterns));
        assert!(matches_any("api.test.com", &patterns));
        assert!(!matches_any("other.com", &patterns));
    }
}
