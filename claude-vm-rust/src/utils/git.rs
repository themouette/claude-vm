use crate::error::{ClaudeVmError, Result};
use std::path::PathBuf;
use std::process::Command;

/// Get the git common directory (handles worktrees)
pub fn get_git_common_dir() -> Result<Option<PathBuf>> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-common-dir"])
        .output()
        .map_err(|e| ClaudeVmError::Git(format!("Failed to run git: {}", e)))?;

    if !output.status.success() {
        return Ok(None);
    }

    let git_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let git_path = PathBuf::from(git_dir);

    if git_path.is_dir() {
        Ok(Some(git_path.canonicalize()?))
    } else {
        Ok(None)
    }
}

/// Get the git worktree directory (if in a worktree)
pub fn get_git_worktree_dir() -> Result<Option<PathBuf>> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map_err(|e| ClaudeVmError::Git(format!("Failed to run git: {}", e)))?;

    if !output.status.success() {
        return Ok(None);
    }

    let git_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // If .git is a file (worktree), we're in a worktree
    let git_path = PathBuf::from(&git_dir);
    if git_path.is_file() {
        return Ok(Some(std::env::current_dir()?));
    }

    Ok(None)
}

/// Check if the current directory is inside a git worktree
/// A worktree is detected when --git-dir differs from --git-common-dir
pub fn is_worktree() -> bool {
    let git_dir = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .ok()
        .and_then(|o| if o.status.success() {
            Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
        } else {
            None
        });

    let git_common_dir = Command::new("git")
        .args(["rev-parse", "--git-common-dir"])
        .output()
        .ok()
        .and_then(|o| if o.status.success() {
            Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
        } else {
            None
        });

    // In a worktree, git-dir and git-common-dir are different
    // In a regular repo, they're the same
    if let (Some(dir), Some(common)) = (git_dir, git_common_dir) {
        // Canonicalize paths for accurate comparison
        let dir_path = PathBuf::from(dir).canonicalize().ok();
        let common_path = PathBuf::from(common).canonicalize().ok();

        if let (Some(d), Some(c)) = (dir_path, common_path) {
            return d != c;
        }
    }

    false
}

/// Get the root directory of the git repository
/// This returns the top-level directory containing the .git folder
pub fn get_git_root() -> Result<Option<PathBuf>> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| ClaudeVmError::Git(format!("Failed to run git: {}", e)))?;

    if !output.status.success() {
        return Ok(None);
    }

    let root_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let root_path = PathBuf::from(root_dir);

    if root_path.is_dir() {
        Ok(Some(root_path.canonicalize()?))
    } else {
        Ok(None)
    }
}
