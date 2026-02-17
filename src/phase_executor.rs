/// Phase execution module for common setup and runtime phase logic
///
/// This module provides shared functionality for executing phases in both
/// setup (template creation) and runtime (session initialization) contexts.
use crate::config::ScriptPhase;
use crate::error::{ClaudeVmError, Result};
use std::path::Path;

/// Phase execution context (setup, runtime, or cleanup)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseContext {
    /// Template creation phase
    Setup,
    /// Session initialization phase
    Runtime,
    /// Session cleanup phase (after command, before VM stop)
    Cleanup,
}

impl PhaseContext {
    pub fn name(&self) -> &'static str {
        match self {
            PhaseContext::Setup => "Setup",
            PhaseContext::Runtime => "Runtime",
            PhaseContext::Cleanup => "Cleanup",
        }
    }
}

/// Validate a phase before execution
///
/// Checks for common issues and returns detailed error messages.
/// Called before attempting to execute a phase.
pub fn validate_phase(phase: &ScriptPhase, context: PhaseContext) -> Result<()> {
    // Check that phase has at least one script source
    if phase.script.is_none() && phase.script_files.is_empty() {
        return Err(ClaudeVmError::InvalidConfig(format!(
            "{} phase '{}' has no script content. \
             Specify either 'script' (inline) or 'script_files' (file paths)",
            context.name(),
            phase.name
        )));
    }

    // Validate environment variable keys
    for key in phase.env.keys() {
        crate::utils::env::validate_env_key(key).map_err(|e| {
            ClaudeVmError::InvalidConfig(format!(
                "{} phase '{}' has invalid environment variable: {}",
                context.name(),
                phase.name,
                e
            ))
        })?;
    }

    Ok(())
}

/// Build environment setup script from phase environment variables
///
/// Returns a string of export statements that can be prepended to scripts.
/// All keys are validated before being included.
///
/// For capability phases (detected by presence of CAPABILITY_ID), this will inject
/// additional capability-specific environment variables from the project context.
pub fn build_phase_env_setup(
    phase: &ScriptPhase,
    project: &crate::project::Project,
    vm_name: &str,
) -> Result<String> {
    let mut env = phase.env.clone();

    // If this is a capability phase, inject capability-specific environment variables
    if phase.env.contains_key("CAPABILITY_ID") {
        inject_capability_env_vars(&mut env, project, vm_name)?;
    }

    if env.is_empty() {
        return Ok(String::new());
    }

    let exports: Result<Vec<String>> = env
        .iter()
        .map(|(k, v)| crate::utils::env::build_env_export(k, v))
        .collect();

    Ok(exports?.join("\n"))
}

/// Inject capability-specific environment variables
///
/// Adds all the standard capability environment variables that scripts expect:
/// - TEMPLATE_NAME, LIMA_INSTANCE, CAPABILITY_ID (already present)
/// - CLAUDE_VM_PHASE (already present), CLAUDE_VM_VERSION
/// - PROJECT_ROOT, PROJECT_NAME
/// - PROJECT_WORKTREE_ROOT, PROJECT_WORKTREE (if git worktree)
fn inject_capability_env_vars(
    env: &mut std::collections::HashMap<String, String>,
    project: &crate::project::Project,
    vm_name: &str,
) -> Result<()> {
    // VM identification
    env.insert(
        "TEMPLATE_NAME".to_string(),
        project.template_name().to_string(),
    );
    env.insert("LIMA_INSTANCE".to_string(), vm_name.to_string());

    // CAPABILITY_ID and CLAUDE_VM_PHASE are already set by merge_capability_phases

    // Version
    env.insert(
        "CLAUDE_VM_VERSION".to_string(),
        crate::version::VERSION.to_string(),
    );

    // Project information
    let project_root = project.root();
    env.insert(
        "PROJECT_ROOT".to_string(),
        project_root.to_string_lossy().to_string(),
    );

    // Extract project name using utility function
    if let Some(name) = crate::utils::git::extract_project_name(project_root) {
        env.insert("PROJECT_NAME".to_string(), name);
    }

    // Detect git worktree using utility function
    if let Some(worktree_info) = crate::utils::git::detect_worktree(project_root) {
        env.insert(
            "PROJECT_WORKTREE_ROOT".to_string(),
            worktree_info.main_root.to_string_lossy().to_string(),
        );
        env.insert(
            "PROJECT_WORKTREE".to_string(),
            worktree_info.worktree_path.to_string_lossy().to_string(),
        );
    }

    // Ensure empty strings for worktree vars if not detected
    env.entry("PROJECT_WORKTREE_ROOT".to_string()).or_default();
    env.entry("PROJECT_WORKTREE".to_string()).or_default();

    Ok(())
}

