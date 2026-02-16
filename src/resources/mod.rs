mod detection;
mod prompt;
mod validation;

pub use detection::{AllocatedResources, HostResources, VmResources};
pub use validation::ResourceCheck;

use crate::config::VmConfig;
use crate::error::Result;

/// Check if creating a new VM would exceed resource thresholds
///
/// This function should be called before creating a VM session.
/// It will detect host resources, query running VMs, check thresholds,
/// and handle user interaction based on the configured mode.
///
/// # Arguments
/// * `config` - VM configuration containing resource thresholds and check mode
/// * `force` - If true, bypass all resource checks (from --force-resources flag)
/// * `verbose` - If true, show warning messages when detection fails
///
/// # Returns
/// * `Ok(())` if resource check passed or user approved proceeding
/// * `Err` if thresholds exceeded and mode is Prevent, or user declined in Ask mode
pub fn check_before_vm_creation(config: &VmConfig, force: bool, verbose: bool) -> Result<()> {
    // Validate configuration
    config.validate()?;

    // Detect host resources
    let host = match HostResources::detect() {
        Ok(h) => h,
        Err(_) => {
            // If detection fails, just warn and continue
            // Don't block VM creation due to detection issues
            if verbose {
                eprintln!("Warning: Could not detect host resources");
            }
            return Ok(());
        }
    };

    // Query running VMs
    let allocated = match AllocatedResources::from_running_vms() {
        Ok(a) => a,
        Err(_) => {
            // If query fails, just warn and continue
            if verbose {
                eprintln!("Warning: Could not query running VMs");
            }
            return Ok(());
        }
    };

    // Check thresholds
    let check_result = validation::check_resources(config, &host, &allocated)?;

    // If no threshold exceeded, continue
    if !check_result.exceeds_threshold {
        return Ok(());
    }

    // Handle warning based on mode
    prompt::handle_resource_warning(
        &config.resource_check_mode,
        &check_result.warning_message,
        force,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ResourceCheckMode;

    #[test]
    fn test_check_before_vm_creation_with_force() {
        // With force flag, should always succeed regardless of thresholds
        let config = VmConfig {
            disk: 20,
            memory: 8,
            cpus: 4,
            cpu_threshold_percent: 1, // Very low threshold to ensure it would fail without force
            memory_threshold_percent: 1,
            resource_check_mode: ResourceCheckMode::Prevent,
        };

        let result = check_before_vm_creation(&config, true, false);
        // force=true should bypass all checks
        // May still fail if config validation fails, but not due to resource limits
        assert!(result.is_ok());
    }
}
