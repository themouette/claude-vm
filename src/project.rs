use crate::error::{ClaudeVmError, Result};
use crate::utils::git;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Maximum length for template names to avoid UNIX_PATH_MAX issues
/// Lima creates socket paths like: ~/.lima/{vm-name}/ssh.sock.{random}
/// With typical base paths (~30 chars) and socket suffix (~28 chars),
/// and VM sessions adding process ID (~11 chars), we need to keep
/// template names under 50 chars to stay well under the 104 char limit.
const MAX_TEMPLATE_NAME_LENGTH: usize = 50;

#[derive(Debug, Clone)]
pub struct Project {
    /// Current working directory (worktree if in worktree, otherwise main repo)
    root: PathBuf,
    /// Main repository root (for template naming)
    main_repo_root: PathBuf,
    template_name: String,
}

impl Project {
    /// Detect the current project and generate its template name
    pub fn detect() -> Result<Self> {
        let (root, main_repo_root) = Self::get_project_roots()?;
        let template_name = Self::generate_template_name(&main_repo_root);
        Ok(Self {
            root,
            main_repo_root,
            template_name,
        })
    }

    /// Get both the current project root and the main repository root
    /// Returns (current_root, main_repo_root)
    /// - current_root: worktree root if in worktree, otherwise main repo root
    /// - main_repo_root: always the main repository root (used for template naming)
    fn get_project_roots() -> Result<(PathBuf, PathBuf)> {
        // Check if we're in a worktree
        if git::is_worktree() {
            // Get worktree root (current working location)
            let worktree_root = Self::get_git_toplevel()?;

            // Get main repo root from common git dir
            let main_repo_root = Self::get_main_repo_root()?;

            Ok((worktree_root, main_repo_root))
        } else {
            // Not in a worktree, use the same root for both
            let root = Self::get_git_toplevel()?;
            Ok((root.clone(), root))
        }
    }

    /// Get the top-level directory (worktree root if in worktree, main repo otherwise)
    fn get_git_toplevel() -> Result<PathBuf> {
        if let Ok(output) = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
        {
            if output.status.success() {
                let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let root_path = PathBuf::from(root);

                if let Ok(canonical) = root_path.canonicalize() {
                    return Ok(canonical);
                }
            }
        }

        // Fallback to current directory
        std::env::current_dir().map_err(|e| {
            ClaudeVmError::ProjectDetection(format!("Failed to get current directory: {}", e))
        })
    }

    /// Get the main repository root (parent of the common git dir)
    fn get_main_repo_root() -> Result<PathBuf> {
        let common_dir = git::get_git_common_dir()?
            .ok_or_else(|| ClaudeVmError::Git("Not in a git repository".to_string()))?;

        // The common dir is .git for the main repo, so parent is the main repo root
        common_dir
            .parent()
            .ok_or_else(|| ClaudeVmError::Git("Invalid git common directory".to_string()))
            .map(|p| p.to_path_buf())
    }

    /// Generate template name: claude-tpl_{sanitized-basename}_{8-char-md5-hash}[-dev]
    /// Enforces MAX_TEMPLATE_NAME_LENGTH to avoid UNIX_PATH_MAX issues with socket paths
    fn generate_template_name(root: &Path) -> String {
        let basename = root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project");

        // Sanitize: lowercase, alphanumeric + dash, collapse multiple dashes
        let sanitized = Self::sanitize_name(basename);

        // Generate 8-character MD5 hash of the full path
        let full_path = root.to_string_lossy();
        let digest = md5::compute(full_path.as_bytes());
        let hash = format!("{:x}", digest);
        let short_hash = &hash[..8];

        // Add -dev suffix for debug builds
        #[cfg(debug_assertions)]
        let suffix = "-dev";
        #[cfg(not(debug_assertions))]
        let suffix = "";

        // Calculate fixed overhead: "claude-tpl_" (11) + "_" (1) + hash (8) + suffix (0 or 4)
        let prefix = "claude-tpl_";
        let fixed_overhead = prefix.len() + 1 + short_hash.len() + suffix.len();

        // Truncate sanitized name if necessary to stay within max length
        let max_sanitized_len = MAX_TEMPLATE_NAME_LENGTH.saturating_sub(fixed_overhead);
        let truncated = if sanitized.len() > max_sanitized_len {
            &sanitized[..max_sanitized_len]
        } else {
            &sanitized
        };

        format!("{}{}_{}{}", prefix, truncated, short_hash, suffix)
    }

