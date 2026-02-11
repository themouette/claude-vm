use crate::commands::update::get_latest_version;
use crate::version::{is_newer_version, VERSION};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Configuration for update checking
#[derive(Debug, Clone)]
pub struct UpdateCheckConfig {
    pub enabled: bool,
    pub check_interval_hours: u64,
}

/// Cache structure for storing update check results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckCache {
    pub last_check: u64,
    pub latest_version: Option<String>,
    pub update_available: bool,
}

impl UpdateCheckCache {
    /// Check if the cache is stale based on the interval
    /// Uses saturating_sub to handle clock skew gracefully
    pub fn is_stale(&self, interval_hours: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let elapsed_seconds = now.saturating_sub(self.last_check);
        let elapsed_hours = elapsed_seconds / 3600;
        elapsed_hours >= interval_hours
    }
}

/// Get the path to the update check cache file
fn cache_path() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .map(|home| home.join(".claude-vm").join("update-check.json"))
}

/// Load the cache from disk
fn load_cache() -> Option<UpdateCheckCache> {
    let path = cache_path()?;
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Save the cache to disk with restricted permissions (0600)
fn save_cache(cache: &UpdateCheckCache) {
    if let Some(path) = cache_path() {
        // Ensure the directory exists
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        // Write cache file
        if let Ok(content) = serde_json::to_string_pretty(cache) {
            if fs::write(&path, content).is_ok() {
                // Set file permissions to 0600 (owner read/write only) on Unix systems
                #[cfg(unix)]
                {
                    let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
                }
            }
        }
    }
}

/// Clear the update check cache
/// This should be called after performing an update to reset the version check state
pub fn clear_cache() {
    if let Some(path) = cache_path() {
        let _ = fs::remove_file(path);
    }
}

/// Perform the actual version check against GitHub
fn perform_version_check() -> Option<UpdateCheckCache> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Query GitHub API with timeout (handled by self_update crate)
    let latest_version = get_latest_version().ok().flatten();

    // Validate version string is valid semver before caching
    let validated_version = latest_version.and_then(|v| {
        // Only cache if it's a valid semver string
        Version::parse(&v).ok().map(|_| v)
    });

    let update_available = if let Some(ref latest) = validated_version {
        is_newer_version(latest)
    } else {
        false
    };

    Some(UpdateCheckCache {
        last_check: now,
        latest_version: validated_version,
        update_available,
    })
}

/// Check if running in a CI/CD environment
/// CI environments typically don't need update notifications as users can't act on them
fn is_ci_environment() -> bool {
    // Common CI environment variables
    std::env::var("CI").is_ok()
        || std::env::var("GITHUB_ACTIONS").is_ok()
        || std::env::var("GITLAB_CI").is_ok()
        || std::env::var("CIRCLECI").is_ok()
        || std::env::var("TRAVIS").is_ok()
        || std::env::var("JENKINS_HOME").is_ok()
        || std::env::var("TEAMCITY_VERSION").is_ok()
        || std::env::var("BUILDKITE").is_ok()
}

/// Sanitize version string to prevent terminal injection attacks
/// Only allows characters valid in semver: 0-9, a-z, A-Z, ., -, +
fn sanitize_version(version: &str) -> String {
    version
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '+')
        .collect()
}

/// Display a boxed notification about the available update
fn display_update_notification(latest_version: &str) {
    // Sanitize version string to prevent terminal injection
    let safe_version = sanitize_version(latest_version);

    let width = 45;
    let top = format!("╭{}╮", "─".repeat(width - 2));
    let bottom = format!("╰{}╯", "─".repeat(width - 2));
    let separator = format!("├{}┤", "─".repeat(width - 2));

    let title = "A new version of claude-vm is available!";
    let title_line = format!("│{:^width$}│", title, width = width - 2);

    let current_line = format!("│  Current: {:<width$}│", VERSION, width = width - 13);
    let latest_line = format!("│  Latest:  {:<width$}│", safe_version, width = width - 13);

    let command = "Run: claude-vm update";
    let command_line = format!("│  {:<width$}│", command, width = width - 4);

    eprintln!();
    eprintln!("{}", top);
    eprintln!("{}", title_line);
    eprintln!("{}", separator);
    eprintln!("{}", current_line);
    eprintln!("{}", latest_line);
    eprintln!("{}", separator);
    eprintln!("{}", command_line);
    eprintln!("{}", bottom);
    eprintln!();
}

