/// Phase execution module for common setup and runtime phase logic
///
/// This module provides shared functionality for executing phases in both
/// setup (template creation) and runtime (session initialization) contexts.
use crate::config::ScriptPhase;
use crate::error::{ClaudeVmError, Result};
use std::path::Path;

/// Phase execution context (setup or runtime)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseContext {
    /// Template creation phase
    Setup,
    /// Session initialization phase
    Runtime,
}

impl PhaseContext {
    pub fn name(&self) -> &'static str {
        match self {
            PhaseContext::Setup => "Setup",
            PhaseContext::Runtime => "Runtime",
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
    if let Some(capability_id) = phase.env.get("CAPABILITY_ID") {
        inject_capability_env_vars(&mut env, project, vm_name, capability_id)?;
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
    _capability_id: &str,
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

    // Extract project name from directory name
    if let Some(name) = project_root.file_name() {
        env.insert(
            "PROJECT_NAME".to_string(),
            name.to_string_lossy().to_string(),
        );
    }

    // Detect git worktree (same logic as executor.rs build_capability_env_vars)
    let git_dir = project_root.join(".git");
    if git_dir.exists() && git_dir.is_file() {
        if let Ok(git_file_content) = std::fs::read_to_string(&git_dir) {
            if let Some(gitdir_line) = git_file_content.lines().next() {
                if let Some(gitdir_path) = gitdir_line.strip_prefix("gitdir: ") {
                    let gitdir_pathbuf = std::path::PathBuf::from(gitdir_path);
                    if let Some(worktrees_parent) = gitdir_pathbuf.parent() {
                        if worktrees_parent.ends_with("worktrees") {
                            if let Some(git_parent) = worktrees_parent.parent() {
                                if let Some(main_root) = git_parent.parent() {
                                    env.insert(
                                        "PROJECT_WORKTREE_ROOT".to_string(),
                                        main_root.to_string_lossy().to_string(),
                                    );
                                    env.insert(
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
}
