//! Capability system for extending claude-vm functionality.
//!
//! This module provides a declarative, TOML-based system for defining and managing
//! capabilities (like Docker, Node, Python, GPG) that can be enabled in VMs.
//!
//! # Architecture
//!
//! Capabilities define lifecycle hooks using the phase system:
//! - **phase.host.***: Host-side phases (before_setup, after_setup, before_runtime, after_runtime, teardown)
//! - **phase.setup**: VM setup phases (run during template creation)
//! - **phase.runtime**: VM runtime phases (run before each session)
//!
//! # Example
//!
//! ```toml
//! [capability]
//! id = "gpg"
//! name = "GPG Agent Forwarding"
//! description = "Forward GPG agent from host to VM"
//!
//! [[phase.host.before_setup]]
//! name = "export-gpg-keys"
//! script_files = ["host_setup.sh"]
//!
//! [[phase.setup]]
//! name = "gpg-import-keys"
//! script_files = ["vm_setup.sh"]
//!
//! [[phase.runtime]]
//! name = "gpg-environment"
//! script = "export GPG_TTY=$(tty)"
//!
//! [[forwards]]
//! type = "unix_socket"
//! host = { detect = "gpgconf --list-dir agent-extra-socket" }
//! guest = "/tmp/gpg-agent.socket"
//! ```

pub mod definition;
pub mod executor;
pub mod registry;

use crate::config::Config;
use crate::error::Result;
use crate::project::Project;
use crate::vm::port_forward::PortForward;

// NOTE: All lifecycle hooks (host and VM) are now handled through the phase system
// See merge_capability_phases() which merges capability phases with user phases

/// Get all MCP servers from enabled capabilities
pub fn get_mcp_servers(config: &Config) -> Result<Vec<definition::McpServer>> {
    let registry = registry::CapabilityRegistry::load()?;
    registry.get_mcp_servers(config)
}

/// Configure all MCP servers in the VM's .claude.json
pub fn configure_mcp_servers(project: &Project, config: &Config) -> Result<()> {
    let servers = get_mcp_servers(config)?;

    if servers.is_empty() {
        return Ok(());
    }

    println!("Configuring MCP servers...");
    executor::configure_mcp_in_vm(project, &servers)?;

    Ok(())
}

// NOTE: Runtime script installation is now handled through the phase system
// Capability runtime phases are merged into config and executed dynamically
// This eliminates the need to pre-install scripts into the template

/// Get all port forwards from enabled capabilities
pub fn get_port_forwards(config: &Config) -> Result<Vec<PortForward>> {
    let registry = registry::CapabilityRegistry::load()?;
    let enabled = registry.get_enabled_capabilities(config)?;

    let mut port_forwards = Vec::new();

    for capability in enabled {
        for forward in &capability.forwards {
            // Detect socket path if needed
            let host_socket = match &forward.host {
                definition::SocketPath::Static(path) => path.clone(),
                definition::SocketPath::Dynamic { detect } => {
                    PortForward::detect_socket_path(detect)?
                }
            };

            port_forwards.push(PortForward::unix_socket(
                host_socket,
                forward.guest.clone(),
            )?);
        }
    }

    Ok(port_forwards)
}

/// Setup all custom repositories from enabled capabilities.
/// This runs BEFORE apt-get update to add custom sources (Docker, Node, gh, etc.)
pub fn setup_repositories(project: &Project, config: &Config) -> Result<()> {
    let registry = registry::CapabilityRegistry::load()?;
    let repo_setups = registry.get_repo_setups(config)?;

    if repo_setups.is_empty() {
        return Ok(());
    }

    println!("Setting up package repositories...");
    executor::execute_repository_setups(project, &repo_setups)?;

    Ok(())
}

/// Batch install all system packages from capabilities and config.
/// This runs a SINGLE apt-get update + install for all packages.
pub fn install_system_packages(project: &Project, config: &Config) -> Result<()> {
    let registry = registry::CapabilityRegistry::load()?;
    let packages = registry.collect_system_packages(config)?;

    if packages.is_empty() {
        return Ok(());
    }

    println!("Installing system packages: {}", packages.join(", "));
    executor::batch_install_system_packages(project, &packages)?;

    Ok(())
}

