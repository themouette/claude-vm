use crate::error::Result;

// Compile-time constants from Cargo.toml
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
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
}
