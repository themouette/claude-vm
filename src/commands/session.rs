use crate::cli::{SessionCmd, SessionSubcommand};
use crate::commands::helpers;
use crate::config::Config;
use crate::error::Result;
use crate::project::Project;
use crate::scripts::host_executor;
use crate::session::{store, SessionRecord};
use crate::vm::limactl::LimaCtl;
use crate::vm::mount;
use crate::vm::session::{extract_persistent_session_id, generate_persistent_vm_name};
use chrono::Utc;
use std::io::{self, Write};

/// Dispatch session subcommand (Start only — Stop/List are dispatched directly from main).
pub fn execute(project: &Project, config: &Config, cmd: &SessionCmd) -> Result<()> {
    match &cmd.subcommand {
        SessionSubcommand::Start => start(project, config),
        SessionSubcommand::Stop { id } => stop(id),
        SessionSubcommand::List => list(),
    }
}

/// Called directly from main for `session stop <id>` (no project required).
pub fn execute_stop(id: &str) -> Result<()> {
    stop(id)
}

/// Called directly from main for `session list` (no project required).
pub fn execute_list() -> Result<()> {
    list()
}

/// Start a persistent session: clone + start VM, persist record, print session ID.
fn start(project: &Project, config: &Config) -> Result<()> {
    // Generate a persistent VM name early so we can extract the session_id for
    // capability mount variable substitution.
    let vm_name = generate_persistent_vm_name(project.template_name());
    let session_id = extract_persistent_session_id(&vm_name).ok_or_else(|| {
        crate::error::ClaudeVmError::InvalidConfig(
            "failed to extract session id from vm name".to_string(),
        )
    })?;

    // Merge capability phases into a local config copy (this is the frozen snapshot)
    let mut config = config.clone();
    crate::capabilities::merge_capability_phases(&mut config, Some(session_id))?;

    // Ensure template exists
    helpers::ensure_template_exists(project, &config)?;

    // Auto-prune stopped ephemeral sessions
    helpers::auto_prune_stopped_sessions(config.verbose);

    // Check resource allocation
    crate::resources::check_before_vm_creation(&config.vm, false, config.verbose)?;

    // Compute mounts
    let mounts = mount::compute_mounts(config.mount_conversations, &config.mounts)?;
    mount::ensure_mount_sources_exist(&mounts)?;

    eprintln!("Starting persistent VM session...");

    // Clone the template
    LimaCtl::clone(project.template_name(), &vm_name, &mounts, config.verbose)?;

    // Start the VM (clean up if start fails)
    if let Err(e) = LimaCtl::start(&vm_name, config.verbose) {
        eprintln!("Failed to start VM, cleaning up...");
        let _ = LimaCtl::stop(&vm_name, config.verbose);
        let _ = LimaCtl::delete(&vm_name, true, config.verbose);
        return Err(e);
    }

    // Execute before_runtime host phases
    if !config.phase.before_runtime.is_empty() {
        host_executor::execute_host_phases(
            &config.phase.before_runtime,
            project,
            &vm_name,
            &host_executor::build_host_env(project, "runtime", Some("session")),
            Some(session_id),
        )?;
    }

    // Persist the session record
    let record = SessionRecord {
        id: session_id.to_string(),
        vm_name: vm_name.clone(),
        template_name: project.template_name().to_string(),
        project_root: project.root().to_path_buf(),
        created_at: Utc::now(),
        config,
    };
    store::create(&record)?;

    // Print session ID to stdout (machine-readable)
    println!("{}", session_id);

    eprintln!("Session started: {} (VM: {})", session_id, vm_name);

    Ok(())
}

/// Stop a persistent session: run teardown phases, stop + delete VM, remove record.
fn stop(id: &str) -> Result<()> {
    let record = store::get(id)?;

    // Verify VM exists before attempting teardown
    let vms = LimaCtl::list()?;
    let vm_status = vms
        .iter()
        .find(|vm| vm.name == record.vm_name)
        .map(|vm| vm.status.as_str())
        .unwrap_or("gone");

    if vm_status == "gone" {
        eprintln!(
            "VM '{}' no longer exists, removing session record.",
            record.vm_name
        );
        store::delete(id)?;
        return Ok(());
    }

    // Build a minimal project for host phase execution
    let project = Project::new_for_test(record.project_root.clone());

    // Execute host-based teardown phases using the frozen config
    if !record.config.phase.host.teardown.is_empty() {
        if let Err(e) = host_executor::execute_host_phases(
            &record.config.phase.host.teardown,
            &project,
            &record.vm_name,
            &host_executor::build_host_env(&project, "teardown", Some("session")),
            Some(&record.id),
        ) {
            eprintln!("Warning: Teardown phases failed: {}", e);
        }
    }

    eprintln!("Stopping VM {}...", record.vm_name);
    LimaCtl::stop(&record.vm_name, false)?;

    eprintln!("Deleting VM {}...", record.vm_name);
    LimaCtl::delete(&record.vm_name, true, false)?;

    store::delete(id)?;
    eprintln!("Session {} stopped.", id);

    Ok(())
}

/// List all persistent sessions.
fn list() -> Result<()> {
    let records = store::list()?;

    if records.is_empty() {
        println!("No active sessions.");
        return Ok(());
    }

    println!(
        "{:<12} {:<50} {:<20} {:<10} {:<20}",
        "ID", "VM Name", "Project", "Status", "Created"
    );
    println!("{}", "-".repeat(115));

    for (record, status) in &records {
        let project = record
            .project_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?");
        let created = record.created_at.format("%Y-%m-%d %H:%M UTC").to_string();
        println!(
            "{:<12} {:<50} {:<20} {:<10} {:<20}",
            record.id, record.vm_name, project, status, created
        );
    }

    Ok(())
}

/// Interactively confirm and clean up orphaned session records.
pub fn prune_orphaned_records(yes: bool) -> Result<()> {
    let removed = if yes {
        store::prune_orphaned_records()?
    } else {
        // Determine which records are orphaned first
        let records = store::list()?;
        let orphaned: Vec<_> = records
            .iter()
            .filter(|(_, status)| status == "gone")
            .collect();

        if orphaned.is_empty() {
            return Ok(());
        }

        println!("Found {} orphaned session record(s):", orphaned.len());
        for (record, _) in &orphaned {
            println!("  {} (VM: {})", record.id, record.vm_name);
        }

        print!("Remove these session records? [y/N]: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes" {
            store::prune_orphaned_records()?
        } else {
            return Ok(());
        }
    };

    if removed > 0 {
        eprintln!("Removed {} orphaned session record(s).", removed);
    }

    Ok(())
}

/// Clean up session records for a specific template (called from clean/clean-all).
pub fn clean_records_for_template(template_name: &str) -> Result<()> {
    let removed = store::prune_records_for_template(template_name)?;
    if removed > 0 {
        eprintln!(
            "Removed {} session record(s) for template {}.",
            removed, template_name
        );
    }
    Ok(())
}
