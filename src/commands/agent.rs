use crate::cli::AgentCmd;
use crate::commands::helpers;
use crate::config::Config;
use crate::error::{ClaudeVmError, Result};
use crate::project::Project;
use crate::scripts::{host_executor, runner};
use crate::session::store as session_store;
use crate::utils::env as env_utils;
use crate::vm::limactl::LimaCtl;
use crate::vm::session::VmSession;

pub fn execute(project: &Project, config: &Config, cmd: &AgentCmd) -> Result<()> {
    if let Some(session_id) = &cmd.session {
        execute_with_session(project, config, cmd, session_id)
    } else {
        execute_ephemeral(project, config, cmd)
    }
}

/// Run Claude using an existing persistent session VM.
fn execute_with_session(
    project: &Project,
    _config: &Config,
    cmd: &AgentCmd,
    session_id: &str,
) -> Result<()> {
    let record = session_store::get(session_id)?;

    // Use the frozen config from the session record (capability phases already merged)
    let config = record.config.clone();

    // Verify the VM is running
    let vms = LimaCtl::list()?;
    let vm_info = vms.iter().find(|vm| vm.name == record.vm_name);
    match vm_info {
        None => {
            return Err(ClaudeVmError::CommandFailed(format!(
                "Session '{}' VM '{}' does not exist. Has it been stopped?",
                session_id, record.vm_name
            )));
        }
        Some(vm) if !vm.status.eq_ignore_ascii_case("running") => {
            return Err(ClaudeVmError::CommandFailed(format!(
                "Session '{}' VM '{}' is not running (status: {}).",
                session_id, record.vm_name, vm.status
            )));
        }
        _ => {}
    }

    // Resolve worktree if --worktree flag present
    if !cmd.runtime.worktree.is_empty() {
        let worktree_path = helpers::resolve_worktree(&cmd.runtime.worktree, &config, project)?;
        std::env::set_current_dir(&worktree_path)?;
    }

    eprintln!(
        "Attaching to persistent session {} ({})...",
        session_id, record.vm_name
    );

    // Wrap existing VM — no cleanup guard (lifetime owned by session)
    let session = VmSession::from_existing(&record.vm_name, project, config.verbose);

    // Execute before_runtime host phases
    if !config.phase.before_runtime.is_empty() {
        host_executor::execute_host_phases(
            &config.phase.before_runtime,
            project,
            session.name(),
            &host_executor::build_host_env(project, "runtime", Some("agent")),
            Some(session_id),
        )?;
    }

    run_claude_in_vm(project, &config, cmd, &session)
}

/// Run Claude in a fresh ephemeral VM.
fn execute_ephemeral(project: &Project, config: &Config, cmd: &AgentCmd) -> Result<()> {
    // Clone config to allow merging capability phases
    let mut config = config.clone();

    // Merge capability-defined phases with user-defined phases
    crate::capabilities::merge_capability_phases(&mut config, None)?;

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
            None,
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
    let check_claude = LimaCtl::shell(session.name(), None, "command", &["-v", "claude"], false);

    if check_claude.is_err() {
        return Err(ClaudeVmError::CommandFailed(
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
        _cleanup.cleanup_flag(),
    )?;

    // Execute host-side after_runtime phases
    if !config.phase.host.after_runtime.is_empty() {
        host_executor::execute_host_phases(
            &config.phase.host.after_runtime,
            project,
            session.name(),
            &host_executor::build_host_env(project, "runtime", Some("agent")),
            None,
        )?;
    }

    Ok(())
}

/// Core Claude execution shared by both ephemeral and session paths.
///
/// Preconditions: `session.name()` VM is already running, `before_runtime`
/// host phases have already been executed.
fn run_claude_in_vm(
    project: &Project,
    config: &Config,
    cmd: &AgentCmd,
    session: &VmSession,
) -> Result<()> {
    // Build Claude command with arguments
    let mut args: Vec<&str> = Vec::new();

    for arg in &config.defaults.claude_args {
        args.push(arg.as_str());
    }
    for arg in &cmd.claude_args {
        args.push(arg.as_str());
    }

    eprintln!("Running Claude in VM: {}", session.name());

    // Check if claude is installed in the VM
    let check_claude = LimaCtl::shell(session.name(), None, "command", &["-v", "claude"], false);
    if check_claude.is_err() {
        return Err(ClaudeVmError::CommandFailed(
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

    let env_vars = env_utils::collect_env_vars(
        &cmd.runtime.env,
        &cmd.runtime.env_file,
        &cmd.runtime.inherit_env,
    )?;

    let current_dir = std::env::current_dir()?;
    let workdir = Some(current_dir.as_path());

    // For persistent sessions we have no cleanup guard — pass dummy atomics.
    use std::sync::atomic::{AtomicBool, AtomicU32};
    use std::sync::Arc;
    let child_pid_slot = Arc::new(AtomicU32::new(0));
    let cleanup_flag = Arc::new(AtomicBool::new(false));

    runner::execute_command_with_runtime_scripts(
        session.name(),
        project,
        config,
        session,
        workdir,
        "claude",
        &args,
        &env_vars,
        "agent",
        child_pid_slot,
        cleanup_flag,
    )?;

    // Execute host-side after_runtime phases
    if !config.phase.host.after_runtime.is_empty() {
        host_executor::execute_host_phases(
            &config.phase.host.after_runtime,
            project,
            session.name(),
            &host_executor::build_host_env(project, "runtime", Some("agent")),
            None,
        )?;
    }

    Ok(())
}
