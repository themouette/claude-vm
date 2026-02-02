//! Data structures for parsing capability TOML files.
//!
//! These types define the schema for capability definitions.

use serde::Deserialize;

/// A capability definition loaded from a TOML file.
///
/// Capabilities define optional lifecycle hooks, MCP servers, and port forwards.
#[derive(Debug, Clone, Deserialize)]
pub struct Capability {
    /// Capability metadata (id, name, description, dependencies)
    pub capability: CapabilityMeta,

    /// Optional host setup script (runs on host during setup)
    #[serde(default)]
    pub host_setup: Option<ScriptConfig>,

    /// Optional VM setup script (runs in VM during template creation)
    #[serde(default)]
    pub vm_setup: Option<ScriptConfig>,

    /// Optional VM runtime script (sourced before each session)
    #[serde(default)]
    pub vm_runtime: Option<ScriptConfig>,

    /// MCP servers to register
    #[serde(default)]
    pub mcp: Vec<McpServer>,

    /// Port forwards to configure
    #[serde(default)]
    pub forwards: Vec<ForwardConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CapabilityMeta {
    pub id: String,
    pub name: String,
    pub description: String,

    #[serde(default)]
    pub requires: Vec<String>,

    #[serde(default)]
    pub conflicts: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScriptConfig {
    /// Inline script content
    #[serde(default)]
    pub script: Option<String>,

    /// Reference to embedded script file
    #[serde(default)]
    pub script_file: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct McpServer {
    pub id: String,
    pub command: String,
    pub args: Vec<String>,

    #[serde(default)]
    pub enabled_when: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ForwardConfig {
    #[serde(rename = "type")]
    pub forward_type: ForwardType,
    pub host: SocketPath,
    pub guest: String,
}

/// Type of port forward
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForwardType {
    /// Unix domain socket forwarding (currently supported)
    UnixSocket,
    /// TCP port forwarding (reserved for future use)
    #[allow(dead_code)]
    Tcp,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum SocketPath {
    Static(String),
    Dynamic { detect: String },
}
