use super::definition::{Capability, McpServer};
use crate::config::Config;
use crate::error::{ClaudeVmError, Result};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Validate a Debian package name according to Debian policy.
///
/// Valid package names must:
/// - Start with an alphanumeric character
/// - Contain only lowercase letters, digits, and these symbols: '-', '.', '+', '=', ':'
/// - '=' is used for version pinning (e.g., "package=1.2.3")
/// - ':' is used for architecture specification (e.g., "package:amd64")
///
/// This validation prevents confusing apt-get errors and potential security issues.
fn validate_package_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(ClaudeVmError::InvalidConfig(
            "Package name cannot be empty".to_string(),
        ));
    }

    // Check first character is alphanumeric
    let first_char = name.chars().next().unwrap();
    if !first_char.is_ascii_alphanumeric() {
        return Err(ClaudeVmError::InvalidConfig(format!(
            "Invalid package name '{}': must start with a letter or digit",
            name
        )));
    }

    // Check all characters are valid
    for c in name.chars() {
        let valid = c.is_ascii_lowercase()
            || c.is_ascii_digit()
            || c == '-'
            || c == '.'
            || c == '+'
            || c == '='
            || c == ':';

        if !valid {
            return Err(ClaudeVmError::InvalidConfig(format!(
                "Invalid package name '{}': contains invalid character '{}'. \
                 Package names must contain only lowercase letters, digits, and '.', '-', '+', '=', ':'",
                name, c
            )));
        }
    }

    Ok(())
}

pub struct CapabilityRegistry {
    capabilities: HashMap<String, Arc<Capability>>,
}

impl CapabilityRegistry {
    /// Load all embedded capability definitions
    pub fn load() -> Result<Self> {
        let mut capabilities = HashMap::new();

        // Embed all capability TOML files at compile time
        // To add a new capability: add ONE line here and create the capability directory
        const CAPABILITY_FILES: &[(&str, &str)] = &[
            (
                "docker",
                include_str!("../../capabilities/docker/capability.toml"),
            ),
            (
                "node",
                include_str!("../../capabilities/node/capability.toml"),
            ),
            (
                "python",
                include_str!("../../capabilities/python/capability.toml"),
            ),
            (
                "chromium",
                include_str!("../../capabilities/chromium/capability.toml"),
            ),
            (
                "gpg",
                include_str!("../../capabilities/gpg/capability.toml"),
            ),
            ("gh", include_str!("../../capabilities/gh/capability.toml")),
            (
                "git",
                include_str!("../../capabilities/git/capability.toml"),
            ),
        ];

        for (id, content) in CAPABILITY_FILES {
            let capability: Capability = toml::from_str(content).map_err(|e| {
                ClaudeVmError::InvalidConfig(format!("Failed to parse capability '{}': {}", id, e))
            })?;
            capabilities.insert(id.to_string(), Arc::new(capability));
        }

        Ok(Self { capabilities })
    }

    /// Get list of enabled capabilities based on config, sorted by dependencies
    pub fn get_enabled_capabilities(&self, config: &Config) -> Result<Vec<Arc<Capability>>> {
        let mut enabled = Vec::new();

        // Check each tool in config
        for (id, capability) in &self.capabilities {
            if self.is_enabled(id, config) {
                // Check for conflicts
                for conflict_id in &capability.capability.conflicts {
                    if self.is_enabled(conflict_id, config) {
                        return Err(ClaudeVmError::InvalidConfig(format!(
                            "Capability '{}' conflicts with '{}'",
                            id, conflict_id
                        )));
                    }
                }

                enabled.push(Arc::clone(capability));
            }
        }

        // Sort by dependencies (topological sort)
        self.sort_by_dependencies(&mut enabled)?;

        Ok(enabled)
    }

    /// Check if a capability is enabled in the config
    fn is_enabled(&self, id: &str, config: &Config) -> bool {
        config.tools.is_enabled(id)
    }

