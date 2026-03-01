use crate::cli::AgentCmd;
use crate::commands::helpers;
use crate::config::Config;
use crate::error::Result;
use crate::project::Project;
use crate::scripts::{host_executor, runner};
use crate::utils::env as env_utils;
use crate::vm::session::VmSession;

pub fn execute(project: &Project, config: &Config, cmd: &AgentCmd) -> Result<()> {
    // Clone config to allow merging capability phases
    let mut config = config.clone();

    // Merge capability-defined phases with user-defined phases
    crate::capabilities::merge_capability_phases(&mut config)?;

    // Ensure template exists (create if missing and user confirms)
    helpers::ensure_template_exists(project, &config)?;

    // Clean up any orphaned stopped session VMs from previous killed sessions
    helpers::auto_prune_stopped_sessions(config.verbose);

    // Check resource allocation before creating VM
    crate::resources::check_before_vm_creation(&config.vm, cmd.force_resources, config.verbose)?;

    // Resolve worktree if --worktree flag present
    if !cmd.runtime.worktree.is_empty() {
        let worktree_path = helpers::resolve_worktree(&cmd.runtime.worktree, &config, project)?;
        std::env::set_current_dir(&worktree_path)?;
    }

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
    let _cleanup = session.ensure_cleanup_with_config(&config, "agent");
    let child_pid_slot = _cleanup.child_pid_slot();
    // Best-effort: warn but don't abort if signal handler registration fails
    if let Err(e) = _cleanup.register_signal_handler() {
        eprintln!("⚠  Could not register signal handler: {}", e);
    }

    // Execute before_runtime host phases
    if !config.phase.before_runtime.is_empty() {
        host_executor::execute_host_phases(
            &config.phase.before_runtime,
            project,
            session.name(),
            &host_executor::build_host_env(project, "runtime", Some("agent")),
        )?;
    }

    // Build Claude command with arguments
    let mut args: Vec<&str> = Vec::new();

    // Add default Claude args from config
    for arg in &config.defaults.claude_args {
        args.push(arg.as_str());
    }

    // Add user-provided Claude args
    for arg in &cmd.claude_args {
        args.push(arg.as_str());
    }

    eprintln!("Running Claude in VM: {}", session.name());

    // Check if claude is installed in the VM
    let check_claude = crate::vm::limactl::LimaCtl::shell(
        session.name(),
        None,
        "command",
        &["-v", "claude"],
        false,
    );

    if check_claude.is_err() {
        return Err(crate::error::ClaudeVmError::CommandFailed(
            "Claude CLI is not installed in the VM.\n\
             \n\
             If you used --no-agent-install during setup, you cannot run 'claude-vm agent'.\n\
             Instead, use:\n\
             - 'claude-vm shell' to open a shell in the VM\n\
             - 'claude-vm shell <command>' to run a specific command\n\
             \n\
             Or run 'claude-vm setup' without --no-agent-install to install the Claude agent."
                .to_string(),
        ));
    }

    // Collect environment variables
    let env_vars = env_utils::collect_env_vars(
        &cmd.runtime.env,
        &cmd.runtime.env_file,
        &cmd.runtime.inherit_env,
    )?;

    // Execute Claude with runtime scripts using entrypoint pattern
    // This runs runtime scripts first, then execs Claude in a single shell invocation
    let current_dir = std::env::current_dir()?;
    let workdir = Some(current_dir.as_path());
    runner::execute_command_with_runtime_scripts(
        session.name(),
        project,
        &config,
        &session,
        workdir,
        "claude",
        &args,
        &env_vars,
        "agent",
        child_pid_slot,
    )?;

    // Execute host-side after_runtime phases
    if !config.phase.host.after_runtime.is_empty() {
        host_executor::execute_host_phases(
            &config.phase.host.after_runtime,
            project,
            session.name(),
            &host_executor::build_host_env(project, "runtime", Some("agent")),
        )?;
    }

    Ok(())
}
