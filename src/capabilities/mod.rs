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

/// Convert capability phases by loading embedded scripts and marking them as capability phases.
///
/// This function:
/// 1. Loads embedded script_files and converts them to inline scripts
/// 2. Marks phases with CAPABILITY_ID and CLAUDE_VM_PHASE environment variables
/// 3. Returns converted phases ready for execution
///
/// # Arguments
/// * `phases` - The capability phases to convert
/// * `capability_id` - The capability ID (e.g., "git", "docker")
/// * `phase_type` - The phase type (e.g., "setup", "runtime", "before_setup")
fn convert_capability_phases(
    phases: &[crate::config::ScriptPhase],
    capability_id: &str,
    phase_type: &str,
) -> Result<Vec<crate::config::ScriptPhase>> {
    let mut converted = Vec::new();

    for phase in phases {
        let mut phase_copy = phase.clone();

        // Convert script_files to inline script
        // Capabilities use embedded scripts (include_str!) so we load them here
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
}

/// Get capability-defined phases and merge them with user-defined phases.
///
/// # Phase Ordering and Execution Guarantees
///
/// Phases execute **sequentially** within each phase type:
/// 1. **Setup phases** (`phase.setup`): Run during template creation
///    - Capability phases execute BEFORE user phases
///    - Within each group, phases execute in definition order
/// 2. **Runtime phases** (`phase.runtime`): Run before each Claude session
///    - Capability phases execute BEFORE user phases
///    - Within each group, phases execute in definition order
/// 3. **Host phases**: Run on the host machine (not inside VM)
///    - `before_setup`: Before VM setup starts
///    - `after_setup`: After VM setup completes, before template save
///    - `before_runtime`: Before VM runtime scripts execute
///    - `after_runtime`: After runtime scripts complete
///    - `teardown`: When session ends
///
/// All phases within a type run sequentially (not in parallel) to ensure:
/// - Deterministic execution order
/// - Safe state mutations
/// - Proper error propagation
///
/// # Arguments
/// * `config` - Configuration to merge capability phases into
///
/// # Example
/// ```ignore
/// // Capability phases run first, then user phases
/// // Given: capability defines setup phase "install-tools"
/// //        user defines setup phase "configure-project"
/// // Result: "install-tools" runs, then "configure-project"
/// ```
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
    let mut capability_cleanup_phases = Vec::new();

    for capability in enabled {
        let phase_config = &capability.phase;
        let capability_id = &capability.capability.id;

        // Convert all phase types using the extracted function
        capability_before_setup.extend(convert_capability_phases(
            &phase_config.before_setup,
            capability_id,
            "before_setup",
        )?);
        capability_after_setup.extend(convert_capability_phases(
            &phase_config.after_setup,
            capability_id,
            "after_setup",
        )?);
        capability_before_runtime.extend(convert_capability_phases(
            &phase_config.before_runtime,
            capability_id,
            "before_runtime",
        )?);
        capability_after_runtime.extend(convert_capability_phases(
            &phase_config.after_runtime,
            capability_id,
            "after_runtime",
        )?);
        capability_teardown.extend(convert_capability_phases(
            &phase_config.host.teardown,
            capability_id,
            "teardown",
        )?);
        capability_setup_phases.extend(convert_capability_phases(
            &phase_config.setup,
            capability_id,
            "setup",
        )?);
        capability_runtime_phases.extend(convert_capability_phases(
            &phase_config.runtime,
            capability_id,
            "runtime",
        )?);
        capability_cleanup_phases.extend(convert_capability_phases(
            &phase_config.cleanup,
            capability_id,
            "cleanup",
        )?);
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
    merge_phase_list(&mut config.phase.host.teardown, capability_teardown);
    merge_phase_list(&mut config.phase.setup, capability_setup_phases);
    merge_phase_list(&mut config.phase.runtime, capability_runtime_phases);
    merge_phase_list(&mut config.phase.cleanup, capability_cleanup_phases);

    Ok(())
}
