use crate::config::Config;
use crate::error::Result;
use crate::project::Project;
use crate::scripts::host_executor;
use crate::vm::{limactl::LimaCtl, mount};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

/// Returns true if the VM name is an ephemeral session clone.
///
/// Works for both release (`…_hash-PID`) and debug (`…_hash-dev-PID`) builds.
///
/// # Contract with `extract_session_pid`
///
/// Always call this function before calling `extract_session_pid`. On template
/// names whose hash segment is all-digits (e.g. `claude-tpl_proj_12345678`),
/// `is_session_vm` correctly returns `false` (no dash in the hash part), while
/// `extract_session_pid` would spuriously return `Some(12345678)`. The two
/// functions are designed to be used together: use `is_session_vm` as a gate.
pub fn is_session_vm(name: &str) -> bool {
    name.rsplit('_')
        .next()
        .and_then(|hash_part| {
            if hash_part.contains('-') {
                hash_part.rsplit('-').next()
            } else {
                None
            }
        })
        .map(|suffix| !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()))
        .unwrap_or(false)
}

/// Extracts the PID from a session VM name.
///
/// Works for both release (`…_hash-PID`) and debug (`…_hash-dev-PID`).
///
/// # Precondition
///
/// Only call this after `is_session_vm` has returned `true` for the same
/// name. On template names with all-digit hashes (no dash in the hash part)
/// this function may return a spurious `Some` value.
pub fn extract_session_pid(name: &str) -> Option<u32> {
    let hash_part = name.rsplit('_').next()?;
    let pid_str = hash_part.rsplit('-').next()?;
    pid_str.parse::<u32>().ok()
}

/// Returns true if a process with the given PID is currently alive on the host.
pub fn is_pid_running(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    {
        std::path::Path::new(&format!("/proc/{}", pid)).exists()
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("ps")
            .args(["-p", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        false
    }
}

/// Represents an ephemeral VM session with RAII cleanup
pub struct VmSession {
    name: String,
    project: Project,
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
            project: project.clone(),
            cleaned_up: Arc::new(AtomicBool::new(false)),
            verbose,
        })
    }

    /// Get the VM name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get a cleanup guard that ensures VM cleanup on drop
    ///
    /// The config parameter is needed for executing cleanup and teardown phases.
    /// The command parameter identifies which command is running ("agent" or "shell")
    /// and is passed to cleanup phases via the CLAUDE_VM_COMMAND environment variable.
    pub fn ensure_cleanup_with_config(&self, config: &Config, command: &str) -> CleanupGuard {
        CleanupGuard {
            vm_name: self.name.clone(),
            project: self.project.clone(),
            config: Some(config.clone()),
            command: Some(command.to_string()),
            cleaned_up: Arc::clone(&self.cleaned_up),
            child_pid: Arc::new(AtomicU32::new(0)),
            verbose: self.verbose,
        }
    }

    /// Get a cleanup guard without config (for backward compatibility and tests)
    /// Cleanup and teardown phases won't run without config
    pub fn ensure_cleanup(&self) -> CleanupGuard {
        CleanupGuard {
            vm_name: self.name.clone(),
            project: self.project.clone(),
            config: None,
            command: None,
            cleaned_up: Arc::clone(&self.cleaned_up),
            child_pid: Arc::new(AtomicU32::new(0)),
            verbose: self.verbose,
        }
    }
}

/// RAII guard that ensures VM cleanup even on panic
pub struct CleanupGuard {
    vm_name: String,
    project: Project,
    config: Option<Config>,
    command: Option<String>,
    cleaned_up: Arc<AtomicBool>,
    /// PID of the active `limactl shell` child process, or 0 if none.
    child_pid: Arc<AtomicU32>,
    verbose: bool,
}

impl CleanupGuard {
    /// Return a shared handle to the child-PID slot so the runner can store
    /// the `limactl shell` child PID while it is alive.
    pub fn child_pid_slot(&self) -> Arc<AtomicU32> {
        Arc::clone(&self.child_pid)
    }

    /// Send SIGTERM to the process identified by `pid` and restore the terminal
    /// (no-op when `pid == 0`).
    fn kill_child_process(pid: u32) {
        if pid == 0 {
            return;
        }
        let _ = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status();
        // Restore terminal modes left in a raw state by the SSH/PTY session.
        let _ = std::process::Command::new("stty").arg("sane").status();
    }

