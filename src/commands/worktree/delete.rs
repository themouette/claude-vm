use crate::error::{ClaudeVmError, Result};
use crate::worktree::{operations, recovery, validation};
use std::io::{self, Write};

pub fn execute(branch: &str, yes: bool) -> Result<()> {
    // Validate git version supports worktrees
    validation::check_git_version()?;

    // Get current worktree list with auto-prune
    let worktrees = recovery::ensure_clean_state()?;

    // Find the worktree matching the given branch name
    let worktree = worktrees
        .iter()
        .find(|e| e.branch.as_deref() == Some(branch))
        .ok_or_else(|| ClaudeVmError::WorktreeNotFound {
            branch: branch.to_string(),
        })?;

    // Check if worktree is locked
    if let Some(ref reason) = worktree.locked {
        let reason_text = if reason.is_empty() {
            "no reason given".to_string()
        } else {
            reason.clone()
        };
        eprintln!(
            "Warning: This worktree is locked (reason: {}). Use 'git worktree unlock {}' first.",
            reason_text,
            worktree.path.display()
        );
        return Err(ClaudeVmError::WorktreeLocked {
            reason: reason_text,
            path: worktree.path.display().to_string(),
        });
    }

    // Display what will happen
    println!("Worktree: {}", branch);
    println!("Path: {}", worktree.path.display());
    println!("This will remove the worktree directory. The branch will be preserved.");
    println!();

    // Prompt for confirmation unless --yes was provided
    if !yes {
        print!("Delete worktree? [y/N] ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let input = input.trim().to_lowercase();

        if input != "y" && input != "yes" {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Delete the worktree
    operations::delete_worktree(branch)?;
    println!("Worktree deleted: {} (branch preserved)", branch);

    Ok(())
}