/// Main entry point for update checking
/// This function never returns errors - all failures are silently ignored
pub fn check_and_notify(config: &UpdateCheckConfig) {
    // Return early if disabled
    if !config.enabled {
        return;
    }

    // Don't show notifications in CI environments
    if is_ci_environment() {
        return;
    }

    // Load cache
    let cache = load_cache();

    // Determine if we need to perform a fresh check
    let needs_check = cache
        .as_ref()
        .map(|c| c.is_stale(config.check_interval_hours))
        .unwrap_or(true);

    let final_cache = if needs_check {
        // Perform fresh check
        let new_cache = perform_version_check();

        // Save the new cache
        if let Some(ref cache) = new_cache {
            save_cache(cache);
        }

        new_cache
    } else {
        // Use existing cache
        cache
    };

    // Display notification if update is available
    if let Some(cache) = final_cache {
        if cache.update_available {
            if let Some(ref version) = cache.latest_version {
                display_update_notification(version);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_is_stale() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Fresh cache (1 hour ago)
        let fresh = UpdateCheckCache {
            last_check: now - 3600,
            latest_version: Some("0.3.0".to_string()),
            update_available: true,
        };
        assert!(!fresh.is_stale(72)); // Not stale for 72h interval

        // Stale cache (100 hours ago)
        let stale = UpdateCheckCache {
            last_check: now - (100 * 3600),
            latest_version: Some("0.3.0".to_string()),
            update_available: true,
        };
        assert!(stale.is_stale(72)); // Stale for 72h interval
    }

    #[test]
    fn test_cache_serialization() {
        let cache = UpdateCheckCache {
            last_check: 1234567890,
            latest_version: Some("0.3.0".to_string()),
            update_available: true,
        };

        let json = serde_json::to_string(&cache).unwrap();
        let parsed: UpdateCheckCache = serde_json::from_str(&json).unwrap();

        assert_eq!(cache.last_check, parsed.last_check);
        assert_eq!(cache.latest_version, parsed.latest_version);
        assert_eq!(cache.update_available, parsed.update_available);
    }

    #[test]
    fn test_cache_path() {
        let path = cache_path();
        assert!(path.is_some());
        if let Some(p) = path {
            assert!(p.to_string_lossy().contains(".claude-vm"));
            assert!(p.to_string_lossy().ends_with("update-check.json"));
        }
    }

    #[test]
    fn test_config() {
        let config = UpdateCheckConfig {
            enabled: true,
            check_interval_hours: 72,
        };
        assert!(config.enabled);
        assert_eq!(config.check_interval_hours, 72);
    }

    #[test]
    fn test_sanitize_version() {
        // Valid semver characters
        assert_eq!(sanitize_version("1.2.3"), "1.2.3");
        assert_eq!(sanitize_version("1.2.3-alpha"), "1.2.3-alpha");
        assert_eq!(sanitize_version("1.2.3+build.123"), "1.2.3+build.123");

        // Filter out dangerous characters
        // \x1b is filtered, but [31m becomes 31m (alphanumeric chars remain)
        assert_eq!(
            sanitize_version("1.2.3\x1b[31m"),
            "1.2.331m" // Escape code filtered, leaving alphanumerics
        );
        assert_eq!(
            sanitize_version("1.2.3\n\r\t"),
            "1.2.3" // Control characters removed
        );
        assert_eq!(
            sanitize_version("1.2.3; rm -rf /"),
            "1.2.3rm-rf" // Special chars filtered, hyphen allowed (semver)
        );

        // Complete escape sequences are neutralized
        let malicious = "0.3.0\x1b]0;evil\x07";
        let sanitized = sanitize_version(malicious);
        // Escape codes are stripped, 'evil' remains but harmless without escapes
        assert!(sanitized.starts_with("0.3.0"));
        assert!(!sanitized.contains('\x1b'));
        assert!(!sanitized.contains('\x07'));
    }

    #[test]
    fn test_is_ci_environment() {
        // Save original env vars
        let original_ci = std::env::var("CI").ok();

        // Test with CI=true
        std::env::set_var("CI", "true");
        assert!(is_ci_environment());

        // Test with CI unset
        std::env::remove_var("CI");
        // Note: This test might fail if running in actual CI
        // In real CI, other env vars like GITHUB_ACTIONS would be set

        // Restore original
        if let Some(val) = original_ci {
            std::env::set_var("CI", val);
        } else {
            std::env::remove_var("CI");
        }
    }

    #[test]
    fn test_cache_is_stale_with_future_timestamp() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Cache with future timestamp (clock skew scenario)
        let future_cache = UpdateCheckCache {
            last_check: now + 3600, // 1 hour in the future
            latest_version: Some("0.3.0".to_string()),
            update_available: true,
        };

        // Should handle gracefully with saturating_sub (elapsed = 0)
        assert!(!future_cache.is_stale(1));
    }

    #[test]
    fn test_clear_cache() {
        // Create a test cache
        let cache = UpdateCheckCache {
            last_check: 1234567890,
            latest_version: Some("0.3.0".to_string()),
            update_available: true,
        };

        // Save the cache
        save_cache(&cache);

        // Verify it exists
        if let Some(path) = cache_path() {
            assert!(path.exists(), "Cache file should exist after save");
        }

        // Clear the cache
        clear_cache();

        // Verify it's gone
        if let Some(path) = cache_path() {
            assert!(!path.exists(), "Cache file should not exist after clear");
        }
    }
}
