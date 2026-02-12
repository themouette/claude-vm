/// VM-based integration tests for phase scripts
///
/// These tests require limactl to be installed and take significant time to run.
/// They are marked with #[ignore] and must be explicitly run:
///
/// Run with: cargo test --test integration_tests integration::phase_scripts_vm -- --ignored --nocapture
///
/// Tests can run in parallel! Use --test-threads=N to control parallelism:
/// - --test-threads=1: Sequential (safest, slowest)
/// - --test-threads=4: 4 parallel threads (recommended for local dev)
/// - (no flag): Use all available cores
///
/// ## Test Organization
///
/// Tests are organized by what they actually test, not just by phase type:
///
/// ### Setup Phase Tests (3 tests) - Build individual templates
/// These test setup-specific behavior:
/// - `test_setup_phase_basic_execution` - Verifies setup scripts run during template build
/// - `test_legacy_and_phase_scripts_coexist` - Tests legacy setup scripts compatibility
/// - `test_phase_realistic_setup_workflow` - Integration test for realistic setup scenarios
///
/// ### Runtime Phase Tests (14 tests) - Share one pre-built template
/// These test the **shared phase logic** that works identically in both setup and runtime:
/// - Script execution (inline, file, mixed)
/// - Execution order
/// - Conditional execution (`when`)
/// - Error handling (`continue_on_error`)
/// - Environment variables
/// - Special character escaping
/// - Source/export behavior (runtime-only feature)
///
/// ## Performance Impact
///
/// **Before reorganization:** 16 tests × ~2 min/template = ~32 minutes (sequential)
/// **After with parallelism:**
/// - CI (--test-threads=1): 3 setup + 1 shared runtime = ~7 minutes
/// - Local (--test-threads=4): 3 setup + 4 shared runtime = ~4 minutes
/// - Local (--test-threads=8): 3 setup + min(8,14) shared = ~3 minutes
///
/// **Speedup:** Up to 90% faster with parallelism!
///
/// Each test thread builds its own runtime template, so with N threads:
/// - Setup tests: 3 templates (always sequential for stability)
/// - Runtime tests: min(N, 14) templates (one per thread)
///
/// This design tests shared features comprehensively where it's fast (runtime),
/// while keeping minimal setup-specific tests for actual setup behavior.
use assert_cmd::Command;
use once_cell::sync::Lazy;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tempfile::TempDir;

/// Helper to create a test directory with a .claude-vm.toml file
fn create_test_project(config_content: &str) -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join(".claude-vm.toml");
    fs::write(&config_path, config_content).expect("Failed to write config file");
    temp_dir
}

/// Helper to run setup command in a test directory
fn run_setup(project_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.current_dir(project_dir)
        .args(["setup", "--no-agent-install"]);

    let output = cmd.output()?;
    if !output.status.success() {
        eprintln!("Setup failed:");
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        return Err("Setup command failed".into());
    }
    Ok(())
}

/// Helper to run a shell command in the VM and return output
fn run_shell_command(
    project_dir: &PathBuf,
    command: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.current_dir(project_dir)
        .args(["shell", "bash", "-c", command]);

    let output = cmd.output()?;

    // If command failed, include stderr in error for debugging
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "Command failed with exit code {:?}\nstdout: {}\nstderr: {}",
            output.status.code(),
            stdout,
            stderr
        )
        .into());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

// Thread-local shared template for runtime-only tests
// Each test thread gets its own shared template to avoid race conditions.
// This allows tests to run in parallel while still benefiting from template reuse.
//
// Performance: With --test-threads=4, we build 4 templates instead of 16 (75% reduction)
thread_local! {
    static SHARED_RUNTIME_TEMPLATE: Lazy<Mutex<(TempDir, PathBuf)>> = Lazy::new(|| {
        use std::thread;
        let thread_id = thread::current().id();
        eprintln!("Building shared runtime template for thread {:?}...", thread_id);

        // Create a minimal config - runtime tests will override with their own runtime configs
        let config = r#"
# Minimal base configuration for runtime testing
# Each test will define its own runtime phases
"#;

        let project_dir = create_test_project(config);
        let path = project_dir.path().to_path_buf();

        // Build the template once per thread
        run_setup(&path).expect("Shared runtime template setup should succeed");

        eprintln!("Shared runtime template ready for thread {:?} at: {:?}", thread_id, path);

        // Keep TempDir alive and return path for reuse within this thread
        Mutex::new((project_dir, path))
    });
}

