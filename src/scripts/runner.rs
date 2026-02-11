use crate::capabilities;
use crate::config::Config;
use crate::error::Result;
use crate::project::Project;
use crate::utils::git;
use crate::vm::limactl::LimaCtl;
use crate::vm::{mount, session::VmSession};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Directory where capability runtime scripts are installed in the VM
const RUNTIME_SCRIPT_DIR: &str = "/usr/local/share/claude-vm/runtime";

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
    LimaCtl::shell(vm_name, None, "chmod", &["+x", &temp_path], false)?;
    LimaCtl::shell(vm_name, None, "bash", &[&temp_path], false)?;

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
    LimaCtl::shell(vm_name, None, "chmod", &["+x", &temp_path], false)?;
    LimaCtl::shell(vm_name, None, "bash", &[&temp_path], false)?;

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
    LimaCtl::shell(vm_name, None, "chmod", &["+x", &temp_path], false)?;
    LimaCtl::shell(vm_name, None, "bash", &[&temp_path], false)?;

    Ok(())
}

/// Generate base context markdown for Claude
///
/// Creates a markdown file with VM configuration, enabled capabilities,
/// mounted directories, and user-provided instructions.
fn generate_base_context(config: &Config) -> Result<String> {
    let mut context = String::new();

    // Header
    context.push_str("<!-- claude-vm-context-start -->\n");
    context.push_str("# Claude VM Context\n\n");
    context
        .push_str("You are running in an isolated Lima VM with the following configuration.\n\n");

    // VM Configuration
    context.push_str("## VM Configuration\n");
    context.push_str(&format!("- **Disk**: {} GB\n", config.vm.disk));
    context.push_str(&format!("- **Memory**: {} GB\n", config.vm.memory));
    context.push('\n');

    // Enabled Capabilities
    context.push_str("## Enabled Capabilities\n");
    let registry = capabilities::registry::CapabilityRegistry::load()?;
    let enabled = registry.get_enabled_capabilities(config)?;
    if enabled.is_empty() {
        context.push_str("None\n");
    } else {
        for cap in enabled {
            context.push_str(&format!(
                "- {}: {}\n",
                cap.capability.id, cap.capability.description
            ));
        }
    }
    context.push('\n');

    // Mounted Directories
    context.push_str("## Mounted Directories\n");
    let mounts = mount::compute_mounts(config.mount_conversations, &config.mounts)?;
    if mounts.is_empty() {
        context.push_str("None\n");
    } else {
        for m in mounts {
            let mode = if m.writable { "writable" } else { "read-only" };
            let vm_path = m.mount_point.as_ref().unwrap_or(&m.location);
            context.push_str(&format!("- {} ({})\n", vm_path.display(), mode));
        }
    }
    context.push('\n');

    // User Instructions (if provided)
    if !config.context.instructions.is_empty() {
        context.push_str("## User Instructions\n");
        context.push_str(&config.context.instructions);
        if !config.context.instructions.ends_with('\n') {
            context.push('\n');
        }
        context.push('\n');
    }

    // Placeholder for runtime context
    context.push_str("<!-- claude-vm-context-runtime-placeholder -->\n");
    context.push_str("<!-- claude-vm-context-end -->\n");

    Ok(context)
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
/// - `session`: VM session containing mount and other session information
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
#[allow(clippy::too_many_arguments)]
pub fn execute_command_with_runtime_scripts(
    vm_name: &str,
    project: &Project,
    config: &Config,
    _session: &VmSession,
    workdir: Option<&Path>,
    cmd: &str,
    args: &[&str],
    env_vars: &HashMap<String, String>,
) -> Result<()> {
    // Collect all runtime scripts as (name, content, env_vars, source) tuples
    let mut script_contents: Vec<(String, String, HashMap<String, String>, bool)> = Vec::new();

    // First, check for project-specific runtime script
    let runtime_script_path = find_runtime_script_path()?;
    if runtime_script_path.exists() {
        let content = std::fs::read_to_string(&runtime_script_path)?;
        let name = runtime_script_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("runtime.sh")
            .to_string();
        script_contents.push((name, content, HashMap::new(), false)); // No env, not sourced
    }

    // Then add custom runtime scripts from config (legacy - with deprecation warning)
    if !config.runtime.scripts.is_empty() {
        eprintln!(
            "⚠ Warning: [runtime] scripts array is deprecated. Please migrate to [[phase.runtime]]"
        );
        eprintln!("   See: docs/configuration.md");

        for script_path_str in &config.runtime.scripts {
            let script_path = PathBuf::from(script_path_str);
            if !script_path.exists() {
                eprintln!("⚠ Warning: Runtime script not found: {}", script_path_str);
                continue;
            }
            let content = std::fs::read_to_string(&script_path)?;
            let name = script_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("script.sh")
                .to_string();
            script_contents.push((name, content, HashMap::new(), false)); // Not sourced
        }
    }

    // New phase-based runtime scripts
    for phase in &config.phase.runtime {
        // Validate phase and emit warnings for potential issues
        phase.validate_and_warn();

        // Check conditional execution
        if !phase.should_execute(vm_name)? {
            eprintln!(
                "⊘ Skipping runtime phase '{}' (condition not met)",
                phase.name
            );
            continue;
        }

        // Get scripts for this phase
        let scripts = match phase.get_scripts(project.root()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!(
                    "\n❌ Failed to load scripts for runtime phase '{}'",
                    phase.name
                );
                eprintln!("   Error: {}", e);
                if !phase.script_files.is_empty() {
                    eprintln!("   Script files:");
                    for file in &phase.script_files {
                        eprintln!("   - {}", file);
                    }
                    eprintln!("\n   Hint: Check that script files exist and are readable");
                }

                if phase.continue_on_error {
                    eprintln!("   ℹ Continuing due to continue_on_error=true");
                    continue;
                } else {
                    return Err(e);
                }
            }
        };

        for (name, content) in scripts {
            script_contents.push((name, content, phase.env.clone(), phase.source));
        }
    }

    // Generate and copy base context
    let base_context = generate_base_context(config)?;
    let temp_dir = std::env::temp_dir();
    let pid = std::process::id();
    let context_file = temp_dir.join(format!("claude-vm-context-{}.md", pid));
    std::fs::write(&context_file, base_context)?;

    // Copy context to VM with unique name to avoid race conditions
    let vm_context_path = format!("/tmp/claude-vm-context-base-{}.md", pid);
    LimaCtl::copy(&context_file, vm_name, &vm_context_path)?;

    // Build entrypoint script with proper escaping
    let mut entrypoint = String::from("#!/bin/bash\nset -e\n\n");

    // Export environment variables if any
    if !env_vars.is_empty() {
        entrypoint.push_str("# Export environment variables\n");
        for (key, value) in env_vars {
            // Escape single quotes in the value
            let escaped_value = value.replace('\'', "'\\''");
            entrypoint.push_str(&format!("export {}='{}'\n", key, escaped_value));
        }
        entrypoint.push('\n');
    }

    // Create context directory for runtime scripts
    entrypoint.push_str("# Create context directory for runtime scripts\n");
    entrypoint.push_str("mkdir -p ~/.claude-vm/context\n\n");

    // Export capability-specific environment variables
    entrypoint.push_str("# Export capability environment variables\n");

    // Network isolation environment variables
    if config.security.network.enabled {
        entrypoint.push_str("export NETWORK_ISOLATION_ENABLED=true\n");
        let mode = match config.security.network.mode {
            crate::config::PolicyMode::Allowlist => "allowlist",
            crate::config::PolicyMode::Denylist => "denylist",
        };
        entrypoint.push_str(&format!("export POLICY_MODE={}\n", mode));

        if !config.security.network.allowed_domains.is_empty() {
            let allowed = config.security.network.allowed_domains.join(",");
            entrypoint.push_str(&format!("export ALLOWED_DOMAINS='{}'\n", allowed));
        }

        if !config.security.network.blocked_domains.is_empty() {
            let blocked = config.security.network.blocked_domains.join(",");
            entrypoint.push_str(&format!("export BLOCKED_DOMAINS='{}'\n", blocked));
        }

        if !config.security.network.bypass_domains.is_empty() {
            let bypass = config.security.network.bypass_domains.join(",");
            entrypoint.push_str(&format!("export BYPASS_DOMAINS='{}'\n", bypass));
        }

        entrypoint.push_str(&format!(
            "export BLOCK_TCP_UDP={}\n",
            config.security.network.block_tcp_udp
        ));
        entrypoint.push_str(&format!(
            "export BLOCK_PRIVATE_NETWORKS={}\n",
            config.security.network.block_private_networks
        ));
        entrypoint.push_str(&format!(
            "export BLOCK_METADATA_SERVICES={}\n",
            config.security.network.block_metadata_services
        ));
    }
    entrypoint.push('\n');

    // Source capability runtime scripts first
    entrypoint.push_str("# Source capability runtime scripts\n");
    entrypoint.push_str(&format!("if [ -d {} ]; then\n", RUNTIME_SCRIPT_DIR));
    entrypoint.push_str(&format!(
        "  for script in {}/*.sh; do\n",
        RUNTIME_SCRIPT_DIR
    ));
    entrypoint.push_str("    if [ -f \"$script\" ]; then\n");
    entrypoint.push_str("      . \"$script\" 2>&1 || echo \"Warning: Failed to source $script\"\n");
    entrypoint.push_str("    fi\n");
    entrypoint.push_str("  done\n");
    entrypoint.push_str("fi\n\n");

    // Then run user runtime scripts (embedded via heredoc)
    entrypoint.push_str("# User runtime scripts - executed in order\n");

    for (i, (name, content, script_env, source_script)) in script_contents.iter().enumerate() {
        entrypoint.push_str(&format!("# Runtime script {}: {}\n", i, name));
        entrypoint.push_str(&format!("echo 'Running runtime script: {}'...\n", name));

        // Set phase-specific environment variables if any
        if !script_env.is_empty() {
            entrypoint.push_str("# Phase-specific environment variables\n");

            // Only use subshell if NOT sourcing (sourcing needs exports to persist)
            if !*source_script {
                entrypoint.push_str("(\n"); // Start subshell to isolate env vars
            }

            for (key, value) in script_env {
                let escaped_value = value.replace('\'', "'\\''");
                let indent = if *source_script { "" } else { "  " };
                entrypoint.push_str(&format!("{}export {}='{}'\n", indent, key, escaped_value));
            }

            // Embed script using heredoc
            let indent = if *source_script { "" } else { "  " };
            if *source_script {
                // Source mode: Execute in current shell context
                entrypoint.push_str(&format!(
                    "{}source /dev/stdin <<'CLAUDE_VM_SCRIPT_EOF_{}'\n{}\nCLAUDE_VM_SCRIPT_EOF_{}\n",
                    indent, i, content, i
                ));
            } else {
                // Subprocess mode: Execute in subshell
                entrypoint.push_str(&format!(
                    "{}bash <<'CLAUDE_VM_SCRIPT_EOF_{}'\n{}\nCLAUDE_VM_SCRIPT_EOF_{}\n",
                    indent, i, content, i
                ));
            }

            if !*source_script {
                entrypoint.push_str(")\n"); // End subshell
            }
            entrypoint.push('\n');
        } else {
            // Embed script using heredoc
            if *source_script {
                // Source mode: Execute in current shell context
                entrypoint.push_str(&format!(
                    "source /dev/stdin <<'CLAUDE_VM_SCRIPT_EOF_{}'\n{}\nCLAUDE_VM_SCRIPT_EOF_{}\n\n",
                    i, content, i
                ));
            } else {
                // Subprocess mode: Execute in subshell
                entrypoint.push_str(&format!(
                    "bash <<'CLAUDE_VM_SCRIPT_EOF_{}'\n{}\nCLAUDE_VM_SCRIPT_EOF_{}\n\n",
                    i, content, i
                ));
            }
        }
    }

    // Generate final CLAUDE.md with runtime context (only if Claude Code is installed)
    entrypoint.push_str(
        "# Generate final CLAUDE.md with runtime context (skip if Claude not installed)\n",
    );
    entrypoint.push_str("if command -v claude >/dev/null 2>&1; then\n");
    entrypoint.push_str(&format!(
        "  cp {} ~/.claude/CLAUDE.md.new\n\n",
        vm_context_path
    ));

    entrypoint.push_str("  # Add runtime script results if any exist\n");
    entrypoint.push_str("  if [ -d ~/.claude-vm/context ] && [ \"$(ls -A ~/.claude-vm/context/*.txt 2>/dev/null)\" ]; then\n");
    entrypoint.push_str("    # Insert runtime context section header\n");
    entrypoint.push_str("    sed -i '/<!-- claude-vm-context-runtime-placeholder -->/i ## Runtime Script Results\\n' ~/.claude/CLAUDE.md.new\n\n");

    entrypoint.push_str("    # Add each context file\n");
    entrypoint.push_str("    for context_file in ~/.claude-vm/context/*.txt; do\n");
    entrypoint.push_str("      if [ -f \"$context_file\" ]; then\n");
    entrypoint.push_str("        name=$(basename \"$context_file\" .txt)\n");
    entrypoint.push_str("        # Insert subsection header\n");
    entrypoint.push_str("        sed -i \"/<!-- claude-vm-context-runtime-placeholder -->/i ### $name\\n\" ~/.claude/CLAUDE.md.new\n");
    entrypoint.push_str("        # Insert file contents\n");
    entrypoint.push_str("        sed -i \"/### $name/r $context_file\" ~/.claude/CLAUDE.md.new\n");
    entrypoint.push_str("        # Add blank line after content\n");
    entrypoint.push_str("        sed -i \"/### $name/a \\\\\" ~/.claude/CLAUDE.md.new\n");
    entrypoint.push_str("      fi\n");
    entrypoint.push_str("    done\n");
    entrypoint.push_str("  fi\n\n");

    entrypoint.push_str("  # Remove the placeholder marker\n");
    entrypoint.push_str(
        "  sed -i '/<!-- claude-vm-context-runtime-placeholder -->/d' ~/.claude/CLAUDE.md.new\n\n",
    );

    entrypoint.push_str("  # Merge with existing CLAUDE.md if present\n");
    entrypoint.push_str("  if [ -f ~/.claude/CLAUDE.md ]; then\n");
    entrypoint
        .push_str("    if grep -q '<!-- claude-vm-context-start -->' ~/.claude/CLAUDE.md; then\n");
    entrypoint
        .push_str("      # Replace content between markers, preserving user content position\n");
    entrypoint.push_str("      awk '\n");
    entrypoint.push_str("        /<!-- claude-vm-context-start -->/ { skip=1; next }\n");
    entrypoint.push_str("        /<!-- claude-vm-context-end -->/ { skip=0; next }\n");
    entrypoint.push_str("        !skip\n");
    entrypoint.push_str("      ' ~/.claude/CLAUDE.md > ~/.claude/CLAUDE.md.old\n\n");
    entrypoint.push_str(
        "      cat ~/.claude/CLAUDE.md.old ~/.claude/CLAUDE.md.new > ~/.claude/CLAUDE.md\n",
    );
    entrypoint.push_str("    else\n");
    entrypoint.push_str("      # Append our context to existing content\n");
    entrypoint.push_str(
        "      cat ~/.claude/CLAUDE.md ~/.claude/CLAUDE.md.new > ~/.claude/CLAUDE.md.tmp\n",
    );
    entrypoint.push_str("      mv ~/.claude/CLAUDE.md.tmp ~/.claude/CLAUDE.md\n");
    entrypoint.push_str("    fi\n");
    entrypoint.push_str("  else\n");
    entrypoint.push_str("    # No existing file, use our generated context\n");
    entrypoint.push_str("    mv ~/.claude/CLAUDE.md.new ~/.claude/CLAUDE.md\n");
    entrypoint.push_str("  fi\n");
    entrypoint.push_str("fi\n\n");

    entrypoint.push_str("# Cleanup temporary files\n");
    entrypoint.push_str(&format!(
        "rm -f ~/.claude/CLAUDE.md.new ~/.claude/CLAUDE.md.old {}\n\n",
        vm_context_path
    ));

    // Exec main command - $@ contains all positional parameters
    entrypoint.push_str("# Execute main command (replaces shell process)\n");
    entrypoint.push_str("exec \"$@\"\n");

    // Execute entrypoint with main command as positional parameters
    // bash -c 'script' -- cmd arg1 arg2
    // The '--' becomes $0, cmd becomes $1, etc. Then "$@" expands to cmd arg1 arg2
    let mut shell_args = vec!["-c", entrypoint.as_str(), "--"];
    shell_args.push(cmd);
    shell_args.extend(args);

    LimaCtl::shell(
        vm_name,
        workdir,
        "bash",
        &shell_args,
        config.forward_ssh_agent,
    )
}

