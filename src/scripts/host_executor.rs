//! Host phase executor - runs scripts on the HOST machine (not inside VM)
//!
//! This module handles execution of host-side lifecycle hooks that need to run
//! on the host machine rather than inside the VM.

use crate::config::ScriptPhase;
use crate::error::{ClaudeVmError, Result};
use crate::project::Project;
use std::collections::HashMap;
use std::process::Command;

/// Execute host phases on the host machine
///
/// This function executes a list of script phases on the host machine (not inside the VM).
/// Each phase can contain inline scripts and/or script files.
///
/// # Arguments
/// * `phases` - List of script phases to execute
/// * `project` - Project context for resolving script paths
/// * `vm_name` - VM name (passed to scripts via environment variable)
/// * `env_vars` - Additional environment variables to pass to scripts
///
/// # Errors
/// Returns error if:
/// - Script execution fails (unless continue_on_error is true)
/// - Script file cannot be read
/// - Condition check fails
pub fn execute_host_phases(
    phases: &[ScriptPhase],
    project: &Project,
    vm_name: &str,
    env_vars: &HashMap<String, String>,
) -> Result<()> {
    for phase in phases {
        println!("\n━━━ Host Phase: {} ━━━", phase.name);

        // Check conditional execution
        if let Some(condition) = &phase.when {
            if !check_host_condition(condition, env_vars)? {
                println!("⊘ Skipped (condition not met)");
                continue;
            }
        }

        // Build environment
        let mut phase_env = env_vars.clone();
        phase_env.extend(phase.env.clone());

        // Add VM name to environment
        phase_env.insert("VM_NAME".to_string(), vm_name.to_string());
        phase_env.insert("LIMA_INSTANCE".to_string(), vm_name.to_string());

        // Get scripts (inline + files)
        let scripts = phase.get_scripts(project.root())?;

        if scripts.is_empty() {
            println!("⚠ Warning: Phase '{}' has no scripts", phase.name);
            continue;
        }

        // Execute each script
        for (name, content) in scripts {
            let result = execute_host_script(&content, &phase_env);

            match result {
                Ok(_) => println!("✓ {} completed", name),
                Err(e) if phase.continue_on_error => {
                    eprintln!("⚠ {} failed: {} (continuing)", name, e);
                }
                Err(e) => return Err(e),
            }
        }
    }
    Ok(())
}

/// Execute a single script on the host machine
///
/// # Arguments
/// * `script` - Script content to execute
/// * `env_vars` - Environment variables to pass to the script
///
/// # Errors
/// Returns error if script execution fails or returns non-zero exit code
fn execute_host_script(script: &str, env_vars: &HashMap<String, String>) -> Result<()> {
    let output = Command::new("bash")
        .arg("-c")
        .arg(script)
        .envs(env_vars)
        .output()
        .map_err(|e| {
            ClaudeVmError::CommandFailed(format!("Failed to execute host script: {}", e))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        let error_msg = if !stderr.is_empty() {
            format!("Host script failed:\n{}", stderr)
        } else if !stdout.is_empty() {
            format!("Host script failed:\n{}", stdout)
        } else {
            format!(
                "Host script failed with exit code {:?}",
                output.status.code()
            )
        };

        return Err(ClaudeVmError::CommandFailed(error_msg));
    }

    // Print stdout if present
    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }

    Ok(())
}

/// Check if a host condition is met
///
/// Executes a shell command on the host and returns true if it exits with status 0.
///
/// # Arguments
/// * `condition` - Shell command to execute
/// * `env_vars` - Environment variables to pass to the command
///
/// # Errors
/// Returns error if the condition command cannot be executed (not if it returns false)
fn check_host_condition(condition: &str, env_vars: &HashMap<String, String>) -> Result<bool> {
    let output = Command::new("bash")
        .arg("-c")
        .arg(condition)
        .envs(env_vars)
        .output()
        .map_err(|e| {
            ClaudeVmError::CommandFailed(format!("Failed to check host condition: {}", e))
        })?;

    Ok(output.status.success())
}

/// Build standard environment variables for host phases
///
/// # Arguments
/// * `project` - Project context
/// * `phase_type` - Type of phase (setup, runtime, teardown)
///
/// # Returns
/// HashMap with standard environment variables
pub fn build_host_env(project: &Project, phase_type: &str) -> HashMap<String, String> {
    let mut env = HashMap::new();

    env.insert(
        "PROJECT_ROOT".to_string(),
        project.root().display().to_string(),
    );
    env.insert(
        "TEMPLATE_NAME".to_string(),
        project.template_name().to_string(),
    );
    env.insert("PHASE_TYPE".to_string(), phase_type.to_string());

    env
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_execute_host_script_success() {
        let script = "echo 'test'";
        let env = HashMap::new();

        let result = execute_host_script(script, &env);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_host_script_failure() {
        let script = "exit 1";
        let env = HashMap::new();

        let result = execute_host_script(script, &env);
        assert!(result.is_err());
    }

    #[test]
    fn test_check_host_condition_true() {
        let condition = "test 1 -eq 1";
        let env = HashMap::new();

        let result = check_host_condition(condition, &env).unwrap();
        assert!(result);
    }

    #[test]
    fn test_check_host_condition_false() {
        let condition = "test 1 -eq 2";
        let env = HashMap::new();

        let result = check_host_condition(condition, &env).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_build_host_env() {
        let project = Project::new_for_test(PathBuf::from("/test/project"));
        let env = build_host_env(&project, "setup");

        assert_eq!(env.get("PROJECT_ROOT").unwrap(), "/test/project");
        assert_eq!(env.get("PHASE_TYPE").unwrap(), "setup");
        assert!(env.contains_key("TEMPLATE_NAME"));
    }
}
