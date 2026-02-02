use super::definition::{Capability, McpServer};
use crate::config::Config;
use crate::error::{ClaudeVmError, Result};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

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
}