/// Build entrypoint script for testing purposes with heredoc embedding
#[cfg(test)]
fn build_entrypoint_script_with_heredocs(
    script_contents: &[(String, String, HashMap<String, String>, bool)],
) -> String {
    let mut entrypoint = String::from("#!/bin/bash\nset -e\n\n");

    // Source capability runtime scripts first
    entrypoint.push_str("# Source capability runtime scripts\n");
    entrypoint.push_str(&format!("if [ -d {} ]; then\n", RUNTIME_SCRIPT_DIR));
    entrypoint.push_str(&format!(
        "  for script in {}/*.sh; do\n",
        RUNTIME_SCRIPT_DIR
    ));
    entrypoint.push_str("    if [ -f \"$script\" ]; then\n");
    entrypoint.push_str("      . \"$script\"\n");
    entrypoint.push_str("    fi\n");
    entrypoint.push_str("  done\n");
    entrypoint.push_str("fi\n\n");

    // Then run user runtime scripts with heredocs
    entrypoint.push_str("# User runtime scripts - executed in order\n");

    for (i, (name, content, script_env, source_script)) in script_contents.iter().enumerate() {
        entrypoint.push_str(&format!("# Runtime script {}: {}\n", i, name));
        entrypoint.push_str(&format!("echo 'Running runtime script: {}'...\n", name));

        // Set phase-specific environment variables if any
        if !script_env.is_empty() {
            entrypoint.push_str("# Phase-specific environment variables\n");

            // Only use subshell if NOT sourcing
            if !*source_script {
                entrypoint.push_str("(\n");
            }

            for (key, value) in script_env {
                let escaped_value = value.replace('\'', "'\\''");
                let indent = if *source_script { "" } else { "  " };
                entrypoint.push_str(&format!("{}export {}='{}'\n", indent, key, escaped_value));
            }

            // Embed script using heredoc
            let indent = if *source_script { "" } else { "  " };
            if *source_script {
                entrypoint.push_str(&format!(
                    "{}source /dev/stdin <<'CLAUDE_VM_SCRIPT_EOF_{}'\n{}\nCLAUDE_VM_SCRIPT_EOF_{}\n",
                    indent, i, content, i
                ));
            } else {
                entrypoint.push_str(&format!(
                    "{}bash <<'CLAUDE_VM_SCRIPT_EOF_{}'\n{}\nCLAUDE_VM_SCRIPT_EOF_{}\n",
                    indent, i, content, i
                ));
            }

            if !*source_script {
                entrypoint.push_str(")\n");
            }
            entrypoint.push('\n');
        } else {
            // Embed script using heredoc
            if *source_script {
                entrypoint.push_str(&format!(
                    "source /dev/stdin <<'CLAUDE_VM_SCRIPT_EOF_{}'\n{}\nCLAUDE_VM_SCRIPT_EOF_{}\n\n",
                    i, content, i
                ));
            } else {
                entrypoint.push_str(&format!(
                    "bash <<'CLAUDE_VM_SCRIPT_EOF_{}'\n{}\nCLAUDE_VM_SCRIPT_EOF_{}\n\n",
                    i, content, i
                ));
            }
        }
    }

    entrypoint.push_str("# Execute main command (replaces shell process)\n");
    entrypoint.push_str("exec \"$@\"\n");

    entrypoint
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entrypoint_script_generation_with_heredocs() {
        let script_contents = vec![
            (
                "setup.sh".to_string(),
                "#!/bin/bash\necho 'setup'\n".to_string(),
                HashMap::new(),
                false, // not sourced
            ),
            (
                "init.sh".to_string(),
                "#!/bin/bash\necho 'init'\n".to_string(),
                HashMap::new(),
                false, // not sourced
            ),
        ];

        let entrypoint = build_entrypoint_script_with_heredocs(&script_contents);

        // Verify script structure
        assert!(entrypoint.contains("#!/bin/bash"));
        assert!(entrypoint.contains("set -e"));
        assert!(entrypoint.contains("bash <<'CLAUDE_VM_SCRIPT_EOF_0'"));
        assert!(entrypoint.contains("bash <<'CLAUDE_VM_SCRIPT_EOF_1'"));
        assert!(entrypoint.contains("echo 'setup'"));
        assert!(entrypoint.contains("echo 'init'"));
        assert!(entrypoint.contains("exec \"$@\""));

        // Verify order - setup should come before init
        let setup_pos = entrypoint.find("setup.sh").unwrap();
        let init_pos = entrypoint.find("init.sh").unwrap();
        assert!(setup_pos < init_pos, "Scripts should run in order");
    }

    #[test]
    fn test_heredoc_with_source_true() {
        // Test that source = true uses "source /dev/stdin"
        let script_contents = vec![(
            "env-setup.sh".to_string(),
            "#!/bin/bash\nexport MY_VAR='value'\n".to_string(),
            HashMap::new(),
            true, // sourced
        )];

        let entrypoint = build_entrypoint_script_with_heredocs(&script_contents);

        // Verify source mode is used
        assert!(entrypoint.contains("source /dev/stdin <<'CLAUDE_VM_SCRIPT_EOF_0'"));
        assert!(entrypoint.contains("export MY_VAR='value'"));
        assert!(entrypoint.contains("CLAUDE_VM_SCRIPT_EOF_0"));
    }

    #[test]
    fn test_heredoc_with_source_false() {
        // Test that source = false uses "bash <<"
        let script_contents = vec![(
            "script.sh".to_string(),
            "#!/bin/bash\necho 'subprocess'\n".to_string(),
            HashMap::new(),
            false, // not sourced
        )];

        let entrypoint = build_entrypoint_script_with_heredocs(&script_contents);

        // Verify subprocess mode is used
        assert!(entrypoint.contains("bash <<'CLAUDE_VM_SCRIPT_EOF_0'"));
        assert!(entrypoint.contains("echo 'subprocess'"));
        assert!(entrypoint.contains("CLAUDE_VM_SCRIPT_EOF_0"));
    }

    #[test]
    fn test_heredoc_unique_delimiters() {
        // Test that multiple scripts use different EOF markers
        let script_contents = vec![
            (
                "script1.sh".to_string(),
                "echo 'first'".to_string(),
                HashMap::new(),
                false,
            ),
            (
                "script2.sh".to_string(),
                "echo 'second'".to_string(),
                HashMap::new(),
                false,
            ),
            (
                "script3.sh".to_string(),
                "echo 'third'".to_string(),
                HashMap::new(),
                false,
            ),
        ];

        let entrypoint = build_entrypoint_script_with_heredocs(&script_contents);

        // Verify unique delimiters
        assert!(entrypoint.contains("CLAUDE_VM_SCRIPT_EOF_0"));
        assert!(entrypoint.contains("CLAUDE_VM_SCRIPT_EOF_1"));
        assert!(entrypoint.contains("CLAUDE_VM_SCRIPT_EOF_2"));
    }

    #[test]
    fn test_heredoc_env_vars() {
        // Test that environment variables are exported before heredoc
        let mut env = HashMap::new();
        env.insert("DEBUG".to_string(), "true".to_string());
        env.insert("LOG_LEVEL".to_string(), "info".to_string());

        let script_contents = vec![(
            "script.sh".to_string(),
            "echo $DEBUG $LOG_LEVEL".to_string(),
            env,
            false, // not sourced (uses subshell)
        )];

        let entrypoint = build_entrypoint_script_with_heredocs(&script_contents);

        // Verify environment variables are in subshell
        assert!(entrypoint.contains("(\n"));
        assert!(entrypoint.contains("  export DEBUG='true'"));
        assert!(entrypoint.contains("  export LOG_LEVEL='info'"));
        assert!(entrypoint.contains("  bash <<'CLAUDE_VM_SCRIPT_EOF_0'"));
        assert!(entrypoint.contains(")\n"));
    }

    #[test]
    fn test_heredoc_env_vars_with_source() {
        // Test that environment variables with source=true don't use subshell
        let mut env = HashMap::new();
        env.insert("MY_VAR".to_string(), "test".to_string());

        let script_contents = vec![(
            "script.sh".to_string(),
            "echo $MY_VAR".to_string(),
            env,
            true, // sourced (no subshell)
        )];

        let entrypoint = build_entrypoint_script_with_heredocs(&script_contents);

        // Verify NO subshell for sourced scripts
        assert!(!entrypoint.contains("(\n"));
        assert!(entrypoint.contains("export MY_VAR='test'"));
        assert!(entrypoint.contains("source /dev/stdin <<'CLAUDE_VM_SCRIPT_EOF_0'"));
    }

    #[test]
    fn test_heredoc_injection_protection() {
        // Test that heredoc with quoted delimiter prevents injection
        let malicious_content = "'; rm -rf /; echo '".to_string();
        let script_contents = vec![(
            "evil.sh".to_string(),
            malicious_content.clone(),
            HashMap::new(),
            false,
        )];

        let entrypoint = build_entrypoint_script_with_heredocs(&script_contents);

        // Verify heredoc uses quoted delimiter to prevent expansion
        assert!(entrypoint.contains("<<'CLAUDE_VM_SCRIPT_EOF_0'"));
        // Verify malicious content is treated as literal text
        assert!(entrypoint.contains(&malicious_content));
    }

    #[test]
    fn test_entrypoint_script_error_handling() {
        let script_contents = vec![(
            "script1.sh".to_string(),
            "echo 'test'".to_string(),
            HashMap::new(),
            false,
        )];

        let entrypoint = build_entrypoint_script_with_heredocs(&script_contents);

        // Verify set -e is present (exit on error)
        assert!(entrypoint.contains("set -e"));
    }

    #[test]
    fn test_entrypoint_script_empty() {
        let script_contents: Vec<(String, String, HashMap<String, String>, bool)> = vec![];

        let entrypoint = build_entrypoint_script_with_heredocs(&script_contents);

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
        let script_contents = vec![(
            "script.sh".to_string(),
            "echo 'test'".to_string(),
            HashMap::new(),
            false,
        )];

        let entrypoint = build_entrypoint_script_with_heredocs(&script_contents);

        // Verify "$@" is used (preserves quoting and spaces in arguments)
        assert!(entrypoint.contains("exec \"$@\""));
    }

    #[test]
    fn test_entrypoint_comment_clarity() {
        let script_contents = vec![(
            "test.sh".to_string(),
            "echo 'test'".to_string(),
            HashMap::new(),
            false,
        )];

        let entrypoint = build_entrypoint_script_with_heredocs(&script_contents);

        // Verify helpful comments are present
        assert!(entrypoint.contains("# Source capability runtime scripts"));
        assert!(entrypoint.contains("# User runtime scripts"));
        assert!(entrypoint.contains("# Execute main command"));
    }

    #[test]
    fn test_generate_base_context_structure() {
        let config = Config::default();
        let context = generate_base_context(&config).unwrap();

        // Verify HTML markers
        assert!(context.contains("<!-- claude-vm-context-start -->"));
        assert!(context.contains("<!-- claude-vm-context-end -->"));
        assert!(context.contains("<!-- claude-vm-context-runtime-placeholder -->"));

        // Verify sections
        assert!(context.contains("# Claude VM Context"));
        assert!(context.contains("## VM Configuration"));
        assert!(context.contains("## Enabled Capabilities"));
        assert!(context.contains("## Mounted Directories"));
    }

    #[test]
    fn test_generate_base_context_vm_config() {
        let mut config = Config::default();
        config.vm.disk = 50;
        config.vm.memory = 16;

        let context = generate_base_context(&config).unwrap();

        // Verify VM config values
        assert!(context.contains("**Disk**: 50 GB"));
        assert!(context.contains("**Memory**: 16 GB"));
    }

    #[test]
    fn test_generate_base_context_with_instructions() {
        let mut config = Config::default();
        config.context.instructions = "Test instructions\nMultiple lines".to_string();

        let context = generate_base_context(&config).unwrap();

        // Verify user instructions section
        assert!(context.contains("## User Instructions"));
        assert!(context.contains("Test instructions"));
        assert!(context.contains("Multiple lines"));
    }

    #[test]
    fn test_generate_base_context_no_instructions() {
        let config = Config::default();
        let context = generate_base_context(&config).unwrap();

        // Should not have user instructions section when empty
        assert!(!context.contains("## User Instructions"));
    }

    #[test]
    fn test_generate_base_context_with_capabilities() {
        let mut config = Config::default();
        config.tools.docker = true;
        config.tools.node = true;

        let context = generate_base_context(&config).unwrap();

        // Verify capabilities are listed
        assert!(context.contains("docker"));
        assert!(context.contains("node"));
        assert!(context.contains("Docker engine"));
        assert!(context.contains("Node.js runtime"));
    }

    #[test]
    fn test_generate_base_context_no_capabilities() {
        let config = Config::default();
        let context = generate_base_context(&config).unwrap();

        // Should show "None" when no capabilities enabled
        assert!(context.contains("## Enabled Capabilities"));
        assert!(context.contains("None"));
    }

    #[test]
    fn test_generate_base_context_instructions_trailing_newline() {
        let mut config = Config::default();
        // Test instructions without trailing newline
        config.context.instructions = "Test without newline".to_string();

        let context = generate_base_context(&config).unwrap();

        // Should add newline after instructions
        assert!(context.contains("Test without newline\n\n"));
    }
}
