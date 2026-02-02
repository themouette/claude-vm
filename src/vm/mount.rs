use crate::error::Result;
use crate::utils::git;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Mount {
    pub location: PathBuf,
    pub mount_point: Option<PathBuf>,
    pub writable: bool,
}

impl Mount {
    pub fn new(location: PathBuf, writable: bool) -> Self {
        Self {
            location,
            mount_point: None,
            writable,
        }
    }

    pub fn with_mount_point(mut self, mount_point: PathBuf) -> Self {
        self.mount_point = Some(mount_point);
        self
    }
}

/// Encode a project path for use as a Claude conversation folder name
/// Converts /Users/user/Projects/lab/my-project to -Users-user-Projects-lab-my-project
fn encode_project_path(path: &PathBuf) -> String {
    path.to_string_lossy().replace('/', "-")
}

/// Get the Claude conversation folder for the current project
/// Claude stores conversations in ~/.claude/projects/ with path-encoded folder names
/// Example: /Users/user/Projects/lab/my-project -> ~/.claude/projects/-Users-user-Projects-lab-my-project
/// Creates the folder if it doesn't exist
pub(crate) fn get_claude_conversation_folder(project_path: &PathBuf) -> Option<PathBuf> {
    // Encode the path: replace / with -
    let encoded = encode_project_path(project_path);

    // Construct the conversation folder path
    let home = std::env::var("HOME").ok()?;
    let conversation_path = PathBuf::from(home)
        .join(".claude")
        .join("projects")
        .join(encoded);

    // Create the folder if it doesn't exist
    if !conversation_path.exists() {
        std::fs::create_dir_all(&conversation_path).ok()?;
    }

    // Return the path if it's a valid directory
    if conversation_path.is_dir() {
        Some(conversation_path)
    } else {
        None
    }
}

