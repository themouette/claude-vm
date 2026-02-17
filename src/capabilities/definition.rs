//! Data structures for parsing capability TOML files.
//!
//! These types define the schema for capability definitions.

use crate::config::PhaseConfig;
use serde::Deserialize;

/// A capability definition loaded from a TOML file.
///
/// Capabilities define optional lifecycle hooks, MCP servers, and port forwards.
#[derive(Debug, Clone, Deserialize)]
pub struct Capability {
    /// Capability metadata (id, name, description, dependencies)
    pub capability: CapabilityMeta,

    /// Package specifications (system packages, optional repo setup)
    #[serde(default)]
    pub packages: Option<PackageSpec>,

    /// Optional host setup script (runs on host during setup)
    /// Host setup is required for operations that need to run on the host machine
    /// (e.g., copying host git config, exporting GPG keys)
    #[serde(default)]
    pub host_setup: Option<ScriptConfig>,

    /// Phase-based execution model for VM setup and runtime scripts
    /// All VM-side operations now use phases for better control and debugging
    pub phase: PhaseConfig,

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

/// Package specifications for a capability.
///
/// The `system` field lists packages to install via apt. If these packages
/// are not available in the default Debian repositories, use `setup_script`
/// to add custom apt sources first.
///
/// ## Version Pinning
///
/// Package names support version constraints using standard Debian syntax:
/// - `package=1.2.3` - Exact version
/// - `package=1.2.*` - Version wildcard
/// - `package>=1.2.0` - Minimum version (requires apt policy)
///
/// Examples:
/// ```toml
/// [packages]
/// system = [
///     "python3",              # Latest available version
///     "nodejs=22.*",          # Node 22.x (any patch version)
///     "docker-ce=5:24.0.0-1", # Exact Docker version with epoch
/// ]
/// ```
///
/// ## Execution Order
///
/// 1. All `setup_script`s run (adds custom repositories/GPG keys)
/// 2. Single `apt-get update` runs (refreshes package lists)
/// 3. Single `apt-get install` runs (installs all system packages)
/// 4. `vm_setup` scripts run (for post-install configuration)
#[derive(Debug, Clone, Deserialize)]
pub struct PackageSpec {
    /// System packages to install via apt.
    ///
    /// Supports version pinning with Debian apt syntax:
    /// - "package" - latest version
    /// - "package=1.2.3" - exact version
    /// - "package=1.2.*" - version wildcard
    /// - "package:amd64" - specific architecture
    #[serde(default)]
    pub system: Vec<String>,

    /// Optional script to run before apt-get update (adds custom repos, GPG keys).
    /// Only needed when packages are NOT in default Debian repositories.
    ///
    /// Example: Docker needs a custom repository setup
    #[serde(default)]
    pub setup_script: Option<String>,
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
