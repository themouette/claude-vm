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

    if command.is_empty() {
        eprintln!("Error: No command specified");
        std::process::exit(1);
    }

    if !config.verbose {
        println!("Starting ephemeral VM session...");
    }

    // Create ephemeral session (like run and shell commands)
    let session = VmSession::new(
        project,
        config.verbose,
        config.mount_conversations,
        &config.mounts,
    )?;
    let _cleanup = session.ensure_cleanup();

    // Collect environment variables
    let env_vars = env_utils::collect_env_vars(&cli.env, &cli.env_file, &cli.inherit_env)?;

    // Build command as bash -c with proper escaping
    let cmd_str = shell_utils::join_args(command);

    println!("Executing command in VM: {}", session.name());

    // Execute command with runtime scripts using entrypoint pattern
    let current_dir = std::env::current_dir()?;
    let workdir = Some(current_dir.as_path());
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
        Ok(()) => Ok(()),
        Err(ClaudeVmError::CommandExitCode(code)) => {
            // Propagate the exact exit code from the command
            std::process::exit(code);
        }
        Err(e) => Err(e),
    }
}
