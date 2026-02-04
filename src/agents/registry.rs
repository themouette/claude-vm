//! Agent registry for loading and managing available agents.

use super::definition::Agent;
use crate::error::{ClaudeVmError, Result};
use std::collections::HashMap;
use std::sync::Arc;

/// Registry of available agents
pub struct AgentRegistry {
    agents: HashMap<String, Arc<Agent>>,
}

impl AgentRegistry {
    /// Load all agents from the agents/ directory
    pub fn load() -> Result<Self> {
        let mut agents = HashMap::new();

        // Load embedded agent definitions
        agents.insert("claude".to_string(), Arc::new(load_claude_agent()?));
        agents.insert("opencode".to_string(), Arc::new(load_opencode_agent()?));

        Ok(Self { agents })
    }

    /// Get an agent by ID
    pub fn get(&self, id: &str) -> Option<Arc<Agent>> {
        self.agents.get(id).cloned()
    }

    /// List all available agent IDs
    pub fn list_available(&self) -> Vec<String> {
        let mut ids: Vec<_> = self.agents.keys().cloned().collect();
        ids.sort();
        ids
    }
}

/// Validate that an agent definition is complete and usable
fn validate_agent(agent: &Agent) -> Result<()> {
    // Verify required fields
    if agent.agent.id.is_empty() {
        return Err(ClaudeVmError::InvalidConfig(
            "Agent id cannot be empty".to_string(),
        ));
    }
    if agent.agent.command.is_empty() {
        return Err(ClaudeVmError::InvalidConfig(format!(
            "Agent '{}' command cannot be empty",
            agent.agent.id
        )));
    }

    // Verify deployment script exists (required for runtime)
    if agent.deploy.is_none() {
        return Err(ClaudeVmError::InvalidConfig(format!(
            "Agent '{}' missing required deploy script",
            agent.agent.id
        )));
    }

    // If agent requires authentication, verify authenticate script exists
    if agent.agent.requires_authentication && agent.authenticate.is_none() {
        return Err(ClaudeVmError::InvalidConfig(format!(
            "Agent '{}' requires authentication but has no authenticate script",
            agent.agent.id
        )));
    }

    Ok(())
}

/// Load the Claude agent definition from embedded TOML
fn load_claude_agent() -> Result<Agent> {
    let toml_content = include_str!("../../agents/claude/agent.toml");
    let agent: Agent = toml::from_str(toml_content).map_err(|e| {
        ClaudeVmError::InvalidConfig(format!("Failed to parse claude agent definition: {}", e))
    })?;
    validate_agent(&agent)?;
    Ok(agent)
}

/// Load the OpenCode agent definition from embedded TOML
fn load_opencode_agent() -> Result<Agent> {
    let toml_content = include_str!("../../agents/opencode/agent.toml");
    let agent: Agent = toml::from_str(toml_content).map_err(|e| {
        ClaudeVmError::InvalidConfig(format!("Failed to parse opencode agent definition: {}", e))
    })?;
    validate_agent(&agent)?;
    Ok(agent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_agents() {
        let registry = AgentRegistry::load().unwrap();
        assert!(registry.get("claude").is_some());
        assert!(registry.get("opencode").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_list_available() {
        let registry = AgentRegistry::load().unwrap();
        let available = registry.list_available();
        assert!(available.contains(&"claude".to_string()));
        assert!(available.contains(&"opencode".to_string()));
    }

    #[test]
    fn test_claude_agent_structure() {
        let registry = AgentRegistry::load().unwrap();
        let claude = registry.get("claude").unwrap();
        assert_eq!(claude.agent.id, "claude");
        assert_eq!(claude.agent.command, "claude");
        assert!(claude.agent.requires_authentication);
        assert_eq!(claude.paths.config_dir, ".claude");
        assert_eq!(claude.paths.context_file, "CLAUDE.md");
    }

    #[test]
    fn test_opencode_agent_structure() {
        let registry = AgentRegistry::load().unwrap();
        let opencode = registry.get("opencode").unwrap();
        assert_eq!(opencode.agent.id, "opencode");
        assert_eq!(opencode.agent.command, "opencode");
        assert!(!opencode.agent.requires_authentication);
        assert_eq!(opencode.paths.config_dir, ".config/opencode");
        assert!(opencode.requires.capabilities.contains(&"node".to_string()));
    }
}
