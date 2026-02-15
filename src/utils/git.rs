use crate::error::{ClaudeVmError, Result};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;
use wait_timeout::ChildExt;

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
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        });

    let git_common_dir = Command::new("git")
        .args(["rev-parse", "--git-common-dir"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
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

/// Detect the repository's default branch from the remote origin HEAD ref.
/// Falls back to "main" if the remote HEAD cannot be determined.
pub fn get_default_branch() -> Result<String> {
    let output = Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD"])
        .output()
        .map_err(|e| ClaudeVmError::Git(format!("Failed to run git: {}", e)))?;

    if !output.status.success() {
        eprintln!("Warning: Could not detect default branch (no remote HEAD ref). Falling back to 'main'.");
        return Ok("main".to_string());
    }

    let symbolic_ref = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Strip the "refs/remotes/origin/" prefix to get just the branch name
    // Example: "refs/remotes/origin/main" -> "main"
    let branch_name = symbolic_ref
        .strip_prefix("refs/remotes/origin/")
        .unwrap_or("main")
        .to_string();

    Ok(branch_name)
}

/// Get the current branch name.
/// Returns an error if not on a branch (detached HEAD).
pub fn get_current_branch() -> Result<String> {
    let output = Command::new("git")
        .args(["symbolic-ref", "--short", "HEAD"])
        .output()
        .map_err(|e| ClaudeVmError::Git(format!("Failed to run git: {}", e)))?;

    if !output.status.success() {
        return Err(ClaudeVmError::Git(
            "Not on a branch (detached HEAD)".to_string(),
        ));
    }

    let branch_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(branch_name)
}

/// Default timeout for git operations (30 seconds)
const DEFAULT_GIT_TIMEOUT: Duration = Duration::from_secs(30);

/// Run a git command and return stdout on success with timeout support.
///
/// # Arguments
/// * `args` - Command arguments (e.g., `&["status", "--short"]`)
/// * `operation` - Human-readable operation description for error messages
/// * `timeout` - Optional timeout duration (defaults to 30 seconds)
///
/// # Example
/// ```ignore
/// let output = run_git_command_timeout(&["rev-parse", "HEAD"], "get commit hash", None)?;
/// ```
fn run_git_command_timeout(
    args: &[&str],
    operation: &str,
    timeout: Option<Duration>,
) -> Result<String> {
    let timeout = timeout.unwrap_or(DEFAULT_GIT_TIMEOUT);

    let mut child = Command::new("git")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| ClaudeVmError::Git(format!("Failed to {}: {}", operation, e)))?;

    match child
        .wait_timeout(timeout)
        .map_err(|e| ClaudeVmError::Git(format!("Failed to wait for git command: {}", e)))?
    {
        Some(status) => {
            let output = child
                .wait_with_output()
                .map_err(|e| ClaudeVmError::Git(format!("Failed to read git output: {}", e)))?;

            if !status.success() {
                return Err(ClaudeVmError::Git(format!(
                    "git {} failed: {}",
                    args[0],
                    String::from_utf8_lossy(&output.stderr)
                )));
            }

            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
        None => {
            // Timeout occurred, kill the process
            let _ = child.kill();
            Err(ClaudeVmError::Git(format!(
                "git {} timed out after {} seconds",
                args[0],
                timeout.as_secs()
            )))
        }
    }
}

/// Run a git command and return stdout on success.
/// Uses a default 30-second timeout.
///
/// # Arguments
/// * `args` - Command arguments (e.g., `&["status", "--short"]`)
/// * `operation` - Human-readable operation description for error messages
///
/// # Example
/// ```ignore
/// let output = run_git_command(&["rev-parse", "HEAD"], "get commit hash")?;
/// ```
pub fn run_git_command(args: &[&str], operation: &str) -> Result<String> {
    run_git_command_timeout(args, operation, None)
}

/// Run a git query command that may legitimately return non-zero exit.
/// Returns None on non-zero exit instead of erroring.
/// Uses a default 30-second timeout.
///
/// # Arguments
/// * `args` - Command arguments (e.g., `&["show-ref", "--verify", "refs/heads/main"]`)
///
/// # Example
/// ```ignore
/// if let Some(sha) = run_git_query(&["show-ref", "--verify", "refs/heads/main"])? {
///     println!("Branch exists: {}", sha);
/// }
/// ```
pub fn run_git_query(args: &[&str]) -> Result<Option<String>> {
    let timeout = DEFAULT_GIT_TIMEOUT;

    let mut child = Command::new("git")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| ClaudeVmError::Git(format!("Failed to run git {}: {}", args[0], e)))?;

    match child
        .wait_timeout(timeout)
        .map_err(|e| ClaudeVmError::Git(format!("Failed to wait for git command: {}", e)))?
    {
        Some(status) => {
            if !status.success() {
                return Ok(None);
            }

            let output = child
                .wait_with_output()
                .map_err(|e| ClaudeVmError::Git(format!("Failed to read git output: {}", e)))?;

            Ok(Some(
                String::from_utf8_lossy(&output.stdout).trim().to_string(),
            ))
        }
        None => {
            // Timeout occurred, kill the process
            let _ = child.kill();
            Err(ClaudeVmError::Git(format!(
                "git {} timed out after {} seconds",
                args[0],
                timeout.as_secs()
            )))
        }
    }
}

/// Run a git command in best-effort mode, returning raw output without erroring on failures.
/// This is useful for cleanup operations that should log warnings but not fail the main operation.
///
/// # Arguments
/// * `args` - Command arguments (e.g., `&["worktree", "prune"]`)
///
/// # Example
/// ```ignore
/// match run_git_best_effort(&["worktree", "prune"]) {
///     Ok(output) if !output.status.success() => {
///         eprintln!("Warning: prune failed: {}", String::from_utf8_lossy(&output.stderr));
///     }
///     Err(e) => eprintln!("Warning: {}", e),
///     _ => {}
/// }
/// ```
pub fn run_git_best_effort(args: &[&str]) -> Result<std::process::Output> {
    let timeout = DEFAULT_GIT_TIMEOUT;

    let mut child = Command::new("git")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| ClaudeVmError::Git(format!("Failed to spawn git {}: {}", args[0], e)))?;

    match child
        .wait_timeout(timeout)
        .map_err(|e| ClaudeVmError::Git(format!("Failed to wait for git command: {}", e)))?
    {
        Some(_) => {
            let output = child
                .wait_with_output()
                .map_err(|e| ClaudeVmError::Git(format!("Failed to read git output: {}", e)))?;
            Ok(output)
        }
        None => {
            // Timeout occurred, kill the process
            let _ = child.kill();
            Err(ClaudeVmError::Git(format!(
                "git {} timed out after {} seconds",
                args[0],
                timeout.as_secs()
            )))
        }
    }
}

