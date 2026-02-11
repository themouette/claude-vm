/// VM-based integration tests for phase scripts
///
/// These tests require limactl to be installed and take significant time to run.
/// They are marked with #[ignore] and must be explicitly run:
///
/// Run with: cargo test --test test_phase_scripts_vm -- --ignored --test-threads=1
///
/// Tests are run sequentially (test-threads=1) because they share VM resources.
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
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_phase_inline_script_execution() {
    let config = r#"
[[phase.setup]]
name = "create-marker"
script = """
#!/bin/bash
echo 'Phase script executed' > /tmp/phase-test-marker.txt
"""
"#;

    let project_dir = create_test_project(config);

    // Run setup
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Verify the marker file was created
    let output = run_shell_command(
        &project_dir.path().to_path_buf(),
        "cat /tmp/phase-test-marker.txt",
    )
    .expect("Command should run");

    assert!(
        output.contains("Phase script executed"),
        "Inline script should have created marker file, got: {}",
        output
    );
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_phase_file_script_execution() {
    let project_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a script file
    let script_path = project_dir.path().join("setup-script.sh");
    fs::write(
        &script_path,
        "#!/bin/bash\necho 'File script executed' > /tmp/file-script-marker.txt\n",
    )
    .expect("Failed to write script file");

    // Create config that references the script
    let config = format!(
        r#"
[[phase.setup]]
name = "run-file-script"
script_files = ["{}"]
"#,
        script_path.display()
    );

    let config_path = project_dir.path().join(".claude-vm.toml");
    fs::write(&config_path, config).expect("Failed to write config file");

    // Run setup
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Verify the marker file was created
    let output = run_shell_command(
        &project_dir.path().to_path_buf(),
        "cat /tmp/file-script-marker.txt",
    )
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

    let project_dir = create_test_project(config);

    // Run setup
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Run a shell command which will trigger runtime scripts
    let output = run_shell_command(
        &project_dir.path().to_path_buf(),
        "cat /tmp/env-test.txt",
    )
    .expect("Command should run");

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
fn test_phase_execution_order() {
    let config = r#"
[[phase.setup]]
name = "first"
script = """
#!/bin/bash
echo '1' > /tmp/phase-order.txt
"""

[[phase.setup]]
name = "second"
script = """
#!/bin/bash
echo '2' >> /tmp/phase-order.txt
"""

[[phase.setup]]
name = "third"
script = """
#!/bin/bash
echo '3' >> /tmp/phase-order.txt
"""
"#;

    let project_dir = create_test_project(config);

    // Run setup
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Verify execution order
    let output = run_shell_command(
        &project_dir.path().to_path_buf(),
        "cat /tmp/phase-order.txt",
    )
    .expect("Command should run");

    // Should contain lines 1, 2, 3 in order
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 3, "Should have 3 lines, got: {:?}", lines);
    assert_eq!(lines[0], "1", "First line should be 1");
    assert_eq!(lines[1], "2", "Second line should be 2");
    assert_eq!(lines[2], "3", "Third line should be 3");
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_phase_conditional_when() {
    let config = r#"
[[phase.setup]]
name = "always-run"
script = """
#!/bin/bash
echo 'always' > /tmp/conditional-test.txt
"""

[[phase.setup]]
name = "conditional-run"
when = "test -f /tmp/conditional-test.txt"
script = """
#!/bin/bash
echo 'conditional executed' >> /tmp/conditional-test.txt
"""

[[phase.setup]]
name = "should-not-run"
when = "test -f /nonexistent-file-xyz"
script = """
#!/bin/bash
echo 'should not appear' >> /tmp/conditional-test.txt
"""
"#;

    let project_dir = create_test_project(config);

    // Run setup
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Check results
    let output = run_shell_command(
        &project_dir.path().to_path_buf(),
        "cat /tmp/conditional-test.txt",
    )
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
fn test_phase_continue_on_error() {
    let config = r#"
[[phase.setup]]
name = "first-success"
script = """
#!/bin/bash
echo 'first' > /tmp/error-test.txt
"""

[[phase.setup]]
name = "second-fails"
continue_on_error = true
script = """
#!/bin/bash
exit 1
"""

[[phase.setup]]
name = "third-should-run"
script = """
#!/bin/bash
echo 'third' >> /tmp/error-test.txt
"""
"#;

    let project_dir = create_test_project(config);

    // Run setup - should succeed despite second phase failing
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed despite error");

    // Verify first and third phases ran
    let output = run_shell_command(
        &project_dir.path().to_path_buf(),
        "cat /tmp/error-test.txt",
    )
    .expect("Command should run");

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
fn test_phase_mixed_inline_and_file() {
    let project_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a script file
    let script_path = project_dir.path().join("extra.sh");
    fs::write(&script_path, "#!/bin/bash\necho 'from-file' >> /tmp/mixed-test.txt\n")
        .expect("Failed to write script file");

    // Create config with both inline and file scripts
    let config = format!(
        r#"
[[phase.setup]]
name = "mixed"
script = """
#!/bin/bash
echo 'from-inline' > /tmp/mixed-test.txt
"""
script_files = ["{}"]
"#,
        script_path.display()
    );

    let config_path = project_dir.path().join(".claude-vm.toml");
    fs::write(&config_path, config).expect("Failed to write config file");

    // Run setup
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Verify both ran (inline first, then file)
    let output = run_shell_command(
        &project_dir.path().to_path_buf(),
        "cat /tmp/mixed-test.txt",
    )
    .expect("Command should run");

    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 2, "Should have 2 lines, got: {:?}", lines);
    assert_eq!(
        lines[0], "from-inline",
        "Inline script should run first"
    );
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

    let project_dir = create_test_project(config);

    // Run setup
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Run a shell command which should trigger runtime scripts
    let output = run_shell_command(
        &project_dir.path().to_path_buf(),
        "cat /tmp/runtime-marker.txt",
    )
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
fn test_phase_with_special_characters() {
    let config = r#"
[[phase.setup]]
name = "special-chars"
env = { VAR = "value with 'quotes' and $dollar" }
script = """
#!/bin/bash
echo "VAR=$VAR" > /tmp/special-chars-test.txt
echo 'Single quotes: '"'"'test'"'"'' >> /tmp/special-chars-test.txt
"""
"#;

    let project_dir = create_test_project(config);

    // Run setup
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Verify special characters are handled correctly
    let output = run_shell_command(
        &project_dir.path().to_path_buf(),
        "cat /tmp/special-chars-test.txt",
    )
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