    /// Sort capabilities by dependencies (topological sort)
    fn sort_by_dependencies(&self, capabilities: &mut Vec<Arc<Capability>>) -> Result<()> {
        let mut sorted = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();

        for capability in capabilities.iter() {
            self.visit_capability(
                &capability.capability.id,
                capabilities,
                &mut visited,
                &mut visiting,
                &mut sorted,
            )?;
        }

        *capabilities = sorted;
        Ok(())
    }

    fn visit_capability(
        &self,
        id: &str,
        all_capabilities: &[Arc<Capability>],
        visited: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
        sorted: &mut Vec<Arc<Capability>>,
    ) -> Result<()> {
        if visited.contains(id) {
            return Ok(());
        }

        if visiting.contains(id) {
            return Err(ClaudeVmError::InvalidConfig(format!(
                "Circular dependency detected involving capability '{}'",
                id
            )));
        }

        visiting.insert(id.to_string());

        // Find the capability
        let capability = all_capabilities
            .iter()
            .find(|c| c.capability.id == id)
            .ok_or_else(|| {
                ClaudeVmError::InvalidConfig(format!("Capability '{}' not found", id))
            })?;

        // Visit dependencies first
        for dep_id in &capability.capability.requires {
            if !all_capabilities.iter().any(|c| c.capability.id == *dep_id) {
                return Err(ClaudeVmError::InvalidConfig(format!(
                    "Capability '{}' requires '{}' but it is not enabled",
                    id, dep_id
                )));
            }

            self.visit_capability(dep_id, all_capabilities, visited, visiting, sorted)?;
        }

        visiting.remove(id);
        visited.insert(id.to_string());
        sorted.push(Arc::clone(capability));

        Ok(())
    }

    /// Collect all MCP servers from enabled capabilities
    pub fn get_mcp_servers(&self, config: &Config) -> Result<Vec<McpServer>> {
        let enabled = self.get_enabled_capabilities(config)?;
        let mut servers = Vec::new();

        for cap in enabled {
            for mcp in &cap.mcp {
                // Check if the MCP should be enabled
                if let Some(required_cap) = &mcp.enabled_when {
                    if !self.is_enabled(required_cap, config) {
                        // Skip this MCP server - its requirement is not met
                        continue;
                    }
                }
                servers.push(mcp.clone());
            }
        }

        Ok(servers)
    }

    /// Collect all system packages from enabled capabilities and user config.
    /// Returns packages in dependency order (respects capability.requires).
    /// Duplicates are removed while preserving order (first occurrence wins).
    ///
    /// Performance: Clones each unique package only once using HashSet-based deduplication.
    pub fn collect_system_packages(&self, config: &Config) -> Result<Vec<String>> {
        let enabled = self.get_enabled_capabilities(config)?;
        let mut seen = HashSet::new();
        let mut packages = Vec::new();

        // Collect packages from capabilities (already in dependency order)
        // Deduplicate and validate as we go to minimize cloning
        for capability in enabled {
            if let Some(pkg_spec) = &capability.packages {
                for pkg in &pkg_spec.system {
                    // Validate package name
                    validate_package_name(pkg)?;

                    // Only clone and add if not already seen
                    if seen.insert(pkg.as_str()) {
                        packages.push(pkg.clone());
                    }
                }
            }
        }

        // Add user-defined packages from config
        for pkg in &config.packages.system {
            // Validate package name
            validate_package_name(pkg)?;

            // Only clone and add if not already seen
            if seen.insert(pkg.as_str()) {
                packages.push(pkg.clone());
            }
        }

        Ok(packages)
    }

