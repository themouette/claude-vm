use crate::cli::Cli;
use crate::config::Config;
use crate::error::{ClaudeVmError, Result};
use crate::project::Project;
use crate::scripts::runner;
use crate::utils::env as env_utils;
use crate::utils::shell as shell_utils;
use crate::vm::{session::VmSession, template};

pub fn execute(project: &Project, config: &Config, cli: &Cli, command: &[String]) -> Result<()> {
    // Verify template exists
    template::verify(project.template_name())?;

    let is_interactive = command.is_empty();

    if !config.verbose {
        if is_interactive {
            println!("Starting ephemeral VM session for shell...");
        } else {
            println!("Starting ephemeral VM session...");
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
    let env_vars = env_utils::collect_env_vars(&cli.env, &cli.env_file, &cli.inherit_env)?;

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
            config,
            &session,
            workdir,
            "bash",
            &["-l"],
            &env_vars,
        )?;
    } else {
        // Command execution mode
        println!("Executing command in VM: {}", session.name());

        let cmd_str = shell_utils::join_args(command);
        match runner::execute_command_with_runtime_scripts(
            session.name(),
            project,
            config,
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
