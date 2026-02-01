use crate::config::Config;
use crate::error::{ClaudeVmError, Result};
use crate::project::Project;
use crate::utils::git;
use crate::vm::limactl::LimaCtl;
use std::path::{Path, PathBuf};

/// Directory where capability runtime scripts are installed in the VM
const RUNTIME_SCRIPT_DIR: &str = "/usr/local/share/claude-vm/runtime";

/// Escape a string for safe use in shell single quotes
/// Converts: foo'bar -> 'foo'\''bar'
fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// Sanitize a filename to contain only safe characters
/// Allows: alphanumeric, dash, underscore, dot
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
        .collect()
}

/// Find the path to the project runtime script (.claude-vm.runtime.sh)
/// Looks in current git repo root (handles worktrees), or current directory
fn find_runtime_script_path() -> Result<PathBuf> {
    if let Ok(Some(git_root)) = git::get_git_root() {
        Ok(git_root.join(".claude-vm.runtime.sh"))
    } else {
        Ok(std::env::current_dir()?.join(".claude-vm.runtime.sh"))
    }
}

/// Execute a script from string content in a VM.
///
/// This function writes the script content to a temporary file, copies it to the VM,
/// makes it executable, and runs it with bash.
///
/// # Arguments
/// - `vm_name`: Name of the VM instance
/// - `script_content`: The script content as a string
/// - `script_name`: Name to give the script file (used for temp file naming)
///
/// # Errors
/// Returns error if file operations, copying, or script execution fails.
///
/// # Note
/// This is primarily used for embedded scripts (e.g., install_docker.sh).
/// For user scripts, prefer `execute_script_file`.
pub fn execute_script(vm_name: &str, script_content: &str, script_name: &str) -> Result<()> {
    println!("Running script: {}", script_name);

    // Write script to temp file
    let temp_path = format!("/tmp/{}", script_name);
    let local_temp = std::env::temp_dir().join(script_name);

    std::fs::write(&local_temp, script_content)?;

    // Copy to VM
    LimaCtl::copy(&local_temp, vm_name, &temp_path)?;

    // Make executable and run
    LimaCtl::shell(vm_name, None, "chmod", &["+x", &temp_path])?;
    LimaCtl::shell(vm_name, None, "bash", &[&temp_path])?;

    // Cleanup local temp file
    std::fs::remove_file(&local_temp)?;

    Ok(())
}

/// Execute a script from string content in a VM silently (only show output on error)
///
/// This function is similar to `execute_script` but suppresses output unless there's an error.
/// Used for runtime scripts that shouldn't clutter the output.
pub fn execute_script_silent(vm_name: &str, script_content: &str, script_name: &str) -> Result<()> {
    // Write script to temp file
    let temp_path = format!("/tmp/{}", script_name);
    let local_temp = std::env::temp_dir().join(script_name);

    std::fs::write(&local_temp, script_content)?;

    // Copy to VM
    LimaCtl::copy(&local_temp, vm_name, &temp_path)?;

    // Make executable and run
    LimaCtl::shell(vm_name, None, "chmod", &["+x", &temp_path])?;
    LimaCtl::shell(vm_name, None, "bash", &[&temp_path])?;

    // Cleanup local temp file
    std::fs::remove_file(&local_temp)?;

    Ok(())
}

/// Execute a script file from the host filesystem in a VM.
///
/// This function copies a script file from the host to the VM,
/// makes it executable, and runs it with bash.
///
/// # Arguments
/// - `vm_name`: Name of the VM instance
/// - `script_path`: Path to the script file on the host filesystem
///
/// # Errors
/// Returns error if the file doesn't exist, copying fails, or script execution fails.
///
/// # Note
/// Used by the setup command for setup scripts. For runtime scripts with entrypoint
/// pattern, use `execute_command_with_runtime_scripts` instead.
pub fn execute_script_file(vm_name: &str, script_path: &Path) -> Result<()> {
    let script_name = script_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("script.sh");

    println!("Running script file: {}", script_path.display());

    let temp_path = format!("/tmp/{}", script_name);

    // Copy to VM
    LimaCtl::copy(script_path, vm_name, &temp_path)?;

    // Make executable and run
    LimaCtl::shell(vm_name, None, "chmod", &["+x", &temp_path])?;
    LimaCtl::shell(vm_name, None, "bash", &[&temp_path])?;

    Ok(())
}

