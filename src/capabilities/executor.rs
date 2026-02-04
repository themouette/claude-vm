use super::definition::{Capability, McpServer, ScriptConfig};
use crate::error::{ClaudeVmError, Result};
use crate::project::Project;
use crate::scripts::runner;
use crate::vm::limactl::LimaCtl;
use std::process::Command;
use std::sync::Arc;

/// Directory where capability runtime scripts are installed in the VM
const RUNTIME_SCRIPT_DIR: &str = "/usr/local/share/claude-vm/runtime";

/// Execute a capability's host_setup hook (runs on host machine)
pub fn execute_host_setup(project: &Project, capability: &Arc<Capability>) -> Result<()> {
    let Some(host_setup) = &capability.host_setup else {
        return Ok(());
    };

    println!("Running host setup for {}...", capability.capability.name);

    execute_host_script(project, host_setup, &capability.capability.id)?;

    Ok(())
}

/// Execute a capability's vm_setup hook (runs in VM)
pub fn execute_vm_setup(project: &Project, capability: &Arc<Capability>) -> Result<()> {
    let Some(vm_setup) = &capability.vm_setup else {
        return Ok(());
    };

    println!("Setting up {}...", capability.capability.name);

    execute_vm_script(
        project.template_name(),
        vm_setup,
        &capability.capability.id,
        false,
    )?;

    Ok(())
}

/// Execute a capability's vm_runtime hook (runs in VM before each session)
pub fn execute_vm_runtime(project: &Project, capability: &Arc<Capability>) -> Result<()> {
    let Some(vm_runtime) = &capability.vm_runtime else {
        return Ok(());
    };

    // Runtime scripts are executed silently unless there's an error
    execute_vm_script(
        project.template_name(),
        vm_runtime,
        &capability.capability.id,
        true,
    )?;

    Ok(())
}

/// Execute a capability's vm_runtime hook in a specific VM instance
pub fn execute_vm_runtime_in_vm(vm_name: &str, capability: &Arc<Capability>) -> Result<()> {
    let Some(vm_runtime) = &capability.vm_runtime else {
        return Ok(());
    };

    // Runtime scripts are executed silently unless there's an error
    execute_vm_script(vm_name, vm_runtime, &capability.capability.id, true)?;

    Ok(())
}

/// Install vm_runtime scripts into the template at /usr/local/share/claude-vm/runtime/
pub fn install_vm_runtime_scripts_to_template(
    project: &Project,
    capabilities: &[Arc<Capability>],
) -> Result<()> {
    let template_name = project.template_name();

    // Create runtime directory in template
    LimaCtl::shell(
        template_name,
        None,
        "sudo",
        &["mkdir", "-p", RUNTIME_SCRIPT_DIR],
        false,
    )?;

    // Install each capability's vm_runtime script
    for capability in capabilities {
        let Some(vm_runtime) = &capability.vm_runtime else {
            continue;
        };

        let script_content = get_script_content(vm_runtime, &capability.capability.id)?;
        let script_name = format!("{}.sh", capability.capability.id);
        let temp_path = format!("/tmp/claude-vm-runtime-{}", script_name);
        let install_path = format!("{}/{}", RUNTIME_SCRIPT_DIR, script_name);

        // Write script to temp file on host
        let local_temp = std::env::temp_dir().join(&script_name);
        std::fs::write(&local_temp, &script_content)?;

        // Ensure cleanup happens even on error
        let result = (|| -> Result<()> {
            // Copy to VM temp location
            LimaCtl::copy(&local_temp, template_name, &temp_path)?;

            // Move to final location with sudo (overwrites if exists - idempotent)
            LimaCtl::shell(
                template_name,
                None,
                "sudo",
                &["mv", "-f", &temp_path, &install_path],
                false,
            )?;

            // Make executable
            LimaCtl::shell(
                template_name,
                None,
                "sudo",
                &["chmod", "+x", &install_path],
                false,
            )?;

            Ok(())
        })();

        // Always cleanup local temp file
        let _ = std::fs::remove_file(&local_temp);

        // Propagate error after cleanup
        result?;

        println!("  ✓ Installed {}", script_name);
    }

    Ok(())
}