/// Convert a Path to &str with proper error handling
///
/// This helper ensures consistent error messages when paths contain invalid UTF-8.
///
/// # Arguments
/// * `path` - The path to convert
/// * `context` - A description of what this path represents (e.g., "worktree path")
///
/// # Example
/// ```ignore
/// let path_str = path_to_str(&worktree_path, "worktree path")?;
/// ```
pub fn path_to_str<'a>(path: &'a std::path::Path, context: &str) -> Result<&'a str> {
    path.to_str().ok_or_else(|| {
        ClaudeVmError::Worktree(format!(
            "{} contains invalid UTF-8: {}",
            context,
            path.display()
        ))
    })
}

/// Extract project name from a project root path.
///
/// Returns the directory name (last component) of the path.
/// This works well for most cases: /home/user/my-project -> "my-project"
///
/// # Example
/// ```ignore
/// let name = extract_project_name(Path::new("/home/user/my-project"));
/// assert_eq!(name, Some("my-project"));
/// ```
pub fn extract_project_name(project_root: &std::path::Path) -> Option<String> {
    project_root
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
}

/// Worktree information extracted from a git repository
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeInfo {
    /// Path to the main repository root
    pub main_root: PathBuf,
    /// Path to the current worktree
    pub worktree_path: PathBuf,
}

