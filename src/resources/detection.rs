use crate::error::{ClaudeVmError, Result};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct HostResources {
    pub total_cpus: u32,
    pub total_memory_gb: u32,
}

#[derive(Debug, Clone)]
pub struct AllocatedResources {
    pub total_cpus: u32,
    pub total_memory_gb: u32,
    pub vm_count: usize,
}

#[derive(Debug, Clone)]
pub struct VmResources {
    pub name: String,
    pub cpus: u32,
    pub memory_gb: u32,
}

impl HostResources {
    /// Detect host system resources (CPUs and memory)
    pub fn detect() -> Result<Self> {
        let os = std::env::consts::OS;

        match os {
            "macos" => Self::detect_macos(),
            "linux" => Self::detect_linux(),
            _ => Err(ClaudeVmError::CommandFailed(format!(
                "Resource detection not supported on OS: {}",
                os
            ))),
        }
    }

    fn detect_macos() -> Result<Self> {
        // Get CPU count
        let output = Command::new("sysctl")
            .args(["-n", "hw.ncpu"])
            .output()
            .map_err(|e| ClaudeVmError::CommandFailed(format!("Failed to detect CPUs: {}", e)))?;

        let cpus_str = String::from_utf8_lossy(&output.stdout);
        let total_cpus = cpus_str.trim().parse::<u32>().map_err(|e| {
            ClaudeVmError::CommandFailed(format!("Failed to parse CPU count: {}", e))
        })?;

        // Get memory in bytes
        let output = Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
            .map_err(|e| ClaudeVmError::CommandFailed(format!("Failed to detect memory: {}", e)))?;

        let mem_str = String::from_utf8_lossy(&output.stdout);
        let mem_bytes = mem_str.trim().parse::<u64>().map_err(|e| {
            ClaudeVmError::CommandFailed(format!("Failed to parse memory size: {}", e))
        })?;

        // Convert bytes to GB
        let total_memory_gb = (mem_bytes / (1024 * 1024 * 1024)) as u32;

        Ok(Self {
            total_cpus,
            total_memory_gb,
        })
    }

    fn detect_linux() -> Result<Self> {
        // Get CPU count from /proc/cpuinfo
        let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").map_err(|e| {
            ClaudeVmError::CommandFailed(format!("Failed to read /proc/cpuinfo: {}", e))
        })?;

        let total_cpus = cpuinfo
            .lines()
            .filter(|line| line.starts_with("processor"))
            .count() as u32;

        if total_cpus == 0 {
            return Err(ClaudeVmError::CommandFailed(
                "Failed to detect CPU count from /proc/cpuinfo".to_string(),
            ));
        }

        // Get memory from /proc/meminfo (in kB)
        let meminfo = std::fs::read_to_string("/proc/meminfo").map_err(|e| {
            ClaudeVmError::CommandFailed(format!("Failed to read /proc/meminfo: {}", e))
        })?;

        let mem_kb = meminfo
            .lines()
            .find(|line| line.starts_with("MemTotal:"))
            .and_then(|line| {
                line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse::<u64>().ok())
            })
            .ok_or_else(|| {
                ClaudeVmError::CommandFailed(
                    "Failed to parse MemTotal from /proc/meminfo".to_string(),
                )
            })?;

        // Convert kB to GB
        let total_memory_gb = (mem_kb / (1024 * 1024)) as u32;

        Ok(Self {
            total_cpus,
            total_memory_gb,
        })
    }
}

impl AllocatedResources {
    /// Query all running claude-vm VMs and sum their allocated resources
    pub fn from_running_vms() -> Result<Self> {
        let vms = Self::query_running_vms()?;

        let total_cpus = vms.iter().map(|vm| vm.cpus).sum();
        let total_memory_gb = vms.iter().map(|vm| vm.memory_gb).sum();
        let vm_count = vms.len();

        Ok(Self {
            total_cpus,
            total_memory_gb,
            vm_count,
        })
    }

