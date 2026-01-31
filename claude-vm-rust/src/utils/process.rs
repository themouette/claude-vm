use crate::error::{ClaudeVmError, Result};
use std::process::{Command, ExitStatus};

/// Execute a command and return its status
pub fn execute(command: &str, args: &[&str]) -> Result<ExitStatus> {
    Command::new(command)
        .args(args)
        .status()
        .map_err(|e| ClaudeVmError::CommandFailed(format!("Failed to execute {}: {}", command, e)))
}

/// Execute a command and capture its output
pub fn execute_with_output(command: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(command)
        .args(args)
        .output()
        .map_err(|e| ClaudeVmError::CommandFailed(format!("Failed to execute {}: {}", command, e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ClaudeVmError::CommandFailed(format!(
            "{} failed: {}",
            command, stderr
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Check if a command exists in PATH
pub fn command_exists(command: &str) -> bool {
    which::which(command).is_ok()
}
