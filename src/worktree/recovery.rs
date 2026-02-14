use crate::error::Result;
use crate::worktree::state::{list_worktrees, WorktreeEntry};
use std::io::{self, Write};

/// Auto-prune orphaned worktree metadata with user confirmation
/// Best-effort operation - logs warnings on failure but doesn't error
pub fn auto_prune() -> Result<()> {
    use std::process::Command;

    // First, do a dry-run to see what would be pruned
    let dry_run = Command::new("git")
        .args(["worktree", "prune", "--dry-run", "--verbose"])
        .output();

    let to_prune = match dry_run {
        Ok(output) => String::from_utf8_lossy(&output.stderr).to_string(),
        Err(e) => {
            eprintln!("Warning: failed to check for orphaned worktrees: {}", e);
            return Ok(());
        }
    };

    // Only prompt if there's something to prune
    if !to_prune.trim().is_empty() {
        eprintln!("Found orphaned worktree metadata:");
        eprintln!("{}", to_prune);
        eprintln!();

        // Prompt for confirmation
        print!("Prune orphaned worktrees? [y/N] ");
        let _ = io::stdout().flush();

        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let input = input.trim().to_lowercase();

        // If user doesn't confirm, skip prune
        if input != "y" && input != "yes" {
            eprintln!("Skipped pruning worktrees.");
            return Ok(());
        }
    }

    // Actually prune
    let output = Command::new("git").args(["worktree", "prune"]).output();

    match output {
        Ok(output) if !output.status.success() => {
            // Log warning but don't fail - prune is best-effort cleanup
            eprintln!(
                "Warning: git worktree prune failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Err(e) => {
            eprintln!("Warning: failed to run git worktree prune: {}", e);
        }
        _ => {
            // Success - optionally show success message if something was pruned
            if !to_prune.trim().is_empty() {
                eprintln!("Pruned orphaned worktrees.");
            }
        }
    }

    // Always return Ok - prune is cleanup, not critical
    Ok(())
}

/// Attempt to repair worktree metadata links
/// Best-effort operation - logs warnings on failure but doesn't error
pub fn try_repair() -> Result<()> {
    use std::process::Command;

    let output = Command::new("git").args(["worktree", "repair"]).output();

    match output {
        Ok(output) if !output.status.success() => {
            // Log warning but don't fail - repair is best-effort
            eprintln!(
                "Warning: git worktree repair failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Err(e) => {
            eprintln!("Warning: failed to run git worktree repair: {}", e);
        }
        _ => {
            // Success or no error - continue
        }
    }

    // Always return Ok - repair is recovery, not critical
    Ok(())
}

/// Ensure clean state by running auto-prune and querying worktrees
/// This is the main entry point Phase 2 will call before operations
pub fn ensure_clean_state() -> Result<Vec<WorktreeEntry>> {
    auto_prune()?;
    list_worktrees()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    #[test]
    fn test_auto_prune_does_not_error() {
        // Create a temporary git repo for testing
        let dir = TempDir::new().unwrap();
        let repo_path = dir.path();

        // Initialize git repo
        Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Create initial commit
        fs::write(repo_path.join("test.txt"), "test").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Change to repo directory for git commands
        std::env::set_current_dir(repo_path).unwrap();

        // Run auto_prune - should not error even if nothing to prune
        let result = auto_prune();
        assert!(result.is_ok());
    }

    #[test]
    fn test_try_repair_does_not_error() {
        // Create a temporary git repo for testing
        let dir = TempDir::new().unwrap();
        let repo_path = dir.path();

        // Initialize git repo
        Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Create initial commit
        fs::write(repo_path.join("test.txt"), "test").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Change to repo directory for git commands
        std::env::set_current_dir(repo_path).unwrap();

        // Run try_repair - should not error
        let result = try_repair();
        assert!(result.is_ok());
    }
}
