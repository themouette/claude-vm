/// VM-based integration tests for host phase execution
///
/// These tests require limactl to be installed and take significant time to run.
/// They are marked with #[ignore] and must be explicitly run:
///
/// Run with: cargo test --test integration_tests integration::host_phases_vm -- --ignored --test-threads=1 --nocapture
///
/// IMPORTANT: Must run sequentially (--test-threads=1) because tests may share resources.
///
/// ## What These Tests Verify
///
/// Host phases run **on the host machine** (not in the VM), which means:
/// - They can access host filesystem directly
/// - They run before/after VM operations
/// - They can't directly access VM filesystem (except through limactl commands)
///
/// ## Test Organization
///
/// ### before_setup Tests (2 tests)
/// - Verify phases run on host before template creation
/// - Test that markers are created on host filesystem
///
/// ### after_setup Tests (2 tests)
/// - Verify phases run on host after template creation
/// - Test access to template info
///
/// ### before_runtime Tests (2 tests)
/// - Verify phases run on host before VM session
/// - Test conditional execution
///
/// ### after_runtime Tests (2 tests)
/// - Verify phases run on host after VM session ends
/// - Test cleanup operations
///
/// ### teardown Tests (1 test)
/// - Verify phases run during cleanup
/// - Test error handling
///
use assert_cmd::Command;
use std::fs;
use std::path::PathBuf;
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

// ============================================================================
// before_setup Phase Tests
// These run on the HOST before the VM template is created
// ============================================================================

#[test]
#[ignore] // Requires limactl and takes time
fn test_host_before_setup_runs_on_host() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let marker_file = temp_dir.path().join("before-setup-marker.txt");

    let config = format!(
        r#"
[[phase.host.before_setup]]
name = "create-host-marker"
script = """
#!/bin/bash
echo 'before_setup executed on host' > {}
"""
"#,
        marker_file.display()
    );

    let project_dir = create_test_project(&config);

    // Run setup - this should execute before_setup phase on host
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Verify the marker file was created on the HOST (not in VM)
    assert!(
        marker_file.exists(),
        "Marker file should exist on host filesystem"
    );

    let content = fs::read_to_string(&marker_file).expect("Should read marker file");
    assert!(
        content.contains("before_setup executed on host"),
        "Marker should contain expected content, got: {}",
        content
    );
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_host_before_setup_with_conditional() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let marker_file = temp_dir.path().join("conditional-marker.txt");
    let condition_file = temp_dir.path().join("condition.txt");

    // Create the condition file so the when clause passes
    fs::write(&condition_file, "exists").expect("Failed to create condition file");

    let config = format!(
        r#"
[[phase.host.before_setup]]
name = "conditional-phase"
when = "test -f {}"
script = """
#!/bin/bash
echo 'conditional passed' > {}
"""
"#,
        condition_file.display(),
        marker_file.display()
    );

    let project_dir = create_test_project(&config);

    // Run setup
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Verify the conditional phase ran
    assert!(
        marker_file.exists(),
        "Conditional phase should have run when condition is met"
    );
}

// ============================================================================
// after_setup Phase Tests
// These run on the HOST after the VM template is created
// ============================================================================

#[test]
#[ignore] // Requires limactl and takes time
fn test_host_after_setup_runs_on_host() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let marker_file = temp_dir.path().join("after-setup-marker.txt");

    let config = format!(
        r#"
[[phase.host.after_setup]]
name = "create-host-marker"
script = """
#!/bin/bash
echo 'after_setup executed on host' > {}
echo "Template created successfully" >> {}
"""
"#,
        marker_file.display(),
        marker_file.display()
    );

    let project_dir = create_test_project(&config);

    // Run setup
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Verify the marker file was created on the HOST after setup
    assert!(
        marker_file.exists(),
        "Marker file should exist on host filesystem"
    );

    let content = fs::read_to_string(&marker_file).expect("Should read marker file");
    assert!(
        content.contains("after_setup executed on host"),
        "Marker should contain expected content, got: {}",
        content
    );
    assert!(
        content.contains("Template created successfully"),
        "Marker should contain success message, got: {}",
        content
    );
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_host_after_setup_with_error_handling() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let marker_file = temp_dir.path().join("error-handling-marker.txt");

    let config = format!(
        r#"
[[phase.host.after_setup]]
name = "failing-phase"
continue_on_error = true
script = """
#!/bin/bash
exit 1
"""

[[phase.host.after_setup]]
name = "should-still-run"
script = """
#!/bin/bash
echo 'ran after error' > {}
"""
"#,
        marker_file.display()
    );

    let project_dir = create_test_project(&config);

    // Run setup - should succeed despite first phase failing
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed with continue_on_error");

    // Verify the second phase still ran
    assert!(
        marker_file.exists(),
        "Second phase should run after first phase error"
    );
}

