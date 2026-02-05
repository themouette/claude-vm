use crate::cli::Cli;
use crate::config::Config;
use crate::error::{ClaudeVmError, Result};
use crate::project::Project;
use crate::utils::env as env_utils;
use crate::utils::shell as shell_utils;
use crate::vm::limactl::LimaCtl;
use crate::vm::template;

pub fn execute(project: &Project, config: &Config, cli: &Cli, command: &[String]) -> Result<()> {
    // Verify template exists
    template::verify(project.template_name())?;

    if command.is_empty() {
        eprintln!("Error: No command specified");
        std::process::exit(1);
    }

    // Collect environment variables
    let env_vars = env_utils::collect_env_vars(&cli.env, &cli.env_file, &cli.inherit_env)?;

    // Build command string with env exports
    let mut cmd_parts = Vec::new();
    if !env_vars.is_empty() {
        cmd_parts.push(env_utils::build_export_commands(&env_vars));
    }
    cmd_parts.push(shell_utils::join_args(command));
    let cmd_str = cmd_parts.join("; ");

    // Execute command in VM
    match LimaCtl::shell(
        project.template_name(),
        Some(project.root()),
        "bash",
        &["-c", &cmd_str],
        config.forward_ssh_agent,
    ) {
        Ok(()) => Ok(()),
        Err(ClaudeVmError::CommandExitCode(code)) => {
            // Propagate the exact exit code from the command
            std::process::exit(code);
        }
        Err(e) => Err(e),
    }
}
