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

/// Configure MCP servers in the VM's .claude.json
pub fn configure_mcp_in_vm(project: &Project, servers: &[McpServer]) -> Result<()> {
    // Build jq commands to add each MCP server
    let mut jq_updates = Vec::new();

    for server in servers {
        let args_json = serde_json::to_string(&server.args).map_err(|e| {
            ClaudeVmError::InvalidConfig(format!("Failed to serialize MCP args: {}", e))
        })?;

        jq_updates.push(format!(
            r#".mcpServers["{}"] = {{"command": "{}", "args": {}}}"#,
            server.id, server.command, args_json
        ));
    }

    let jq_expr = jq_updates.join(" | ");

    let mcp_config_script = format!(
        r#"
CONFIG="$HOME/.claude.json"
if [ -f "$CONFIG" ]; then
  jq '{}' "$CONFIG" > "$CONFIG.tmp" && mv "$CONFIG.tmp" "$CONFIG"
else
  jq -n '{{}}' | jq '{}' > "$CONFIG"
fi
echo "MCP servers configured in $CONFIG"
"#,
        jq_expr, jq_expr
    );

    LimaCtl::shell(
        project.template_name(),
        None,
        "bash",
        &["-c", &mcp_config_script],
        false,
    )?;

    Ok(())
}

/// Execute repository setup scripts (adds custom apt sources before apt-get update)
pub fn execute_repository_setups(
    project: &Project,
    repo_setups: &[(String, String)],
) -> Result<()> {
    for (capability_id, setup_script) in repo_setups {
        println!("  Setting up repositories for {}...", capability_id);

        let template_name = project.template_name();

        // Execute the repo setup script with enhanced error context
        execute_vm_script(
            template_name,
            &ScriptConfig {
                script: Some(setup_script.clone()),
                script_file: None,
            },
            capability_id,
            false,
        )
        .map_err(|e| {
            ClaudeVmError::LimaExecution(format!(
                "Failed to setup {} repository: {}\n\n\
                 This error occurred while adding custom apt repositories.\n\n\
                 Common causes:\n\
                 • Network issues downloading GPG keys or repository lists\n\
                 • Firewall blocking access to repository servers\n\
                 • Changes in repository URLs or key locations\n\n\
                 Troubleshooting:\n\
                 1. Check network connectivity\n\
                 2. Run 'claude-vm shell' and manually execute the setup commands\n\
                 3. Verify the repository URLs are still valid\n\
                 4. Check if your network requires proxy configuration",
                capability_id, e
            ))
        })?;
    }

    Ok(())
}

/// Batch install system packages via apt (SINGLE apt-get update + install)
pub fn batch_install_system_packages(project: &Project, packages: &[String]) -> Result<()> {
    if packages.is_empty() {
        return Ok(());
    }

    let template_name = project.template_name();

    // Phase 1: Update package lists with detailed error context
    println!("  Running apt-get update...");
    LimaCtl::shell(
        template_name,
        None,
        "sudo",
        &["DEBIAN_FRONTEND=noninteractive", "apt-get", "update"],
        false,
    )
    .map_err(|e| {
        ClaudeVmError::LimaExecution(format!(
            "Failed to update package lists: {}\n\n\
             This error typically indicates:\n\
             • Network connectivity issues\n\
             • Invalid or unreachable repository URLs\n\
             • Repository GPG key verification failures\n\n\
             Troubleshooting steps:\n\
             1. Check your internet connection\n\
             2. Verify custom repositories were added correctly\n\
             3. Run 'claude-vm shell' and manually execute:\n\
                sudo apt-get update\n\
             4. Check /etc/apt/sources.list.d/ for malformed entries",
            e
        ))
    })?;

    // Phase 2: Install packages with detailed error context
    println!(
        "  Installing {} packages: {}",
        packages.len(),
        packages.join(", ")
    );
    println!("  (This may take several minutes for large packages)");

    // Build command: sudo DEBIAN_FRONTEND=noninteractive apt-get install -y pkg1 pkg2 ...
    let mut args = vec!["DEBIAN_FRONTEND=noninteractive", "apt-get", "install", "-y"];

    let package_refs: Vec<&str> = packages.iter().map(|s| s.as_str()).collect();
    args.extend(package_refs);

    LimaCtl::shell(template_name, None, "sudo", &args, false).map_err(|e| {
        ClaudeVmError::LimaExecution(format!(
            "Failed to install packages: {}\n\n\
             Attempted to install: {}\n\n\
             Common causes:\n\
             • Package name misspelled or doesn't exist\n\
             • Package available only in specific Debian versions\n\
             • Missing dependencies or conflicts with installed packages\n\
             • Insufficient disk space\n\n\
             Troubleshooting steps:\n\
             1. Verify package names are correct for Debian\n\
             2. Run 'claude-vm shell' and check package availability:\n\
                apt-cache search <package-name>\n\
             3. Try installing packages individually to identify the problematic one\n\
             4. Check available disk space: df -h",
            e,
            packages.join(", ")
        ))
    })?;

    println!("  ✓ System packages installed successfully");
    Ok(())
}
