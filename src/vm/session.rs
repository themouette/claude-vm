use crate::config::Config;
use crate::error::Result;
use crate::project::Project;
use crate::scripts::host_executor;
use crate::vm::{limactl::LimaCtl, mount};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

/// Returns true if the VM name is a persistent session VM.
///
/// Persistent session VMs end with `-s{6 hex chars}`, e.g.
/// `claude-tpl_project_abcd1234-sa3f7c2`.
pub fn is_persistent_session_vm(name: &str) -> bool {
    name.rsplit('_')
        .next()
        .and_then(|hash_part| {
            if hash_part.contains('-') {
                hash_part.rsplit('-').next()
            } else {
                None
            }
        })
        .map(|suffix| {
            suffix.len() >= 2
                && suffix.starts_with('s')
                && suffix[1..].chars().all(|c| c.is_ascii_hexdigit())
        })
        .unwrap_or(false)
}

/// Extracts the session ID (the `s{hex}` suffix) from a persistent session VM name.
pub fn extract_persistent_session_id(name: &str) -> Option<&str> {
    let hash_part = name.rsplit('_').next()?;
    let suffix = hash_part.rsplit('-').next()?;
    if suffix.len() >= 2
        && suffix.starts_with('s')
        && suffix[1..].chars().all(|c| c.is_ascii_hexdigit())
    {
        Some(suffix)
    } else {
        None
    }
}

