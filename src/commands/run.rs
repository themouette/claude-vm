use crate::cli::Cli;
use crate::commands::helpers;
use crate::config::Config;
use crate::error::Result;
use crate::project::Project;
use crate::scripts::runner;
use crate::utils::env as env_utils;
use crate::vm::session::VmSession;

pub fn execute(
    project: &Project,
    config: &Config,
    cli: &Cli,
    claude_args: &[String],
) -> Result<()> {
    // Ensure template exists (create if missing and user confirms)
    helpers::ensure_template_exists(project, config)?;

    if !config.verbose {
        eprintln!("Starting ephemeral VM session...");
    }

    // Create session
    let session = VmSession::new(
        project,
        config.verbose,
        config.mount_conversations,
        &config.mounts,
    )?;
    let _cleanup = session.ensure_cleanup();

    // Build Claude command with arguments
    let mut args: Vec<&str> = Vec::new();

    // Add default Claude args from config
    for arg in &config.defaults.claude_args {
        args.push(arg.as_str());
    }

    // Add user-provided Claude args
    for arg in claude_args {
        args.push(arg.as_str());
    }

    eprintln!("Running Claude in VM: {}", session.name());

    // Collect environment variables
    let env_vars = env_utils::collect_env_vars(&cli.env, &cli.env_file, &cli.inherit_env)?;

    // Execute Claude with runtime scripts using entrypoint pattern
    // This runs runtime scripts first, then execs Claude in a single shell invocation
    let current_dir = std::env::current_dir()?;
    let workdir = Some(current_dir.as_path());
    runner::execute_command_with_runtime_scripts(
        session.name(),
        project,
        config,
        &session,
        workdir,
        "claude",
        &args,
        &env_vars,
    )?;

    Ok(())
}
