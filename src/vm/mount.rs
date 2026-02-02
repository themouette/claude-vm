use crate::error::{ClaudeVmError, Result};
use crate::utils::git;
use std::path::{Path, PathBuf};

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

    /// Parse a docker-style mount specification string
    /// Formats:
    /// - `/host/path` - writable, same path in VM
    /// - `/host/path:ro` - read-only, same path in VM
    /// - `/host/path:/vm/path` - writable, custom VM path
    /// - `/host/path:/vm/path:ro` - read-only, custom VM path
    pub fn from_spec(spec: &str) -> Result<Self> {
        let parts: Vec<&str> = spec.split(':').collect();

        let (host_path, vm_path, writable) = match parts.len() {
            1 => {
                // Format: /host/path
                let path = expand_path(parts[0])?;
                (path.clone(), None, true)
            }
            2 => {
                // Format: /host/path:ro OR /host/path:/vm/path
                let host = expand_path(parts[0])?;
                if parts[1] == "ro" || parts[1] == "rw" {
                    // Format: /host/path:ro
                    let writable = parts[1] == "rw";
                    (host, None, writable)
                } else {
                    // Format: /host/path:/vm/path
                    let vm = expand_path(parts[1])?;
                    (host, Some(vm), true)
                }
            }
            3 => {
                // Format: /host/path:/vm/path:ro
                let host = expand_path(parts[0])?;
                let vm = expand_path(parts[1])?;
                let writable = parts[2] == "rw";
                if parts[2] != "ro" && parts[2] != "rw" {
                    return Err(ClaudeVmError::InvalidConfig(format!(
                        "Invalid mount mode '{}': must be 'ro' or 'rw'",
                        parts[2]
                    )));
                }
                (host, Some(vm), writable)
            }
            _ => {
                return Err(ClaudeVmError::InvalidConfig(format!(
                    "Invalid mount specification '{}': too many colons",
                    spec
                )));
            }
        };

        let mut mount = Mount::new(host_path, writable);
        if let Some(vm) = vm_path {
            mount = mount.with_mount_point(vm);
        }
        Ok(mount)
    }
}

/// Expand path with ~ support and make it absolute
pub fn expand_path(path: &str) -> Result<PathBuf> {
    let expanded = if path.starts_with('~') {
        let home = std::env::var("HOME").map_err(|_| {
            ClaudeVmError::InvalidConfig("HOME environment variable not set".to_string())
        })?;
        PathBuf::from(path.replacen('~', &home, 1))
    } else {
        PathBuf::from(path)
    };

    // Ensure path is absolute
    if !expanded.is_absolute() {
        return Err(ClaudeVmError::InvalidConfig(format!(
            "Mount path must be absolute: {}",
            path
        )));
    }

    Ok(expanded)
}

/// Encode a project path for use as a Claude conversation folder name
/// Matches Claude Code's encoding logic:
/// 1. Canonicalize path (resolve symlinks like /tmp -> /private/tmp)
/// 2. Replace all non-alphanumeric characters with dashes
///
///    Example: /tmp/project@2024:v1.0 -> -private-tmp-project-2024-v1-0
fn encode_project_path(path: &Path) -> String {
    // Canonicalize path first (resolve symlinks)
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    // Replace all non-alphanumeric characters with dashes
    canonical
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect()
}