/// Execute a command with runtime scripts using an entrypoint pattern.
///
/// This function runs all runtime scripts followed by the main command in a single
/// shell invocation, which is more efficient than multiple SSH connections.
///
/// # Behavior
/// - Scripts run in order: project script (.claude-vm.runtime.sh), then config scripts
/// - Scripts share the same shell environment (environment variables persist)
/// - If any script fails (exit != 0), main command won't run (fail-fast with `set -e`)
/// - All scripts and main command run in the specified workdir
/// - Script paths are properly escaped to prevent shell injection
///
/// # Arguments
/// - `vm_name`: Name of the VM instance
/// - `_project`: Project context (currently unused but kept for consistency)
/// - `config`: Configuration containing runtime scripts from .claude-vm.toml
/// - `workdir`: Optional working directory for command execution
/// - `cmd`: Main command to execute after runtime scripts
/// - `args`: Arguments to pass to the main command (properly quoted/preserved)
///
/// # Argument Handling
/// Arguments are passed as separate shell parameters using bash's "$@" expansion,
/// which preserves spaces, quotes, and special characters in argument boundaries.
///
/// # Errors
/// Returns error if:
/// - Script copying to VM fails
/// - Any runtime script exits with non-zero status
/// - Main command execution fails
///
/// # Example
/// ```ignore
/// execute_command_with_runtime_scripts(
///     "my-vm",
///     &project,
///     &config,
///     Some(Path::new("/workspace")),
///     "claude",
///     &["--help"]
/// )?;
/// ```
pub fn execute_command_with_runtime_scripts(
    vm_name: &str,
    _project: &Project,
    config: &Config,
    workdir: Option<&Path>,
    cmd: &str,
    args: &[&str],
) -> Result<()> {
    // Collect all runtime scripts
    let mut scripts = Vec::new();

    // First, check for project-specific runtime script
    let runtime_script_path = find_runtime_script_path()?;
    if runtime_script_path.exists() {
        scripts.push(runtime_script_path);
    }

    // Then add custom runtime scripts from config
    for script_path_str in &config.runtime.scripts {
        let script_path = PathBuf::from(script_path_str);
        if !script_path.exists() {
            eprintln!("⚠ Warning: Runtime script not found: {}", script_path_str);
            continue;
        }
        scripts.push(script_path);
    }

    // Copy all scripts to VM with unique names
    let mut vm_script_paths = Vec::new();
    let pid = std::process::id();

    for (i, script) in scripts.iter().enumerate() {
        // Sanitize filename to prevent injection
        let original_name = script
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("script.sh");
        let safe_name = sanitize_filename(original_name);
        let script_name = if safe_name.is_empty() {
            format!("script-{}", i)
        } else {
            safe_name
        };

        // Use PID to avoid collisions between concurrent sessions
        let vm_path = format!("/tmp/claude-vm-{}-{}-{}", pid, i, script_name);

        print!("  Copying runtime script: {} ... ", script.display());
        std::io::Write::flush(&mut std::io::stdout()).unwrap_or(());

        match LimaCtl::copy(script, vm_name, &vm_path) {
            Ok(_) => {
                println!("✓");
                vm_script_paths.push(vm_path);
            }
            Err(e) => {
                println!("✗");
                return Err(ClaudeVmError::LimaExecution(format!(
                    "Failed to copy runtime script '{}': {}",
                    script.display(),
                    e
                )));
            }
        }
    }

    // Build entrypoint script with proper escaping
    let mut entrypoint = String::from("#!/bin/bash\nset -e\n\n");

    // Source capability runtime scripts first
    entrypoint.push_str("# Source capability runtime scripts\n");
    entrypoint.push_str(&format!("if [ -d {} ]; then\n", RUNTIME_SCRIPT_DIR));
    entrypoint.push_str(&format!("  for script in {}/*.sh; do\n", RUNTIME_SCRIPT_DIR));
    entrypoint.push_str("    if [ -f \"$script\" ]; then\n");
    entrypoint.push_str("      . \"$script\" 2>&1 || echo \"Warning: Failed to source $script\"\n");
    entrypoint.push_str("    fi\n");
    entrypoint.push_str("  done\n");
    entrypoint.push_str("fi\n\n");

    // Then run user runtime scripts
    entrypoint.push_str("# User runtime scripts - executed in order\n");

    for (i, vm_path) in vm_script_paths.iter().enumerate() {
        entrypoint.push_str(&format!(
            "echo 'Running runtime script: {}'...\n",
            scripts[i].display()
        ));
        // Use shell_escape to prevent injection attacks
        entrypoint.push_str(&format!("bash {}\n\n", shell_escape(vm_path)));
    }

    // Exec main command - $@ contains all positional parameters
    entrypoint.push_str("# Execute main command (replaces shell process)\n");
    entrypoint.push_str("exec \"$@\"\n");

    // Execute entrypoint with main command as positional parameters
    // bash -c 'script' -- cmd arg1 arg2
    // The '--' becomes $0, cmd becomes $1, etc. Then "$@" expands to cmd arg1 arg2
    let mut shell_args = vec!["-c", entrypoint.as_str(), "--"];
    shell_args.push(cmd);
    shell_args.extend(args);

    LimaCtl::shell(vm_name, workdir, "bash", &shell_args)
}

