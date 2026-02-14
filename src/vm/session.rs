use crate::error::Result;
use crate::project::Project;
use crate::vm::{limactl::LimaCtl, mount};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Represents an ephemeral VM session with RAII cleanup
pub struct VmSession {
    name: String,
    cleaned_up: Arc<AtomicBool>,
    verbose: bool,
}

impl VmSession {
    /// Create a new VM session by cloning the template.
    ///
    /// This function ensures cleanup even if VM creation fails partway through:
    /// - If clone fails: No cleanup needed (VM doesn't exist)
    /// - If start fails: VM is deleted automatically
    /// - If successful: Cleanup guard is registered for later cleanup
    pub fn new(
        project: &Project,
        verbose: bool,
        mount_conversations: bool,
        custom_mounts: &[crate::config::MountEntry],
    ) -> Result<Self> {
        let name = format!("{}-{}", project.template_name(), std::process::id());

        // Compute mounts for worktree support, conversation folder, and custom mounts
        let mounts = mount::compute_mounts(mount_conversations, custom_mounts)?;

        // Clone the template with additional mounts
        // If this fails, no cleanup needed (VM doesn't exist yet)
        LimaCtl::clone(project.template_name(), &name, &mounts, verbose)?;

        // Start the VM
        // If this fails, we must clean up the cloned VM to prevent leaks
        if let Err(e) = LimaCtl::start(&name, verbose) {
            eprintln!("❌ Failed to start VM, cleaning up...");
            // Best effort cleanup - ignore errors during cleanup
            let _ = LimaCtl::stop(&name, verbose);
            let _ = LimaCtl::delete(&name, true, verbose);
            return Err(e);
        }

        Ok(Self {
            name,
            cleaned_up: Arc::new(AtomicBool::new(false)),
            verbose,
        })
    }

    /// Get the VM name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get a cleanup guard that ensures VM cleanup on drop
    pub fn ensure_cleanup(&self) -> CleanupGuard {
        CleanupGuard {
            vm_name: self.name.clone(),
            cleaned_up: Arc::clone(&self.cleaned_up),
            verbose: self.verbose,
        }
    }
}

/// RAII guard that ensures VM cleanup even on panic
pub struct CleanupGuard {
    vm_name: String,
    cleaned_up: Arc<AtomicBool>,
    verbose: bool,
}

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        // Only cleanup if not already done
        if !self.cleaned_up.swap(true, Ordering::SeqCst) {
            eprintln!("Cleaning up VM: {}", self.vm_name);

            // Best effort cleanup - ignore errors
            let _ = LimaCtl::stop(&self.vm_name, self.verbose);
            let _ = LimaCtl::delete(&self.vm_name, true, self.verbose);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_guard_sets_flag() {
        let cleaned_up = Arc::new(AtomicBool::new(false));
        {
            let _guard = CleanupGuard {
                vm_name: "test-vm".to_string(),
                cleaned_up: Arc::clone(&cleaned_up),
                verbose: false,
            };
            assert!(!cleaned_up.load(Ordering::SeqCst));
        }
        // After drop, flag should be set
        assert!(cleaned_up.load(Ordering::SeqCst));
    }

    #[test]
    fn test_cleanup_guard_called_on_error() {
        // This test verifies the concept that cleanup happens on error
        // In real code, if VmSession::new() returns Err after cloning,
        // the cleanup code in the error path will run
        let cleaned_up = Arc::new(AtomicBool::new(false));

        // Simulate error scenario
        let result: Result<()> = {
            let _guard = CleanupGuard {
                vm_name: "test-vm".to_string(),
                cleaned_up: Arc::clone(&cleaned_up),
                verbose: false,
            };
            // Simulate failure
            Err(crate::error::ClaudeVmError::LimaExecution(
                "simulated error".to_string(),
            ))
        };

        // Verify error was returned
        assert!(result.is_err());
        // Verify cleanup happened despite error
        assert!(cleaned_up.load(Ordering::SeqCst));
    }
}