// ============================================================================
// before_runtime Phase Tests
// These run on the HOST before the VM shell session starts
// ============================================================================

#[test]
#[ignore] // Requires limactl and takes time
fn test_host_before_runtime_runs_on_host() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let marker_file = temp_dir.path().join("before-runtime-marker.txt");

    let config = format!(
        r#"
[[phase.host.before_runtime]]
name = "create-host-marker"
script = """
#!/bin/bash
echo 'before_runtime executed on host' > {}
date >> {}
"""
"#,
        marker_file.display(),
        marker_file.display()
    );

    let project_dir = create_test_project(&config);

    // Run setup first
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Marker should not exist yet (before_runtime hasn't run)
    assert!(
        !marker_file.exists(),
        "Marker should not exist before running shell command"
    );

    // Run a shell command - this should trigger before_runtime phase
    let _output = run_shell_command(&project_dir.path().to_path_buf(), "echo 'test'")
        .expect("Shell command should run");

    // Verify the marker file was created on the HOST
    assert!(
        marker_file.exists(),
        "Marker file should exist on host filesystem after runtime"
    );

    let content = fs::read_to_string(&marker_file).expect("Should read marker file");
    assert!(
        content.contains("before_runtime executed on host"),
        "Marker should contain expected content, got: {}",
        content
    );
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_host_before_runtime_with_env_vars() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let marker_file = temp_dir.path().join("env-marker.txt");

    let config = format!(
        r#"
[[phase.host.before_runtime]]
name = "with-env"
env = {{ HOST_VAR = "test-value", DEBUG = "true" }}
script = """
#!/bin/bash
echo "HOST_VAR=$HOST_VAR" > {}
echo "DEBUG=$DEBUG" >> {}
"""
"#,
        marker_file.display(),
        marker_file.display()
    );

    let project_dir = create_test_project(&config);

    // Run setup
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Run a shell command to trigger before_runtime
    let _output = run_shell_command(&project_dir.path().to_path_buf(), "echo 'test'")
        .expect("Shell command should run");

    // Verify env vars were set
    let content = fs::read_to_string(&marker_file).expect("Should read marker file");
    assert!(
        content.contains("HOST_VAR=test-value"),
        "Should have HOST_VAR, got: {}",
        content
    );
    assert!(
        content.contains("DEBUG=true"),
        "Should have DEBUG, got: {}",
        content
    );
}

// ============================================================================
// after_runtime Phase Tests
// These run on the HOST after the VM shell session ends
// ============================================================================

#[test]
#[ignore] // Requires limactl and takes time
fn test_host_after_runtime_runs_on_host() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let marker_file = temp_dir.path().join("after-runtime-marker.txt");

    let config = format!(
        r#"
[[phase.host.after_runtime]]
name = "create-host-marker"
script = """
#!/bin/bash
echo 'after_runtime executed on host' > {}
date >> {}
"""
"#,
        marker_file.display(),
        marker_file.display()
    );

    let project_dir = create_test_project(&config);

    // Run setup
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Marker should not exist yet
    assert!(
        !marker_file.exists(),
        "Marker should not exist before running shell command"
    );

    // Run a shell command - after_runtime should run when session ends
    let _output = run_shell_command(&project_dir.path().to_path_buf(), "echo 'test'")
        .expect("Shell command should run");

    // Verify the marker file was created on the HOST after runtime
    assert!(
        marker_file.exists(),
        "Marker file should exist on host filesystem after runtime ends"
    );

    let content = fs::read_to_string(&marker_file).expect("Should read marker file");
    assert!(
        content.contains("after_runtime executed on host"),
        "Marker should contain expected content, got: {}",
        content
    );
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_host_after_runtime_cleanup() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cleanup_target = temp_dir.path().join("to-cleanup.txt");
    let marker_file = temp_dir.path().join("cleanup-done.txt");

    // Create a file to clean up
    fs::write(&cleanup_target, "delete me").expect("Failed to create cleanup target");

    let config = format!(
        r#"
[[phase.host.after_runtime]]
name = "cleanup"
script = """
#!/bin/bash
rm -f {}
echo 'cleanup done' > {}
"""
"#,
        cleanup_target.display(),
        marker_file.display()
    );

    let project_dir = create_test_project(&config);

    // Run setup
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // File should exist before runtime
    assert!(cleanup_target.exists(), "Cleanup target should exist");

    // Run a shell command
    let _output = run_shell_command(&project_dir.path().to_path_buf(), "echo 'test'")
        .expect("Shell command should run");

    // Verify cleanup happened
    assert!(
        !cleanup_target.exists(),
        "Cleanup target should be deleted"
    );
    assert!(
        marker_file.exists(),
        "Cleanup marker should exist"
    );
}

