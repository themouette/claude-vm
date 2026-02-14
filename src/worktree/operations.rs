use crate::error::{ClaudeVmError, Result};
use crate::worktree::config::WorktreeConfig;
use crate::worktree::recovery::ensure_clean_state;
use crate::worktree::template::{compute_worktree_path, TemplateContext};
use crate::worktree::validation;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

/// Represents the status of a branch in relation to worktrees
#[derive(Debug, PartialEq)]
pub enum BranchStatus {
    /// Branch is already checked out in a worktree at the given path
    InWorktree(PathBuf),
    /// Branch exists as a ref but is not checked out in any worktree
    ExistsNotCheckedOut,
    /// Branch does not exist as a ref
    DoesNotExist,
}

/// Represents the result of creating a worktree
#[derive(Debug, PartialEq)]
pub enum CreateResult {
    /// Branch already had a worktree; returning existing path
    Resumed(PathBuf),
    /// New worktree created at this path
    Created(PathBuf),
}

impl CreateResult {
    /// Get the worktree path regardless of whether it was created or resumed
    pub fn path(&self) -> &PathBuf {
        match self {
            CreateResult::Resumed(path) | CreateResult::Created(path) => path,
        }
    }

    /// Generate user-facing message for this result
    pub fn message(&self, branch: &str) -> String {
        match self {
            CreateResult::Resumed(path) => {
                format!(
                    "Resuming worktree for branch '{}' at {}",
                    branch,
                    path.display()
                )
            }
            CreateResult::Created(path) => {
                format!(
                    "Created worktree for branch '{}' at {}",
                    branch,
                    path.display()
                )
            }
        }
    }
}

/// Detect the status of a branch
///
/// Returns:
/// - `InWorktree(path)` if the branch is checked out in an existing worktree
/// - `ExistsNotCheckedOut` if the branch exists but is not in a worktree
/// - `DoesNotExist` if the branch does not exist
pub fn detect_branch_status(branch: &str) -> Result<BranchStatus> {
    // Get current worktrees
    let worktrees = ensure_clean_state()?;

    // Check if branch is in any worktree
    for entry in worktrees {
        if let Some(ref entry_branch) = entry.branch {
            if entry_branch == branch {
                return Ok(BranchStatus::InWorktree(entry.path));
            }
        }
    }

    // Check if branch exists as a ref
    let output = Command::new("git")
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{}", branch),
        ])
        .output()
        .map_err(|e| ClaudeVmError::Git(format!("Failed to check branch existence: {}", e)))?;

    if output.status.success() {
        Ok(BranchStatus::ExistsNotCheckedOut)
    } else {
        Ok(BranchStatus::DoesNotExist)
    }
}

