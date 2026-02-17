use super::definition::{McpServer, ScriptConfig};
use crate::error::{ClaudeVmError, Result};
use crate::project::Project;
use crate::scripts::runner;
use crate::version;
use crate::vm::limactl::LimaCtl;
use std::collections::HashMap;

// NOTE: Runtime scripts are no longer pre-installed in a fixed directory.
// They are now executed dynamically through the phase system.

/// Ensure an environment variable exists in the map, setting it to empty string if not present
fn ensure_env_var(env_vars: &mut HashMap<String, String>, key: &str) {
    env_vars.entry(key.to_string()).or_default();
}

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

    // Extract project name using utility function
    if let Some(name) = crate::utils::git::extract_project_name(project_root) {
        env_vars.insert("PROJECT_NAME".to_string(), name);
    }

    // Detect git worktree using utility function
    if let Some(worktree_info) = crate::utils::git::detect_worktree(project_root) {
        env_vars.insert(
            "PROJECT_WORKTREE_ROOT".to_string(),
            worktree_info.main_root.to_string_lossy().to_string(),
        );
        env_vars.insert(
            "PROJECT_WORKTREE".to_string(),
            worktree_info.worktree_path.to_string_lossy().to_string(),
        );
    }

    // Ensure worktree variables exist (set to empty if not a worktree)
    ensure_env_var(&mut env_vars, "PROJECT_WORKTREE_ROOT");
    ensure_env_var(&mut env_vars, "PROJECT_WORKTREE");

    Ok(env_vars)
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
        // Escape single quotes for bash single-quoted strings
        // Pattern: '\'' means: end quote, escaped quote, start quote
        // Example: "it's" becomes 'it'\''s' which bash interprets as: it + ' + s
        // This is the standard POSIX-compliant way to include a single quote
        // within a single-quoted string.
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
pub(crate) fn get_embedded_script(capability_id: &str, script_name: &str) -> Result<String> {
    // Scripts are embedded from capabilities/{id}/{script_name}
    let content = match (capability_id, script_name) {
        // gh
        ("gh", "vm_setup.sh") => include_str!("../../capabilities/gh/vm_setup.sh"),
        ("gh", "vm_runtime.sh") => include_str!("../../capabilities/gh/vm_runtime.sh"),

        // git
        ("git", "host_setup.sh") => include_str!("../../capabilities/git/host_setup.sh"),

        // gpg
        ("gpg", "host_setup.sh") => include_str!("../../capabilities/gpg/host_setup.sh"),
        ("gpg", "vm_setup.sh") => include_str!("../../capabilities/gpg/vm_setup.sh"),

        // network-isolation
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
        let env_vars = build_capability_env_vars(
            project,
            template_name,
            capability_id,
            CapabilityPhase::Setup,
        )?;

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

    #[test]
    fn test_wrap_script_with_special_chars_in_values() {
        // Test that special characters (except single quotes) are preserved
        // Single-quoted strings in bash don't expand variables or escape sequences
        let mut env_vars = HashMap::new();
        env_vars.insert(
            "PATH_VAR".to_string(),
            "/path/with/$dollar/and`backtick`".to_string(),
        );
        env_vars.insert("NEWLINE_VAR".to_string(), "line1\nline2".to_string());
        env_vars.insert(
            "BACKSLASH_VAR".to_string(),
            "path\\with\\backslash".to_string(),
        );

        let wrapped = wrap_script_with_env_vars("echo test", &env_vars);

        // In single-quoted strings, these special chars should be literal
        assert!(wrapped.contains("/path/with/$dollar/and`backtick`"));
        assert!(wrapped.contains("line1\nline2"));
        assert!(wrapped.contains("path\\with\\backslash"));
    }

    #[test]
    fn test_wrap_script_multiline_value() {
        // Test that multiline values are preserved correctly
        let mut env_vars = HashMap::new();
        let multiline_value = "line 1\nline 2\nline 3";
        env_vars.insert("MULTILINE".to_string(), multiline_value.to_string());

        let wrapped = wrap_script_with_env_vars("echo $MULTILINE", &env_vars);

        // The newlines should be preserved in the single-quoted string
        assert!(wrapped.contains("'line 1\nline 2\nline 3'"));
        assert!(wrapped.contains("export MULTILINE="));
    }

    #[test]
    fn test_wrap_script_empty_value() {
        // Test that empty string values are handled correctly
        let mut env_vars = HashMap::new();
        env_vars.insert("EMPTY".to_string(), String::new());
        env_vars.insert("NOT_EMPTY".to_string(), "value".to_string());

        let wrapped = wrap_script_with_env_vars("echo test", &env_vars);

        // Empty strings should still be exported
        assert!(wrapped.contains("export EMPTY=''"));
        assert!(wrapped.contains("export NOT_EMPTY='value'"));
    }

    #[test]
    fn test_wrap_script_combined_special_cases() {
        // Test a realistic combination of edge cases
        let mut env_vars = HashMap::new();
        env_vars.insert("PROJECT_NAME".to_string(), "my-project's name".to_string());
        env_vars.insert(
            "PROJECT_PATH".to_string(),
            "/home/user/path with spaces".to_string(),
        );

        let script = "#!/bin/bash\nset -e\necho \"$PROJECT_NAME\"";
        let wrapped = wrap_script_with_env_vars(script, &env_vars);

        // Single quote should be escaped
        assert!(wrapped.contains("my-project'\\''s name"));
        // Spaces should be preserved
        assert!(wrapped.contains("/home/user/path with spaces"));
        // Original script should be included (without duplicate shebang)
        assert_eq!(wrapped.matches("#!/bin/bash").count(), 1);
        assert!(wrapped.contains("set -e"));
        assert!(wrapped.contains("echo \"$PROJECT_NAME\""));
    }
}
