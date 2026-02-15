use crate::cli::ShellCmd;
use crate::commands::helpers;
use crate::config::Config;
use crate::error::{ClaudeVmError, Result};
use crate::project::Project;
use crate::scripts::runner;
use crate::utils::env as env_utils;
use crate::utils::shell as shell_utils;
use crate::vm::session::VmSession;

pub fn execute(project: &Project, config: &Config, cmd: &ShellCmd) -> Result<()> {
    // Clone config to allow merging capability phases
    let mut config = config.clone();

    // Merge capability-defined phases with user-defined phases
    crate::capabilities::merge_capability_phases(&mut config)?;

    // Ensure template exists (create if missing and user confirms)
    helpers::ensure_template_exists(project, &config)?;

    // Resolve worktree if --worktree flag present
    if !cmd.runtime.worktree.is_empty() {
        let worktree_path = helpers::resolve_worktree(&cmd.runtime.worktree, &config, project)?;
        std::env::set_current_dir(&worktree_path)?;
    }

    let is_interactive = cmd.command.is_empty();

    if !config.verbose {
        if is_interactive {
            eprintln!("Starting ephemeral VM session for shell...");
        } else {
            eprintln!("Starting ephemeral VM session...");
        }
    }

    // Create ephemeral session
    let session = VmSession::new(
        project,
        config.verbose,
        config.mount_conversations,
        &config.mounts,
    )?;
    let _cleanup = session.ensure_cleanup();

    // Use current directory for workdir (not project root)
    // This ensures we cd into the worktree, not the main repo
    let current_dir = std::env::current_dir()?;

    // Collect environment variables
    let env_vars = env_utils::collect_env_vars(
        &cmd.runtime.env,
        &cmd.runtime.env_file,
        &cmd.runtime.inherit_env,
    )?;

    let workdir = Some(current_dir.as_path());

    if is_interactive {
        // Interactive shell mode
        println!(
            "VM: {} | Dir: {} | Project: {}",
            session.name(),
            current_dir.display(),
            project.template_name()
        );
        println!("Type 'exit' to stop and delete the VM");

        runner::execute_command_with_runtime_scripts(
            session.name(),
            project,
            &config,
            &session,
            workdir,
            "bash",
            &["-l"],
            &env_vars,
        )?;
    } else {
        // Command execution mode
        eprintln!("Executing command in VM: {}", session.name());

        let cmd_str = shell_utils::join_args(&cmd.command);
        match runner::execute_command_with_runtime_scripts(
            session.name(),
            project,
            &config,
            &session,
            workdir,
            "bash",
            &["-c", &cmd_str],
            &env_vars,
        ) {
            Ok(()) => {}
            Err(ClaudeVmError::CommandExitCode(code)) => {
                // Propagate the exact exit code from the command
                std::process::exit(code);
            }
            Err(e) => return Err(e),
        }
    }

    Ok(())
}
