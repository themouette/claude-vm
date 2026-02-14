use crate::error::{ClaudeVmError, Result};
use crate::utils::git::run_git_command;
use std::path::Path;
use std::sync::OnceLock;

/// Parse git version output into (major, minor, patch) tuple
/// Handles formats like "git version 2.47.3" and "git version 2.39.2 (Apple Git-143)"
fn parse_git_version(output: &str) -> Option<(u32, u32, u32)> {
    // Expected format: "git version X.Y.Z [suffix]"
    let parts: Vec<&str> = output.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }

    // Third token should be version number
    let version_str = parts[2];

    // Split on dots and parse first 3 components
    let components: Vec<&str> = version_str.split('.').collect();
    if components.len() < 3 {
        return None;
    }

    let major = components[0].parse::<u32>().ok()?;
    let minor = components[1].parse::<u32>().ok()?;
    let patch = components[2].parse::<u32>().ok()?;

    Some((major, minor, patch))
}

/// Check if version meets minimum requirement
fn meets_minimum_version(version: (u32, u32, u32), minimum: (u32, u32, u32)) -> bool {
    // Compare major, then minor, then patch
    match version.0.cmp(&minimum.0) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => match version.1.cmp(&minimum.1) {
            std::cmp::Ordering::Greater => true,
            std::cmp::Ordering::Less => false,
            std::cmp::Ordering::Equal => version.2 >= minimum.2,
        },
    }
}

/// Check git version meets minimum requirement (2.5+)
pub fn check_git_version() -> Result<()> {
    let output_str = run_git_command(&["--version"], "check git version")?;

    let version = parse_git_version(&output_str)
        .ok_or_else(|| ClaudeVmError::Worktree("Could not parse git version".to_string()))?;

    let minimum = (2, 5, 0);
    if !meets_minimum_version(version, minimum) {
        return Err(ClaudeVmError::GitVersionTooOld {
            version: format!("{}.{}.{}", version.0, version.1, version.2),
        });
    }

    Ok(())
}

/// Check if repository has submodules by looking for .gitmodules file
pub fn has_submodules(repo_root: &Path) -> bool {
    repo_root.join(".gitmodules").exists()
}

/// Check for submodules and warn once per process if found
pub fn check_submodules_and_warn(repo_root: &Path) {
    static WARNING_SHOWN: OnceLock<bool> = OnceLock::new();

    if has_submodules(repo_root) && WARNING_SHOWN.get().is_none() {
        eprintln!(
            "Warning: This repository contains submodules. Git worktree support for submodules is experimental."
        );
        eprintln!("See: https://git-scm.com/docs/git-worktree#_bugs");
        WARNING_SHOWN.get_or_init(|| true);
    }
}

/// Validate that a branch name is a valid git ref
///
/// Rejects names that:
/// - Start with a dash (could be confused with flags)
/// - Contain null bytes
/// - Contain path traversal sequences (..)
/// - Are special refs like HEAD
pub fn validate_branch_name(branch: &str) -> Result<()> {
    // Check for empty
    if branch.is_empty() {
        return Err(ClaudeVmError::Worktree(
            "Branch name cannot be empty".to_string(),
        ));
    }

    // Check for dangerous patterns
    if branch.starts_with('-') {
        return Err(ClaudeVmError::Worktree(
            "Branch name cannot start with a dash".to_string(),
        ));
    }

    if branch.contains('\0') {
        return Err(ClaudeVmError::Worktree(
            "Branch name cannot contain null bytes".to_string(),
        ));
    }

    if branch.contains("..") {
        return Err(ClaudeVmError::Worktree(
            "Branch name cannot contain '..'".to_string(),
        ));
    }

    // Check for reserved names
    let reserved = ["HEAD", "FETCH_HEAD", "ORIG_HEAD", "MERGE_HEAD"];
    if reserved.contains(&branch) {
        return Err(ClaudeVmError::Worktree(format!(
            "'{}' is a reserved git ref name",
            branch
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // Git version parsing tests (pure logic, no git subprocess)

    #[test]
    fn test_meets_minimum_version_exact() {
        assert!(meets_minimum_version((2, 5, 0), (2, 5, 0)));
    }

    #[test]
    fn test_meets_minimum_version_higher_minor() {
        assert!(meets_minimum_version((2, 47, 3), (2, 5, 0)));
    }

    #[test]
    fn test_meets_minimum_version_higher_major() {
        assert!(meets_minimum_version((3, 0, 0), (2, 5, 0)));
    }

    #[test]
    fn test_meets_minimum_version_too_old() {
        assert!(!meets_minimum_version((2, 4, 9), (2, 5, 0)));
    }

    #[test]
    fn test_meets_minimum_version_very_old() {
        assert!(!meets_minimum_version((1, 9, 0), (2, 5, 0)));
    }

    #[test]
    fn test_parse_git_version_standard() {
        assert_eq!(parse_git_version("git version 2.47.3"), Some((2, 47, 3)));
    }

    #[test]
    fn test_parse_git_version_with_suffix() {
        assert_eq!(
            parse_git_version("git version 2.39.2 (Apple Git-143)"),
            Some((2, 39, 2))
        );
    }

    #[test]
    fn test_parse_git_version_invalid() {
        assert_eq!(parse_git_version("not a version"), None);
    }

    // Submodule detection tests (filesystem-based, use tempdir)

    #[test]
    fn test_has_submodules_true() {
        let dir = TempDir::new().unwrap();
        let gitmodules = dir.path().join(".gitmodules");
        fs::write(&gitmodules, "[submodule \"test\"]\n").unwrap();

        assert!(has_submodules(dir.path()));
    }

    #[test]
    fn test_has_submodules_false() {
        let dir = TempDir::new().unwrap();
        assert!(!has_submodules(dir.path()));
    }

    #[test]
    fn test_submodule_warning_resets() {
        // This tests the warn-once mechanism
        // Create a static to track if warning was shown
        static WARNING_SHOWN: OnceLock<bool> = OnceLock::new();

        // First call should initialize
        assert!(WARNING_SHOWN.get().is_none());
        WARNING_SHOWN.get_or_init(|| true);

        // Second call should see it's already initialized
        assert!(WARNING_SHOWN.get().is_some());
        assert!(*WARNING_SHOWN.get().unwrap());
    }

    // Branch name validation tests

    #[test]
    fn test_validate_branch_name_valid() {
        assert!(validate_branch_name("feature-branch").is_ok());
        assert!(validate_branch_name("feature/auth").is_ok());
        assert!(validate_branch_name("fix_bug").is_ok());
        assert!(validate_branch_name("v1.2.3").is_ok());
    }

    #[test]
    fn test_validate_branch_name_empty() {
        assert!(validate_branch_name("").is_err());
    }

    #[test]
    fn test_validate_branch_name_starts_with_dash() {
        assert!(validate_branch_name("-feature").is_err());
    }

    #[test]
    fn test_validate_branch_name_contains_null() {
        let name = format!("feature{}null", '\0');
        assert!(validate_branch_name(&name).is_err());
    }

    #[test]
    fn test_validate_branch_name_path_traversal() {
        assert!(validate_branch_name("../etc/passwd").is_err());
        assert!(validate_branch_name("feature/../main").is_err());
    }

    #[test]
    fn test_validate_branch_name_reserved() {
        assert!(validate_branch_name("HEAD").is_err());
        assert!(validate_branch_name("FETCH_HEAD").is_err());
    }
}