/// Generate a persistent session VM name for the given template.
///
/// Format: `{template_name}-s{random_hex_6}`
pub fn generate_persistent_vm_name(template_name: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let pid = std::process::id();
    // Simple pseudo-random 6-char hex using time + pid
    let hash_val = (nanos as u64)
        .wrapping_mul(6364136223846793005)
        .wrapping_add(pid as u64);
    let hex_id: String = format!("{:016x}", hash_val).chars().take(6).collect();
    format!("{}-s{}", template_name, hex_id)
}

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

    /// Wrap an already-running VM without creating or starting a new one.
    ///
    /// Used when `--session <id>` is provided: the VM was started by
    /// `session start` and its lifetime is managed externally, so no
    /// `CleanupGuard` should be created.
    pub fn from_existing(vm_name: &str, project: &Project, verbose: bool) -> Self {
        Self {
            name: vm_name.to_string(),
            project: project.clone(),
            cleaned_up: Arc::new(AtomicBool::new(false)),
            verbose,
        }
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
        // Wait for the child to fully exit (and be reaped by the main thread's
        // child.wait()) before restoring the terminal.  The SSH client may still
        // be manipulating the TTY settings during its exit handling; running
        // `stty sane` too early would be overridden.  Cap the wait at 2 s.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while std::time::Instant::now() < deadline && is_pid_running(pid) {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        // Restore terminal modes left in a raw state by the SSH/PTY session.
        let _ = std::process::Command::new("stty").arg("sane").status();
    }

    /// Returns a shared handle to the `cleaned_up` flag so callers can detect
    /// when the signal handler has taken ownership of cleanup.
    pub fn cleanup_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cleaned_up)
    }

    /// Install SIGINT/SIGTERM handler that explicitly cleans up the VM before exit.
    /// Without this, Ctrl+C kills the process without running Drop.
    pub fn register_signal_handler(&self) -> crate::error::Result<()> {
        let cleaned_up = Arc::clone(&self.cleaned_up);
        let child_pid = Arc::clone(&self.child_pid);
        let vm_name = self.vm_name.clone();
        let project = self.project.clone();
        let config = self.config.clone();
        let command = self.command.clone();
        let verbose = self.verbose;

        ctrlc::set_handler(move || {
            if !cleaned_up.swap(true, Ordering::SeqCst) {
                eprintln!("\nInterrupted — cleaning up VM {}...", vm_name);
                perform_cleanup(
                    &vm_name,
                    &project,
                    &config,
                    &command,
                    child_pid.load(Ordering::SeqCst),
                    verbose,
                );
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

/// Runs the full cleanup sequence: kill child, after-runtime phases, host teardown, stop/delete VM.
///
/// Called from both `Drop::drop()` and the signal handler so both paths execute identical logic.
fn perform_cleanup(
    vm_name: &str,
    project: &Project,
    config: &Option<Config>,
    command: &Option<String>,
    child_pid_val: u32,
    verbose: bool,
) {
    // Step 1: Kill child process + restore terminal
    CleanupGuard::kill_child_process(child_pid_val);

    if let Some(cfg) = config {
        // Step 2: Execute VM-based after-runtime phases INSIDE VM (before host teardown)
        if !cfg.phase.after_runtime.is_empty() {
            if let Some(cmd) = command {
                let workdir = Some(project.root());
                if let Err(e) = crate::scripts::runner::execute_after_runtime_phases(
                    vm_name, project, cfg, cmd, workdir,
                ) {
                    eprintln!("⚠ Warning: After-runtime phases failed: {}", e);
                }
            }
        }

        // Step 3: Execute host-based teardown phases
        if !cfg.phase.host.teardown.is_empty() {
            if let Err(e) = host_executor::execute_host_phases(
                &cfg.phase.host.teardown,
                project,
                vm_name,
                &host_executor::build_host_env(project, "teardown", command.as_deref()),
            ) {
                eprintln!("⚠ Warning: Teardown phases failed: {}", e);
            }
        }
    }

    // Step 4: Stop and delete VM
    eprintln!("Cleaning up VM: {}", vm_name);
    let _ = LimaCtl::stop(vm_name, verbose);
    let _ = LimaCtl::delete(vm_name, true, verbose);
}

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        if !self.cleaned_up.swap(true, Ordering::SeqCst) {
            perform_cleanup(
                &self.vm_name,
                &self.project,
                &self.config,
                &self.command,
                self.child_pid.load(Ordering::SeqCst),
                self.verbose,
            );
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

    // Tests for is_persistent_session_vm()

    #[test]
    fn test_is_persistent_session_vm_positive() {
        // Valid: ends with -s{6 hex chars}
        assert!(is_persistent_session_vm(
            "claude-tpl_project_abcd1234-sa3f7c2"
        ));
    }

    #[test]
    fn test_is_persistent_session_vm_negative_pid_suffix() {
        // Ephemeral session (PID suffix) should NOT be persistent
        assert!(!is_persistent_session_vm(
            "claude-tpl_project_abcd1234-68951"
        ));
    }

    #[test]
    fn test_is_persistent_session_vm_negative_template() {
        // Plain template: no dash in hash_part
        assert!(!is_persistent_session_vm("claude-tpl_project_abcd1234"));
    }

    #[test]
    fn test_is_persistent_session_vm_negative_dev_suffix() {
        // Debug template: ends with -dev, not -s{hex}
        assert!(!is_persistent_session_vm("claude-tpl_project_abcd1234-dev"));
    }

    // Tests for extract_persistent_session_id()

    #[test]
    fn test_extract_persistent_session_id_valid() {
        assert_eq!(
            extract_persistent_session_id("claude-tpl_project_abcd1234-sa3f7c2"),
            Some("sa3f7c2")
        );
    }

    #[test]
    fn test_extract_persistent_session_id_not_persistent() {
        // PID suffix does not start with 's'
        assert_eq!(
            extract_persistent_session_id("claude-tpl_project_abcd1234-68951"),
            None
        );
    }

    // Tests for generate_persistent_vm_name()

    #[test]
    fn test_generate_persistent_vm_name_format() {
        let name = generate_persistent_vm_name("claude-tpl_project_abcd1234");
        // Should start with template name
        assert!(name.starts_with("claude-tpl_project_abcd1234-s"));
        // Should pass is_persistent_session_vm check
        assert!(is_persistent_session_vm(&name));
        // Should NOT be detected as ephemeral session
        assert!(!is_session_vm(&name));
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