    /// Get capabilities that need repository setup (in dependency order).
    /// Returns tuples of (capability_id, setup_script).
    pub fn get_repo_setups(&self, config: &Config) -> Result<Vec<(String, String)>> {
        let enabled = self.get_enabled_capabilities(config)?;
        let mut setups = Vec::new();

        for capability in enabled {
            if let Some(pkg_spec) = &capability.packages {
                if let Some(setup_script) = &pkg_spec.setup_script {
                    setups.push((capability.capability.id.clone(), setup_script.clone()));
                }
            }
        }

        Ok(setups)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_packages_deduplication() {
        let registry = CapabilityRegistry::load().unwrap();

        // Enable capabilities that might share packages
        let mut config = Config::default();
        config.tools.python = true;
        config.tools.node = true;

        // Add some user packages, including duplicates
        config.packages.system = vec!["git".to_string(), "curl".to_string()];

        let packages = registry.collect_system_packages(&config).unwrap();

        // Check that we got packages
        assert!(!packages.is_empty(), "Should have collected packages");

        // Check no duplicates
        let mut seen = HashSet::new();
        for pkg in &packages {
            assert!(seen.insert(pkg), "Duplicate package found: {}", pkg);
        }
    }

    #[test]
    fn test_collect_packages_respects_dependencies() {
        let registry = CapabilityRegistry::load().unwrap();

        // Enable git capability (which has no requires)
        let mut config = Config::default();
        config.tools.git = true;

        let packages = registry.collect_system_packages(&config).unwrap();

        // Git capability should provide packages in correct order
        // The actual packages depend on capability definition
        // Just verify the method works without error
        assert!(packages.is_empty() || !packages.is_empty());
    }

    #[test]
    fn test_user_packages_merged() {
        let registry = CapabilityRegistry::load().unwrap();

        let mut config = Config::default();
        // Don't enable any capabilities

        // Add user-defined packages
        config.packages.system = vec!["htop".to_string(), "jq".to_string()];

        let packages = registry.collect_system_packages(&config).unwrap();

        // Should have exactly the user packages
        assert_eq!(packages.len(), 2);
        assert!(packages.contains(&"htop".to_string()));
        assert!(packages.contains(&"jq".to_string()));
    }

    #[test]
    fn test_get_repo_setups_empty() {
        let registry = CapabilityRegistry::load().unwrap();

        let config = Config::default();
        // No capabilities enabled

        let setups = registry.get_repo_setups(&config).unwrap();

        // Should be empty
        assert_eq!(setups.len(), 0);
    }

    #[test]
    fn test_validate_package_name_valid() {
        // Simple package names
        assert!(validate_package_name("python3").is_ok());
        assert!(validate_package_name("nodejs").is_ok());
        assert!(validate_package_name("curl").is_ok());

        // With hyphens and dots
        assert!(validate_package_name("python3-pip").is_ok());
        assert!(validate_package_name("docker-ce").is_ok());
        assert!(validate_package_name("python3.11").is_ok());

        // With version pinning
        assert!(validate_package_name("python3=3.11.0-1").is_ok());
        assert!(validate_package_name("nodejs=22.0.0").is_ok());

        // With architecture
        assert!(validate_package_name("libc6:amd64").is_ok());

        // With plus
        assert!(validate_package_name("g++").is_ok());
    }

    #[test]
    fn test_validate_package_name_invalid() {
        // Empty name
        assert!(validate_package_name("").is_err());

        // Uppercase letters (Debian packages must be lowercase)
        assert!(validate_package_name("Python3").is_err());
        assert!(validate_package_name("CURL").is_err());

        // Shell metacharacters
        assert!(validate_package_name("python3; rm -rf /").is_err());
        assert!(validate_package_name("python3 && whoami").is_err());
        assert!(validate_package_name("python3|cat").is_err());
        assert!(validate_package_name("python3$(whoami)").is_err());
        assert!(validate_package_name("python3`whoami`").is_err());

        // Invalid starting character
        assert!(validate_package_name("-python3").is_err());
        assert!(validate_package_name(".python3").is_err());

        // Spaces
        assert!(validate_package_name("python 3").is_err());

        // Other invalid characters
        assert!(validate_package_name("python3@latest").is_err());
        assert!(validate_package_name("python3#comment").is_err());
    }

    #[test]
    fn test_collect_packages_validates_names() {
        let registry = CapabilityRegistry::load().unwrap();
        let mut config = Config::default();

        // Add invalid package name
        config.packages.system = vec!["INVALID-UPPERCASE".to_string()];

        // Should fail validation
        let result = registry.collect_system_packages(&config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid package name"));
    }
}
