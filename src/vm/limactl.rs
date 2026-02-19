use crate::error::{ClaudeVmError, Result};
use crate::vm::mount::Mount;
use crate::vm::port_forward::PortForward;
use std::path::Path;
use std::process::{Command, Stdio};

/// Serialize a single [`Mount`] to a JSON object string.
///
/// Uses `serde_json` so that path values containing `"` or `\` are properly
/// escaped, preventing malformed JSON when paths have unusual characters.
fn mount_to_json(m: &Mount) -> String {
    let mut obj = serde_json::json!({
        "location": m.location.to_string_lossy().as_ref(),
        "writable": m.writable,
    });
    if let Some(ref mp) = m.mount_point {
        obj["mountPoint"] = serde_json::Value::String(mp.to_string_lossy().into_owned());
    }
    // serde_json serialization is infallible for these value types
    obj.to_string()
}

/// Build the `.mounts=[…]` yq/jq set-expression for a list of mounts.
fn mounts_set_expr(mounts: &[Mount]) -> String {
    if mounts.is_empty() {
        ".mounts=[]".to_string()
    } else {
        let items: Vec<String> = mounts.iter().map(mount_to_json).collect();
        format!(".mounts=[{}]", items.join(","))
    }
}

pub struct LimaCtl;

/// VM configuration based on the host operating system
struct VmConfig {
    vm_type: &'static str,
    mount_type: &'static str,
    use_rosetta: bool,
}

impl VmConfig {
    fn for_current_os() -> Self {
        #[cfg(target_os = "macos")]
        {
            // Check if Rosetta should be disabled
            // Disable in CI environments or if explicitly disabled via env var
            let is_ci = std::env::var("CI").is_ok()
                || std::env::var("GITHUB_ACTIONS").is_ok()
                || std::env::var("GITLAB_CI").is_ok()
                || std::env::var("CIRCLECI").is_ok();
            let disable_rosetta = std::env::var("CLAUDE_VM_DISABLE_ROSETTA").is_ok() || is_ci;

            Self {
                vm_type: "vz",
                mount_type: "virtiofs",
                use_rosetta: std::env::consts::ARCH == "aarch64" && !disable_rosetta,
            }
        }

        #[cfg(target_os = "linux")]
        {
            Self {
                vm_type: "qemu",
                mount_type: "reverse-sshfs",
                use_rosetta: false,
            }
        }

        #[cfg(target_os = "windows")]
        {
            Self {
                vm_type: "qemu",
                mount_type: "reverse-sshfs",
                use_rosetta: false,
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            Self {
                vm_type: "qemu",
                mount_type: "reverse-sshfs",
                use_rosetta: false,
            }
        }
    }
}

impl LimaCtl {
    /// Check if limactl is installed
    pub fn is_installed() -> bool {
        which::which("limactl").is_ok()
    }

    /// Create a new Lima VM from template
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        name: &str,
        template: &str,
        disk: u32,
        memory: u32,
        cpus: u32,
        port_forwards: &[PortForward],
        mounts: &[Mount],
        verbose: bool,
    ) -> Result<()> {
        let mut cmd = Command::new("limactl");

        // Format template with template: prefix if not already present
        let template_arg = if template.starts_with("template:") {
            template.to_string()
        } else {
            format!("template:{}", template)
        };

        let vm_config = VmConfig::for_current_os();

        cmd.arg("create")
            .arg(format!("--name={}", name))
            .arg(&template_arg)
            .arg(format!("--vm-type={}", vm_config.vm_type))
            .arg(format!("--mount-type={}", vm_config.mount_type))
            .arg("--tty=false");

        if vm_config.use_rosetta {
            cmd.arg("--rosetta");
        }

        // Build mounts JSON array (same format as clone)
        cmd.arg("--set").arg(mounts_set_expr(mounts));

        cmd.arg(format!("--disk={}", disk))
            .arg(format!("--memory={}", memory))
            .arg(format!("--cpus={}", cpus));

        // Add port forwards using --set flags
        for (index, port_forward) in port_forwards.iter().enumerate() {
            for (key, value) in port_forward.to_set_args(index) {
                cmd.arg("--set").arg(format!("{}={}", key, value));
            }
        }

        let result = if verbose {
            cmd.status()
        } else {
            cmd.stdout(Stdio::null()).stderr(Stdio::null()).status()
        };

        let status = result
            .map_err(|e| ClaudeVmError::LimaExecution(format!("Failed to create VM: {}", e)))?;

        if !status.success() {
            return Err(ClaudeVmError::LimaExecution(format!(
                "Failed to create VM {}",
                name
            )));
        }

        Ok(())
    }

