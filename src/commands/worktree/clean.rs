use crate::error::Result;
use crate::worktree::{operations, recovery, validation};
use std::io::{self, Write};

pub fn execute(merged_base: &str, yes: bool) -> Result<()> {
    // Validate git version supports worktrees
    validation::check_git_version()?;

    // Get merged branches (this validates base branch exists)
    let merged_branches = operations::list_merged_branches(merged_base)?;

    // Get current worktree list with auto-prune
    let worktrees = recovery::ensure_clean_state()?;

    // Cross-reference: find worktrees whose branch is in the merged list
    // Skip the main worktree (first entry) and worktrees without a branch
    let mut merged_worktrees = Vec::new();
    for worktree in worktrees.iter().skip(1) {
        if let Some(ref branch) = worktree.branch {
            if merged_branches.contains(branch) {
                merged_worktrees.push((branch.as_str(), &worktree.path));
            }
        }
    }

    // If no merged worktrees found, exit early
    if merged_worktrees.is_empty() {
        println!("No merged worktrees to clean.");
        return Ok(());
    }

    // Display what will be cleaned
    println!(
        "The following worktrees have been merged into '{}':",
        merged_base
    );
    for (branch, path) in &merged_worktrees {
        println!("  {} -> {}", branch, path.display());
    }
    println!();
    println!("This will remove the worktree directories. Branches will be preserved.");
    println!();

    // Prompt for confirmation unless --yes was provided
    if !yes {
        print!("Clean merged worktrees? [y/N] ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let input = input.trim().to_lowercase();

        if input != "y" && input != "yes" {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Delete each merged worktree (best-effort, continue on individual failures)
    let mut cleaned_count = 0;
    for (branch, _path) in &merged_worktrees {
        print!("Removing: {}...", branch);
        io::stdout().flush().unwrap();

        match operations::delete_worktree(branch) {
            Ok(_) => {
                println!(" done");
                cleaned_count += 1;
            }
            Err(e) => {
                println!(" failed");
                eprintln!("Warning: Failed to delete worktree '{}': {}", branch, e);
            }
        }
    }

    println!("Cleaned {} merged worktree(s).", cleaned_count);

    Ok(())
}
