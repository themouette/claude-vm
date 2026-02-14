//! Capability system for extending claude-vm functionality.
//!
//! This module provides a declarative, TOML-based system for defining and managing
//! capabilities (like Docker, Node, Python, GPG) that can be enabled in VMs.
//!
//! # Architecture
//!
//! Capabilities define three lifecycle hooks:
//! - **host_setup**: Runs on the host machine during `claude-vm setup`
//! - **vm_setup**: Runs in the VM during template creation
//! - **vm_runtime**: Installed to `/usr/local/share/claude-vm/runtime/` and sourced on every session
//!
//! # Example
//!
//! ```toml
//! [capability]
//! id = "gpg"
//! name = "GPG Agent Forwarding"
//! description = "Forward GPG agent from host to VM"
//!
//! [host_setup]
//! script_file = "host_setup.sh"
//!
//! [vm_setup]
//! script_file = "vm_setup.sh"
//!
//! [vm_runtime]
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

/// Execute all enabled capabilities' host setup hooks
pub fn execute_host_setup(project: &Project, config: &Config) -> Result<()> {
    let registry = registry::CapabilityRegistry::load()?;
    let enabled = registry.get_enabled_capabilities(config)?;

    for capability in enabled {
        executor::execute_host_setup(project, &capability)?;
    }

    Ok(())
}

// NOTE: VM setup and runtime are now handled through the phase system
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

    let mut capability_setup_phases = Vec::new();
    let mut capability_runtime_phases = Vec::new();

    for capability in enabled {
        let phase_config = &capability.phase;
        let capability_id = &capability.capability.id;

        // Convert embedded script_files to inline scripts for capability phases
        // Capabilities use embedded scripts (include_str!) so we need to load them
        // and convert to inline format for the phase system

        // Add capability setup phases
        for phase in &phase_config.setup {
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
            capability_setup_phases.push(phase_copy);
        }

        // Add capability runtime phases
        for phase in &phase_config.runtime {
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
            capability_runtime_phases.push(phase_copy);
        }
    }

    // Merge: capability phases BEFORE user phases
    // This ensures capabilities initialize before user-defined scripts run
    if !capability_setup_phases.is_empty() {
        let user_setup = std::mem::take(&mut config.phase.setup);
        config.phase.setup = capability_setup_phases;
        config.phase.setup.extend(user_setup);
    }

    if !capability_runtime_phases.is_empty() {
        let user_runtime = std::mem::take(&mut config.phase.runtime);
        config.phase.runtime = capability_runtime_phases;
        config.phase.runtime.extend(user_runtime);
    }

    Ok(())
}
