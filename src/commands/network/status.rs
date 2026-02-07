use crate::config::Config;
use crate::error::{ClaudeVmError, Result};
use crate::project::Project;
use std::process::Command;

pub fn execute(project: &Project, config: &Config) -> Result<()> {
    let instance_name = project.template_name();

    println!("Network Security Status");
    println!("═══════════════════════════════════════════════");
    println!();

    // Check if network security is enabled in config
    if !config.security.network.enabled {
        println!("Status: DISABLED");
        println!();
        println!("Network security is not enabled for this project.");
        println!();
        println!("To enable network security:");
        println!("  1. Add to .claude-vm.toml:");
        println!("     [security.network]");
        println!("     enabled = true");
        println!("  2. Recreate the VM:");
        println!("     claude-vm clean && claude-vm setup");
        println!();
        println!("Or use the CLI shortcut:");
        println!("  claude-vm setup --network-security");
        return Ok(());
    }

    // Check if VM is running
    let status_output = Command::new("limactl")
        .args(["list", "--format", "{{.Status}}", instance_name])
        .output()
        .map_err(|e| ClaudeVmError::CommandFailed(format!("Failed to check VM status: {}", e)))?;

    let vm_status = String::from_utf8_lossy(&status_output.stdout)
        .trim()
        .to_string();

    if vm_status != "Running" {
        println!("Status: INACTIVE (VM not running)");
        println!();
        println!("VM Status: {}", vm_status);
        println!();
        println!("Start the VM first with:");
        println!("  claude-vm shell");
        return Ok(());
    }

    // Check if proxy process is running
    let check_pid = Command::new("limactl")
        .args(["shell", instance_name, "test", "-f", "/tmp/mitmproxy.pid"])
        .output()
        .map_err(|e| {
            ClaudeVmError::CommandFailed(format!("Failed to check proxy status: {}", e))
        })?;

    if !check_pid.status.success() {
        println!("Status: INACTIVE (Proxy not started)");
        println!();
        println!("The network security proxy has not been started yet.");
        println!("It will start automatically when you run:");
        println!("  claude-vm        # Run Claude");
        println!("  claude-vm shell  # Open shell");
        return Ok(());
    }

    // Read proxy PID
    let pid_output = Command::new("limactl")
        .args(["shell", instance_name, "cat", "/tmp/mitmproxy.pid"])
        .output()
        .map_err(|e| ClaudeVmError::CommandFailed(format!("Failed to read proxy PID: {}", e)))?;

    let proxy_pid = String::from_utf8_lossy(&pid_output.stdout)
        .trim()
        .to_string();

    // Check if proxy process is actually running
    let check_running = Command::new("limactl")
        .args(["shell", instance_name, "kill", "-0", &proxy_pid])
        .output()
        .map_err(|e| {
            ClaudeVmError::CommandFailed(format!("Failed to check proxy process: {}", e))
        })?;

    if !check_running.status.success() {
        println!("Status: INACTIVE (Proxy stopped)");
        println!();
        println!("The proxy process (PID: {}) is not running.", proxy_pid);
        println!("It may have crashed or been stopped.");
        println!();
        println!("Check logs: claude-vm network logs");
        return Ok(());
    }

    // Proxy is running - show active status
    println!("Status: ACTIVE ✓");
    println!();

    // Proxy process info
    println!("Proxy Process:");
    println!("  PID: {}", proxy_pid);
    println!("  Listening: localhost:8080");

    // Get uptime if available
    let uptime_output = Command::new("limactl")
        .args([
            "shell",
            instance_name,
            "ps",
            "-p",
            &proxy_pid,
            "-o",
            "etime=",
        ])
        .output();

    if let Ok(output) = uptime_output {
        if output.status.success() {
            let uptime = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !uptime.is_empty() {
                println!("  Uptime: {}", uptime);
            }
        }
    }

    println!();

    // Policy configuration
    println!("Policy Configuration:");
    println!("  Mode: {}", config.security.network.mode.as_str());

    let allowed_count = config.security.network.allowed_domains.len();
    let blocked_count = config.security.network.blocked_domains.len();
    let bypass_count = config.security.network.bypass_domains.len();

    println!(
        "  Allowed domains: {} pattern{}",
        allowed_count,
        if allowed_count != 1 { "s" } else { "" }
    );
    println!(
        "  Blocked domains: {} pattern{}",
        blocked_count,
        if blocked_count != 1 { "s" } else { "" }
    );
    println!(
        "  Bypass domains: {} pattern{}",
        bypass_count,
        if bypass_count != 1 { "s" } else { "" }
    );

    println!();

    // Protocol blocks
    println!("Protocol Blocks:");
    println!(
        "  Raw TCP/UDP: {}",
        if config.security.network.block_tcp_udp {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!(
        "  Private networks: {}",
        if config.security.network.block_private_networks {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!(
        "  Cloud metadata: {}",
        if config.security.network.block_metadata_services {
            "enabled"
        } else {
            "disabled"
        }
    );

    println!();

    // Try to read statistics if available
    let stats_output = Command::new("limactl")
        .args(["shell", instance_name, "cat", "/tmp/mitmproxy_stats.json"])
        .output();

    if let Ok(output) = stats_output {
        if output.status.success() {
            let stats_json = String::from_utf8_lossy(&output.stdout);
            if let Ok(stats) = serde_json::from_str::<serde_json::Value>(&stats_json) {
                println!("Statistics:");
                if let Some(total) = stats["requests_total"].as_u64() {
                    println!("  Requests seen: {}", total);
                }
                if let Some(allowed) = stats["requests_allowed"].as_u64() {
                    println!("  Requests allowed: {}", allowed);
                }
                if let Some(blocked) = stats["requests_blocked"].as_u64() {
                    println!("  Requests blocked: {}", blocked);
                }
                println!();
            }
        }
    }

    println!("View logs: claude-vm network logs");

    Ok(())
}