/// Execute a script on the host machine
fn execute_host_script(
    project: &Project,
    script_config: &ScriptConfig,
    capability_id: &str,
) -> Result<()> {
    let script_content = get_script_content(script_config, capability_id)?;

    // Set up environment variables
    let project_root = project.root().to_string_lossy();
    let template_name = project.template_name();

    let output = Command::new("bash")
        .arg("-c")
        .arg(&script_content)
        .env("PROJECT_ROOT", project_root.as_ref())
        .env("TEMPLATE_NAME", template_name)
        .env("LIMA_INSTANCE", template_name)
        .env("CAPABILITY_ID", capability_id)
        .output()
        .map_err(|e| {
            ClaudeVmError::LimaExecution(format!(
                "Failed to execute host script for capability '{}': {}",
                capability_id, e
            ))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ClaudeVmError::LimaExecution(format!(
            "Host setup failed for capability '{}': {}",
            capability_id, stderr
        )));
    }

    // Print stdout if any
    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }

    Ok(())
}

/// Execute a script in the VM
fn execute_vm_script(
    vm_name: &str,
    script_config: &ScriptConfig,
    capability_id: &str,
    silent: bool,
) -> Result<()> {
    let script_content = get_script_content(script_config, capability_id)?;

    let filename = format!("{}_{}.sh", capability_id, "script");

    if silent {
        // For runtime scripts, execute without printing output unless there's an error
        runner::execute_script_silent(vm_name, &script_content, &filename)?;
    } else {
        // For setup scripts, show output
        runner::execute_script(vm_name, &script_content, &filename)?;
    }

    Ok(())
}

/// Get script content from config (either inline or from embedded file)
fn get_script_content(script_config: &ScriptConfig, capability_id: &str) -> Result<String> {
    if let Some(inline) = &script_config.script {
        return Ok(inline.clone());
    }

    if let Some(file) = &script_config.script_file {
        return get_embedded_script(capability_id, file);
    }

    Err(ClaudeVmError::InvalidConfig(
        "Script config must have either 'script' or 'script_file'".to_string(),
    ))
}

/// Get embedded script content by capability ID and script filename
fn get_embedded_script(capability_id: &str, script_name: &str) -> Result<String> {
    // Scripts are now embedded from capabilities/{id}/{script_name}
    let content = match (capability_id, script_name) {
        ("docker", "vm_setup.sh") => include_str!("../../capabilities/docker/vm_setup.sh"),
        ("node", "vm_setup.sh") => include_str!("../../capabilities/node/vm_setup.sh"),
        ("python", "vm_setup.sh") => include_str!("../../capabilities/python/vm_setup.sh"),
        ("chromium", "vm_setup.sh") => include_str!("../../capabilities/chromium/vm_setup.sh"),
        ("gh", "vm_setup.sh") => include_str!("../../capabilities/gh/vm_setup.sh"),
        ("gpg", "host_setup.sh") => include_str!("../../capabilities/gpg/host_setup.sh"),
        ("gpg", "vm_setup.sh") => include_str!("../../capabilities/gpg/vm_setup.sh"),
        ("git", "host_setup.sh") => include_str!("../../capabilities/git/host_setup.sh"),
        _ => {
            return Err(ClaudeVmError::InvalidConfig(format!(
                "Embedded script '{}' not found for capability '{}'",
                script_name, capability_id
            )))
        }
    };

    Ok(content.to_string())
}

/// Register MCP servers in the VM's MCP registry
/// Each capability writes its MCP servers to /usr/local/share/claude-vm/mcp.d/{capability}.json
pub fn register_mcp_servers(
    project: &Project,
    capability_id: &str,
    servers: &[McpServer],
) -> Result<()> {
    if servers.is_empty() {
        return Ok(());
    }

    // Build JSON for this capability's MCP servers
    let mut mcp_servers = serde_json::Map::new();
    for server in servers {
        let server_config = serde_json::json!({
            "command": server.command,
            "args": server.args
        });
        mcp_servers.insert(server.id.clone(), server_config);
    }

    let registry_entry = serde_json::json!({
        "mcpServers": mcp_servers
    });

    let registry_json = serde_json::to_string_pretty(&registry_entry).map_err(|e| {
        ClaudeVmError::InvalidConfig(format!("Failed to serialize MCP registry: {}", e))
    })?;

    // Write to VM's MCP registry
    let script = format!(
        r#"
# Register MCP servers for capability: {}
REGISTRY_DIR="/usr/local/share/claude-vm/mcp.d"
sudo mkdir -p "$REGISTRY_DIR"

cat > /tmp/mcp-{}.json << 'EOF'
{}
EOF

sudo mv /tmp/mcp-{}.json "$REGISTRY_DIR/{}.json"
echo "MCP servers for {} registered in $REGISTRY_DIR/{}.json"
"#,
        capability_id,
        capability_id,
        registry_json,
        capability_id,
        capability_id,
        capability_id,
        capability_id
    );

    LimaCtl::shell(
        project.template_name(),
        None,
        "bash",
        &["-c", &script],
        false,
    )?;

    Ok(())
}