/// Compute the mounts needed for the VM
/// Mounts the git repository root (if in a git repo), plus main repo if in a worktree,
/// plus the Claude conversation folder for the current project (if mount_conversations is true)
pub fn compute_mounts(mount_conversations: bool) -> Result<Vec<Mount>> {
    let mut mounts = Vec::new();
    let mut project_path: Option<PathBuf> = None;

    // Try to mount the git repository root (so .git is accessible)
    // This ensures git works even when running from subdirectories
    if let Ok(Some(git_root)) = git::get_git_root() {
        project_path = Some(git_root.clone());
        mounts.push(Mount::new(git_root, true));
    } else {
        // Fallback: mount current directory if not in a git repo
        if let Ok(current_dir) = std::env::current_dir() {
            project_path = Some(current_dir.clone());
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

    // Mount the Claude conversation folder for the current project (if enabled)
    if mount_conversations {
        if let Some(project) = project_path {
            if let Some(conversation_folder) = get_claude_conversation_folder(&project) {
                // Only add if not already mounted
                if !mounts.iter().any(|m| m.location == conversation_folder) {
                    // Extract the folder name (encoded project path)
                    if let Some(folder_name) = conversation_folder.file_name() {
                        // Map to VM home directory
                        // Host: /Users/user/.claude/projects/... -> VM: /home/lima.linux/.claude/projects/...
                        let vm_mount_point = PathBuf::from("/home/lima.linux")
                            .join(".claude")
                            .join("projects")
                            .join(folder_name);

                        mounts.push(
                            Mount::new(conversation_folder, true).with_mount_point(vm_mount_point),
                        );
                    }
                }
            }
        }
    }

    Ok(mounts)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test 1: Mount struct tests
    #[test]
    fn test_mount_creation() {
        let mount = Mount::new(PathBuf::from("/home/user/project"), true);
        assert_eq!(mount.location, PathBuf::from("/home/user/project"));
        assert_eq!(mount.mount_point, None);
        assert!(mount.writable);
    }

    #[test]
    fn test_mount_with_mount_point() {
        let mount = Mount::new(PathBuf::from("/host/path"), true)
            .with_mount_point(PathBuf::from("/vm/path"));

        assert_eq!(mount.location, PathBuf::from("/host/path"));
        assert_eq!(mount.mount_point, Some(PathBuf::from("/vm/path")));
        assert!(mount.writable);
    }

    #[test]
    fn test_mount_without_mount_point() {
        let mount = Mount::new(PathBuf::from("/host/path"), false);

        assert_eq!(mount.mount_point, None);
        assert!(!mount.writable);
    }

    #[test]
    fn test_mount_builder_pattern() {
        let mount = Mount::new(PathBuf::from("/some/path"), false)
            .with_mount_point(PathBuf::from("/target/path"));

        assert_eq!(mount.location, PathBuf::from("/some/path"));
        assert_eq!(mount.mount_point, Some(PathBuf::from("/target/path")));
        assert!(!mount.writable);
    }

    // Test 2: Path encoding logic tests
    #[test]
    fn test_encode_project_path() {
        let path = PathBuf::from("/Users/user/Projects/lab/my-project");
        assert_eq!(
            encode_project_path(&path),
            "-Users-user-Projects-lab-my-project"
        );
    }

    #[test]
    fn test_encode_project_path_with_spaces() {
        let path = PathBuf::from("/Users/user/My Projects/test project");
        assert_eq!(
            encode_project_path(&path),
            "-Users-user-My Projects-test project"
        );
    }

    #[test]
    fn test_encode_project_path_root() {
        let path = PathBuf::from("/");
        assert_eq!(encode_project_path(&path), "-");
    }

    #[test]
    fn test_encode_project_path_no_leading_slash() {
        let path = PathBuf::from("relative/path");
        assert_eq!(encode_project_path(&path), "relative-path");
    }

    // Test 3: Integration tests with temp directories
    #[test]
    fn test_get_claude_conversation_folder_creates_directory() {
        use std::env;

        let temp_dir = std::env::temp_dir().join("claude-vm-test-home");
        let _ = std::fs::remove_dir_all(&temp_dir); // Clean up if exists

        // Set HOME to temp directory
        let original_home = env::var("HOME").ok();
        env::set_var("HOME", &temp_dir);

        let project_path = PathBuf::from("/Users/test/my-project");
        let result = get_claude_conversation_folder(&project_path);

        // Restore original HOME
        if let Some(home) = original_home {
            env::set_var("HOME", home);
        } else {
            env::remove_var("HOME");
        }

        assert!(result.is_some());
        let folder = result.unwrap();
        assert!(folder.exists());
        assert!(folder.is_dir());
        assert_eq!(folder.file_name().unwrap(), "-Users-test-my-project");

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_get_claude_conversation_folder_existing_directory() {
        use std::env;

        let temp_dir = std::env::temp_dir().join("claude-vm-test-home-existing");
        let _ = std::fs::remove_dir_all(&temp_dir); // Clean up if exists

        // Pre-create the conversation folder
        let project_path = PathBuf::from("/Users/test/existing-project");
        let encoded = encode_project_path(&project_path);
        let conversation_path = temp_dir.join(".claude").join("projects").join(&encoded);
        std::fs::create_dir_all(&conversation_path).unwrap();

        // Set HOME to temp directory
        let original_home = env::var("HOME").ok();
        env::set_var("HOME", &temp_dir);

        let result = get_claude_conversation_folder(&project_path);

        // Restore original HOME
        if let Some(home) = original_home {
            env::set_var("HOME", home);
        } else {
            env::remove_var("HOME");
        }

        assert!(result.is_some());
        let folder = result.unwrap();
        assert!(folder.exists());
        assert!(folder.is_dir());
        assert_eq!(folder, conversation_path);

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_get_claude_conversation_folder_no_home() {
        use std::env;

        let original_home = env::var("HOME").ok();
        env::remove_var("HOME");

        let project_path = PathBuf::from("/Users/test/my-project");
        let result = get_claude_conversation_folder(&project_path);

        // Restore original HOME
        if let Some(home) = original_home {
            env::set_var("HOME", home);
        }

        assert!(result.is_none());
    }
}