/// Get the Claude conversation folder for the current project
/// Claude stores conversations in ~/.claude/projects/ with path-encoded folder names
/// Example: /Users/user/Projects/lab/my-project -> ~/.claude/projects/-Users-user-Projects-lab-my-project
/// Creates the folder if it doesn't exist
pub(crate) fn get_claude_conversation_folder(project_path: &Path) -> Option<PathBuf> {
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

/// Convert a slice of MountEntry configs to Mount structs with validation
/// Checks for duplicates, conflicts, and warns about non-existent paths
pub fn convert_mount_entries(mount_entries: &[crate::config::MountEntry]) -> Result<Vec<Mount>> {
    let mut mounts: Vec<Mount> = Vec::new();

    for mount_entry in mount_entries {
        // Expand and validate the host path
        let host_path = expand_path(&mount_entry.location)?;

        // Create mount with explicit values from config
        let mut mount = Mount::new(host_path, mount_entry.writable);

        // Set mount point if provided
        if let Some(ref mount_point) = mount_entry.mount_point {
            let vm_path = expand_path(mount_point)?;
            mount = mount.with_mount_point(vm_path);
        }

        // Validate host path exists
        if !mount.location.exists() {
            eprintln!(
                "Warning: Mount path does not exist: {}",
                mount.location.display()
            );
        }

        // Check for duplicate host locations
        if mounts.iter().any(|m| m.location == mount.location) {
            continue; // Skip duplicate
        }

        // Check for conflicting VM mount points
        let target_path = mount.mount_point.as_ref().unwrap_or(&mount.location);
        if mounts.iter().any(|m| {
            let existing_target = m.mount_point.as_ref().unwrap_or(&m.location);
            existing_target == target_path
        }) {
            return Err(ClaudeVmError::InvalidConfig(format!(
                "Mount point conflict: {} is already mounted",
                target_path.display()
            )));
        }

        mounts.push(mount);
    }

    Ok(mounts)
}

/// Compute the mounts needed for the VM
/// Mounts the git repository root (if in a git repo), plus main repo if in a worktree,
/// plus the Claude conversation folder for the current project (if mount_conversations is true),
/// plus any custom mounts from the configuration
pub fn compute_mounts(
    mount_conversations: bool,
    custom_mounts: &[crate::config::MountEntry],
) -> Result<Vec<Mount>> {
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
                    // Mount as writable to allow git operations from worktree
                    mounts.push(Mount::new(main_repo, true));
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

    // Add custom mounts from configuration
    let custom_mount_list = convert_mount_entries(custom_mounts)?;

    // Merge custom mounts, checking for conflicts with existing mounts
    for custom_mount in custom_mount_list {
        // Check for duplicate host locations
        if mounts.iter().any(|m| m.location == custom_mount.location) {
            continue; // Skip duplicate
        }

        // Check for conflicting VM mount points with existing mounts
        let target_path = custom_mount
            .mount_point
            .as_ref()
            .unwrap_or(&custom_mount.location);
        if mounts.iter().any(|m| {
            let existing_target = m.mount_point.as_ref().unwrap_or(&m.location);
            existing_target == target_path
        }) {
            return Err(ClaudeVmError::InvalidConfig(format!(
                "Mount point conflict: {} is already mounted",
                target_path.display()
            )));
        }

        mounts.push(custom_mount);
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
        // Use a path that exists to test canonicalization
        let temp_dir = std::env::temp_dir();
        let test_path = temp_dir.join("test-encode-basic");
        std::fs::create_dir_all(&test_path).unwrap();

        let encoded = encode_project_path(&test_path);
        // All non-alphanumeric chars should be replaced with dashes
        assert!(encoded.chars().all(|c| c.is_alphanumeric() || c == '-'));
        assert!(encoded.contains("test-encode-basic"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&test_path);
    }

    #[test]
    fn test_encode_project_path_with_spaces() {
        let temp_dir = std::env::temp_dir();
        let test_path = temp_dir.join("My Test Project");
        std::fs::create_dir_all(&test_path).unwrap();

        let encoded = encode_project_path(&test_path);
        // Spaces should be replaced with dashes
        assert!(!encoded.contains(' '));
        assert!(encoded.contains("My-Test-Project"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&test_path);
    }

    #[test]
    fn test_encode_project_path_special_chars() {
        let temp_dir = std::env::temp_dir();
        let test_path = temp_dir.join("project@2024:v1.0");
        std::fs::create_dir_all(&test_path).unwrap();

        let encoded = encode_project_path(&test_path);
        // @ : and . should all be replaced with dashes
        assert!(!encoded.contains('@'));
        assert!(!encoded.contains(':'));
        assert!(!encoded.contains('.'));
        assert!(encoded.contains("project-2024-v1-0"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&test_path);
    }

    #[test]
    fn test_encode_project_path_root() {
        let path = PathBuf::from("/");
        let encoded = encode_project_path(&path);
        // Root should be all dashes
        assert!(encoded.chars().all(|c| c == '-'));
    }

    #[test]
    fn test_encode_project_path_nonexistent() {
        // For non-existent paths, should still encode them
        let path = PathBuf::from("/nonexistent/path/to/project");
        let encoded = encode_project_path(&path);
        // All non-alphanumeric chars should be replaced
        assert!(encoded.chars().all(|c| c.is_alphanumeric() || c == '-'));
        assert!(encoded.contains("nonexistent-path-to-project"));
    }

    // Test 3: Integration tests with temp directories
    #[test]
    #[serial_test::serial]
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
    #[serial_test::serial]
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
    #[serial_test::serial]
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

    // Test 4: Docker-style mount spec parsing
    #[test]
    fn test_from_spec_simple_path() {
        let mount = Mount::from_spec("/absolute/path").unwrap();
        assert_eq!(mount.location, PathBuf::from("/absolute/path"));
        assert_eq!(mount.mount_point, None);
        assert!(mount.writable);
    }

    #[test]
    fn test_from_spec_readonly() {
        let mount = Mount::from_spec("/absolute/path:ro").unwrap();
        assert_eq!(mount.location, PathBuf::from("/absolute/path"));
        assert_eq!(mount.mount_point, None);
        assert!(!mount.writable);
    }

    #[test]
    fn test_from_spec_readwrite() {
        let mount = Mount::from_spec("/absolute/path:rw").unwrap();
        assert_eq!(mount.location, PathBuf::from("/absolute/path"));
        assert_eq!(mount.mount_point, None);
        assert!(mount.writable);
    }

    #[test]
    fn test_from_spec_custom_mount_point() {
        let mount = Mount::from_spec("/host/path:/vm/path").unwrap();
        assert_eq!(mount.location, PathBuf::from("/host/path"));
        assert_eq!(mount.mount_point, Some(PathBuf::from("/vm/path")));
        assert!(mount.writable);
    }

    #[test]
    fn test_from_spec_custom_mount_point_readonly() {
        let mount = Mount::from_spec("/host/path:/vm/path:ro").unwrap();
        assert_eq!(mount.location, PathBuf::from("/host/path"));
        assert_eq!(mount.mount_point, Some(PathBuf::from("/vm/path")));
        assert!(!mount.writable);
    }

    #[test]
    fn test_from_spec_custom_mount_point_readwrite() {
        let mount = Mount::from_spec("/host/path:/vm/path:rw").unwrap();
        assert_eq!(mount.location, PathBuf::from("/host/path"));
        assert_eq!(mount.mount_point, Some(PathBuf::from("/vm/path")));
        assert!(mount.writable);
    }

    #[test]
    fn test_from_spec_tilde_expansion() {
        use std::env;
        let home = env::var("HOME").unwrap();
        let mount = Mount::from_spec("~/my-folder").unwrap();
        assert_eq!(mount.location, PathBuf::from(format!("{}/my-folder", home)));
        assert!(mount.writable);
    }

    #[test]
    fn test_from_spec_tilde_expansion_both_paths() {
        use std::env;
        let home = env::var("HOME").unwrap();
        let mount = Mount::from_spec("~/host:~/vm").unwrap();
        assert_eq!(mount.location, PathBuf::from(format!("{}/host", home)));
        assert_eq!(
            mount.mount_point,
            Some(PathBuf::from(format!("{}/vm", home)))
        );
        assert!(mount.writable);
    }

    #[test]
    fn test_from_spec_relative_path_error() {
        let result = Mount::from_spec("relative/path");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be absolute"));
    }

    #[test]
    fn test_from_spec_invalid_mode() {
        let result = Mount::from_spec("/host:/vm:invalid");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must be 'ro' or 'rw'"));
    }

    #[test]
    fn test_from_spec_too_many_colons() {
        let result = Mount::from_spec("/host:/vm:ro:extra");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too many colons"));
    }

    #[test]
    fn test_expand_path_absolute() {
        let path = expand_path("/absolute/path").unwrap();
        assert_eq!(path, PathBuf::from("/absolute/path"));
    }

    #[test]
    #[serial_test::serial]
    fn test_expand_path_tilde() {
        use std::env;
        let home = env::var("HOME").unwrap();
        let path = expand_path("~/folder").unwrap();
        assert_eq!(path, PathBuf::from(format!("{}/folder", home)));
    }

    #[test]
    fn test_expand_path_relative_error() {
        let result = expand_path("relative/path");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be absolute"));
    }

    #[test]
    #[serial_test::serial]
    fn test_expand_path_no_home() {
        use std::env;
        let original_home = env::var("HOME").ok();
        env::remove_var("HOME");

        let result = expand_path("~/folder");

        // Restore HOME
        if let Some(home) = original_home {
            env::set_var("HOME", home);
        }

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("HOME environment variable"));
    }

    // Test 5: Mount point conflict detection
    #[test]
    fn test_mount_point_conflict() {
        use crate::config::MountEntry;

        let custom_mounts = vec![
            MountEntry {
                location: "/host/path1".to_string(),
                writable: true,
                mount_point: Some("/vm/shared".to_string()),
            },
            MountEntry {
                location: "/host/path2".to_string(),
                writable: true,
                mount_point: Some("/vm/shared".to_string()), // Conflict!
            },
        ];

        let result = compute_mounts(false, &custom_mounts);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Mount point conflict"));
    }

    #[test]
    fn test_mount_deduplication() {
        use crate::config::MountEntry;

        let custom_mounts = vec![
            MountEntry {
                location: "/host/data".to_string(),
                writable: true,
                mount_point: None,
            },
            MountEntry {
                location: "/host/data".to_string(), // Duplicate location
                writable: false,
                mount_point: None,
            },
        ];

        let result = compute_mounts(false, &custom_mounts).unwrap();
        // Should only have one mount (duplicate filtered)
        assert_eq!(
            result
                .iter()
                .filter(|m| m.location.to_string_lossy() == "/host/data")
                .count(),
            1
        );
    }

    #[test]
    fn test_writable_override() {
        use crate::config::MountEntry;

        // Mount entry with writable=false should create read-only mount
        let custom_mounts = vec![MountEntry {
            location: "/host/data".to_string(),
            writable: false, // Explicitly read-only
            mount_point: None,
        }];

        let result = compute_mounts(false, &custom_mounts).unwrap();
        let mount = result
            .iter()
            .find(|m| m.location.to_string_lossy() == "/host/data");
        assert!(mount.is_some());
        assert!(!mount.unwrap().writable); // Should be read-only
    }
}