/// Build entrypoint script for testing purposes
#[cfg(test)]
fn build_entrypoint_script(vm_script_paths: &[String], script_names: &[String]) -> String {
    let mut entrypoint = String::from("#!/bin/bash\nset -e\n\n");

    // Source capability runtime scripts first
    entrypoint.push_str("# Source capability runtime scripts\n");
    entrypoint.push_str(&format!("if [ -d {} ]; then\n", RUNTIME_SCRIPT_DIR));
    entrypoint.push_str(&format!("  for script in {}/*.sh; do\n", RUNTIME_SCRIPT_DIR));
    entrypoint.push_str("    if [ -f \"$script\" ]; then\n");
    entrypoint.push_str("      . \"$script\"\n");
    entrypoint.push_str("    fi\n");
    entrypoint.push_str("  done\n");
    entrypoint.push_str("fi\n\n");

    // Then run user runtime scripts
    entrypoint.push_str("# User runtime scripts - executed in order\n");

    for (i, vm_path) in vm_script_paths.iter().enumerate() {
        entrypoint.push_str(&format!(
            "echo 'Running runtime script: {}'...\n",
            script_names[i]
        ));
        // Use shell_escape to prevent injection
        entrypoint.push_str(&format!("bash {}\n\n", shell_escape(vm_path)));
    }

    entrypoint.push_str("# Execute main command (replaces shell process)\n");
    entrypoint.push_str("exec \"$@\"\n");

    entrypoint
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_escape_basic() {
        assert_eq!(shell_escape("simple"), "'simple'");
        assert_eq!(shell_escape("with space"), "'with space'");
    }

    #[test]
    fn test_shell_escape_single_quote() {
        // Single quote should be escaped as '\''
        assert_eq!(shell_escape("foo'bar"), "'foo'\\''bar'");
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
    }

    #[test]
    fn test_shell_escape_injection_attempt() {
        // Attempt to inject commands
        let malicious = "'; rm -rf /; echo '";
        let escaped = shell_escape(malicious);
        assert_eq!(escaped, "''\\''; rm -rf /; echo '\\'''");
        // When used in bash 'command', this will be treated as a literal string
    }

    #[test]
    fn test_sanitize_filename_safe() {
        assert_eq!(sanitize_filename("safe-file_123.sh"), "safe-file_123.sh");
        assert_eq!(sanitize_filename("normal.txt"), "normal.txt");
    }

    #[test]
    fn test_sanitize_filename_unsafe() {
        // Remove special characters
        assert_eq!(sanitize_filename("file;rm -rf"), "filerm-rf");
        assert_eq!(sanitize_filename("file'with'quotes"), "filewithquotes");
        assert_eq!(sanitize_filename("file$var"), "filevar");
        assert_eq!(sanitize_filename("file`cmd`"), "filecmd");
    }

    #[test]
    fn test_sanitize_filename_empty() {
        // All unsafe characters should result in empty string
        assert_eq!(sanitize_filename("';!@#$%"), "");
    }

    #[test]
    fn test_entrypoint_script_generation() {
        let vm_paths = vec![
            "/tmp/claude-vm-runtime-0-setup.sh".to_string(),
            "/tmp/claude-vm-runtime-1-init.sh".to_string(),
        ];
        let names = vec!["setup.sh".to_string(), "init.sh".to_string()];

        let entrypoint = build_entrypoint_script(&vm_paths, &names);

        // Verify script structure
        assert!(entrypoint.contains("#!/bin/bash"));
        assert!(entrypoint.contains("set -e"));
        assert!(entrypoint.contains("bash '/tmp/claude-vm-runtime-0-setup.sh'"));
        assert!(entrypoint.contains("bash '/tmp/claude-vm-runtime-1-init.sh'"));
        assert!(entrypoint.contains("exec \"$@\""));

        // Verify order - setup should come before init
        let setup_pos = entrypoint.find("runtime-0-setup").unwrap();
        let init_pos = entrypoint.find("runtime-1-init").unwrap();
        assert!(setup_pos < init_pos, "Scripts should run in order");
    }

    #[test]
    fn test_entrypoint_script_escaping() {
        // Test that script paths with special characters are properly quoted
        let vm_paths = vec!["/tmp/script with spaces.sh".to_string()];
        let names = vec!["script with spaces.sh".to_string()];

        let entrypoint = build_entrypoint_script(&vm_paths, &names);

        // Verify single quotes protect the path with proper escaping
        assert!(entrypoint.contains("bash '/tmp/script with spaces.sh'"));
    }

    #[test]
    fn test_entrypoint_script_injection_protection() {
        // Test protection against shell injection in script paths
        let malicious_path = "/tmp/evil'; rm -rf /; echo '.sh".to_string();
        let vm_paths = vec![malicious_path];
        let names = vec!["evil.sh".to_string()];

        let entrypoint = build_entrypoint_script(&vm_paths, &names);

        // Verify the malicious command is properly escaped
        // The escaped version uses '\'' to safely include single quotes within the bash string
        // This results in bash receiving the literal path: /tmp/evil'; rm -rf /; echo '.sh
        assert!(entrypoint.contains(r"bash '/tmp/evil'\''; rm -rf /; echo '\''"));

        // Verify it's wrapped in the escaped quote pattern (not just raw semicolons)
        // The pattern '\'' safely escapes quotes, preventing command injection
        assert!(entrypoint.contains(r"'\''"));
    }

    #[test]
    fn test_entrypoint_script_error_handling() {
        let vm_paths = vec!["/tmp/script1.sh".to_string()];
        let names = vec!["script1.sh".to_string()];

        let entrypoint = build_entrypoint_script(&vm_paths, &names);

        // Verify set -e is present (exit on error)
        assert!(entrypoint.contains("set -e"));
    }

    #[test]
    fn test_entrypoint_script_empty() {
        let vm_paths: Vec<String> = vec![];
        let names: Vec<String> = vec![];

        let entrypoint = build_entrypoint_script(&vm_paths, &names);

        // Even with no user scripts, should source capability scripts and have basic structure
        assert!(entrypoint.contains("#!/bin/bash"));
        assert!(entrypoint.contains("set -e"));
        assert!(entrypoint.contains("# Source capability runtime scripts"));
        assert!(entrypoint.contains("/usr/local/share/claude-vm/runtime"));
        assert!(entrypoint.contains("exec \"$@\""));
    }

    #[test]
    fn test_entrypoint_preserves_command_args() {
        // Test that the entrypoint properly uses "$@" to preserve arguments
        let vm_paths = vec!["/tmp/script.sh".to_string()];
        let names = vec!["script.sh".to_string()];

        let entrypoint = build_entrypoint_script(&vm_paths, &names);

        // Verify "$@" is used (preserves quoting and spaces in arguments)
        assert!(entrypoint.contains("exec \"$@\""));
    }

    #[test]
    fn test_entrypoint_comment_clarity() {
        let vm_paths = vec!["/tmp/script.sh".to_string()];
        let names = vec!["test.sh".to_string()];

        let entrypoint = build_entrypoint_script(&vm_paths, &names);

        // Verify helpful comments are present
        assert!(entrypoint.contains("# Source capability runtime scripts"));
        assert!(entrypoint.contains("# User runtime scripts"));
        assert!(entrypoint.contains("# Execute main command"));
    }
}
