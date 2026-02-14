use crate::error::Result;
use crate::worktree::{filter, operations, recovery, state, validation};

pub fn execute(merged_base: Option<&str>, locked: bool, detached: bool) -> Result<()> {
    // Validate git version
    validation::check_git_version()?;

    // Get worktree list with auto-prune
    let worktrees = recovery::ensure_clean_state()?;

    // Apply filters - IMPORTANT: skip main BEFORE other filters to avoid the bug
    // where skip(1) removes the wrong entry when main doesn't match filters

    // Get merged branches if needed (must live long enough for the iterator)
    let merged_branches = if let Some(base) = merged_base {
        Some(operations::list_merged_branches(base)?)
    } else {
        None
    };

    // Build filter chain
    let iter = filter::skip_main(worktrees.iter());

    // Apply merged filter if requested
    let filtered_worktrees: Vec<&state::WorktreeEntry> = if let Some(ref branches) = merged_branches
    {
        let iter = filter::filter_merged(iter, branches);

        // Chain additional filters
        if locked {
            filter::filter_locked(iter).collect()
        } else if detached {
            filter::filter_detached(iter).collect()
        } else {
            iter.collect()
        }
    } else {
        // No merged filter, just apply locked/detached if requested
        if locked {
            filter::filter_locked(iter).collect()
        } else if detached {
            filter::filter_detached(iter).collect()
        } else {
            iter.collect()
        }
    };

    // Display results
    if filtered_worktrees.is_empty() {
        println!("No additional worktrees found matching filters.");
        return Ok(());
    }

    // Note: No need for skip(1) here - we already skipped main above
    let display_worktrees = &filtered_worktrees;

    println!("Worktrees:");
    for worktree in display_worktrees {
        let branch_display = worktree.branch.as_deref().unwrap_or("<detached>");

        let mut status_tags = Vec::new();
        if worktree.is_bare {
            status_tags.push("bare".to_string());
        }
        if worktree.is_detached {
            status_tags.push("detached".to_string());
        }
        if let Some(ref reason) = worktree.locked {
            if reason.is_empty() {
                status_tags.push("locked".to_string());
            } else {
                status_tags.push(format!("locked: {}", reason));
            }
        }

        let status = if status_tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", status_tags.join(", "))
        };

        // Show last activity if available
        if let Some(last_activity) = operations::get_last_activity(&worktree.path) {
            let formatted_time = operations::format_activity(last_activity);
            println!(
                "  {} -> {} ({}){}",
                branch_display,
                worktree.path.display(),
                formatted_time,
                status
            );
        } else {
            println!(
                "  {} -> {}{}",
                branch_display,
                worktree.path.display(),
                status
            );
        }
    }

    Ok(())
}
