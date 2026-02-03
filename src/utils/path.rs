use std::path::{Path, PathBuf};
use users::os::unix::UserExt;

/// Expand tilde (~) in paths to actual home directories.
///
/// Supports:
/// - `~` or `~/path` - expands to current user's home directory
/// - `~username/path` - expands to the specified user's home directory
///
/// # Examples
///
/// ```
/// use claude_vm::utils::path::expand_tilde;
///
/// // Current user's home
/// let path = expand_tilde("~/Documents").unwrap();
/// assert!(path.starts_with("/"));
///
/// // Other user's home (if user exists)
/// let path = expand_tilde("~root/.bashrc");
/// ```
pub fn expand_tilde<P: AsRef<Path>>(path: P) -> Option<PathBuf> {
    let path = path.as_ref();
    let path_str = path.to_str()?;

    if !path_str.starts_with('~') {
        return Some(path.to_path_buf());
    }

    // Handle the path after ~
    let after_tilde = &path_str[1..];

    // Case 1: Just ~ or ~/...
    if after_tilde.is_empty() || after_tilde.starts_with('/') {
        // Use current user's home directory
        let home = std::env::var("HOME").ok()?;
        return Some(PathBuf::from(home).join(after_tilde.trim_start_matches('/')));
    }

    // Case 2: ~username/... or ~username
    // Find the end of the username (either '/' or end of string)
    let username_end = after_tilde.find('/').unwrap_or(after_tilde.len());
    let username = &after_tilde[..username_end];
    let rest = &after_tilde[username_end..].trim_start_matches('/');

    // Look up the user's home directory
    let user = users::get_user_by_name(username)?;
    let home_dir = user.home_dir();

    Some(home_dir.join(rest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_expand_tilde_current_user() {
        let home = env::var("HOME").unwrap();

        // Just ~
        let expanded = expand_tilde("~").unwrap();
        assert_eq!(expanded, PathBuf::from(&home));

        // ~/path
        let expanded = expand_tilde("~/Documents").unwrap();
        assert_eq!(expanded, PathBuf::from(format!("{}/Documents", home)));

        // ~/path/to/file
        let expanded = expand_tilde("~/path/to/file.txt").unwrap();
        assert_eq!(
            expanded,
            PathBuf::from(format!("{}/path/to/file.txt", home))
        );
    }

    #[test]
    fn test_expand_tilde_other_user() {
        // Test with root user (should exist on most Unix systems)
        let expanded = expand_tilde("~root/.bashrc");

        // Should either succeed or return None if root user lookup fails
        // On most systems, root's home is /root
        if let Some(path) = expanded {
            assert!(path.starts_with("/"));
            assert!(path.ends_with(".bashrc"));
        }
    }

    #[test]
    fn test_expand_tilde_nonexistent_user() {
        // User that should not exist
        let expanded = expand_tilde("~nonexistentuser12345/file");

        // Should return None for non-existent user
        assert!(expanded.is_none());
    }

    #[test]
    fn test_expand_tilde_no_tilde() {
        // Absolute path without tilde
        let expanded = expand_tilde("/absolute/path").unwrap();
        assert_eq!(expanded, PathBuf::from("/absolute/path"));

        // Relative path without tilde
        let expanded = expand_tilde("relative/path").unwrap();
        assert_eq!(expanded, PathBuf::from("relative/path"));
    }

    #[test]
    #[serial_test::serial]
    fn test_expand_tilde_no_home_env() {
        let original_home = env::var("HOME").ok();
        env::remove_var("HOME");

        // Should fail gracefully when HOME is not set
        let expanded = expand_tilde("~/file");
        assert!(expanded.is_none());

        // Restore HOME
        if let Some(home) = original_home {
            env::set_var("HOME", home);
        }
    }

    #[test]
    fn test_expand_tilde_username_only() {
        // ~root without trailing slash
        let expanded = expand_tilde("~root");

        if let Some(path) = expanded {
            // Should expand to root's home directory
            assert!(path.starts_with("/"));
        }
    }

    #[test]
    fn test_expand_tilde_edge_cases() {
        // Path with tilde not at start should not expand
        let expanded = expand_tilde("/path/~user/file").unwrap();
        assert_eq!(expanded, PathBuf::from("/path/~user/file"));

        // Multiple tildes (only first should be considered)
        let home = env::var("HOME").unwrap();
        let expanded = expand_tilde("~/~file").unwrap();
        assert_eq!(expanded, PathBuf::from(format!("{}/~file", home)));
    }
}