/// Handle phase execution error with context and continue_on_error support
///
/// Prints detailed error information and returns appropriate Result based on
/// phase configuration.
pub fn handle_phase_error(
    phase: &ScriptPhase,
    context: PhaseContext,
    error: ClaudeVmError,
    script_name: Option<&str>,
) -> Result<()> {
    eprintln!("\n❌ {} phase '{}' failed", context.name(), phase.name);

    if let Some(name) = script_name {
        eprintln!("   Script: {}", name);
    }

    eprintln!("   Error: {}", error);

    // Show condition if present
    if let Some(ref condition) = phase.when {
        eprintln!("   Condition: {}", condition);
    }

    if phase.continue_on_error {
        eprintln!("   ℹ Continuing due to continue_on_error=true");
        Ok(())
    } else {
        Err(error)
    }
}

/// Print detailed error for script loading failures
pub fn handle_script_load_error(
    phase: &ScriptPhase,
    context: PhaseContext,
    error: ClaudeVmError,
) -> Result<()> {
    eprintln!(
        "\n❌ Failed to load scripts for {} phase '{}'",
        context.name().to_lowercase(),
        phase.name
    );
    eprintln!("   Error: {}", error);

    if !phase.script_files.is_empty() {
        eprintln!("   Script files:");
        for file in &phase.script_files {
            eprintln!("   - {}", file);
        }
        eprintln!("   Hint: Check that script files exist and are readable");
    }

    if phase.continue_on_error {
        eprintln!("   ℹ Continuing due to continue_on_error=true");
        Ok(())
    } else {
        Err(error)
    }
}

