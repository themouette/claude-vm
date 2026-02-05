use crate::cli::Cli;
use crate::config::Config;
use crate::error::Result;
use crate::project::Project;
use crate::utils::env as env_utils;
use crate::vm::limactl::LimaCtl;
use crate::vm::template;
use std::collections::HashMap;

pub fn execute(project: &Project, config: &Config, cli: &Cli, command: &[String]) -> Result<()> {
    // Verify template exists
    template::verify(project.template_name())?;

    if command.is_empty() {
        eprintln!("Error: No command specified");
        std::process::exit(1);
    }

    // Collect environment variables
    let mut env_vars = HashMap::new();

    // Load from env files
    for file in &cli.env_file {
        env_vars.extend(env_utils::load_env_file(file)?);
    }

    // Add --env args
    env_vars.extend(env_utils::parse_env_args(&cli.env)?);

    // Add inherited vars
    env_vars.extend(env_utils::get_inherited_vars(&cli.inherit_env));

    // Build command string with env exports
    let mut cmd_parts = Vec::new();
    if !env_vars.is_empty() {
        cmd_parts.push(env_utils::build_export_commands(&env_vars));
    }
    cmd_parts.push(command.join(" "));
    let cmd_str = cmd_parts.join("; ");

    // Execute command in VM
    LimaCtl::shell(
        project.template_name(),
        Some(project.root()),
        "bash",
        &["-c", &cmd_str],
        config.forward_ssh_agent,
    )?;

    Ok(())
}
