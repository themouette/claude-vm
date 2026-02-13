use crate::error::Result;
use crate::worktree::operations::{format_activity, get_last_activity};
use crate::worktree::recovery::ensure_clean_state;

/// Execute the list worktrees command
///
/// Displays all worktrees with branch name, path, and last activity
pub fn execute() -> Result<()> {
    let entries = ensure_clean_state()?;

    if entries.is_empty() {
        println!("No worktrees found");
        return Ok(());
    }

    // Print header
    println!("{:<20} {:<50} {:<20}", "BRANCH", "PATH", "LAST ACTIVITY");

    // Print each worktree entry
    for entry in entries {
        // Format branch name or show detached HEAD with short hash
        let branch_display = if let Some(ref branch) = entry.branch {
            let mut display = branch.clone();
            if entry.locked.is_some() {
                display.push_str(" (locked)");
            }
            display
        } else {
            // Detached HEAD - show short hash
            let short_hash = &entry.head[..7.min(entry.head.len())];
            format!("(detached: {})", short_hash)
        };

        // Get last activity timestamp
        let activity = get_last_activity(&entry.path)
            .map(format_activity)
            .unwrap_or_else(|| "unknown".to_string());

        // Format path as string
        let path_display = entry.path.display().to_string();

        println!(
            "{:<20} {:<50} {:<20}",
            branch_display, path_display, activity
        );
    }

    Ok(())
}