/// Create a worktree for a branch
///
/// This function handles all three branch states:
/// - If the branch is already in a worktree, returns the existing path as Resumed
/// - If the branch exists but not in a worktree, checks it out in a new worktree
/// - If the branch doesn't exist, creates it from the base and checks it out
///
/// Returns CreateResult indicating whether an existing worktree was resumed or a new one created.
/// The caller (command handler) is responsible for printing user-facing messages.
pub fn create_worktree(
    config: &WorktreeConfig,
    repo_root: &Path,
    branch: &str,
    base: Option<&str>,
) -> Result<CreateResult> {
    // Validate branch name first
    validation::validate_branch_name(branch)?;

    let status = detect_branch_status(branch)?;

    match status {
        BranchStatus::InWorktree(path) => {
            // Branch already has a worktree - return existing path
            Ok(CreateResult::Resumed(path))
        }
        BranchStatus::ExistsNotCheckedOut => {
            // Branch exists, check it out in a new worktree
            let short_hash = get_short_hash()?;
            let repo_name = repo_root
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("repo");
            let context = TemplateContext::new(repo_name, branch, &short_hash);
            let worktree_path = compute_worktree_path(config, repo_root, &context)?;

            let path_str = worktree_path.to_str().ok_or_else(|| {
                ClaudeVmError::Worktree(format!(
                    "Worktree path contains invalid UTF-8: {}",
                    worktree_path.display()
                ))
            })?;
            let output = Command::new("git")
                .args(["worktree", "add", path_str, branch])
                .output()
                .map_err(|e| ClaudeVmError::Git(format!("Failed to create worktree: {}", e)))?;

            if !output.status.success() {
                return Err(ClaudeVmError::Git(format!(
                    "git worktree add failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }

            Ok(CreateResult::Created(worktree_path))
        }
        BranchStatus::DoesNotExist => {
            // Branch doesn't exist, create it from base
            let short_hash = get_short_hash()?;
            let repo_name = repo_root
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("repo");
            let context = TemplateContext::new(repo_name, branch, &short_hash);
            let worktree_path = compute_worktree_path(config, repo_root, &context)?;

            let path_str = worktree_path.to_str().ok_or_else(|| {
                ClaudeVmError::Worktree(format!(
                    "Worktree path contains invalid UTF-8: {}",
                    worktree_path.display()
                ))
            })?;
            let mut args = vec!["worktree", "add", "-b", branch, path_str];
            if let Some(base_branch) = base {
                args.push(base_branch);
            }

            let output = Command::new("git")
                .args(&args)
                .output()
                .map_err(|e| ClaudeVmError::Git(format!("Failed to create worktree: {}", e)))?;

            if !output.status.success() {
                return Err(ClaudeVmError::Git(format!(
                    "git worktree add failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }

            Ok(CreateResult::Created(worktree_path))
        }
    }
}

/// Delete a worktree by branch name
///
/// This removes the worktree directory and updates git metadata, but preserves the branch.
pub fn delete_worktree(branch: &str) -> Result<()> {
    // Validate branch name first
    validation::validate_branch_name(branch)?;

    let worktrees = ensure_clean_state()?;

    // Find worktree by branch
    let worktree = worktrees
        .iter()
        .find(|e| e.branch.as_deref() == Some(branch))
        .ok_or_else(|| ClaudeVmError::WorktreeNotFound {
            branch: branch.to_string(),
        })?;

    // Use git worktree remove to delete the directory and update metadata
    let path_str = worktree.path.to_str().ok_or_else(|| {
        ClaudeVmError::Worktree(format!(
            "Worktree path contains invalid UTF-8: {}",
            worktree.path.display()
        ))
    })?;
    let output = Command::new("git")
        .args(["worktree", "remove", path_str])
        .output()
        .map_err(|e| ClaudeVmError::Git(format!("Failed to remove worktree: {}", e)))?;

    if !output.status.success() {
        return Err(ClaudeVmError::Git(format!(
            "git worktree remove failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(())
}

/// List branches that have been merged into the base branch
///
/// Returns a list of branch names (excluding the base branch itself)
pub fn list_merged_branches(base: &str) -> Result<Vec<String>> {
    // First validate that base branch exists (check both local and remote)
    let ref_paths = vec![
        format!("refs/heads/{}", base),   // Local branch
        format!("refs/remotes/{}", base), // Remote branch (e.g., origin/main)
    ];

    let mut branch_exists = false;
    for ref_path in &ref_paths {
        let output = Command::new("git")
            .args(["show-ref", "--verify", ref_path])
            .output()
            .map_err(|e| ClaudeVmError::Git(format!("Failed to verify base branch: {}", e)))?;

        if output.status.success() {
            branch_exists = true;
            break;
        }
    }

    if !branch_exists {
        return Err(ClaudeVmError::BranchNotFound {
            branch: base.to_string(),
        });
    }

    // Get merged branches
    let output = Command::new("git")
        .args(["branch", "--merged", base, "--format=%(refname:short)"])
        .output()
        .map_err(|e| ClaudeVmError::Git(format!("Failed to list merged branches: {}", e)))?;

    if !output.status.success() {
        return Err(ClaudeVmError::Git(format!(
            "git branch --merged failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let branches: Vec<String> = output_str
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s != base)
        .collect();

    Ok(branches)
}

/// Check if a specific branch is merged into the base branch
pub fn is_branch_merged(branch: &str, base: &str) -> Result<bool> {
    let merged = list_merged_branches(base)?;
    Ok(merged.contains(&branch.to_string()))
}

/// Get the last activity time for a worktree directory
pub fn get_last_activity(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

/// Format a SystemTime as a human-readable timestamp
pub fn format_activity(time: SystemTime) -> String {
    use chrono::{DateTime, Local};
    let datetime: DateTime<Local> = time.into();
    datetime.format("%Y-%m-%d %H:%M").to_string()
}

/// Get the short hash of the current HEAD
fn get_short_hash() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .map_err(|e| ClaudeVmError::Git(format!("Failed to get short hash: {}", e)))?;

    if !output.status.success() {
        return Err(ClaudeVmError::Git(format!(
            "git rev-parse failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branch_status_variants() {
        // Test that BranchStatus variants exist and can be compared
        let status1 = BranchStatus::InWorktree(PathBuf::from("/test"));
        let status2 = BranchStatus::InWorktree(PathBuf::from("/test"));
        let status3 = BranchStatus::ExistsNotCheckedOut;
        let status4 = BranchStatus::DoesNotExist;

        assert_eq!(status1, status2);
        assert_ne!(status1, status3);
        assert_ne!(status3, status4);
    }

    #[test]
    fn test_create_result_variants() {
        // Test that CreateResult variants exist and can be compared
        let result1 = CreateResult::Resumed(PathBuf::from("/test"));
        let result2 = CreateResult::Resumed(PathBuf::from("/test"));
        let result3 = CreateResult::Created(PathBuf::from("/test"));

        assert_eq!(result1, result2);
        assert_ne!(result1, result3);
    }

    #[test]
    fn test_create_result_path() {
        let path1 = PathBuf::from("/test");
        let result1 = CreateResult::Resumed(path1.clone());
        assert_eq!(result1.path(), &path1);

        let path2 = PathBuf::from("/test2");
        let result2 = CreateResult::Created(path2.clone());
        assert_eq!(result2.path(), &path2);
    }

    #[test]
    fn test_create_result_message() {
        let path = PathBuf::from("/tmp/worktrees/feature");

        let resumed = CreateResult::Resumed(path.clone());
        let msg = resumed.message("feature");
        assert!(msg.contains("Resuming"));
        assert!(msg.contains("feature"));

        let created = CreateResult::Created(path.clone());
        let msg = created.message("feature");
        assert!(msg.contains("Created"));
        assert!(msg.contains("feature"));
    }

    #[test]
    fn test_get_last_activity_nonexistent_path() {
        let result = get_last_activity(Path::new("/nonexistent/path"));
        assert!(result.is_none());
    }

    #[test]
    fn test_format_activity() {
        use std::time::{Duration, UNIX_EPOCH};

        // Create a known timestamp: 2024-01-15 12:34:56 UTC
        let timestamp = UNIX_EPOCH + Duration::from_secs(1705322096);
        let formatted = format_activity(timestamp);

        // Format should be YYYY-MM-DD HH:MM (length 16)
        assert_eq!(formatted.len(), 16);
        // Should start with date
        assert!(formatted.starts_with("2024-01-15"));
    }
}