    /// Install SIGINT/SIGTERM handler that explicitly cleans up the VM before exit.
    /// Without this, Ctrl+C kills the process without running Drop.
    pub fn register_signal_handler(&self) -> crate::error::Result<()> {
        let cleaned_up = Arc::clone(&self.cleaned_up);
        let vm_name = self.vm_name.clone();
        let verbose = self.verbose;
        let child_pid = Arc::clone(&self.child_pid);

        ctrlc::set_handler(move || {
            // Kill the child limactl shell process first so the VM is idle
            // before we attempt to stop it.
            let pid = child_pid.load(Ordering::SeqCst);
            CleanupGuard::kill_child_process(pid);

            if !cleaned_up.swap(true, Ordering::SeqCst) {
                eprintln!("\nInterrupted — cleaning up VM {}...", vm_name);
                let _ = LimaCtl::stop(&vm_name, verbose);
                let _ = LimaCtl::delete(&vm_name, true, verbose);
            }
            std::process::exit(130); // conventional SIGINT exit code
        })
        .map_err(|e| {
            crate::error::ClaudeVmError::LimaExecution(format!(
                "Failed to register signal handler: {}",
                e
            ))
        })
    }
}

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        // Only cleanup if not already done
        if !self.cleaned_up.swap(true, Ordering::SeqCst) {
            // Kill the child limactl shell process first so the VM is idle
            // before we attempt to stop it.
            let pid = self.child_pid.load(Ordering::SeqCst);
            Self::kill_child_process(pid);

            if let Some(config) = &self.config {
                // 1. Execute VM-based after-runtime phases INSIDE VM (before host after_runtime)
                if !config.phase.after_runtime.is_empty() {
                    if let Some(command) = &self.command {
                        // Use project root as workdir (same as runtime phases)
                        let workdir = Some(self.project.root());
                        if let Err(e) = crate::scripts::runner::execute_after_runtime_phases(
                            &self.vm_name,
                            &self.project,
                            config,
                            command,
                            workdir,
                        ) {
                            eprintln!("⚠ Warning: After-runtime phases failed: {}", e);
                            // Continue with teardown anyway
                        }
                    }
                }

                // 2. Execute host-based teardown phases
                if !config.phase.host.teardown.is_empty() {
                    if let Err(e) = host_executor::execute_host_phases(
                        &config.phase.host.teardown,
                        &self.project,
                        &self.vm_name,
                        &host_executor::build_host_env(
                            &self.project,
                            "teardown",
                            self.command.as_deref(),
                        ),
                    ) {
                        eprintln!("⚠ Warning: Teardown phases failed: {}", e);
                        // Continue with VM cleanup anyway
                    }
                }
            }

            // 3. Stop and delete VM
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

    // Tests for is_session_vm()

    #[test]
    fn test_is_session_vm_release_template() {
        // Release template: …_abcd1234 → not a session VM
        assert!(!is_session_vm("claude-tpl_project_abcd1234"));
    }

    #[test]
    fn test_is_session_vm_release_session() {
        // Release session: …_abcd1234-68951 → session VM
        assert!(is_session_vm("claude-tpl_project_abcd1234-68951"));
    }

    #[test]
    fn test_is_session_vm_debug_template() {
        // Debug template: …_abcd1234-dev → not a session VM
        assert!(!is_session_vm("claude-tpl_project_abcd1234-dev"));
    }

    #[test]
    fn test_is_session_vm_debug_session() {
        // Debug session: …_abcd1234-dev-68951 → session VM
        assert!(is_session_vm("claude-tpl_project_abcd1234-dev-68951"));
    }

    #[test]
    fn test_is_session_vm_all_digit_hash_edge_case() {
        // All-digit hash: …_12345678 → no dash in hash_part → not a session VM
        assert!(!is_session_vm("claude-tpl_project_12345678"));
    }

    #[test]
    fn test_is_session_vm_non_template_name() {
        // Non-template VMs should not be session VMs
        assert!(!is_session_vm("my-regular-vm"));
        assert!(!is_session_vm("default"));
    }

    // Tests for extract_session_pid()

    #[test]
    fn test_extract_session_pid_release_session() {
        // Release session: …_abcd1234-68951 → PID 68951
        assert_eq!(
            extract_session_pid("claude-tpl_project_abcd1234-68951"),
            Some(68951)
        );
    }

    #[test]
    fn test_extract_session_pid_debug_session() {
        // Debug session: …_abcd1234-dev-68951 → PID 68951
        assert_eq!(
            extract_session_pid("claude-tpl_project_abcd1234-dev-68951"),
            Some(68951)
        );
    }

    #[test]
    fn test_extract_session_pid_release_template() {
        // Release template: …_abcd1234 → no dash → None
        assert_eq!(extract_session_pid("claude-tpl_project_abcd1234"), None);
    }

    #[test]
    fn test_extract_session_pid_debug_template() {
        // Debug template: …_abcd1234-dev → last segment "dev" → parse fails → None
        assert_eq!(extract_session_pid("claude-tpl_project_abcd1234-dev"), None);
    }

    #[test]
    fn test_extract_session_pid_all_digit_hash() {
        // All-digit hash template: …_12345678 → rsplit('-') gives "12345678" → Some(12345678)
        // This is expected: extract_session_pid doesn't check for the no-dash condition
        // is_session_vm handles correctness, extract_session_pid is a simple extractor
        assert_eq!(
            extract_session_pid("claude-tpl_project_12345678"),
            Some(12345678)
        );
    }

    #[test]
    fn test_cleanup_guard_sets_flag() {
        let cleaned_up = Arc::new(AtomicBool::new(false));
        {
            let _guard = CleanupGuard {
                vm_name: "test-vm".to_string(),
                project: Project::new_for_test(std::path::PathBuf::from("/test")),
                config: None,
                command: None,
                cleaned_up: Arc::clone(&cleaned_up),
                child_pid: Arc::new(AtomicU32::new(0)),
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
                project: Project::new_for_test(std::path::PathBuf::from("/test")),
                config: None,
                command: None,
                cleaned_up: Arc::clone(&cleaned_up),
                child_pid: Arc::new(AtomicU32::new(0)),
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
