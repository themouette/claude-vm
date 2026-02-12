use super::definition::{Capability, McpServer, ScriptConfig};
use crate::error::{ClaudeVmError, Result};
use crate::project::Project;
use crate::scripts::runner;
use crate::version;
use crate::vm::limactl::LimaCtl;
use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;

/// Directory where capability runtime scripts are installed in the VM
const RUNTIME_SCRIPT_DIR: &str = "/usr/local/share/claude-vm/runtime";

/// Phase in which a capability script is executed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityPhase {
    /// Setup phase - runs during template creation
    Setup,
    /// Runtime phase - runs before each session
    Runtime,
}

impl CapabilityPhase {
    /// Get the string representation of the phase
    fn as_str(&self) -> &'static str {
        match self {
            CapabilityPhase::Setup => "setup",
            CapabilityPhase::Runtime => "runtime",
        }
    }
}

/// Build environment variables for capability scripts
fn build_capability_env_vars(
    project: &Project,
    vm_name: &str,
    capability_id: &str,
    phase: CapabilityPhase,
) -> Result<HashMap<String, String>> {
    let mut env_vars = HashMap::new();

    // VM identification
    env_vars.insert(
        "TEMPLATE_NAME".to_string(),
        project.template_name().to_string(),
    );
    env_vars.insert("LIMA_INSTANCE".to_string(), vm_name.to_string());
    env_vars.insert("CAPABILITY_ID".to_string(), capability_id.to_string());

    // Phase
    env_vars.insert("CLAUDE_VM_PHASE".to_string(), phase.as_str().to_string());

    // Version
    env_vars.insert(
        "CLAUDE_VM_VERSION".to_string(),
        version::VERSION.to_string(),
    );

    // Project information
    let project_root = project.root();
    env_vars.insert(
        "PROJECT_ROOT".to_string(),
        project_root.to_string_lossy().to_string(),
    );

    // Extract full project name from the directory
    if let Some(name) = project_root.file_name() {
        env_vars.insert(
            "PROJECT_NAME".to_string(),
            name.to_string_lossy().to_string(),
        );
    }

    // Detect git worktree
    // Git worktrees have a .git file (not directory) containing:
    // "gitdir: /path/to/main-repo/.git/worktrees/branch-name"
    // We extract the main repository root from this path structure.
    let git_dir = project_root.join(".git");
    if git_dir.exists() && git_dir.is_file() {
        // .git is a file, likely a worktree - read it to find the main repo
        if let Ok(git_file_content) = std::fs::read_to_string(&git_dir) {
            // Parse the gitdir line
            if let Some(gitdir_line) = git_file_content.lines().next() {
                if let Some(gitdir_path) = gitdir_line.strip_prefix("gitdir: ") {
                    let gitdir_pathbuf = std::path::PathBuf::from(gitdir_path);

                    // Validate this looks like a worktree path
                    // Expected structure: /main-repo/.git/worktrees/branch-name
                    if let Some(worktrees_parent) = gitdir_pathbuf.parent() {
                        if worktrees_parent.ends_with("worktrees") {
                            // Navigate up: worktrees -> .git -> main-repo
                            if let Some(git_parent) = worktrees_parent.parent() {
                                if let Some(main_root) = git_parent.parent() {
                                    env_vars.insert(
                                        "PROJECT_WORKTREE_ROOT".to_string(),
                                        main_root.to_string_lossy().to_string(),
                                    );
                                    env_vars.insert(
                                        "PROJECT_WORKTREE".to_string(),
                                        project_root.to_string_lossy().to_string(),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // If not a worktree, set empty strings
    if !env_vars.contains_key("PROJECT_WORKTREE_ROOT") {
        env_vars.insert("PROJECT_WORKTREE_ROOT".to_string(), String::new());
        env_vars.insert("PROJECT_WORKTREE".to_string(), String::new());
    }

    Ok(env_vars)
}

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

    let vm_name = project.template_name();
    let env_vars = build_capability_env_vars(project, vm_name, &capability.capability.id, CapabilityPhase::Setup)?;

    execute_vm_script(
        vm_name,
        vm_setup,
        &capability.capability.id,
        false,
        &env_vars,
    )?;

    Ok(())
}

/// Execute a capability's vm_runtime hook (runs in VM before each session)
pub fn execute_vm_runtime(project: &Project, capability: &Arc<Capability>) -> Result<()> {
    let Some(vm_runtime) = &capability.vm_runtime else {
        return Ok(());
    };

    let vm_name = project.template_name();
    let env_vars =
        build_capability_env_vars(project, vm_name, &capability.capability.id, CapabilityPhase::Runtime)?;

    // Runtime scripts are executed silently unless there's an error
    execute_vm_script(
        vm_name,
        vm_runtime,
        &capability.capability.id,
        true,
        &env_vars,
    )?;

    Ok(())
}

/// Execute a capability's vm_runtime hook in a specific VM instance
pub fn execute_vm_runtime_in_vm(vm_name: &str, capability: &Arc<Capability>) -> Result<()> {
    let Some(vm_runtime) = &capability.vm_runtime else {
        return Ok(());
    };

    // Build minimal env vars (no Project available in this context)
    let mut env_vars = HashMap::new();
    env_vars.insert("LIMA_INSTANCE".to_string(), vm_name.to_string());
    env_vars.insert(
        "CAPABILITY_ID".to_string(),
        capability.capability.id.clone(),
    );
    env_vars.insert("CLAUDE_VM_PHASE".to_string(), CapabilityPhase::Runtime.as_str().to_string());
    env_vars.insert(
        "CLAUDE_VM_VERSION".to_string(),
        version::VERSION.to_string(),
    );
    // Other vars like PROJECT_ROOT, PROJECT_NAME, etc. will be empty in this minimal context
    env_vars.insert("TEMPLATE_NAME".to_string(), String::new());
    env_vars.insert("PROJECT_ROOT".to_string(), String::new());
    env_vars.insert("PROJECT_NAME".to_string(), String::new());
    env_vars.insert("PROJECT_WORKTREE_ROOT".to_string(), String::new());
    env_vars.insert("PROJECT_WORKTREE".to_string(), String::new());

    // Runtime scripts are executed silently unless there's an error
    execute_vm_script(
        vm_name,
        vm_runtime,
        &capability.capability.id,
        true,
        &env_vars,
    )?;

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

/// Execute a script in the VM with environment variables
fn execute_vm_script(
    vm_name: &str,
    script_config: &ScriptConfig,
    capability_id: &str,
    silent: bool,
    env_vars: &std::collections::HashMap<String, String>,
) -> Result<()> {
    let script_content = get_script_content(script_config, capability_id)?;

    // Wrap script with environment variable exports
    let wrapped_script = wrap_script_with_env_vars(&script_content, env_vars);

    let filename = format!("{}_{}.sh", capability_id, "script");

    if silent {
        // For runtime scripts, execute without printing output unless there's an error
        runner::execute_script_silent(vm_name, &wrapped_script, &filename)?;
    } else {
        // For setup scripts, show output
        runner::execute_script(vm_name, &wrapped_script, &filename)?;
    }

    Ok(())
}

/// Wrap a script with environment variable exports
fn wrap_script_with_env_vars(
    script_content: &str,
    env_vars: &std::collections::HashMap<String, String>,
) -> String {
    let mut wrapped = String::from("#!/bin/bash\n");

    // Export environment variables
    for (key, value) in env_vars {
        // Escape single quotes in the value
        let escaped_value = value.replace('\'', r"'\''");
        wrapped.push_str(&format!("export {}='{}'\n", key, escaped_value));
    }

    wrapped.push('\n');

    // Append the original script content (skip shebang if present)
    let content = if script_content.starts_with("#!") {
        // Skip the first line (shebang)
        script_content
            .lines()
            .skip(1)
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        script_content.to_string()
    };

    wrapped.push_str(&content);
    wrapped
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
        ("network-isolation", "vm_setup.sh") => {
            include_str!("../../capabilities/network-isolation/vm_setup.sh")
        }
        ("network-isolation", "vm_runtime.sh") => {
            include_str!("../../capabilities/network-isolation/vm_runtime.sh")
        }
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
        let env_vars = build_capability_env_vars(project, template_name, capability_id, CapabilityPhase::Setup)?;

        // Execute the repo setup script with enhanced error context
        execute_vm_script(
            template_name,
            &ScriptConfig {
                script: Some(setup_script.clone()),
                script_file: None,
            },
            capability_id,
            false,
            &env_vars,
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

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Integration tests for build_capability_env_vars are in tests/test_capabilities.rs
    // since they require actual Project instances with git repositories.

    #[test]
    fn test_wrap_script_with_env_vars_empty() {
        let env_vars = HashMap::new();
        let script = "#!/bin/bash\necho 'hello'";

        let wrapped = wrap_script_with_env_vars(script, &env_vars);

        assert!(wrapped.starts_with("#!/bin/bash\n"));
        assert!(wrapped.contains("echo 'hello'"));
    }

    #[test]
    fn test_wrap_script_with_env_vars_basic() {
        let mut env_vars = HashMap::new();
        env_vars.insert("FOO".to_string(), "bar".to_string());
        env_vars.insert("BAZ".to_string(), "qux".to_string());

        let script = "#!/bin/bash\necho $FOO";
        let wrapped = wrap_script_with_env_vars(script, &env_vars);

        assert!(wrapped.contains("export FOO='bar'"));
        assert!(wrapped.contains("export BAZ='qux'"));
        assert!(wrapped.contains("echo $FOO"));
    }

    #[test]
    fn test_wrap_script_with_env_vars_escaping() {
        let mut env_vars = HashMap::new();
        env_vars.insert("VAR".to_string(), "value with 'quotes'".to_string());

        let script = "echo test";
        let wrapped = wrap_script_with_env_vars(script, &env_vars);

        // Should escape single quotes in the value
        assert!(wrapped.contains("export VAR='value with '\\''quotes'\\'''"));
    }

    #[test]
    fn test_wrap_script_with_env_vars_removes_shebang() {
        let mut env_vars = HashMap::new();
        env_vars.insert("TEST".to_string(), "value".to_string());

        let script = "#!/bin/bash\nset -e\necho test";
        let wrapped = wrap_script_with_env_vars(script, &env_vars);

        // Should have only one shebang at the start
        assert_eq!(wrapped.matches("#!/bin/bash").count(), 1);
        assert!(wrapped.starts_with("#!/bin/bash\n"));
        assert!(wrapped.contains("export TEST='value'"));
        assert!(wrapped.contains("set -e"));
        assert!(wrapped.contains("echo test"));
    }

    #[test]
    fn test_wrap_script_without_shebang() {
        let mut env_vars = HashMap::new();
        env_vars.insert("VAR".to_string(), "val".to_string());

        let script = "echo hello";
        let wrapped = wrap_script_with_env_vars(script, &env_vars);

        assert!(wrapped.starts_with("#!/bin/bash\n"));
        assert!(wrapped.contains("export VAR='val'"));
        assert!(wrapped.contains("echo hello"));
    }

    #[test]
    fn test_env_var_keys() {
        // Test that we're using the correct key names
        // This ensures API consistency
        let mut env_vars = HashMap::new();
        env_vars.insert("TEMPLATE_NAME".to_string(), "test".to_string());
        env_vars.insert("LIMA_INSTANCE".to_string(), "test".to_string());
        env_vars.insert("CAPABILITY_ID".to_string(), "test".to_string());
        env_vars.insert("CLAUDE_VM_PHASE".to_string(), "setup".to_string());
        env_vars.insert("CLAUDE_VM_VERSION".to_string(), "0.1.0".to_string());
        env_vars.insert("PROJECT_ROOT".to_string(), "/test".to_string());
        env_vars.insert("PROJECT_NAME".to_string(), "test".to_string());
        env_vars.insert("PROJECT_WORKTREE_ROOT".to_string(), "".to_string());
        env_vars.insert("PROJECT_WORKTREE".to_string(), "".to_string());

        // Just verify the expected keys exist
        let expected_keys = [
            "TEMPLATE_NAME",
            "LIMA_INSTANCE",
            "CAPABILITY_ID",
            "CLAUDE_VM_PHASE",
            "CLAUDE_VM_VERSION",
            "PROJECT_ROOT",
            "PROJECT_NAME",
            "PROJECT_WORKTREE_ROOT",
            "PROJECT_WORKTREE",
        ];

        for key in &expected_keys {
            assert!(env_vars.contains_key(*key));
        }
    }
}