/// Get the thread-local shared runtime template path
/// This returns a path to a pre-built template that can be used for runtime-only tests.
/// Each test thread has its own template to avoid conflicts.
fn get_shared_runtime_project() -> PathBuf {
    SHARED_RUNTIME_TEMPLATE.with(|template| {
        let guard = template.lock().unwrap();
        guard.1.clone()
    })
}

/// Helper specifically for runtime tests that uses thread-local shared template
/// This creates a config file in the shared template directory with runtime-specific config.
/// Safe for parallel execution since each thread has its own template directory.
fn setup_runtime_test(config_content: &str) -> PathBuf {
    let project_path = get_shared_runtime_project();
    let config_path = project_path.join(".claude-vm.toml");
    fs::write(&config_path, config_content).expect("Failed to write runtime config file");
    project_path
}

// ============================================================================
// Setup Phase Tests (3 tests)
// These test setup-specific behavior and build individual templates
// ============================================================================

#[test]
#[ignore] // Requires limactl and takes time
fn test_setup_phase_basic_execution() {
    let config = r#"
[[phase.setup]]
name = "basic-setup"
script = """
#!/bin/bash
echo 'Setup phase executed' > /tmp/setup-marker.txt
"""
"#;

    let project_dir = create_test_project(config);

    // Run setup - this builds the template and runs setup scripts
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Verify the marker file was created during template build
    let output = run_shell_command(
        &project_dir.path().to_path_buf(),
        "cat /tmp/setup-marker.txt",
    )
    .expect("Command should run");

    assert!(
        output.contains("Setup phase executed"),
        "Setup script should have run during template build, got: {}",
        output
    );
}

// ============================================================================
// Runtime Phase Tests (14 tests)
// These test shared features using a single pre-built template for speed
// ============================================================================

