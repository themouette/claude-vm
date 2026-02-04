//! Data structures for parsing agent TOML files.

use serde::Deserialize;

/// An agent definition loaded from a TOML file.
#[derive(Debug, Clone, Deserialize)]
pub struct Agent {
    /// Agent metadata (id, name, description, command)
    pub agent: AgentMeta,

    /// Required capabilities
    #[serde(default)]
    pub requires: AgentRequirements,

    /// Path configuration for agent
    pub paths: AgentPaths,

    /// Installation script
    #[serde(default)]
    pub install: Option<ScriptConfig>,

    /// Authentication script (optional)
    #[serde(default)]
    pub authenticate: Option<ScriptConfig>,

    /// Deployment script
    #[serde(default)]
    pub deploy: Option<ScriptConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentMeta {
    pub id: String,
    pub name: String,
    pub description: String,
    pub command: String,

    #[serde(default)]
    pub requires_authentication: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AgentRequirements {
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentPaths {
    pub config_dir: String,
    pub context_file: String,
    pub mcp_config_file: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScriptConfig {
    /// Inline script content
    #[serde(default)]
    pub script: Option<String>,

    /// Reference to script file
    #[serde(default)]
    pub script_file: Option<String>,
}