/// Detect if the project is a git worktree and extract worktree information.
///
/// Git worktrees have a .git file (not directory) containing:
/// `gitdir: /path/to/main-repo/.git/worktrees/branch-name`
///
/// This function parses the .git file and validates the worktree structure.
///
/// # Returns
/// - `Some(WorktreeInfo)` if the project is a valid git worktree
/// - `None` if not a worktree or invalid structure
///
/// # Example
/// ```ignore
/// if let Some(info) = detect_worktree(project_root) {
///     println!("Main repo: {}", info.main_root.display());
///     println!("Worktree: {}", info.worktree_path.display());
/// }
/// ```
pub fn detect_worktree(project_root: &std::path::Path) -> Option<WorktreeInfo> {
    let git_dir = project_root.join(".git");

    // Check if .git exists and is a file (worktrees have .git file, not directory)
    if !git_dir.exists() || !git_dir.is_file() {
        return None;
    }

    // Read .git file content
    let git_file_content = std::fs::read_to_string(&git_dir).ok()?;

    // Parse the gitdir line
    let gitdir_line = git_file_content.lines().next()?;
    let gitdir_path = gitdir_line.strip_prefix("gitdir: ")?;
    let gitdir_pathbuf = PathBuf::from(gitdir_path);

    // Validate this looks like a worktree path
    // Expected structure: /main-repo/.git/worktrees/branch-name
    let worktrees_parent = gitdir_pathbuf.parent()?;
    if !worktrees_parent.ends_with("worktrees") {
        return None;
    }

    // Navigate up: worktrees -> .git -> main-repo
    let git_parent = worktrees_parent.parent()?;
    let main_root = git_parent.parent()?;

    Some(WorktreeInfo {
        main_root: main_root.to_path_buf(),
        worktree_path: project_root.to_path_buf(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_extract_project_name() {
        assert_eq!(
            extract_project_name(std::path::Path::new("/home/user/my-project")),
            Some("my-project".to_string())
        );

        assert_eq!(
            extract_project_name(std::path::Path::new("/path/to/claude-vm")),
            Some("claude-vm".to_string())
        );

        // Root path edge case - file_name() returns None for root
        assert_eq!(extract_project_name(std::path::Path::new("/")), None);
    }

    #[test]
    fn test_detect_worktree_not_a_git_repo() {
        let temp_dir = TempDir::new().unwrap();
        assert_eq!(detect_worktree(temp_dir.path()), None);
    }

    #[test]
    fn test_detect_worktree_regular_git_repo() {
        let temp_dir = TempDir::new().unwrap();
        // Regular repos have .git directory, not file
        fs::create_dir(temp_dir.path().join(".git")).unwrap();
        assert_eq!(detect_worktree(temp_dir.path()), None);
    }

    #[test]
    fn test_detect_worktree_valid_worktree() {
        let temp_dir = TempDir::new().unwrap();
        let main_repo = temp_dir.path().join("main-repo");
        let worktree = temp_dir.path().join("feature-branch");

        // Create main repo structure
        fs::create_dir_all(&main_repo).unwrap();
        fs::create_dir(main_repo.join(".git")).unwrap();
        fs::create_dir(main_repo.join(".git").join("worktrees")).unwrap();

        // Create worktree
        fs::create_dir_all(&worktree).unwrap();

        // Create .git file pointing to worktree
        let gitdir_path = main_repo.join(".git/worktrees/feature-branch");
        fs::create_dir_all(&gitdir_path).unwrap();

        let git_file_content = format!("gitdir: {}", gitdir_path.display());
        fs::write(worktree.join(".git"), git_file_content).unwrap();

        let result = detect_worktree(&worktree);
        assert!(result.is_some());

        let info = result.unwrap();
        assert_eq!(info.main_root, main_repo);
        assert_eq!(info.worktree_path, worktree);
    }

    #[test]
    fn test_detect_worktree_invalid_gitdir_format() {
        let temp_dir = TempDir::new().unwrap();
        let worktree = temp_dir.path().join("worktree");
        fs::create_dir_all(&worktree).unwrap();

        // Create .git file with invalid format (no "gitdir: " prefix)
        fs::write(worktree.join(".git"), "/some/path").unwrap();

        assert_eq!(detect_worktree(&worktree), None);
    }

    #[test]
    fn test_detect_worktree_not_worktrees_structure() {
        let temp_dir = TempDir::new().unwrap();
        let worktree = temp_dir.path().join("worktree");
        fs::create_dir_all(&worktree).unwrap();

        // Create .git file with valid prefix but invalid structure
        // (not under worktrees directory)
        fs::write(worktree.join(".git"), "gitdir: /some/other/path").unwrap();

        assert_eq!(detect_worktree(&worktree), None);
    }
}