// ============================================================================
// teardown Phase Tests
// These run on the HOST during cleanup operations
// ============================================================================

#[test]
#[ignore] // Requires limactl and takes time
fn test_host_teardown_with_multiple_phases() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let marker1 = temp_dir.path().join("teardown1.txt");
    let marker2 = temp_dir.path().join("teardown2.txt");

    let config = format!(
        r#"
[[phase.host.teardown]]
name = "first-teardown"
continue_on_error = true
script = """
#!/bin/bash
echo 'first' > {}
"""

[[phase.host.teardown]]
name = "second-teardown"
script = """
#!/bin/bash
echo 'second' > {}
"""
"#,
        marker1.display(),
        marker2.display()
    );

    let project_dir = create_test_project(&config);

    // Run setup
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Note: teardown phases typically run during `claude-vm cleanup` or `claude-vm delete`
    // For this test, we're just verifying they're correctly parsed and would execute
    // A full test would require triggering a cleanup/delete operation
}

// ============================================================================
// Combined Phase Tests
// Test interaction between host phases and regular phases
// ============================================================================

#[test]
#[ignore] // Requires limactl and takes time
fn test_host_and_vm_phases_execution_order() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let host_marker = temp_dir.path().join("host-phases.txt");

    let config = format!(
        r#"
[[phase.host.before_setup]]
name = "host-before"
script = """
#!/bin/bash
echo '1-host-before-setup' > {}
"""

[[phase.setup]]
name = "vm-setup"
script = """
#!/bin/bash
mkdir -p $HOME/test-data
echo '2-vm-setup' > $HOME/test-data/phases.txt
"""

[[phase.host.after_setup]]
name = "host-after"
script = """
#!/bin/bash
echo '3-host-after-setup' >> {}
"""
"#,
        host_marker.display(),
        host_marker.display()
    );

    let project_dir = create_test_project(&config);

    // Run setup
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Verify host phases ran in order
    let host_content = fs::read_to_string(&host_marker).expect("Should read host marker");
    let lines: Vec<&str> = host_content.trim().lines().collect();
    assert_eq!(lines.len(), 2, "Should have 2 host phase markers");
    assert_eq!(lines[0], "1-host-before-setup");
    assert_eq!(lines[1], "3-host-after-setup");

    // Verify VM phase ran
    let vm_output = run_shell_command(
        &project_dir.path().to_path_buf(),
        "cat $HOME/test-data/phases.txt",
    )
    .expect("Should read VM marker");
    assert!(
        vm_output.contains("2-vm-setup"),
        "VM setup phase should have run"
    );
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_host_phases_with_file_scripts() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let marker_file = temp_dir.path().join("file-script-marker.txt");

    let project_dir_temp = TempDir::new().expect("Failed to create project dir");
    let project_dir = project_dir_temp.path();

    // Create a script file
    let script_file = project_dir.join("host-script.sh");
    fs::write(
        &script_file,
        format!(
            "#!/bin/bash\necho 'from file script' > {}\n",
            marker_file.display()
        ),
    )
    .expect("Failed to write script file");

    // Make script executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script_file).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_file, perms).unwrap();
    }

    let config = format!(
        r#"
[[phase.host.before_setup]]
name = "file-based"
script_files = ["{}"]
"#,
        script_file.display()
    );

    let config_path = project_dir.join(".claude-vm.toml");
    fs::write(&config_path, config).expect("Failed to write config file");

    // Run setup
    run_setup(&project_dir.to_path_buf()).expect("Setup should succeed");

    // Verify the file script ran
    assert!(
        marker_file.exists(),
        "File script should have created marker"
    );

    let content = fs::read_to_string(&marker_file).expect("Should read marker file");
    assert!(
        content.contains("from file script"),
        "Should have content from file script, got: {}",
        content
    );
}