/// Get capability-defined phases and merge them with user-defined phases.
/// Capability phases are inserted BEFORE user phases to ensure proper initialization.
pub fn merge_capability_phases(config: &mut Config) -> Result<()> {
    let registry = registry::CapabilityRegistry::load()?;
    let enabled = registry.get_enabled_capabilities(config)?;

    // Collect all phase types
    let mut capability_before_setup = Vec::new();
    let mut capability_after_setup = Vec::new();
    let mut capability_before_runtime = Vec::new();
    let mut capability_after_runtime = Vec::new();
    let mut capability_teardown = Vec::new();
    let mut capability_setup_phases = Vec::new();
    let mut capability_runtime_phases = Vec::new();

    for capability in enabled {
        let phase_config = &capability.phase;
        let capability_id = &capability.capability.id;

        // Convert embedded script_files to inline scripts for capability phases
        // Capabilities use embedded scripts (include_str!) so we need to load them
        // and convert to inline format for the phase system

        // Helper closure to convert phases
        let convert_phases = |phases: &[crate::config::ScriptPhase],
                              phase_type: &str|
         -> Result<Vec<crate::config::ScriptPhase>> {
            let mut converted = Vec::new();
            for phase in phases {
                let mut phase_copy = phase.clone();
                // Convert script_files to inline script
                if !phase_copy.script_files.is_empty() {
                    let mut combined_script = String::new();
                    for script_file in &phase_copy.script_files {
                        let content = executor::get_embedded_script(capability_id, script_file)?;
                        combined_script.push_str(&content);
                        combined_script.push('\n');
                    }
                    phase_copy.script = Some(combined_script);
                    phase_copy.script_files.clear();
                }

                // Mark this as a capability phase by adding CAPABILITY_ID
                // This signals to the phase executor to inject full capability env vars
                phase_copy
                    .env
                    .insert("CAPABILITY_ID".to_string(), capability_id.to_string());
                phase_copy
                    .env
                    .insert("CLAUDE_VM_PHASE".to_string(), phase_type.to_string());

                converted.push(phase_copy);
            }
            Ok(converted)
        };

        // Convert all phase types
        capability_before_setup.extend(convert_phases(&phase_config.before_setup, "before_setup")?);
        capability_after_setup.extend(convert_phases(&phase_config.after_setup, "after_setup")?);
        capability_before_runtime.extend(convert_phases(
            &phase_config.before_runtime,
            "before_runtime",
        )?);
        capability_after_runtime.extend(convert_phases(
            &phase_config.after_runtime,
            "after_runtime",
        )?);
        capability_teardown.extend(convert_phases(&phase_config.teardown, "teardown")?);
        capability_setup_phases.extend(convert_phases(&phase_config.setup, "setup")?);
        capability_runtime_phases.extend(convert_phases(&phase_config.runtime, "runtime")?);
    }

    // Helper to merge phase lists (capability phases BEFORE user phases)
    let merge_phase_list =
        |config_phases: &mut Vec<crate::config::ScriptPhase>,
         capability_phases: Vec<crate::config::ScriptPhase>| {
            if !capability_phases.is_empty() {
                let user_phases = std::mem::take(config_phases);
                *config_phases = capability_phases;
                config_phases.extend(user_phases);
            }
        };

    // Merge all phase types: capability phases BEFORE user phases
    // This ensures capabilities initialize before user-defined scripts run
    merge_phase_list(&mut config.phase.before_setup, capability_before_setup);
    merge_phase_list(&mut config.phase.after_setup, capability_after_setup);
    merge_phase_list(&mut config.phase.before_runtime, capability_before_runtime);
    merge_phase_list(&mut config.phase.after_runtime, capability_after_runtime);
    merge_phase_list(&mut config.phase.teardown, capability_teardown);
    merge_phase_list(&mut config.phase.setup, capability_setup_phases);
    merge_phase_list(&mut config.phase.runtime, capability_runtime_phases);

    Ok(())
}