    /// Sanitize name: lowercase, alphanumeric + dash, collapse dashes
    fn sanitize_name(name: &str) -> String {
        let mut result = String::new();
        let mut last_was_dash = false;

        for c in name.to_lowercase().chars() {
            if c.is_alphanumeric() {
                result.push(c);
                last_was_dash = false;
            } else if !last_was_dash {
                result.push('-');
                last_was_dash = true;
            }
        }

        // Trim leading/trailing dashes
        result.trim_matches('-').to_string()
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn main_repo_root(&self) -> &Path {
        &self.main_repo_root
    }

    pub fn template_name(&self) -> &str {
        &self.template_name
    }

    /// Check if the project is in a worktree
    pub fn is_worktree(&self) -> bool {
        self.root != self.main_repo_root
    }

    /// Create a Project instance for testing purposes
    /// Uses the provided path as both root and main_repo_root
    ///
    /// # Note
    /// This is only intended for use in tests. In production code, use `Project::detect()`.
    #[doc(hidden)]
    pub fn new_for_test(root: PathBuf) -> Self {
        let template_name = Self::generate_template_name(&root);
        Self {
            root: root.clone(),
            main_repo_root: root,
            template_name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_name() {
        assert_eq!(Project::sanitize_name("MyProject"), "myproject");
        assert_eq!(Project::sanitize_name("my-project"), "my-project");
        assert_eq!(Project::sanitize_name("my_project"), "my-project");
        assert_eq!(Project::sanitize_name("my  project"), "my-project");
        assert_eq!(Project::sanitize_name("my---project"), "my-project");
        assert_eq!(Project::sanitize_name("123-project"), "123-project");
        assert_eq!(Project::sanitize_name("project!!!"), "project");
    }

    #[test]
    fn test_generate_template_name() {
        let path = PathBuf::from("/home/user/my-project");
        let template_name = Project::generate_template_name(&path);

        assert!(template_name.starts_with("claude-tpl_my-project_"));

        // Check length based on build profile
        #[cfg(debug_assertions)]
        {
            // Format: claude-tpl_my-project_12345678-dev
            assert!(template_name.ends_with("-dev"));
            assert_eq!(
                template_name.len(),
                "claude-tpl_my-project_".len() + 8 + "-dev".len()
            );
        }
        #[cfg(not(debug_assertions))]
        {
            // Format: claude-tpl_my-project_12345678
            assert!(!template_name.ends_with("-dev"));
            assert_eq!(template_name.len(), "claude-tpl_my-project_".len() + 8);
        }
    }

    #[test]
    fn test_generate_template_name_dev_suffix() {
        let path = PathBuf::from("/home/user/test-project");
        let template_name = Project::generate_template_name(&path);

        // Verify format is correct
        assert!(template_name.starts_with("claude-tpl_test-project_"));

        // In debug builds, should have -dev suffix
        #[cfg(debug_assertions)]
        assert!(
            template_name.ends_with("-dev"),
            "Debug build should have -dev suffix: {}",
            template_name
        );

        // In release builds, should not have -dev suffix
        #[cfg(not(debug_assertions))]
        assert!(
            !template_name.ends_with("-dev"),
            "Release build should not have -dev suffix: {}",
            template_name
        );
    }

    #[test]
    fn test_generate_template_name_length_limit() {
        // Test that very long project names are truncated to stay within MAX_TEMPLATE_NAME_LENGTH
        let long_name = "a".repeat(100); // 100 'a's
        let path = PathBuf::from(format!("/home/user/{}", long_name));
        let template_name = Project::generate_template_name(&path);

        // Template name should not exceed the max length
        assert!(
            template_name.len() <= MAX_TEMPLATE_NAME_LENGTH,
            "Template name too long: {} chars (max: {})",
            template_name.len(),
            MAX_TEMPLATE_NAME_LENGTH
        );

        // Should still have the correct format
        assert!(template_name.starts_with("claude-tpl_"));

        // Should contain the hash (8 chars before suffix)
        #[cfg(debug_assertions)]
        {
            assert!(template_name.ends_with("-dev"));
            // Hash should be 8 chars before "-dev"
            let without_suffix = template_name.trim_end_matches("-dev");
            assert!(without_suffix.len() >= 8);
        }

        #[cfg(not(debug_assertions))]
        {
            assert!(!template_name.ends_with("-dev"));
            // Hash should be last 8 chars
            assert!(template_name.len() >= 8);
        }
    }

    #[test]
    fn test_generate_template_name_specific_long_example() {
        // Test the specific case from the bug report
        let path =
            PathBuf::from("/home/user/claude-orchestrator-themouette-add-user-authentication");
        let template_name = Project::generate_template_name(&path);

        // Should be truncated
        assert!(
            template_name.len() <= MAX_TEMPLATE_NAME_LENGTH,
            "Template name too long: {} chars (max: {})",
            template_name.len(),
            MAX_TEMPLATE_NAME_LENGTH
        );

        // Should start with the prefix and some of the project name
        assert!(template_name.starts_with("claude-tpl_claude-orchestr"));

        // Should contain hash
        assert!(template_name.contains('_'));
    }

    #[test]
    fn test_generate_template_name_short_unchanged() {
        // Test that short names are not affected
        let path = PathBuf::from("/home/user/my-app");
        let template_name = Project::generate_template_name(&path);

        // Should contain the full project name (not truncated)
        assert!(template_name.contains("my-app"));

        // Should be well under the limit
        assert!(template_name.len() <= MAX_TEMPLATE_NAME_LENGTH);
    }

    #[test]
    fn test_generate_template_name_ensures_vm_session_safety() {
        // Test that template names leave enough room for VM session names
        // VM session format: {template_name}-{process_id}
        // Process IDs can be up to 10 digits, so we need ~11 extra chars (including dash)

        let long_name = "very-long-project-name-that-should-be-truncated-safely";
        let path = PathBuf::from(format!("/home/user/{}", long_name));
        let template_name = Project::generate_template_name(&path);

        // Simulate VM session name (template + "-" + max PID)
        let vm_session_name = format!("{}-9999999999", template_name);

        // VM session should fit within safe socket path limits
        // Typical socket path: ~/.lima/{vm-name}/ssh.sock.{random}
        // Base: ~30 chars, suffix: ~28 chars, total overhead: ~58 chars
        // UNIX_PATH_MAX: 104 chars, so vm_name should be < 46 chars
        const SAFE_VM_NAME_LENGTH: usize = 65; // Conservative limit
        assert!(
            vm_session_name.len() <= SAFE_VM_NAME_LENGTH,
            "VM session name too long: {} chars (safe limit: {})",
            vm_session_name.len(),
            SAFE_VM_NAME_LENGTH
        );
    }
}
