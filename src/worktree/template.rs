use crate::error::Result;
use crate::worktree::config::WorktreeConfig;
use chrono::Local;
use std::path::{Path, PathBuf};

/// Sanitize a path component by replacing invalid characters with safe alternatives
/// - Replace `/` and `\` with `-`
/// - Replace spaces and control characters with `_`
fn sanitize_path_component(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c == '/' || c == '\\' {
                '-'
            } else if c == ' ' || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect()
}

/// Context for template variable expansion
pub struct TemplateContext {
    pub repo: String,
    pub branch: String,
    pub user: String,
    pub date: String,
    pub short_hash: String,
}

impl TemplateContext {
    /// Create a new template context
    pub fn new(repo_name: &str, branch: &str, short_hash: &str) -> Self {
        let user = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
        let date = Local::now().format("%Y-%m-%d").to_string();
        let short_hash = if short_hash.len() > 8 {
            short_hash[..8].to_string()
        } else {
            short_hash.to_string()
        };

        Self {
            repo: repo_name.to_string(),
            branch: branch.to_string(),
            user,
            date,
            short_hash,
        }
    }

    /// Expand template variables in a template string
    ///
    /// Replaces known variables with sanitized values:
    /// - {repo} -> sanitized repo name
    /// - {branch} -> sanitized branch name
    /// - {user} -> sanitized username
    /// - {date} -> sanitized date (YYYY-MM-DD)
    /// - {short_hash} -> sanitized short hash (8 chars or less)
    ///
    /// Unknown variables are left unexpanded
    pub fn expand(&self, template: &str) -> String {
        template
            .replace("{repo}", &sanitize_path_component(&self.repo))
            .replace("{branch}", &sanitize_path_component(&self.branch))
            .replace("{user}", &sanitize_path_component(&self.user))
            .replace("{date}", &sanitize_path_component(&self.date))
            .replace("{short_hash}", &sanitize_path_component(&self.short_hash))
    }
}

