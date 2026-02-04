use crate::config::Config;
use crate::error::Result;
use crate::project::Project;
use crate::scripts::runner;
use crate::vm::{session::VmSession, template};

pub fn execute(project: &Project, config: &Config, agent_args: &[String]) -> Result<()> {
    // Verify template exists
    template::verify(project.template_name())?;

    if !config.verbose {
        println!("Starting ephemeral VM session...");
    }

    // Create session
    let session = VmSession::new(
        project,
        config.verbose,
        config.mount_conversations,
        &config.mounts,
    )?;
    let _cleanup = session.ensure_cleanup();

    // Build agent command with arguments
    let mut args: Vec<&str> = Vec::new();

    // Add default agent args from config
    for arg in &config.defaults.agent_args {
        args.push(arg.as_str());
    }

    // Add user-provided agent args
    for arg in agent_args {
        args.push(arg.as_str());
    }

    println!("Running agent in VM: {}", session.name());

    // Execute agent with runtime scripts using entrypoint pattern
    // This runs runtime scripts first, then execs the agent wrapper in a single shell invocation
    // The agent wrapper will exec the actual configured agent (claude, opencode, etc.)
    let current_dir = std::env::current_dir()?;
    let workdir = Some(current_dir.as_path());
    runner::execute_command_with_runtime_scripts(
        session.name(),
        project,
        config,
        &session,
        workdir,
        "agent",
        &args,
    )?;

    Ok(())
}
