use crate::cli::ShellCmd;
use crate::commands::helpers;
use crate::config::Config;
use crate::error::{ClaudeVmError, Result};
use crate::project::Project;
use crate::scripts::{host_executor, runner};
use crate::session::store as session_store;
use crate::utils::env as env_utils;
use crate::utils::shell as shell_utils;
use crate::vm::limactl::LimaCtl;
use crate::vm::session::VmSession;

pub fn execute(project: &Project, config: &Config, cmd: &ShellCmd) -> Result<()> {
    if let Some(session_id) = &cmd.session {
        execute_with_session(project, config, cmd, session_id)
    } else {
        execute_ephemeral(project, config, cmd)
    }
}

/// Open a shell using an existing persistent session VM.
fn execute_with_session(
    project: &Project,
    _config: &Config,
    cmd: &ShellCmd,
    session_id: &str,
) -> Result<()> {
    let record = session_store::get(session_id)?;

    // Use the frozen config from the session record
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

    // Wrap existing VM — no cleanup guard
    let session = VmSession::from_existing(&record.vm_name, project, config.verbose);

    // Execute before_runtime host phases
    if !config.phase.before_runtime.is_empty() {
        host_executor::execute_host_phases(
            &config.phase.before_runtime,
            project,
            session.name(),
            &host_executor::build_host_env(project, "runtime", Some("shell")),
            Some(session_id),
        )?;
    }

    run_shell_in_vm(project, &config, cmd, &session)
}

/// Open a shell in a fresh ephemeral VM.
fn execute_ephemeral(project: &Project, config: &Config, cmd: &ShellCmd) -> Result<()> {
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
    let _cleanup = session.ensure_cleanup_with_config(&config, "shell");
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
            &host_executor::build_host_env(project, "runtime", Some("shell")),
            None,
        )?;
    }

    // Use current directory for workdir
    let current_dir = std::env::current_dir()?;

    // Collect environment variables
    let env_vars = env_utils::collect_env_vars(
        &cmd.runtime.env,
        &cmd.runtime.env_file,
        &cmd.runtime.inherit_env,
    )?;

    let workdir = Some(current_dir.as_path());

    if is_interactive {
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
            "shell",
            child_pid_slot,
            _cleanup.cleanup_flag(),
        )?;
    } else {
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
            "shell",
            child_pid_slot,
            _cleanup.cleanup_flag(),
        ) {
            Ok(()) => {}
            Err(ClaudeVmError::CommandExitCode(code)) => {
                std::process::exit(code);
            }
            Err(e) => return Err(e),
        }
    }

    // Execute host-side after_runtime phases
    if !config.phase.host.after_runtime.is_empty() {
        host_executor::execute_host_phases(
            &config.phase.host.after_runtime,
            project,
            session.name(),
            &host_executor::build_host_env(project, "runtime", Some("shell")),
            None,
        )?;
    }

    Ok(())
}

/// Core shell execution shared by both ephemeral and session paths.
fn run_shell_in_vm(
    project: &Project,
    config: &Config,
    cmd: &ShellCmd,
    session: &VmSession,
) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let env_vars = env_utils::collect_env_vars(
        &cmd.runtime.env,
        &cmd.runtime.env_file,
        &cmd.runtime.inherit_env,
    )?;
    let workdir = Some(current_dir.as_path());

    use std::sync::atomic::{AtomicBool, AtomicU32};
    use std::sync::Arc;
    let child_pid_slot = Arc::new(AtomicU32::new(0));
    let cleanup_flag = Arc::new(AtomicBool::new(false));

    if cmd.command.is_empty() {
        println!(
            "VM: {} | Dir: {} | Project: {}",
            session.name(),
            current_dir.display(),
            project.template_name()
        );
        println!("Type 'exit' to leave the shell (VM stays running)");

        runner::execute_command_with_runtime_scripts(
            session.name(),
            project,
            config,
            session,
            workdir,
            "bash",
            &["-l"],
            &env_vars,
            "shell",
            child_pid_slot,
            cleanup_flag,
        )?;
    } else {
        eprintln!("Executing command in VM: {}", session.name());

        let cmd_str = shell_utils::join_args(&cmd.command);
        match runner::execute_command_with_runtime_scripts(
            session.name(),
            project,
            config,
            session,
            workdir,
            "bash",
            &["-c", &cmd_str],
            &env_vars,
            "shell",
            child_pid_slot,
            cleanup_flag,
        ) {
            Ok(()) => {}
            Err(ClaudeVmError::CommandExitCode(code)) => {
                std::process::exit(code);
            }
            Err(e) => return Err(e),
        }
    }

    // Execute host-side after_runtime phases
    if !config.phase.host.after_runtime.is_empty() {
        host_executor::execute_host_phases(
            &config.phase.host.after_runtime,
            project,
            session.name(),
            &host_executor::build_host_env(project, "runtime", Some("shell")),
            None,
        )?;
    }

    Ok(())
}