/// Compute the full worktree path from config, repo root, and template context
/// - If config.location is Some, use that as the base directory
/// - If config.location is None, compute sibling directory: {repo_root}-worktrees
/// - Expand the template with the context
/// - Join the base directory with the expanded template
/// - Validates that the final path stays within the base directory
pub fn compute_worktree_path(
    config: &WorktreeConfig,
    repo_root: &Path,
    context: &TemplateContext,
) -> Result<PathBuf> {
    let base_dir = if let Some(location) = &config.location {
        PathBuf::from(location)
    } else {
        // Compute sibling directory
        let repo_name = repo_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("repo");
        let parent = repo_root.parent().unwrap_or(repo_root);
        parent.join(format!("{}-worktrees", repo_name))
    };

    let expanded_template = context.expand(&config.template);
    let final_path = base_dir.join(&expanded_template);

    // Canonicalize both paths and verify final_path is under base_dir
    // Only do this check if base_dir exists; if it doesn't exist yet, we can't canonicalize
    if base_dir.exists() {
        let canonical_base = base_dir.canonicalize().map_err(|e| {
            crate::error::ClaudeVmError::Worktree(format!(
                "Failed to canonicalize base directory: {}",
                e
            ))
        })?;

        // For non-existent final_path, check parent directory
        let check_path = if final_path.exists() {
            final_path.canonicalize().map_err(|e| {
                crate::error::ClaudeVmError::Worktree(format!(
                    "Failed to canonicalize worktree path: {}",
                    e
                ))
            })?
        } else {
            // Check that parent would be under base_dir
            // We can't canonicalize a non-existent path, so we check the expanded template
            // doesn't try to escape with ".."
            if expanded_template.contains("..") || expanded_template.starts_with('/') {
                return Err(crate::error::ClaudeVmError::WorktreePathTraversal {
                    path: final_path.display().to_string(),
                });
            }
            // Construct the check path from the canonical base to handle symlinks correctly
            // (e.g., on macOS where /var -> /private/var)
            canonical_base.join(&expanded_template)
        };

        // Verify the path is under base directory
        if !check_path.starts_with(&canonical_base) {
            return Err(crate::error::ClaudeVmError::WorktreePathTraversal {
                path: final_path.display().to_string(),
            });
        }
    } else {
        // Base dir doesn't exist yet - do basic check on expanded template
        if expanded_template.contains("..") || expanded_template.starts_with('/') {
            return Err(crate::error::ClaudeVmError::WorktreePathTraversal {
                path: final_path.display().to_string(),
            });
        }
    }

    Ok(final_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Sanitization tests ==========

    #[test]
    fn test_sanitize_forward_slash() {
        assert_eq!(sanitize_path_component("feature/auth"), "feature-auth");
    }

    #[test]
    fn test_sanitize_backslash() {
        assert_eq!(sanitize_path_component("feature\\auth"), "feature-auth");
    }

    #[test]
    fn test_sanitize_spaces() {
        assert_eq!(sanitize_path_component("my branch"), "my_branch");
    }

    #[test]
    fn test_sanitize_control_chars() {
        let input = format!("test{}tab{}null", '\t', '\0');
        let result = sanitize_path_component(&input);
        assert_eq!(result, "test_tab_null");
    }

    #[test]
    fn test_sanitize_mixed() {
        assert_eq!(
            sanitize_path_component("feature/my branch"),
            "feature-my_branch"
        );
    }

    #[test]
    fn test_sanitize_clean_string() {
        assert_eq!(
            sanitize_path_component("clean-branch-name"),
            "clean-branch-name"
        );
    }

    #[test]
    fn test_sanitize_dots_preserved() {
        assert_eq!(sanitize_path_component("v1.2.3"), "v1.2.3");
    }

    #[test]
    fn test_sanitize_consecutive_separators() {
        // Each character replaced individually, no collapsing
        assert_eq!(
            sanitize_path_component("feature//double"),
            "feature--double"
        );
    }

    // ========== Template expansion tests ==========

    #[test]
    fn test_expand_branch_only() {
        let ctx = TemplateContext {
            repo: "myrepo".to_string(),
            branch: "feature/auth".to_string(),
            user: "testuser".to_string(),
            date: "2024-01-01".to_string(),
            short_hash: "abc12345".to_string(),
        };
        assert_eq!(ctx.expand("{branch}"), "feature-auth");
    }

    #[test]
    fn test_expand_repo_branch() {
        let ctx = TemplateContext {
            repo: "myproject".to_string(),
            branch: "main".to_string(),
            user: "testuser".to_string(),
            date: "2024-01-01".to_string(),
            short_hash: "abc12345".to_string(),
        };
        assert_eq!(ctx.expand("{repo}-{branch}"), "myproject-main");
    }

    #[test]
    fn test_expand_all_variables() {
        let ctx = TemplateContext {
            repo: "myrepo".to_string(),
            branch: "dev".to_string(),
            user: "alice".to_string(),
            date: "2024-01-15".to_string(),
            short_hash: "def45678".to_string(),
        };
        let result = ctx.expand("{repo}-{branch}-{user}-{date}-{short_hash}");
        assert_eq!(result, "myrepo-dev-alice-2024-01-15-def45678");
    }

    #[test]
    fn test_expand_date_format() {
        let ctx = TemplateContext::new("repo", "branch", "abc12345");
        let result = ctx.expand("{date}");
        // Verify date format is YYYY-MM-DD (10 characters)
        assert_eq!(result.len(), 10);
        // Verify it matches YYYY-MM-DD pattern
        let re = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
        assert!(re.is_match(&result), "Date should match YYYY-MM-DD format");
    }

    #[test]
    fn test_expand_short_hash_length() {
        let ctx = TemplateContext::new("repo", "branch", "abcdef1234567890");
        let result = ctx.expand("{short_hash}");
        assert_eq!(result.len(), 8);
        assert_eq!(result, "abcdef12");
    }

    #[test]
    fn test_expand_short_hash_shorter_than_eight() {
        let ctx = TemplateContext::new("repo", "branch", "abc");
        let result = ctx.expand("{short_hash}");
        assert_eq!(result, "abc");
    }

    #[test]
    fn test_expand_no_variables() {
        let ctx = TemplateContext::new("repo", "branch", "abc12345");
        assert_eq!(ctx.expand("fixed-path"), "fixed-path");
    }

    #[test]
    fn test_expand_unknown_variable() {
        let ctx = TemplateContext::new("repo", "branch", "abc12345");
        // Unknown variables left as-is
        assert_eq!(ctx.expand("{unknown}"), "{unknown}");
    }

    #[test]
    fn test_expand_branch_with_slashes() {
        let ctx = TemplateContext {
            repo: "myrepo".to_string(),
            branch: "feature/deep/nested".to_string(),
            user: "testuser".to_string(),
            date: "2024-01-01".to_string(),
            short_hash: "abc12345".to_string(),
        };
        assert_eq!(ctx.expand("{branch}"), "feature-deep-nested");
    }

    // ========== Worktree path computation tests ==========

    #[test]
    fn test_default_path_sibling_directory() {
        let config = WorktreeConfig::default(); // No location, template = "{branch}"
        let repo_root = PathBuf::from("/home/user/myproject");
        let ctx = TemplateContext::new("myproject", "feature", "abc12345");

        let result = compute_worktree_path(&config, &repo_root, &ctx).unwrap();
        assert_eq!(
            result,
            PathBuf::from("/home/user/myproject-worktrees/feature")
        );
    }

    #[test]
    fn test_custom_location() {
        let config = WorktreeConfig {
            location: Some("/tmp/worktrees".to_string()),
            template: "{branch}".to_string(),
        };
        let repo_root = PathBuf::from("/home/user/myproject");
        let ctx = TemplateContext::new("myproject", "main", "abc12345");

        let result = compute_worktree_path(&config, &repo_root, &ctx).unwrap();
        assert_eq!(result, PathBuf::from("/tmp/worktrees/main"));
    }

    #[test]
    fn test_custom_template_with_location() {
        let config = WorktreeConfig {
            location: Some("/work".to_string()),
            template: "{repo}-{branch}".to_string(),
        };
        let repo_root = PathBuf::from("/home/user/proj");
        let ctx = TemplateContext::new("proj", "dev", "abc12345");

        let result = compute_worktree_path(&config, &repo_root, &ctx).unwrap();
        assert_eq!(result, PathBuf::from("/work/proj-dev"));
    }

    #[test]
    fn test_default_template_branch() {
        let config = WorktreeConfig::default();
        let repo_root = PathBuf::from("/home/user/myrepo");
        let ctx = TemplateContext::new("myrepo", "hotfix", "abc12345");

        let result = compute_worktree_path(&config, &repo_root, &ctx).unwrap();
        // Default template is "{branch}", so worktree named "hotfix" in base dir
        assert_eq!(result, PathBuf::from("/home/user/myrepo-worktrees/hotfix"));
    }

    #[test]
    fn test_sibling_directory_uses_repo_name() {
        // The sibling directory should use the actual repo directory name, not template variable
        let config = WorktreeConfig::default();
        let repo_root = PathBuf::from("/projects/my-awesome-project");
        let ctx = TemplateContext::new("my-awesome-project", "feature/new", "abc12345");

        let result = compute_worktree_path(&config, &repo_root, &ctx).unwrap();
        // Sibling directory name: my-awesome-project-worktrees
        // Branch sanitized: feature-new
        assert_eq!(
            result,
            PathBuf::from("/projects/my-awesome-project-worktrees/feature-new")
        );
    }

    // ========== Path traversal prevention tests ==========

    #[test]
    fn test_path_traversal_prevention_dotdot() {
        let config = WorktreeConfig {
            location: Some("/tmp/worktrees".to_string()),
            template: "../escape".to_string(),
        };
        let repo_root = PathBuf::from("/home/user/myproject");
        let ctx = TemplateContext::new("myproject", "branch", "abc12345");

        // This should fail because template contains ".."
        let result = compute_worktree_path(&config, &repo_root, &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_path_traversal_prevention_absolute() {
        let config = WorktreeConfig {
            location: Some("/tmp/worktrees".to_string()),
            template: "/etc/passwd".to_string(),
        };
        let repo_root = PathBuf::from("/home/user/myproject");
        let ctx = TemplateContext::new("myproject", "branch", "abc12345");

        // This should fail because template is absolute path
        let result = compute_worktree_path(&config, &repo_root, &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_nested_template_allowed() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        // Canonicalize the temp dir path to handle macOS symlinks (/var -> /private/var)
        let canonical_temp = temp_dir.path().canonicalize().unwrap();
        let config = WorktreeConfig {
            location: Some(canonical_temp.to_string_lossy().to_string()),
            template: "nested/path/{branch}".to_string(),
        };
        let repo_root = PathBuf::from("/home/user/myproject");
        let ctx = TemplateContext::new("myproject", "feature", "abc12345");

        // This should succeed - nested paths are fine as long as they stay under base
        let result = compute_worktree_path(&config, &repo_root, &ctx);
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains("nested/path/feature"));
    }
}