    /// Start a Lima VM
    pub fn start(name: &str, verbose: bool) -> Result<()> {
        let mut cmd = Command::new("limactl");
        cmd.args(["start", name]);

        let result = if verbose {
            cmd.status()
        } else {
            cmd.stdout(Stdio::null()).stderr(Stdio::null()).status()
        };

        let status = result
            .map_err(|e| ClaudeVmError::LimaExecution(format!("Failed to start VM: {}", e)))?;

        if !status.success() {
            return Err(ClaudeVmError::LimaExecution(format!(
                "Failed to start VM {}",
                name
            )));
        }

        Ok(())
    }

    /// Stop a Lima VM
    pub fn stop(name: &str, verbose: bool) -> Result<()> {
        let mut cmd = Command::new("limactl");
        cmd.args(["stop", name]);

        let result = if verbose {
            cmd.status()
        } else {
            cmd.stdout(Stdio::null()).stderr(Stdio::null()).status()
        };

        let status = result
            .map_err(|e| ClaudeVmError::LimaExecution(format!("Failed to stop VM: {}", e)))?;

        if !status.success() {
            return Err(ClaudeVmError::LimaExecution(format!(
                "Failed to stop VM {}",
                name
            )));
        }

        Ok(())
    }

    /// Delete a Lima VM
    pub fn delete(name: &str, force: bool, verbose: bool) -> Result<()> {
        let mut args = vec!["delete"];
        if force {
            args.push("--force");
        }
        args.push(name);

        let mut cmd = Command::new("limactl");
        cmd.args(&args);

        let result = if verbose {
            cmd.status()
        } else {
            cmd.stdout(Stdio::null()).stderr(Stdio::null()).status()
        };

        let status = result
            .map_err(|e| ClaudeVmError::LimaExecution(format!("Failed to delete VM: {}", e)))?;

        if !status.success() {
            return Err(ClaudeVmError::LimaExecution(format!(
                "Failed to delete VM {}",
                name
            )));
        }

        Ok(())
    }

    /// Clone a Lima VM with additional mounts
    pub fn clone(source: &str, dest: &str, mounts: &[Mount], verbose: bool) -> Result<()> {
        // Try "clone" first (older Lima), then "copy" (newer Lima)
        // This ensures compatibility across Lima versions
        let result = Self::try_clone_command("clone", source, dest, mounts, verbose);

        if result.is_ok() {
            return result;
        }

        // If clone failed, try copy (Lima >= 0.17)
        Self::try_clone_command("copy", source, dest, mounts, verbose)
    }

    fn try_clone_command(
        command: &str,
        source: &str,
        dest: &str,
        mounts: &[Mount],
        verbose: bool,
    ) -> Result<()> {
        let mut cmd = Command::new("limactl");
        cmd.arg(command).arg(source).arg(dest).arg("--tty=false");

        // Add mount specification if mounts are provided
        if !mounts.is_empty() {
            cmd.arg("--set").arg(mounts_set_expr(mounts));
        }

        // Suppress output unless in verbose mode
        if !verbose {
            cmd.stdout(Stdio::null()).stderr(Stdio::null());
        }

        let status = cmd.status().map_err(|e| {
            ClaudeVmError::LimaExecution(format!("Failed to {} VM: {}", command, e))
        })?;

        if !status.success() {
            return Err(ClaudeVmError::LimaExecution(format!(
                "Failed to {} VM from {} to {}",
                command, source, dest
            )));
        }

        Ok(())
    }