#[test]
#[ignore] // Requires limactl and takes time
fn test_runtime_inline_script_execution() {
    let config = r#"
[[phase.runtime]]
name = "create-marker"
script = """
#!/bin/bash
echo 'Phase script executed' > /tmp/phase-test-marker.txt
"""
"#;

    // Use shared template - tests shared script loading logic
    let project_path = setup_runtime_test(config);

    // Run a shell command which will trigger runtime scripts
    let output = run_shell_command(&project_path, "cat /tmp/phase-test-marker.txt")
        .expect("Command should run");

    assert!(
        output.contains("Phase script executed"),
        "Inline script should have created marker file, got: {}",
        output
    );
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_runtime_file_script_execution() {
    // Use shared template but need a separate temp dir for the script file
    let project_path = get_shared_runtime_project();

    // Create a script file in the project directory
    let script_path = project_path.join("runtime-script.sh");
    fs::write(
        &script_path,
        "#!/bin/bash\necho 'File script executed' > /tmp/file-script-marker.txt\n",
    )
    .expect("Failed to write script file");

    // Create config that references the script
    let config = format!(
        r#"
[[phase.runtime]]
name = "run-file-script"
script_files = ["{}"]
"#,
        script_path.display()
    );

    let config_path = project_path.join(".claude-vm.toml");
    fs::write(&config_path, config).expect("Failed to write config file");

    // Run a shell command which will trigger runtime scripts
    let output = run_shell_command(&project_path, "cat /tmp/file-script-marker.txt")
        .expect("Command should run");

    assert!(
        output.contains("File script executed"),
        "File-based script should have created marker file, got: {}",
        output
    );
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_phase_env_vars() {
    let config = r#"
[[phase.runtime]]
name = "test-env"
env = { TEST_VAR = "hello-from-phase", DEBUG = "true" }
script = """
#!/bin/bash
echo "TEST_VAR=$TEST_VAR" > /tmp/env-test.txt
echo "DEBUG=$DEBUG" >> /tmp/env-test.txt
"""
"#;

    // Use shared template - no need to rebuild for runtime-only test
    let project_path = setup_runtime_test(config);

    // Run a shell command which will trigger runtime scripts
    let output =
        run_shell_command(&project_path, "cat /tmp/env-test.txt").expect("Command should run");

    assert!(
        output.contains("TEST_VAR=hello-from-phase"),
        "Environment variable TEST_VAR should be set, got: {}",
        output
    );

    assert!(
        output.contains("DEBUG=true"),
        "Environment variable DEBUG should be set, got: {}",
        output
    );
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_runtime_execution_order() {
    let config = r#"
[[phase.runtime]]
name = "first"
script = """
#!/bin/bash
echo '1' > /tmp/phase-order.txt
"""

[[phase.runtime]]
name = "second"
script = """
#!/bin/bash
echo '2' >> /tmp/phase-order.txt
"""

[[phase.runtime]]
name = "third"
script = """
#!/bin/bash
echo '3' >> /tmp/phase-order.txt
"""
"#;

    // Use shared template - tests shared execution order logic
    let project_path = setup_runtime_test(config);

    // Run a shell command which will trigger runtime scripts
    let output =
        run_shell_command(&project_path, "cat /tmp/phase-order.txt").expect("Command should run");

    // Should contain lines 1, 2, 3 in order
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 3, "Should have 3 lines, got: {:?}", lines);
    assert_eq!(lines[0], "1", "First line should be 1");
    assert_eq!(lines[1], "2", "Second line should be 2");
    assert_eq!(lines[2], "3", "Third line should be 3");
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_runtime_conditional_when() {
    let config = r#"
[[phase.runtime]]
name = "always-run"
script = """
#!/bin/bash
echo 'always' > /tmp/conditional-test.txt
"""

[[phase.runtime]]
name = "conditional-run"
when = "test -f /tmp/conditional-test.txt"
script = """
#!/bin/bash
echo 'conditional executed' >> /tmp/conditional-test.txt
"""

[[phase.runtime]]
name = "should-not-run"
when = "test -f /nonexistent-file-xyz"
script = """
#!/bin/bash
echo 'should not appear' >> /tmp/conditional-test.txt
"""
"#;

    // Use shared template - tests shared conditional logic
    let project_path = setup_runtime_test(config);

    // Run a shell command which will trigger runtime scripts
    let output = run_shell_command(&project_path, "cat /tmp/conditional-test.txt")
        .expect("Command should run");

    assert!(
        output.contains("always"),
        "First phase should always run, got: {}",
        output
    );

    assert!(
        output.contains("conditional executed"),
        "Conditional phase should run when condition is met, got: {}",
        output
    );

    assert!(
        !output.contains("should not appear"),
        "Conditional phase should not run when condition fails, got: {}",
        output
    );
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_runtime_continue_on_error() {
    let config = r#"
[[phase.runtime]]
name = "first-success"
script = """
#!/bin/bash
echo 'first' > /tmp/error-test.txt
"""

[[phase.runtime]]
name = "second-fails"
continue_on_error = true
script = """
#!/bin/bash
exit 1
"""

[[phase.runtime]]
name = "third-should-run"
script = """
#!/bin/bash
echo 'third' >> /tmp/error-test.txt
"""
"#;

    // Use shared template - tests shared error handling logic
    let project_path = setup_runtime_test(config);

    // Run a shell command - should succeed despite second phase failing
    let output = run_shell_command(&project_path, "cat /tmp/error-test.txt")
        .expect("Command should run despite error");

    assert!(
        output.contains("first"),
        "First phase should run, got: {}",
        output
    );

    assert!(
        output.contains("third"),
        "Third phase should run even after second fails, got: {}",
        output
    );
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_runtime_mixed_inline_and_file() {
    // Use shared template but need to create script file
    let project_path = get_shared_runtime_project();

    // Create a script file
    let script_path = project_path.join("extra.sh");
    fs::write(
        &script_path,
        "#!/bin/bash\necho 'from-file' >> /tmp/mixed-test.txt\n",
    )
    .expect("Failed to write script file");

    // Create config with both inline and file scripts
    let config = format!(
        r#"
[[phase.runtime]]
name = "mixed"
script = """
#!/bin/bash
echo 'from-inline' > /tmp/mixed-test.txt
"""
script_files = ["{}"]
"#,
        script_path.display()
    );

    let config_path = project_path.join(".claude-vm.toml");
    fs::write(&config_path, config).expect("Failed to write config file");

    // Run a shell command which will trigger runtime scripts
    let output =
        run_shell_command(&project_path, "cat /tmp/mixed-test.txt").expect("Command should run");

    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 2, "Should have 2 lines, got: {:?}", lines);
    assert_eq!(lines[0], "from-inline", "Inline script should run first");
    assert_eq!(lines[1], "from-file", "File script should run second");
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_runtime_phase_execution() {
    let config = r#"
[[phase.runtime]]
name = "create-runtime-marker"
script = """
#!/bin/bash
date > /tmp/runtime-marker.txt
echo 'Runtime phase executed' >> /tmp/runtime-marker.txt
"""
"#;

    // Use shared template - no need to rebuild for runtime-only test
    let project_path = setup_runtime_test(config);

    // Run a shell command which should trigger runtime scripts
    let output = run_shell_command(&project_path, "cat /tmp/runtime-marker.txt")
        .expect("Command should run");

    assert!(
        output.contains("Runtime phase executed"),
        "Runtime phase should have executed, got: {}",
        output
    );
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_legacy_and_phase_scripts_coexist() {
    let project_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a legacy setup script
    let legacy_script = project_dir.path().join("legacy.sh");
    fs::write(
        &legacy_script,
        "#!/bin/bash\necho 'legacy' > /tmp/coexist-test.txt\n",
    )
    .expect("Failed to write legacy script");

    // Create config with both legacy and phase scripts
    let config = format!(
        r#"
[setup]
scripts = ["{}"]

[[phase.setup]]
name = "phase-script"
script = """
#!/bin/bash
echo 'phase' >> /tmp/coexist-test.txt
"""
"#,
        legacy_script.display()
    );

    let config_path = project_dir.path().join(".claude-vm.toml");
    fs::write(&config_path, config).expect("Failed to write config file");

    // Run setup
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Verify both ran
    let output = run_shell_command(
        &project_dir.path().to_path_buf(),
        "cat /tmp/coexist-test.txt",
    )
    .expect("Command should run");

    assert!(
        output.contains("legacy"),
        "Legacy script should run, got: {}",
        output
    );

    assert!(
        output.contains("phase"),
        "Phase script should run, got: {}",
        output
    );
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_runtime_with_special_characters() {
    let config = r#"
[[phase.runtime]]
name = "special-chars"
env = { VAR = "value with 'quotes' and $dollar" }
script = """
#!/bin/bash
echo "VAR=$VAR" > /tmp/special-chars-test.txt
echo 'Single quotes: '"'"'test'"'"'' >> /tmp/special-chars-test.txt
"""
"#;

    // Use shared template - tests shared escaping logic
    let project_path = setup_runtime_test(config);

    // Run a shell command which will trigger runtime scripts
    let output = run_shell_command(&project_path, "cat /tmp/special-chars-test.txt")
        .expect("Command should run");

    assert!(
        output.contains("VAR=value with 'quotes' and $dollar"),
        "Special characters in env vars should be handled, got: {}",
        output
    );

    assert!(
        output.contains("Single quotes: 'test'"),
        "Special characters in scripts should be handled, got: {}",
        output
    );
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_phase_realistic_setup_workflow() {
    let config = r#"
[[phase.setup]]
name = "verify-requirements"
script = """
#!/bin/bash
echo 'Checking requirements...'
test $(nproc) -ge 1 || exit 1
echo 'requirements-ok' > /tmp/workflow-test.txt
"""

[[phase.setup]]
name = "install-tools"
when = "test -f /tmp/workflow-test.txt"
script = """
#!/bin/bash
echo 'Installing tools...'
echo 'tools-installed' >> /tmp/workflow-test.txt
"""

[[phase.setup]]
name = "configure"
env = { CONFIG = "production" }
script = """
#!/bin/bash
echo "Configuring with CONFIG=$CONFIG..."
echo "configured-$CONFIG" >> /tmp/workflow-test.txt
"""
"#;

    let project_dir = create_test_project(config);

    // Run setup
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Verify all phases ran
    let output = run_shell_command(
        &project_dir.path().to_path_buf(),
        "cat /tmp/workflow-test.txt",
    )
    .expect("Command should run");

    assert!(
        output.contains("requirements-ok"),
        "Verify phase should run, got: {}",
        output
    );

    assert!(
        output.contains("tools-installed"),
        "Install phase should run, got: {}",
        output
    );

    assert!(
        output.contains("configured-production"),
        "Configure phase should run with env vars, got: {}",
        output
    );
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_phase_source_exports_persist() {
    let config = r#"
[[phase.runtime]]
name = "export-var"
source = true
script = """
export TEST_VAR="hello from source"
"""

[[phase.runtime]]
name = "use-var"
script = """
#!/bin/bash
echo "TEST_VAR=$TEST_VAR" > /tmp/source-test.txt
"""
"#;

    // Use shared template - no need to rebuild for runtime-only test
    let project_path = setup_runtime_test(config);

    // Run a shell command which will trigger runtime scripts
    let output =
        run_shell_command(&project_path, "cat /tmp/source-test.txt").expect("Command should run");

    assert!(
        output.contains("TEST_VAR=hello from source"),
        "Sourced export should persist to next phase, got: {}",
        output
    );
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_phase_source_path_modification() {
    let config = r#"
[[phase.runtime]]
name = "add-to-path"
source = true
script = """
mkdir -p ~/.local/bin
echo '#!/bin/bash' > ~/.local/bin/custom-tool
echo 'echo "custom tool works"' >> ~/.local/bin/custom-tool
chmod +x ~/.local/bin/custom-tool
export PATH="$HOME/.local/bin:$PATH"
"""

[[phase.runtime]]
name = "use-custom-tool"
script = """
#!/bin/bash
custom-tool > /tmp/path-test.txt
"""
"#;

    // Use shared template - no need to rebuild for runtime-only test
    let project_path = setup_runtime_test(config);

    // Run a shell command which will trigger runtime scripts
    let output =
        run_shell_command(&project_path, "cat /tmp/path-test.txt").expect("Command should run");

    assert!(
        output.contains("custom tool works"),
        "Custom tool from modified PATH should work, got: {}",
        output
    );
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_phase_source_with_env_vars() {
    let config = r#"
[[phase.runtime]]
name = "source-with-env"
source = true
env = { INITIAL_VALUE = "initial" }
script = """
export COMBINED_VAR="$INITIAL_VALUE-plus-more"
"""

[[phase.runtime]]
name = "check-combined"
script = """
#!/bin/bash
echo "COMBINED_VAR=$COMBINED_VAR" > /tmp/combined-test.txt
"""
"#;

    // Use shared template - no need to rebuild for runtime-only test
    let project_path = setup_runtime_test(config);

    // Run a shell command which will trigger runtime scripts
    let output =
        run_shell_command(&project_path, "cat /tmp/combined-test.txt").expect("Command should run");

    assert!(
        output.contains("COMBINED_VAR=initial-plus-more"),
        "Sourced script should have access to env vars and export should persist, got: {}",
        output
    );
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_phase_no_source_exports_dont_persist() {
    let config = r#"
[[phase.runtime]]
name = "export-without-source"
source = false
script = """
#!/bin/bash
export NO_PERSIST_VAR="should not persist"
"""

[[phase.runtime]]
name = "check-var"
script = """
#!/bin/bash
echo "NO_PERSIST_VAR=$NO_PERSIST_VAR" > /tmp/no-persist-test.txt
"""
"#;

    // Use shared template - no need to rebuild for runtime-only test
    let project_path = setup_runtime_test(config);

    // Run a shell command which will trigger runtime scripts
    let output = run_shell_command(&project_path, "cat /tmp/no-persist-test.txt")
        .expect("Command should run");

    assert!(
        output.contains("NO_PERSIST_VAR=") && !output.contains("should not persist"),
        "Non-sourced export should NOT persist, got: {}",
        output
    );
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_phase_source_multiple_exports() {
    let config = r#"
[[phase.runtime]]
name = "export-multiple"
source = true
script = """
export VAR1="value1"
export VAR2="value2"
export VAR3="value3"
"""

[[phase.runtime]]
name = "use-all-vars"
script = """
#!/bin/bash
echo "VAR1=$VAR1" > /tmp/multi-export-test.txt
echo "VAR2=$VAR2" >> /tmp/multi-export-test.txt
echo "VAR3=$VAR3" >> /tmp/multi-export-test.txt
"""
"#;

    // Use shared template - no need to rebuild for runtime-only test
    let project_path = setup_runtime_test(config);

    // Run a shell command which will trigger runtime scripts
    let output = run_shell_command(&project_path, "cat /tmp/multi-export-test.txt")
        .expect("Command should run");

    assert!(
        output.contains("VAR1=value1"),
        "VAR1 should persist, got: {}",
        output
    );
    assert!(
        output.contains("VAR2=value2"),
        "VAR2 should persist, got: {}",
        output
    );
    assert!(
        output.contains("VAR3=value3"),
        "VAR3 should persist, got: {}",
        output
    );
}
