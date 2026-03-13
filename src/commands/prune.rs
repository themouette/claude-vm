use crate::error::Result;
use crate::session::store as session_store;
use crate::vm::limactl::{LimaCtl, VmInfo};
use crate::vm::session::{extract_session_pid, is_pid_running, is_session_vm};
use std::io::{self, Write};

struct OrphanVm {
    info: VmInfo,
    pid: Option<u32>,
}

pub fn execute(yes: bool) -> Result<()> {
    let vms = LimaCtl::list()?;

    // Filter to session VMs only, then classify each
    let orphans: Vec<OrphanVm> = vms
        .into_iter()
        .filter(|vm| is_session_vm(&vm.name))
        .filter_map(|vm| {
            let pid = extract_session_pid(&vm.name);
            let is_stopped = vm.status.eq_ignore_ascii_case("stopped");
            let is_running = vm.status.eq_ignore_ascii_case("running");

            if is_stopped {
                // Stopped VMs are always orphans
                Some(OrphanVm { info: vm, pid })
            } else if is_running {
                // Running VMs are orphans only if their parent process is dead
                let pid_alive = pid.map(is_pid_running).unwrap_or(false);
                if pid_alive {
                    None // Active session, skip
                } else {
                    Some(OrphanVm { info: vm, pid })
                }
            } else {
                None // Unknown status, skip
            }
        })
        .collect();

    if orphans.is_empty() {
        println!("No orphaned session VMs found.");
        return Ok(());
    }

    let has_running_orphans = orphans
        .iter()
        .any(|o| o.info.status.eq_ignore_ascii_case("running"));

    // Print table
    println!("{:<50} {:<10} {:<12}", "VM Name", "Status", "PID");
    println!("{}", "-".repeat(74));
    for orphan in &orphans {
        let pid_str = match orphan.pid {
            Some(pid) => pid.to_string(),
            None => "unknown".to_string(),
        };
        println!(
            "{:<50} {:<10} {:<12}",
            orphan.info.name, orphan.info.status, pid_str,
        );
    }
    println!();

    if has_running_orphans {
        println!("⚠  Warning: Some VMs are Running but their parent process is gone.");
        println!("   These will be force-stopped before deletion.");
        println!();
    }

    // Prompt for confirmation unless --yes provided
    if !yes {
        print!("Delete {} orphaned session VM(s)? [y/N]: ", orphans.len());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input != "y" && input != "yes" {
            println!("Aborted.");
            return Ok(());
        }
    } else if has_running_orphans {
        println!("   Proceeding with force-stop and deletion (--yes).");
        println!();
    }

    // Delete each orphan
    let mut deleted = 0;
    let mut failed = 0;
    for orphan in &orphans {
        let name = &orphan.info.name;
        if orphan.info.status.eq_ignore_ascii_case("running") {
            print!("  Stopping {}... ", name);
            io::stdout().flush()?;
            if let Err(e) = LimaCtl::stop(name, false) {
                println!("failed to stop: {}", e);
                failed += 1;
                continue;
            }
            println!("done");
        }
        print!("  Deleting {}... ", name);
        io::stdout().flush()?;
        match LimaCtl::delete(name, true, false) {
            Ok(()) => {
                println!("done");
                deleted += 1;
            }
            Err(e) => {
                println!("failed: {}", e);
                failed += 1;
            }
        }
    }

    println!();
    println!(
        "Pruned {} VM(s){}.",
        deleted,
        if failed > 0 {
            format!(", {} failed", failed)
        } else {
            String::new()
        }
    );

    // Also clean up orphaned session records (records whose VM no longer exists)
    match session_store::prune_orphaned_records() {
        Ok(removed) if removed > 0 => {
            println!("Removed {} orphaned session record(s).", removed);
        }
        Ok(_) => {}
        Err(e) => {
            eprintln!("Warning: Failed to prune orphaned session records: {}", e);
        }
    }

    Ok(())
}
