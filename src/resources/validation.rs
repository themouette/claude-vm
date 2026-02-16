use crate::config::VmConfig;
use crate::error::Result;
use crate::resources::detection::{AllocatedResources, HostResources};

#[derive(Debug)]
pub struct ResourceCheck {
    pub exceeds_threshold: bool,
    pub warning_message: String,
}

/// Check if creating a new VM would exceed resource thresholds
pub fn check_resources(
    config: &VmConfig,
    host: &HostResources,
    allocated: &AllocatedResources,
) -> Result<ResourceCheck> {
    // Calculate what the new total would be after adding this VM (with overflow protection)
    let new_total_cpus = allocated
        .total_cpus
        .checked_add(config.cpus)
        .ok_or_else(|| {
            crate::error::ClaudeVmError::CommandFailed(
                "CPU count overflow: too many VMs allocated".to_string(),
            )
        })?;

    let new_total_memory = allocated
        .total_memory_gb
        .checked_add(config.memory)
        .ok_or_else(|| {
            crate::error::ClaudeVmError::CommandFailed(
                "Memory overflow: too many VMs allocated".to_string(),
            )
        })?;

    // Calculate thresholds
    let cpu_threshold =
        (host.total_cpus as f64 * config.cpu_threshold_percent as f64 / 100.0).ceil() as u32;
    let memory_threshold = (host.total_memory_gb as f64 * config.memory_threshold_percent as f64
        / 100.0)
        .ceil() as u32;

    // Check if exceeded
    let cpu_exceeded = new_total_cpus > cpu_threshold;
    let memory_exceeded = new_total_memory > memory_threshold;

    if !cpu_exceeded && !memory_exceeded {
        return Ok(ResourceCheck {
            exceeds_threshold: false,
            warning_message: String::new(),
        });
    }

    // Build detailed warning message
    let cpu_warning = if cpu_exceeded {
        format!(
            " (exceeds {}% threshold of {})",
            config.cpu_threshold_percent, cpu_threshold
        )
    } else {
        String::new()
    };

    let memory_warning = if memory_exceeded {
        format!(
            " (exceeds {}% threshold of {})",
            config.memory_threshold_percent, memory_threshold
        )
    } else {
        String::new()
    };

    let stability_warning = if cpu_exceeded && new_total_cpus >= host.total_cpus {
        "⚠️  WARNING: All CPU cores will be allocated!\n   This can cause system instability and forced reboots.\n\n"
    } else {
        ""
    };

    let message = format!(
        "⚠️  Resource Overprovisioning Warning\n\n\
         Host System:\n\
           CPUs:   {} cores\n\
           Memory: {} GB\n\n\
         Currently Allocated ({} running VMs):\n\
           CPUs:   {} cores\n\
           Memory: {} GB\n\n\
         After Creating This VM:\n\
           CPUs:   {} cores{}\n\
           Memory: {} GB{}\n\n\
         {}",
        host.total_cpus,
        host.total_memory_gb,
        allocated.vm_count,
        allocated.total_cpus,
        allocated.total_memory_gb,
        new_total_cpus,
        cpu_warning,
        new_total_memory,
        memory_warning,
        stability_warning
    );

    Ok(ResourceCheck {
        exceeds_threshold: true,
        warning_message: message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ResourceCheckMode;

    fn make_test_config() -> VmConfig {
        VmConfig {
            disk: 20,
            memory: 8,
            cpus: 4,
            cpu_threshold_percent: 75,
            memory_threshold_percent: 70,
            resource_check_mode: ResourceCheckMode::Ask,
        }
    }

    #[test]
    fn test_within_limits() {
        let config = make_test_config();
        let host = HostResources {
            total_cpus: 16,
            total_memory_gb: 64,
        };
        let allocated = AllocatedResources {
            total_cpus: 0,
            total_memory_gb: 0,
            vm_count: 0,
        };

        let result = check_resources(&config, &host, &allocated).unwrap();
        assert!(!result.exceeds_threshold);
        assert!(result.warning_message.is_empty());
    }

    #[test]
    fn test_exceeds_cpu_threshold() {
        let config = make_test_config();
        let host = HostResources {
            total_cpus: 16,
            total_memory_gb: 64,
        };
        // 75% of 16 = 12, so 9 + 4 = 13 > 12
        let allocated = AllocatedResources {
            total_cpus: 9,
            total_memory_gb: 16,
            vm_count: 2,
        };

        let result = check_resources(&config, &host, &allocated).unwrap();
        assert!(result.exceeds_threshold);
        assert!(result.warning_message.contains("Resource Overprovisioning"));
        assert!(result.warning_message.contains("exceeds 75% threshold"));
    }

    #[test]
    fn test_exceeds_memory_threshold() {
        let config = make_test_config();
        let host = HostResources {
            total_cpus: 16,
            total_memory_gb: 64,
        };
        // 70% of 64 = 44.8 -> 45, so 38 + 8 = 46 > 45
        let allocated = AllocatedResources {
            total_cpus: 4,
            total_memory_gb: 38,
            vm_count: 4,
        };

        let result = check_resources(&config, &host, &allocated).unwrap();
        assert!(result.exceeds_threshold);
        assert!(result.warning_message.contains("Resource Overprovisioning"));
        assert!(result.warning_message.contains("exceeds 70% threshold"));
    }

    #[test]
    fn test_all_cpus_allocated() {
        let config = make_test_config();
        let host = HostResources {
            total_cpus: 16,
            total_memory_gb: 64,
        };
        // 12 + 4 = 16 (all CPUs)
        let allocated = AllocatedResources {
            total_cpus: 12,
            total_memory_gb: 16,
            vm_count: 3,
        };

        let result = check_resources(&config, &host, &allocated).unwrap();
        assert!(result.exceeds_threshold);
        assert!(result
            .warning_message
            .contains("All CPU cores will be allocated"));
        assert!(result
            .warning_message
            .contains("system instability and forced reboots"));
    }

    #[test]
    fn test_exactly_at_threshold() {
        let config = make_test_config();
        let host = HostResources {
            total_cpus: 16,
            total_memory_gb: 64,
        };
        // 75% of 16 = 12, so 8 + 4 = 12 (exactly at threshold, not over)
        let allocated = AllocatedResources {
            total_cpus: 8,
            total_memory_gb: 16,
            vm_count: 2,
        };

        let result = check_resources(&config, &host, &allocated).unwrap();
        assert!(!result.exceeds_threshold);
    }
}
