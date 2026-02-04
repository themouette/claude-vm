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
        executor::execute_host_setup(project, &capability, config)?;
    }

    Ok(())
}

/// Execute all enabled capabilities' vm_setup hooks in VM
pub fn execute_vm_setup(project: &Project, config: &Config) -> Result<()> {
    let registry = registry::CapabilityRegistry::load()?;
    let enabled = registry.get_enabled_capabilities(config)?;

    for capability in enabled {
        executor::execute_vm_setup(project, &capability, config)?;
    }

    Ok(())
}

/// Execute all enabled capabilities' vm_runtime hooks in VM
/// vm_name: The actual VM instance name (e.g., ephemeral session name)
pub fn execute_vm_runtime(vm_name: &str, config: &Config) -> Result<()> {
    let registry = registry::CapabilityRegistry::load()?;
    let enabled = registry.get_enabled_capabilities(config)?;

    for capability in enabled {
        executor::execute_vm_runtime_in_vm(vm_name, &capability, config)?;
    }

    Ok(())
}

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

/// Install vm_runtime scripts into the template
pub fn install_vm_runtime_scripts(project: &Project, config: &Config) -> Result<()> {
    let registry = registry::CapabilityRegistry::load()?;
    let enabled = registry.get_enabled_capabilities(config)?;

    // Filter capabilities that have vm_runtime scripts
    let capabilities_with_runtime: Vec<_> = enabled
        .into_iter()
        .filter(|cap| cap.vm_runtime.is_some())
        .collect();

    if capabilities_with_runtime.is_empty() {
        return Ok(());
    }

    println!("Installing runtime scripts into template...");
    executor::install_vm_runtime_scripts_to_template(project, &capabilities_with_runtime)?;

    Ok(())
}

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