    /// Execute a shell command in a Lima VM
    pub fn shell(
        name: &str,
        workdir: Option<&Path>,
        cmd: &str,
        args: &[&str],
        forward_ssh_agent: bool,
    ) -> Result<()> {
        let mut command = Command::new("limactl");
        command.arg("shell");

        // Add --workdir BEFORE the VM name (limactl syntax)
        if let Some(wd) = workdir {
            command.args(["--workdir", &wd.to_string_lossy()]);
        }

        // Add SSH agent forwarding if requested
        if forward_ssh_agent {
            command.arg("-A");
        }

        // Now add VM name and command
        command.arg(name);
        command.arg(cmd);
        command.args(args);

        let status = command
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|e| ClaudeVmError::LimaExecution(format!("Failed to execute shell: {}", e)))?;

        if !status.success() {
            // Return exit code if available, otherwise return generic error
            return Err(match status.code() {
                Some(code) => ClaudeVmError::CommandExitCode(code),
                None => ClaudeVmError::LimaExecution("Command terminated by signal".to_string()),
            });
        }

        Ok(())
    }

    /// Copy a file into a Lima VM
    pub fn copy(src: &Path, vm_name: &str, dest: &str) -> Result<()> {
        let dest_path = format!("{}:{}", vm_name, dest);
        let status = Command::new("limactl")
            .args(["copy", &src.to_string_lossy(), &dest_path])
            .status()
            .map_err(|e| ClaudeVmError::LimaExecution(format!("Failed to copy file: {}", e)))?;

        if !status.success() {
            return Err(ClaudeVmError::LimaExecution(
                "Failed to copy file".to_string(),
            ));
        }

        Ok(())
    }

    /// List all Lima VMs with their resource allocations
    pub fn list() -> Result<Vec<VmInfo>> {
        let output = Command::new("limactl")
            .args([
                "list",
                "--format",
                "{{.Name}}\t{{.Status}}\t{{.CPUs}}\t{{.Memory}}",
            ])
            .output()
            .map_err(|e| ClaudeVmError::LimaExecution(format!("Failed to list VMs: {}", e)))?;

        if !output.status.success() {
            return Err(ClaudeVmError::LimaExecution(
                "Failed to list VMs".to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let vms = stdout
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 2 {
                    let cpus = if parts.len() >= 3 {
                        parts[2].trim().parse::<u32>().ok()
                    } else {
                        None
                    };

                    let memory_gb = if parts.len() >= 4 {
                        parse_memory_string(parts[3].trim())
                    } else {
                        None
                    };

                    Some(VmInfo {
                        name: parts[0].to_string(),
                        status: parts[1].to_string(),
                        cpus,
                        memory_gb,
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(vms)
    }

    /// Check if a VM exists
    pub fn vm_exists(name: &str) -> Result<bool> {
        let vms = Self::list()?;
        Ok(vms.iter().any(|vm| vm.name == name))
    }
}

/// Parse Lima memory format (e.g., "8GiB", "8G", "2048MiB", or raw bytes) to GB
fn parse_memory_string(s: &str) -> Option<u32> {
    // Remove quotes if present
    let s = s.trim_matches('"');

    // Extract number and unit
    let (num_str, unit) = s
        .chars()
        .position(|c| c.is_alphabetic())
        .map(|pos| s.split_at(pos))
        .unwrap_or((s, ""));

    let num = num_str.parse::<f64>().ok()?;

    let gb = match unit.to_lowercase().as_str() {
        "gib" | "g" => num,
        "mib" | "m" => num / 1024.0,
        "kib" | "k" => num / (1024.0 * 1024.0),
        "" => {
            // If no unit and the number is very large (> 1000), assume it's in bytes
            // This handles output from limactl list --format "{{.Memory}}"
            if num > 1000.0 {
                num / (1024.0 * 1024.0 * 1024.0) // Convert bytes to GB
            } else {
                num // Small numbers without units are assumed to be GB
            }
        }
        _ => return None,
    };

    Some(gb.ceil() as u32)
}

#[derive(Debug, Clone)]
pub struct VmInfo {
    pub name: String,
    pub status: String,
    pub cpus: Option<u32>,
    pub memory_gb: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_config_for_current_os() {
        let config = VmConfig::for_current_os();

        // vm_type must be a valid Lima VM type
        assert!(
            ["qemu", "vz", "wsl2"].contains(&config.vm_type),
            "vm_type '{}' is not a valid Lima VM type",
            config.vm_type
        );

        // mount_type must be a valid Lima mount type
        assert!(
            ["reverse-sshfs", "9p", "virtiofs"].contains(&config.mount_type),
            "mount_type '{}' is not a valid Lima mount type",
            config.mount_type
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_vm_config_linux() {
        let config = VmConfig::for_current_os();

        assert_eq!(config.vm_type, "qemu");
        assert_eq!(config.mount_type, "reverse-sshfs");
        assert!(!config.use_rosetta);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_vm_config_macos() {
        let config = VmConfig::for_current_os();

        assert_eq!(config.vm_type, "vz");
        assert_eq!(config.mount_type, "virtiofs");

        // Rosetta is only enabled on ARM64 macOS when not in CI
        let is_ci = std::env::var("CI").is_ok()
            || std::env::var("GITHUB_ACTIONS").is_ok()
            || std::env::var("GITLAB_CI").is_ok()
            || std::env::var("CIRCLECI").is_ok();
        let disable_rosetta = std::env::var("CLAUDE_VM_DISABLE_ROSETTA").is_ok() || is_ci;

        if std::env::consts::ARCH == "aarch64" && !disable_rosetta {
            assert!(config.use_rosetta);
        } else {
            assert!(!config.use_rosetta);
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_vm_config_windows() {
        let config = VmConfig::for_current_os();

        assert_eq!(config.vm_type, "qemu");
        assert_eq!(config.mount_type, "reverse-sshfs");
        assert!(!config.use_rosetta);
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn test_vm_config_no_rosetta_on_non_macos() {
        let config = VmConfig::for_current_os();
        assert!(
            !config.use_rosetta,
            "Rosetta should only be enabled on macOS"
        );
    }

    #[test]
    fn test_parse_memory_string() {
        // Formatted strings (GiB, MiB, etc.)
        assert_eq!(parse_memory_string("8GiB"), Some(8));
        assert_eq!(parse_memory_string("8G"), Some(8));
        assert_eq!(parse_memory_string("16GiB"), Some(16));
        assert_eq!(parse_memory_string("1024MiB"), Some(1));
        assert_eq!(parse_memory_string("2048MiB"), Some(2));
        assert_eq!(parse_memory_string("512MiB"), Some(1)); // Rounds up
        assert_eq!(parse_memory_string("\"8GiB\""), Some(8)); // Handles quotes
        assert_eq!(parse_memory_string("1048576KiB"), Some(1)); // KiB to GB

        // Raw byte values (from limactl list --format "{{.Memory}}")
        assert_eq!(parse_memory_string("8589934592"), Some(8)); // 8 GB in bytes
        assert_eq!(parse_memory_string("17179869184"), Some(16)); // 16 GB in bytes
        assert_eq!(parse_memory_string("4294967296"), Some(4)); // 4 GB in bytes

        // Small numbers without units (assumed to be GB)
        assert_eq!(parse_memory_string("8"), Some(8)); // No unit, small number = GB
        assert_eq!(parse_memory_string("16"), Some(16));

        // Invalid formats
        assert_eq!(parse_memory_string("invalid"), None); // Invalid format
        assert_eq!(parse_memory_string("8TiB"), None); // Unsupported unit
    }
}