    /// Query all running claude-vm VMs
    fn query_running_vms() -> Result<Vec<VmResources>> {
        // Use limactl list to get running VMs
        let vms = crate::vm::limactl::LimaCtl::list()?;

        // Filter for running claude-vm VMs
        let running_vms: Vec<_> = vms
            .into_iter()
            .filter(|vm| {
                vm.status == "Running"
                    && (vm.name.starts_with("claude-tpl_") || vm.name.starts_with("claude-vm-"))
            })
            .collect();

        // Query resources for each VM
        let mut vm_resources = Vec::new();
        for vm in running_vms {
            if let Ok(resources) = Self::read_vm_resources(&vm.name) {
                vm_resources.push(resources);
            }
        }

        Ok(vm_resources)
    }

    /// Read resource allocation from a VM's Lima config file
    fn read_vm_resources(vm_name: &str) -> Result<VmResources> {
        // Lima config is at ~/.lima/<vm-name>/lima.yaml
        let home = std::env::var("HOME").map_err(|_| {
            ClaudeVmError::CommandFailed("Failed to get HOME directory".to_string())
        })?;

        let config_path = format!("{}/.lima/{}/lima.yaml", home, vm_name);
        let config_content = std::fs::read_to_string(&config_path).map_err(|e| {
            ClaudeVmError::CommandFailed(format!(
                "Failed to read Lima config for {}: {}",
                vm_name, e
            ))
        })?;

        // Parse YAML for cpus and memory
        // Look for lines like "cpus: 4" and "memory: 8GiB"
        let mut cpus = 4; // Default
        let mut memory_gb = 8; // Default

        for line in config_content.lines() {
            let line = line.trim();
            if line.starts_with("cpus:") {
                if let Some(cpu_str) = line.split(':').nth(1) {
                    cpus = cpu_str.trim().parse::<u32>().unwrap_or(4);
                }
            } else if line.starts_with("memory:") {
                if let Some(mem_str) = line.split(':').nth(1) {
                    memory_gb = Self::parse_memory_size(mem_str.trim());
                }
            }
        }

        Ok(VmResources {
            name: vm_name.to_string(),
            cpus,
            memory_gb,
        })
    }

    /// Parse memory size string (e.g., "8GiB", "8G", "8192MiB") to GB
    fn parse_memory_size(size_str: &str) -> u32 {
        let size_str = size_str.trim_matches('"');

        // Extract number and unit
        let (num_str, unit) = size_str
            .chars()
            .position(|c| c.is_alphabetic())
            .map(|pos| size_str.split_at(pos))
            .unwrap_or((size_str, ""));

        let num = num_str.parse::<f64>().unwrap_or(8.0);

        match unit.to_lowercase().as_str() {
            "gib" | "g" => num as u32,
            "mib" | "m" => (num / 1024.0).ceil() as u32,
            "kib" | "k" => (num / (1024.0 * 1024.0)).ceil() as u32,
            _ => num as u32, // Assume GB if no unit
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_memory_size() {
        assert_eq!(AllocatedResources::parse_memory_size("8GiB"), 8);
        assert_eq!(AllocatedResources::parse_memory_size("8G"), 8);
        assert_eq!(AllocatedResources::parse_memory_size("16GiB"), 16);
        assert_eq!(AllocatedResources::parse_memory_size("1024MiB"), 1);
        assert_eq!(AllocatedResources::parse_memory_size("2048MiB"), 2);
        assert_eq!(AllocatedResources::parse_memory_size("512MiB"), 1); // Rounds up
        assert_eq!(AllocatedResources::parse_memory_size("\"8GiB\""), 8);
    }

    #[test]
    fn test_host_resources_detect() {
        // This test just verifies detection doesn't panic
        // Actual values depend on the system
        let result = HostResources::detect();
        if result.is_ok() {
            let resources = result.unwrap();
            assert!(resources.total_cpus > 0);
            assert!(resources.total_memory_gb > 0);
        }
    }
}
