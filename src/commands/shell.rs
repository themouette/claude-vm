use crate::config::Config;
use crate::error::Result;
use crate::project::Project;
use crate::scripts::runner;
use crate::vm::{session::VmSession, template};

pub fn execute(project: &Project, config: &Config) -> Result<()> {
    // Verify template exists
    template::verify(project.template_name())?;

    if !config.verbose {
        println!("Starting ephemeral VM session for shell...");
    }

    // Create ephemeral session (like run command)
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

    println!(
        "VM: {} | Dir: {} | Project: {}",
        session.name(),
        current_dir.display(),
        project.template_name()
    );
    println!("Type 'exit' to stop and delete the VM");

    // Open interactive shell with runtime scripts using entrypoint pattern
    let workdir = Some(current_dir.as_path());
    runner::execute_command_with_runtime_scripts(
        session.name(),
        project,
        config,
        workdir,
        "bash",
        &["-l"],
    )?;

    Ok(())
}
