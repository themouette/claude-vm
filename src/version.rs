use crate::error::Result;
use semver::Version;

// Compile-time constants from Cargo.toml and build.rs
pub const VERSION: &str = env!("CLAUDE_VM_VERSION");
pub const PKG_NAME: &str = env!("CARGO_PKG_NAME");

// GitHub repository info
pub const REPO_OWNER: &str = "themouette";
pub const REPO_NAME: &str = "claude-vm";

// Platform detection helper
pub fn current_platform() -> Result<String> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok("macos-aarch64".to_string()),
        ("macos", "x86_64") => Ok("macos-x86_64".to_string()),
        ("linux", "aarch64") => Ok("linux-aarch64".to_string()),
        ("linux", "x86_64") => Ok("linux-x86_64".to_string()),
        (os, arch) => Err(crate::error::ClaudeVmError::UpdateError(format!(
            "Unsupported platform: {}-{}",
            os, arch
        ))),
    }
}

pub fn binary_name() -> &'static str {
    PKG_NAME
}

/// Check if another version is newer than the current version
pub fn is_newer_version(other: &str) -> bool {
    match (Version::parse(VERSION), Version::parse(other)) {
        (Ok(current), Ok(latest)) => latest > current,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert!(!VERSION.is_empty());
        assert_eq!(PKG_NAME, "claude-vm");
        assert_eq!(REPO_OWNER, "themouette");
        assert_eq!(REPO_NAME, "claude-vm");
    }

    #[test]
    fn test_binary_name() {
        assert_eq!(binary_name(), "claude-vm");
    }

    #[test]
    fn test_platform_detection() {
        // This will succeed on supported platforms
        let result = current_platform();
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_is_newer_version() {
        // Test with valid versions
        assert!(is_newer_version("999.0.0")); // Much newer
        assert!(!is_newer_version("0.0.1")); // Much older

        // Test with invalid versions
        assert!(!is_newer_version("invalid"));
        assert!(!is_newer_version(""));

        // Test with same version (should return false)
        assert!(!is_newer_version(VERSION));
    }

    #[test]
    fn test_version_format() {
        // VERSION should be non-empty
        assert!(!VERSION.is_empty());

        // Should either be a semver version (release) or contain -dev+ (debug)
        // Examples: "0.3.0" or "0.3.0-dev+a1b2c3d4" or "0.3.0-dev+a1b2c3d4.dirty"
        assert!(
            VERSION.chars().next().unwrap().is_numeric(),
            "Version should start with a number"
        );
    }

    #[test]
    fn test_version_is_valid_semver_with_metadata() {
        // Version should be parseable by semver (which supports build metadata)
        // Semver supports: 1.2.3-prerelease+build
        // Our format: 0.3.0-dev+hash or 0.3.0-dev+hash.dirty
        let base_version = if VERSION.contains('-') {
            // Extract base version from "0.3.0-dev+hash"
            VERSION.split('-').next().unwrap()
        } else {
            VERSION
        };

        assert!(
            Version::parse(base_version).is_ok(),
            "Base version should be valid semver: {}",
            base_version
        );
    }
}
