use crate::error::{ClaudeVmError, Result};
use crate::utils::git;
use std::path::{Path, PathBuf};
use std::process::Command;

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

        format!("claude-tpl_{}_{}{}", sanitized, short_hash, suffix)
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
}
