use crate::error::{ClaudeVmError, Result};
use crate::project::Project;
use crate::utils::shell::escape as shell_escape;
use std::process::Command;

pub fn execute(
    project: &Project,
    lines: usize,
    filter: Option<&str>,
    all: bool,
    follow: bool,
) -> Result<()> {
    // Find running ephemeral VMs
    let running_vms = super::find_running_vms(project)?;

    if running_vms.is_empty() {
        eprintln!("No ephemeral VMs are currently running for this project.");
        eprintln!("Network security logs are only available while a VM is running.");
        eprintln!();
        eprintln!("Start a VM with:");
        eprintln!("  claude-vm        # Run Claude");
        eprintln!("  claude-vm shell  # Open shell");
        return Err(ClaudeVmError::CommandFailed("No running VMs".to_string()));
    }

    // Select VM (prompts user if multiple)
    let instance_name = super::select_vm(&running_vms)?;

    // Check if network security is enabled by checking if the log file exists
    let check_log = Command::new("limactl")
        .args(["shell", &instance_name, "test", "-f", "/tmp/mitmproxy.log"])
        .output()
        .map_err(|e| ClaudeVmError::CommandFailed(format!("Failed to check log file: {}", e)))?;

    if !check_log.status.success() {
        eprintln!("Network security logs not found.");
        eprintln!();
        eprintln!("Network security may not be enabled for this VM.");
        eprintln!("To enable network security:");
        eprintln!("  1. Add to .claude-vm.toml:");
        eprintln!("     [security.network]");
        eprintln!("     enabled = true");
        eprintln!("  2. Recreate the VM: claude-vm clean && claude-vm setup");
        return Ok(());
    }

    // Build the command to read logs
    let read_cmd = if follow {
        // Follow mode: use tail -f for real-time streaming
        let mut cmd = "tail -f /tmp/mitmproxy.log".to_string();

        // Add grep filter if specified
        if let Some(pattern) = filter {
            cmd.push_str(&format!(
                " | grep --line-buffered -i {}",
                shell_escape(pattern)
            ));
        }

        cmd
    } else {
        // Static read mode
        let mut cmd = String::new();

        if let Some(pattern) = filter {
            // Use grep to filter (pattern is shell-escaped to prevent injection)
            cmd.push_str(&format!(
                "grep -i {} /tmp/mitmproxy.log",
                shell_escape(pattern)
            ));
        } else {
            cmd.push_str("cat /tmp/mitmproxy.log");
        }

        // Apply line limit
        if !all {
            cmd.push_str(&format!(" | tail -n {}", lines));
        }

        cmd
    };

    // Execute the command
    if follow {
        // Follow mode: stream output in real-time
        println!("Network Security Logs (following)");
        println!("═════════════════════════════════════════════════════════════");
        println!("VM: {}", instance_name);
        if let Some(pattern) = filter {
            println!("Filter: {}", pattern);
        }
        println!("Press Ctrl+C to stop following");
        println!("═════════════════════════════════════════════════════════════");
        println!();

        let status = Command::new("limactl")
            .args(["shell", &instance_name, "sh", "-c", &read_cmd])
            .status()
            .map_err(|e| ClaudeVmError::CommandFailed(format!("Failed to follow logs: {}", e)))?;

        if !status.success() {
            return Err(ClaudeVmError::CommandFailed(
                "Log streaming terminated with error".to_string(),
            ));
        }
    } else {
        // Static mode: read all at once
        let output = Command::new("limactl")
            .args(["shell", &instance_name, "sh", "-c", &read_cmd])
            .output()
            .map_err(|e| ClaudeVmError::CommandFailed(format!("Failed to read logs: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ClaudeVmError::CommandFailed(format!(
                "Failed to read logs: {}",
                stderr
            )));
        }

        let logs = String::from_utf8_lossy(&output.stdout);

        if logs.trim().is_empty() {
            if let Some(pattern) = filter {
                println!("No logs matching filter: {}", pattern);
            } else {
                println!("No logs available yet.");
                println!();
                println!("Network security is enabled but no requests have been logged.");
                println!(
                    "The proxy may still be starting up, or no network requests have been made."
                );
            }
        } else {
            // Print header
            println!("Network Security Logs");
            println!("═════════════════════════════════════════════════════════════");
            println!("VM: {}", instance_name);
            if let Some(pattern) = filter {
                println!("Filter: {}", pattern);
            }
            if !all {
                println!("Showing last {} lines", lines);
            }
            println!("═════════════════════════════════════════════════════════════");
            println!();

            // Print logs
            print!("{}", logs);

            // Print footer with usage info
            println!();
            println!("═════════════════════════════════════════════════════════════");
            println!("Options:");
            println!("  --all          Show all logs (no line limit)");
            println!("  -n <lines>     Show last N lines (default: 50)");
            println!("  -f <pattern>   Filter logs by domain pattern");
            println!("  --follow       Follow log output in real-time");
            println!();
            println!("Log file: /tmp/mitmproxy.log (inside VM)");
        }
    }

    Ok(())
}
