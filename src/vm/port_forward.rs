//! Port forwarding configuration for Lima VMs.
//!
//! This module provides support for Unix socket forwarding between the host
//! and guest VM, enabling features like GPG agent forwarding.
//!
//! # Security
//!
//! - Socket paths are validated to prevent path traversal attacks
//! - Detection commands are whitelisted to prevent command injection
//! - All paths must be absolute for security

use crate::error::{ClaudeVmError, Result};
use std::process::Command;

/// Represents a Lima port forward configuration for Unix sockets.
///
/// # Example
///
/// ```ignore
/// let pf = PortForward::unix_socket(
///     "/Users/me/.gnupg/S.gpg-agent.extra".to_string(),
///     "/tmp/gpg-agent.socket".to_string()
/// )?;
/// ```
#[derive(Debug, Clone)]
pub struct PortForward {
    /// Whether this is a reverse forward (host -> guest)
    pub reverse: bool,
    /// Host socket path (validated)
    pub host_socket: String,
    /// Guest socket path (validated)
    pub guest_socket: String,
}

impl PortForward {
    /// Create a new Unix socket port forward (reverse = true means host -> guest)
    ///
    /// Validates socket paths to prevent path traversal and injection attacks.
    pub fn unix_socket(host_socket: String, guest_socket: String) -> Result<Self> {
        Self::validate_socket_path(&host_socket)?;
        Self::validate_socket_path(&guest_socket)?;

        Ok(Self {
            reverse: true,
            host_socket,
            guest_socket,
        })
    }

    /// Validate a socket path for security
    fn validate_socket_path(path: &str) -> Result<()> {
        // Check for path traversal attempts
        if path.contains("..") {
            return Err(ClaudeVmError::InvalidConfig(format!(
                "Socket path contains path traversal: '{}'",
                path
            )));
        }

        // Check for null bytes
        if path.contains('\0') {
            return Err(ClaudeVmError::InvalidConfig(
                "Socket path contains null byte".to_string(),
            ));
        }

        // Must be absolute path for security
        if !path.starts_with('/') {
            return Err(ClaudeVmError::InvalidConfig(format!(
                "Socket path must be absolute: '{}'",
                path
            )));
        }

        // Check for suspicious characters
        if path.contains('\n') || path.contains('\r') {
            return Err(ClaudeVmError::InvalidConfig(format!(
                "Socket path contains invalid characters: '{}'",
                path
            )));
        }

        Ok(())
    }

    /// Detect socket path by running a command on the host
    ///
    /// For security, only whitelisted commands are allowed.
    pub fn detect_socket_path(command: &str) -> Result<String> {
        // Whitelist of allowed socket detection commands to prevent command injection
        const ALLOWED_COMMANDS: &[&str] = &[
            "gpgconf --list-dir agent-extra-socket",
            "gpgconf --list-dir agent-socket",
        ];

        if !ALLOWED_COMMANDS.contains(&command) {
            return Err(ClaudeVmError::InvalidConfig(format!(
                "Socket detection command not allowed: '{}'. Allowed commands: {:?}",
                command, ALLOWED_COMMANDS
            )));
        }

        // Try detection with retries for reliability
        for attempt in 1..=3 {
            match Self::try_detect_socket(command) {
                Ok(path) => return Ok(path),
                Err(e) if attempt < 3 => {
                    eprintln!(
                        "Socket detection attempt {}/3 failed: {}. Retrying...",
                        attempt, e
                    );
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
                Err(e) => return Err(e),
            }
        }

        unreachable!()
    }

    /// Try to detect socket path (single attempt)
    fn try_detect_socket(command: &str) -> Result<String> {
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .map_err(|e| {
                ClaudeVmError::LimaExecution(format!(
                    "Failed to execute socket detection command '{}': {}",
                    command, e
                ))
            })?;

        if !output.status.success() {
            return Err(ClaudeVmError::LimaExecution(format!(
                "Socket detection command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if path.is_empty() {
            return Err(ClaudeVmError::LimaExecution(
                "Socket detection returned empty path".to_string(),
            ));
        }

        Ok(path)
    }

    /// Generate --set arguments for limactl create
    /// Returns a Vec of (key, value) pairs for --set flags
    pub fn to_set_args(&self, index: usize) -> Vec<(String, String)> {
        vec![
            (
                format!(".portForwards[{}].reverse", index),
                self.reverse.to_string(),
            ),
            (
                format!(".portForwards[{}].hostSocket", index),
                format!("\"{}\"", self.host_socket),
            ),
            (
                format!(".portForwards[{}].guestSocket", index),
                format!("\"{}\"", self.guest_socket),
            ),
            (
                format!(".portForwards[{}].hostPortRange", index),
                "[0,0]".to_string(),
            ),
            (
                format!(".portForwards[{}].guestPortRange", index),
                "[0,0]".to_string(),
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_forward_to_set_args() {
        let pf = PortForward::unix_socket("/host/socket".to_string(), "/guest/socket".to_string())
            .expect("Valid socket paths");

        let args = pf.to_set_args(0);

        assert_eq!(args.len(), 5);
        assert_eq!(
            args[0],
            (".portForwards[0].reverse".to_string(), "true".to_string())
        );
        assert_eq!(
            args[1],
            (
                ".portForwards[0].hostSocket".to_string(),
                "\"/host/socket\"".to_string()
            )
        );
        assert_eq!(
            args[2],
            (
                ".portForwards[0].guestSocket".to_string(),
                "\"/guest/socket\"".to_string()
            )
        );
    }

    #[test]
    fn test_socket_path_validation_path_traversal() {
        let result = PortForward::unix_socket("/tmp/../etc/passwd".to_string(), "/tmp/socket".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("path traversal"));
    }

    #[test]
    fn test_socket_path_validation_relative_path() {
        let result = PortForward::unix_socket("tmp/socket".to_string(), "/tmp/socket".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be absolute"));
    }

    #[test]
    fn test_socket_path_validation_null_byte() {
        let result = PortForward::unix_socket("/tmp/socket\0".to_string(), "/tmp/socket".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("null byte"));
    }

    #[test]
    fn test_socket_path_validation_newline() {
        let result = PortForward::unix_socket("/tmp/socket\n".to_string(), "/tmp/socket".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid characters"));
    }

    #[test]
    fn test_socket_path_validation_valid() {
        let result = PortForward::unix_socket("/tmp/socket".to_string(), "/var/run/socket".to_string());
        assert!(result.is_ok());
    }
}
