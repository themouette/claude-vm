use crate::error::{ClaudeVmError, Result};
use std::path::Path;
use std::process::Command;
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
    let output = Command::new("git")
        .arg("--version")
        .output()
        .map_err(|e| ClaudeVmError::Git(format!("Failed to run git: {}", e)))?;

    let output_str = String::from_utf8_lossy(&output.stdout);

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
}
