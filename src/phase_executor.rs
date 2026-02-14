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
pub fn build_phase_env_setup(phase: &ScriptPhase) -> Result<String> {
    if phase.env.is_empty() {
        return Ok(String::new());
    }

    let exports: Result<Vec<String>> = phase
        .env
        .iter()
        .map(|(k, v)| crate::utils::env::build_env_export(k, v))
        .collect();

    Ok(exports?.join("\n"))
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
        eprintln!("\n   Hint: Check that script files exist and are readable");
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

    #[test]
    fn test_build_phase_env_setup_empty() {
        let phase = ScriptPhase {
            name: "test".to_string(),
            ..Default::default()
        };

        let result = build_phase_env_setup(&phase).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_build_phase_env_setup_single() {
        let mut env = HashMap::new();
        env.insert("MY_VAR".to_string(), "value".to_string());

        let phase = ScriptPhase {
            name: "test".to_string(),
            env,
            ..Default::default()
        };

        let result = build_phase_env_setup(&phase).unwrap();
        assert_eq!(result, "export MY_VAR='value'");
    }

    #[test]
    fn test_build_phase_env_setup_multiple() {
        let mut env = HashMap::new();
        env.insert("VAR1".to_string(), "value1".to_string());
        env.insert("VAR2".to_string(), "value2".to_string());

        let phase = ScriptPhase {
            name: "test".to_string(),
            env,
            ..Default::default()
        };

        let result = build_phase_env_setup(&phase).unwrap();
        // HashMap iteration order is not guaranteed, so check both are present
        assert!(result.contains("export VAR1='value1'"));
        assert!(result.contains("export VAR2='value2'"));
    }

    #[test]
    fn test_phase_context_name() {
        assert_eq!(PhaseContext::Setup.name(), "Setup");
        assert_eq!(PhaseContext::Runtime.name(), "Runtime");
    }
}
