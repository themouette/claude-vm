use crate::error::Result;
use crate::worktree::{operations, recovery, state, validation};

pub fn execute(merged_base: Option<&str>, locked: bool, detached: bool) -> Result<()> {
    // Validate git version
    validation::check_git_version()?;

    // Get worktree list with auto-prune
    let worktrees = recovery::ensure_clean_state()?;

    // Apply filters
    let mut filtered_worktrees: Vec<&state::WorktreeEntry> = worktrees.iter().collect();

    // Filter by merged status if requested
    if let Some(base) = merged_base {
        let merged_branches = operations::list_merged_branches(base)?;
        filtered_worktrees.retain(|w| {
            w.branch.as_ref()
                .map(|b| merged_branches.contains(b))
                .unwrap_or(false)
        });
    }

    // Filter by locked status if requested
    if locked {
        filtered_worktrees.retain(|w| w.locked.is_some());
    }

    // Filter by detached status if requested
    if detached {
        filtered_worktrees.retain(|w| w.is_detached);
    }

    // Display results
    if filtered_worktrees.is_empty() {
        println!("No worktrees found matching filters.");
        return Ok(());
    }

    // Skip first entry (main worktree) for display
    let display_worktrees: Vec<_> = filtered_worktrees
        .iter()
        .skip(1)
        .collect();

    if display_worktrees.is_empty() {
        println!("No additional worktrees found matching filters.");
        return Ok(());
    }

    println!("Worktrees:");
    for worktree in display_worktrees {
        let branch_display = worktree
            .branch
            .as_deref()
            .unwrap_or("<detached>");

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
            println!("  {} -> {}{}", branch_display, worktree.path.display(), status);
        }
    }

    Ok(())
}
