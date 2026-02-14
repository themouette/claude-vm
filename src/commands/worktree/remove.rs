use crate::error::{ClaudeVmError, Result};
use crate::worktree::state::WorktreeEntry;
use crate::worktree::{filter, operations, recovery, validation};
use std::io::{self, Write};

pub fn execute(
    branches: Option<&[String]>,
    merged_base: Option<&str>,
    yes: bool,
    dry_run: bool,
    locked: bool,
) -> Result<()> {
    // Validate git version
    validation::check_git_version()?;

    // Validate that --locked requires --merged
    if locked && merged_base.is_none() {
        return Err(ClaudeVmError::Worktree(
            "--locked flag requires --merged flag".to_string(),
        ));
    }

    // Validate exclusive modes - either explicit branches OR merged filter
    match (branches, merged_base) {
        (Some(_), Some(_)) => {
            return Err(ClaudeVmError::Worktree(
                "Cannot use both explicit branches and --merged flag".to_string(),
            ));
        }
        (None, None) => {
            return Err(ClaudeVmError::Worktree(
                "Must specify either branch names or --merged flag".to_string(),
            ));
        }
        _ => {}
    }

    // Get current worktree list with auto-prune
    let worktrees = recovery::ensure_clean_state()?;

    // Select worktrees based on mode
    let to_remove: Vec<(String, _)> = if let Some(branch_names) = branches {
        // Explicit branch mode (from delete.rs)
        select_by_explicit_branches(&worktrees, branch_names)?
    } else {
        // Merged branch mode (from clean.rs)
        select_by_merged_status(&worktrees, merged_base, locked)?
    };

    // If no worktrees to remove, exit early
    if to_remove.is_empty() {
        if merged_base.is_some() {
            println!("No merged worktrees to remove.");
        } else {
            println!("No worktrees found to remove.");
        }
        return Ok(());
    }

    // Display what will be removed
    display_worktrees_to_remove(&to_remove, merged_base);

    // If dry-run, exit after displaying
    if dry_run {
        println!("[Dry run - no changes made]");
        return Ok(());
    }

    // Prompt for confirmation unless --yes was provided
    if !yes && !confirm_removal(&to_remove, merged_base)? {
        println!("Aborted.");
        return Ok(());
    }

    // Execute deletion with best-effort error handling
    execute_deletion(&to_remove, merged_base)?;

    Ok(())
}

/// Select worktrees by explicit branch names
fn select_by_explicit_branches(
    worktrees: &[WorktreeEntry],
    branch_names: &[String],
) -> Result<Vec<(String, std::path::PathBuf)>> {
    let mut to_remove = Vec::new();
    let mut missing = Vec::new();

    for branch in branch_names {
        match worktrees
            .iter()
            .find(|e| e.branch.as_deref() == Some(branch))
        {
            Some(worktree) => to_remove.push((branch.clone(), worktree.path.clone())),
            None => missing.push(branch.as_str()),
        }
    }

    // Report missing branches
    if !missing.is_empty() {
        eprintln!("Warning: The following branches have no worktree:");
        for branch in &missing {
            eprintln!("  {}", branch);
        }
        if to_remove.is_empty() {
            return Err(ClaudeVmError::Worktree(
                "No valid worktrees found to remove".to_string(),
            ));
        }
        eprintln!();
    }

    Ok(to_remove)
}

/// Select worktrees by merged status
fn select_by_merged_status(
    worktrees: &[WorktreeEntry],
    merged_base: Option<&str>,
    locked: bool,
) -> Result<Vec<(String, std::path::PathBuf)>> {
    // Resolve the actual base branch
    let merged_base = match merged_base {
        Some(base) if !base.is_empty() => base.to_string(),
        _ => {
            // None or empty string - use current branch
            let branch = crate::utils::git::get_current_branch()?;
            println!("Using current branch: {}", branch);
            branch
        }
    };

    // Get merged branches (this validates base branch exists)
    let merged_branches = operations::list_merged_branches(&merged_base)?;

    // Apply filters: skip main, filter merged, conditionally exclude locked
    let iter = filter::skip_main(worktrees.iter());
    let iter = filter::filter_merged(iter, &merged_branches);

    let filtered_worktrees: Vec<_> = if locked {
        // Include locked worktrees when --locked flag is set
        iter.collect()
    } else {
        // Exclude locked worktrees by default
        filter::exclude_locked(iter).collect()
    };

    // Extract branch and path for display and deletion
    let to_remove: Vec<(String, _)> = filtered_worktrees
        .iter()
        .filter_map(|w| w.branch.as_ref().map(|b| (b.clone(), w.path.clone())))
        .collect();

    Ok(to_remove)
}

/// Display worktrees that will be removed
fn display_worktrees_to_remove(
    to_remove: &[(String, std::path::PathBuf)],
    merged_base: Option<&str>,
) {
    if let Some(base) = merged_base {
        // Merged mode display
        println!("The following worktrees have been merged into '{}':", base);
        for (branch, path) in to_remove {
            println!("  {} -> {}", branch, path.display());
        }
    } else {
        // Explicit mode display
        if to_remove.len() == 1 {
            println!("Worktree: {}", to_remove[0].1.display());
            println!("Branch: {}", &to_remove[0].0);
        } else {
            println!("The following worktrees will be removed:");
            for (branch, path) in to_remove {
                println!("  {} -> {}", branch, path.display());
            }
        }
    }

    println!();
    println!(
        "This will remove the worktree director{}. Branches will be preserved.",
        if to_remove.len() == 1 { "y" } else { "ies" }
    );
    println!();
}

/// Prompt for confirmation
fn confirm_removal(
    to_remove: &[(String, std::path::PathBuf)],
    merged_base: Option<&str>,
) -> Result<bool> {
    let prompt = if merged_base.is_some() {
        "Remove merged worktrees? [y/N] "
    } else if to_remove.len() == 1 {
        "Remove worktree? [y/N] "
    } else {
        "Remove worktrees? [y/N] "
    };

    print!("{}", prompt);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let input = input.trim().to_lowercase();

    Ok(input == "y" || input == "yes")
}

/// Execute deletion of worktrees
fn execute_deletion(
    to_remove: &[(String, std::path::PathBuf)],
    merged_base: Option<&str>,
) -> Result<()> {
    let mut removed_count = 0;
    let multi_worktree = to_remove.len() > 1;
    let is_merged_mode = merged_base.is_some();

    for (branch, _path) in to_remove {
        if multi_worktree {
            print!("Removing: {}...", branch);
            io::stdout().flush().unwrap();
        }

        match operations::delete_worktree(branch.as_str()) {
            Ok(_) => {
                if multi_worktree {
                    println!(" done");
                } else if !is_merged_mode {
                    // Only print individual message for explicit mode with single worktree
                    println!("Worktree removed: {}", branch);
                }
                removed_count += 1;
            }
            Err(e) => {
                if multi_worktree {
                    println!(" failed");
                }
                eprintln!("Warning: Failed to remove worktree '{}': {}", branch, e);
            }
        }
    }

    // Summary message
    if is_merged_mode {
        // Always show summary for merged mode
        println!("Removed {} merged worktree(s).", removed_count);
    } else if multi_worktree {
        // Show summary for multi-worktree explicit mode
        println!();
        println!(
            "Removed {} of {} worktree(s).",
            removed_count,
            to_remove.len()
        );
    }

    Ok(())
}
