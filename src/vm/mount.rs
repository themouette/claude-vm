use crate::error::Result;
use crate::utils::git;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Mount {
    pub location: PathBuf,
    pub writable: bool,
}

impl Mount {
    pub fn new(location: PathBuf, writable: bool) -> Self {
        Self { location, writable }
    }
}

/// Compute the mounts needed for the VM
/// Mounts the git repository root (if in a git repo), plus main repo if in a worktree
pub fn compute_mounts() -> Result<Vec<Mount>> {
    let mut mounts = Vec::new();

    // Try to mount the git repository root (so .git is accessible)
    // This ensures git works even when running from subdirectories
    if let Ok(Some(git_root)) = git::get_git_root() {
        mounts.push(Mount::new(git_root, true));
    } else {
        // Fallback: mount current directory if not in a git repo
        if let Ok(current_dir) = std::env::current_dir() {
            mounts.push(Mount::new(current_dir, true));
        }
    }

    // If in a git worktree, also mount the main repo (for git access)
    if git::is_worktree() {
        if let Ok(Some(git_common_dir)) = git::get_git_common_dir() {
            if let Some(main_repo) = git_common_dir.parent() {
                let main_repo = main_repo.to_path_buf();
                // Only add if different from already mounted directories
                if !mounts.iter().any(|m| m.location == main_repo) {
                    mounts.push(Mount::new(main_repo, false));
                }
            }
        }
    }

    Ok(mounts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mount_creation() {
        let mount = Mount::new(PathBuf::from("/home/user/project"), true);
        assert_eq!(mount.location, PathBuf::from("/home/user/project"));
        assert!(mount.writable);
    }
}