/// Load scripts from a phase with error handling
///
/// Returns the list of (name, content) tuples or handles errors based on
/// phase configuration.
pub fn load_phase_scripts(
    phase: &ScriptPhase,
    project_root: &Path,
    context: PhaseContext,
) -> Result<Option<Vec<(String, String)>>> {
    match phase.get_scripts(project_root) {
        Ok(scripts) => Ok(Some(scripts)),
        Err(e) => {
            handle_script_load_error(phase, context, e)?;
            Ok(None) // continue_on_error = true case
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ScriptPhase;
    use std::collections::HashMap;

    #[test]
    fn test_validate_phase_no_script() {
        let phase = ScriptPhase {
            name: "test".to_string(),
            ..Default::default()
        };

        assert!(validate_phase(&phase, PhaseContext::Setup).is_err());
    }

    #[test]
    fn test_validate_phase_inline_script() {
        let phase = ScriptPhase {
            name: "test".to_string(),
            script: Some("echo hello".to_string()),
            ..Default::default()
        };

        assert!(validate_phase(&phase, PhaseContext::Setup).is_ok());
    }

    #[test]
    fn test_validate_phase_script_files() {
        let phase = ScriptPhase {
            name: "test".to_string(),
            script_files: vec!["test.sh".to_string()],
            ..Default::default()
        };

        assert!(validate_phase(&phase, PhaseContext::Setup).is_ok());
    }

    #[test]
    fn test_validate_phase_invalid_env_key() {
        let mut env = HashMap::new();
        env.insert("INVALID-KEY".to_string(), "value".to_string());

        let phase = ScriptPhase {
            name: "test".to_string(),
            script: Some("echo hello".to_string()),
            env,
            ..Default::default()
        };

        assert!(validate_phase(&phase, PhaseContext::Setup).is_err());
    }

    // Note: build_phase_env_setup tests are now integration tests in tests/
    // since they require Project instances. The tests below cover basic validation
    // without capability env injection.

    #[test]
    fn test_phase_context_name() {
        assert_eq!(PhaseContext::Setup.name(), "Setup");
        assert_eq!(PhaseContext::Runtime.name(), "Runtime");
    }

    #[test]
    fn test_validate_phase_multiple_invalid_env_keys() {
        let mut env = HashMap::new();
        env.insert("VALID_KEY".to_string(), "value".to_string());
        env.insert("INVALID-KEY".to_string(), "value".to_string());
        env.insert("ANOTHER$BAD".to_string(), "value".to_string());

        let phase = ScriptPhase {
            name: "test".to_string(),
            script: Some("echo hello".to_string()),
            env,
            ..Default::default()
        };

        let result = validate_phase(&phase, PhaseContext::Setup);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("invalid environment variable"));
    }

    #[test]
    fn test_validate_phase_both_script_and_files() {
        let phase = ScriptPhase {
            name: "test".to_string(),
            script: Some("echo inline".to_string()),
            script_files: vec!["test.sh".to_string()],
            ..Default::default()
        };

        // Should be valid - having both is allowed
        assert!(validate_phase(&phase, PhaseContext::Setup).is_ok());
    }

    #[test]
    fn test_validate_phase_empty_script_content() {
        let phase = ScriptPhase {
            name: "test".to_string(),
            script: Some("".to_string()),
            ..Default::default()
        };

        // Empty string is still "some" script content, so it's valid
        assert!(validate_phase(&phase, PhaseContext::Setup).is_ok());
    }

    #[test]
    fn test_handle_phase_error_with_continue_on_error() {
        let phase = ScriptPhase {
            name: "test-phase".to_string(),
            script: Some("exit 1".to_string()),
            continue_on_error: true,
            ..Default::default()
        };

        let error = crate::error::ClaudeVmError::CommandFailed("Test error".to_string());
        let result = handle_phase_error(&phase, PhaseContext::Setup, error, Some("test.sh"));

        // Should return Ok because continue_on_error is true
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_phase_error_without_continue_on_error() {
        let phase = ScriptPhase {
            name: "test-phase".to_string(),
            script: Some("exit 1".to_string()),
            continue_on_error: false,
            ..Default::default()
        };

        let error = crate::error::ClaudeVmError::CommandFailed("Test error".to_string());
        let result = handle_phase_error(&phase, PhaseContext::Runtime, error, None);

        // Should propagate the error
        assert!(result.is_err());
    }

    #[test]
    fn test_handle_script_load_error_with_continue() {
        let phase = ScriptPhase {
            name: "test-phase".to_string(),
            script_files: vec!["missing.sh".to_string()],
            continue_on_error: true,
            ..Default::default()
        };

        let error = crate::error::ClaudeVmError::InvalidConfig("File not found".to_string());
        let result = handle_script_load_error(&phase, PhaseContext::Setup, error);

        // Should return Ok because continue_on_error is true
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_script_load_error_without_continue() {
        let phase = ScriptPhase {
            name: "test-phase".to_string(),
            script_files: vec!["missing.sh".to_string()],
            continue_on_error: false,
            ..Default::default()
        };

        let error = crate::error::ClaudeVmError::InvalidConfig("File not found".to_string());
        let result = handle_script_load_error(&phase, PhaseContext::Runtime, error);

        // Should propagate the error
        assert!(result.is_err());
    }
}
