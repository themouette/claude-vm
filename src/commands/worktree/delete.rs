use crate::error::{ClaudeVmError, Result};
use crate::worktree::{operations, recovery, validation};
use std::io::{self, Write};

pub fn execute(branches: &[String], yes: bool, dry_run: bool) -> Result<()> {
    // Validate git version
    validation::check_git_version()?;

    // Get current worktree list
    let worktrees = recovery::ensure_clean_state()?;

    // Find all requested worktrees and collect errors for missing ones
    let mut to_delete = Vec::new();
    let mut missing = Vec::new();

    for branch in branches {
        match worktrees.iter().find(|e| e.branch.as_deref() == Some(branch)) {
            Some(worktree) => to_delete.push((branch.as_str(), &worktree.path)),
            None => missing.push(branch.as_str()),
        }
    }

    // Report missing branches
    if !missing.is_empty() {
        eprintln!("Warning: The following branches have no worktree:");
        for branch in &missing {
            eprintln!("  {}", branch);
        }
        if to_delete.is_empty() {
            return Err(ClaudeVmError::Worktree(
                "No valid worktrees found to delete".to_string()
            ));
        }
        eprintln!();
    }

    // Display what will be deleted
    if to_delete.len() == 1 {
        println!("Worktree: {}", to_delete[0].1.display());
        println!("Branch: {}", to_delete[0].0);
    } else {
        println!("The following worktrees will be deleted:");
        for (branch, path) in &to_delete {
            println!("  {} -> {}", branch, path.display());
        }
    }
    println!();
    println!("This will remove the worktree director{}. Branches will be preserved.",
        if to_delete.len() == 1 { "y" } else { "ies" });
    println!();

    // If dry-run, exit after displaying
    if dry_run {
        println!("[Dry run - no changes made]");
        return Ok(());
    }

    // Prompt for confirmation unless --yes was provided
    if !yes {
        print!("Delete worktree{}? [y/N] ", if to_delete.len() == 1 { "" } else { "s" });
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let input = input.trim().to_lowercase();

        if input != "y" && input != "yes" {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Delete worktrees with best-effort error handling
    let mut deleted_count = 0;
    for (branch, _path) in &to_delete {
        if to_delete.len() > 1 {
            print!("Deleting: {}...", branch);
            io::stdout().flush().unwrap();
        }

        match operations::delete_worktree(branch) {
            Ok(_) => {
                if to_delete.len() > 1 {
                    println!(" done");
                } else {
                    println!("Worktree deleted: {}", branch);
                }
                deleted_count += 1;
            }
            Err(e) => {
                if to_delete.len() > 1 {
                    println!(" failed");
                }
                eprintln!("Warning: Failed to delete worktree '{}': {}", branch, e);
            }
        }
    }

    // Summary for batch operations
    if to_delete.len() > 1 {
        println!();
        println!("Deleted {} of {} worktree(s).", deleted_count, to_delete.len());
    }

    Ok(())
}
