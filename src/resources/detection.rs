use crate::error::{ClaudeVmError, Result};
use std::process::Command;

/// Default CPU allocation assumed when unable to read VM config
const DEFAULT_VM_CPUS: u32 = 4;

/// Default memory allocation (GB) assumed when unable to read VM config
const DEFAULT_VM_MEMORY_GB: u32 = 8;

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

        // Convert bytes to GB (round up to avoid underestimating)
        let total_memory_gb = (mem_bytes as f64 / (1024.0 * 1024.0 * 1024.0)).ceil() as u32;

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

        // Convert kB to GB (round up to avoid underestimating)
        let total_memory_gb = (mem_kb as f64 / (1024.0 * 1024.0)).ceil() as u32;

        Ok(Self {
            total_cpus,
            total_memory_gb,
        })
    }
}

impl AllocatedResources {
    /// Query all running claude-vm VMs and sum their allocated resources
    pub fn from_running_vms() -> Result<Self> {
        // Get all VMs with resource information from limactl
        let vms = crate::vm::limactl::LimaCtl::list()?;

        // Filter for running claude-vm VMs
        let running_vms: Vec<_> = vms
            .into_iter()
            .filter(|vm| {
                vm.status == "Running"
                    && (vm.name.starts_with("claude-tpl_") || vm.name.starts_with("claude-vm-"))
            })
            .collect();

        let mut total_cpus = 0;
        let mut total_memory_gb = 0;

        for vm in &running_vms {
            // Use provided values or conservative defaults
            // Defaults are security-critical to avoid underestimating resource usage
            total_cpus += vm.cpus.unwrap_or(DEFAULT_VM_CPUS);
            total_memory_gb += vm.memory_gb.unwrap_or(DEFAULT_VM_MEMORY_GB);
        }

        Ok(Self {
            total_cpus,
            total_memory_gb,
            vm_count: running_vms.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_resources_detect() {
        // This test just verifies detection doesn't panic
        // Actual values depend on the system
        if let Ok(resources) = HostResources::detect() {
            assert!(resources.total_cpus > 0);
            assert!(resources.total_memory_gb > 0);
        }
    }

    #[test]
    fn test_allocated_resources_from_running_vms() {
        // This test verifies the function doesn't panic
        // It may return an error if limactl is not available
        let result = AllocatedResources::from_running_vms();
        // Should either succeed or fail gracefully
        assert!(result.is_ok() || result.is_err());
    }
}
