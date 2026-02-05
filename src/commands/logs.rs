use crate::error::{ClaudeVmError, Result};
use crate::project::Project;
use crate::vm::template;
use std::path::PathBuf;
use std::process::Command;

pub fn execute(project: &Project, follow: bool) -> Result<()> {
    // Check if template exists
    if !template::exists(project.template_name())? {
        return Err(ClaudeVmError::TemplateNotFound(
            project.template_name().to_string(),
        ));
    }

    let home = std::env::var("HOME")
        .map_err(|_| ClaudeVmError::InvalidConfig("HOME not set".to_string()))?;
    let log_dir = PathBuf::from(home)
        .join(".lima")
        .join(project.template_name());

    if !log_dir.exists() {
        return Err(ClaudeVmError::InvalidConfig(format!(
            "Log directory not found: {}",
            log_dir.display()
        )));
    }

    // Find log files
    let serial_log = log_dir.join("serial.log");
    let ha_stdout_log = log_dir.join("ha.stdout.log");

    // Prefer ha.stdout.log if it exists, otherwise serial.log
    let log_file = if ha_stdout_log.exists() {
        ha_stdout_log
    } else if serial_log.exists() {
        serial_log
    } else {
        return Err(ClaudeVmError::InvalidConfig(format!(
            "No log files found in {}",
            log_dir.display()
        )));
    };

    println!("Viewing logs: {}", log_file.display());
    println!();

    // Use tail to view logs
    let mut cmd = Command::new("tail");
    if follow {
        cmd.arg("-f");
    } else {
        cmd.arg("-n").arg("100"); // Show last 100 lines
    }
    cmd.arg(&log_file);

    let status = cmd
        .status()
        .map_err(|e| ClaudeVmError::LimaExecution(format!("Failed to view logs: {}", e)))?;

    if !status.success() {
        return Err(ClaudeVmError::LimaExecution(
            "Failed to view logs".to_string(),
        ));
    }

    Ok(())
}
