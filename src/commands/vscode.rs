use crate::cli::VscodeCmd;
use crate::commands::helpers;
use crate::config::Config;
use crate::error::{ClaudeVmError, Result};
use crate::project::Project;
use crate::scripts::host_executor;
use crate::vm::session::VmSession;
use crate::vm::ssh_config;
use std::sync::atomic::Ordering;

pub fn execute(project: &Project, config: &Config, cmd: &VscodeCmd) -> Result<()> {
    let mut config = config.clone();
    crate::capabilities::merge_capability_phases(&mut config)?;

    helpers::ensure_template_exists(project, &config)?;
    helpers::auto_prune_stopped_sessions(config.verbose);
    crate::resources::check_before_vm_creation(&config.vm, cmd.force_resources, config.verbose)?;

    // Mount persistent vscode-server dir so extensions/settings/auth survive across sessions
    let home = std::env::var("HOME")
        .map_err(|_| ClaudeVmError::CommandFailed("HOME not set".into()))?;
    let user = std::env::var("USER")
        .map_err(|_| ClaudeVmError::CommandFailed("USER not set".into()))?;
    let server_dir_name = if config.vscode_binary.as_deref() == Some("code-insiders") {
        ".vscode-server-insiders"
    } else {
        ".vscode-server"
    };
    // Lima always creates homedir as /home/{USER}.linux
    let vm_homedir = format!("/home/{}.linux", user);
    let persist_path = std::path::PathBuf::from(&home)
        .join(".claude-vm")
        .join("vscode-server")
        .join(project.template_name());
    std::fs::create_dir_all(&persist_path)?;
    config.mounts.push(crate::config::MountEntry {
        location: persist_path.to_string_lossy().to_string(),
        writable: true,
        mount_point: Some(format!("{}/{}", vm_homedir, server_dir_name)),
    });

    // Resolve worktree if --worktree flag present
    if !cmd.runtime.worktree.is_empty() {
        let worktree_path = helpers::resolve_worktree(&cmd.runtime.worktree, &config, project)?;
        std::env::set_current_dir(&worktree_path)?;
    }

    // Resolve VSCode binary
    let vscode_bin = config
        .vscode_binary
        .as_deref()
        .unwrap_or("code");

    // Verify VSCode binary is available on host
    if which::which(vscode_bin).is_err() {
        return Err(ClaudeVmError::CommandFailed(format!(
            "'{}' not found on PATH. Install VSCode or set vscode_binary in config.",
            vscode_bin
        )));
    }

    if !config.verbose {
        eprintln!("Starting ephemeral VM session for VSCode...");
    }

    // Create ephemeral session
    let session = VmSession::new(
        project,
        config.verbose,
        config.mount_conversations,
        &config.mounts,
    )?;
    let _cleanup = session.ensure_cleanup_with_config(&config, "vscode");
    if let Err(e) = _cleanup.register_signal_handler() {
        eprintln!("⚠  Could not register signal handler: {}", e);
    }

    // Execute before_runtime host phases
    if !config.phase.before_runtime.is_empty() {
        host_executor::execute_host_phases(
            &config.phase.before_runtime,
            project,
            session.name(),
            &host_executor::build_host_env(project, "runtime", Some("vscode")),
        )?;
    }

    // Get SSH config from Lima and write managed config
    let ssh_config_block = crate::vm::limactl::LimaCtl::show_ssh_config(session.name())?;
    if config.verbose {
        eprintln!("SSH config:\n{}", ssh_config_block);
    }

    ssh_config::ensure_ssh_include()?;
    let host_alias = ssh_config::write_session_config(session.name())?;

    // Resolve workdir
    let current_dir = std::env::current_dir()?;

    // Launch VSCode
    let folder_uri = format!(
        "vscode-remote://ssh-remote+{}/{}",
        host_alias,
        current_dir.display()
    );

    eprintln!(
        "Opening VSCode: {} -> {}",
        vscode_bin,
        current_dir.display()
    );

    let status = std::process::Command::new(vscode_bin)
        .args(["--folder-uri", &folder_uri])
        .status()
        .map_err(|e| ClaudeVmError::CommandFailed(format!("Failed to launch VSCode: {}", e)))?;

    if !status.success() {
        return Err(ClaudeVmError::CommandFailed(format!(
            "VSCode exited with status: {}",
            status
        )));
    }

    eprintln!(
        "\nVM: {} | Dir: {} | Project: {}",
        session.name(),
        current_dir.display(),
        project.template_name()
    );
    eprintln!("VM is running. Press Ctrl+C to stop and delete.");

    // Block until signal handler fires
    let cleanup_flag = _cleanup.cleanup_flag();
    loop {
        std::thread::park();
        if cleanup_flag.load(Ordering::SeqCst) {
            break;
        }
    }

    // Clean up SSH config
    ssh_config::remove_session_config();

    // Execute host-side after_runtime phases
    if !config.phase.host.after_runtime.is_empty() {
        host_executor::execute_host_phases(
            &config.phase.host.after_runtime,
            project,
            session.name(),
            &host_executor::build_host_env(project, "runtime", Some("vscode")),
        )?;
    }

    Ok(())
}
