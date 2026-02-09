use crate::error::{ClaudeVmError, Result};
use crate::vm::mount::Mount;
use crate::vm::port_forward::PortForward;
use std::path::Path;
use std::process::{Command, Stdio};

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
            Self {
                vm_type: "vz",
                mount_type: "virtiofs",
                use_rosetta: std::env::consts::ARCH == "aarch64",
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
    pub fn create(
        name: &str,
        template: &str,
        disk: u32,
        memory: u32,
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
        if !mounts.is_empty() {
            let mount_jsons: Vec<String> = mounts
                .iter()
                .map(|m| {
                    if let Some(ref mount_point) = m.mount_point {
                        format!(
                            "{{\"location\":\"{}\",\"mountPoint\":\"{}\",\"writable\":{}}}",
                            m.location.display(),
                            mount_point.display(),
                            m.writable
                        )
                    } else {
                        format!(
                            "{{\"location\":\"{}\",\"writable\":{}}}",
                            m.location.display(),
                            m.writable
                        )
                    }
                })
                .collect();
            cmd.arg("--set")
                .arg(format!(".mounts=[{}]", mount_jsons.join(",")));
        } else {
            cmd.arg("--set").arg(".mounts=[]");
        }

        cmd.arg(format!("--disk={}", disk))
            .arg(format!("--memory={}", memory));

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
        // Build mounts JSON array (matches bash format)
        let mounts_array = if !mounts.is_empty() {
            let mount_jsons: Vec<String> = mounts
                .iter()
                .map(|m| {
                    if let Some(ref mount_point) = m.mount_point {
                        format!(
                            "{{\"location\":\"{}\",\"mountPoint\":\"{}\",\"writable\":{}}}",
                            m.location.display(),
                            mount_point.display(),
                            m.writable
                        )
                    } else {
                        format!(
                            "{{\"location\":\"{}\",\"writable\":{}}}",
                            m.location.display(),
                            m.writable
                        )
                    }
                })
                .collect();

            Some(format!(".mounts=[{}]", mount_jsons.join(",")))
        } else {
            None
        };

        let mut cmd = Command::new("limactl");
        cmd.arg(command).arg(source).arg(dest).arg("--tty=false");

        // Add mount specification if present
        if let Some(ref mounts_spec) = mounts_array {
            cmd.arg("--set").arg(mounts_spec);
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

    /// List all Lima VMs
    pub fn list() -> Result<Vec<VmInfo>> {
        let output = Command::new("limactl")
            .args(["list", "--format", "{{.Name}}\t{{.Status}}"])
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
                    Some(VmInfo {
                        name: parts[0].to_string(),
                        status: parts[1].to_string(),
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

#[derive(Debug)]
pub struct VmInfo {
    pub name: String,
    pub status: String,
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

        if std::env::consts::ARCH == "aarch64" {
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
}
