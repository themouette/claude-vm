use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help_output() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "Run Claude Code inside sandboxed Lima VMs",
        ))
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_version_output() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.arg("--version");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("claude-vm"));
}

#[test]
fn test_version_format() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.arg("--version");

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Version should be in format "claude-vm X.Y.Z" or "claude-vm X.Y.Z-dev+hash[.dirty]"
    assert!(stdout.starts_with("claude-vm "));

    // Extract version part after "claude-vm "
    let version_part = stdout.strip_prefix("claude-vm ").unwrap().trim();

    // Should start with a digit (semver)
    assert!(
        version_part.chars().next().unwrap().is_numeric(),
        "Version should start with a number: {}",
        version_part
    );

    // Debug builds should have -dev or be a plain version
    // Release builds should be plain semver
    #[cfg(debug_assertions)]
    {
        // In debug builds, should contain -dev+ (unless built in CI or without git)
        // or be plain version if git unavailable
        assert!(
            version_part.contains("-dev+") || !version_part.contains("-dev"),
            "Debug build version should contain -dev+ or be plain version: {}",
            version_part
        );
    }

    #[cfg(not(debug_assertions))]
    {
        // In release builds, should NOT contain -dev
        assert!(
            !version_part.contains("-dev"),
            "Release build version should not contain -dev: {}",
            version_part
        );
    }
}

#[test]
fn test_setup_help() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["setup", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Set up a new template VM"))
        .stdout(predicate::str::contains("--docker"))
        .stdout(predicate::str::contains("--node"))
        .stdout(predicate::str::contains("--python"))
        .stdout(predicate::str::contains("--chromium"))
        .stdout(predicate::str::contains("--git"));
}

#[test]
fn test_list_command_exists() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.arg("list");

    // Should run (may fail if limactl not installed, but command exists)
    let result = cmd.assert();
    // We don't check for success because limactl might not be installed in test env
    // Just verify the command is recognized
    result.code(predicate::ne(2)); // Exit code 2 is for CLI parse errors
}

#[test]
fn test_shell_command_exists() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.arg("shell");

    // Should run (may fail if template doesn't exist, but command exists)
    let result = cmd.assert();
    result.code(predicate::ne(2)); // Exit code 2 is for CLI parse errors
}

#[test]
fn test_clean_command_exists() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.arg("clean");

    // Should run (may fail if template doesn't exist, but command exists)
    let result = cmd.assert();
    result.code(predicate::ne(2)); // Exit code 2 is for CLI parse errors
}

#[test]
fn test_clean_all_command_exists() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.arg("clean-all");

    // Should run (may fail, but command exists)
    let result = cmd.assert();
    result.code(predicate::ne(2)); // Exit code 2 is for CLI parse errors
}

#[test]
fn test_runtime_script_flag() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["agent", "--runtime-script", "/tmp/test.sh", "--help"]);

    // Should accept the flag (runtime-script is now on agent/shell only)
    cmd.assert().success();
}

#[test]
fn test_disk_memory_flags() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["setup", "--help", "--disk", "30", "--memory", "16"]);

    // Should accept the flags
    cmd.assert().success();
}

// Phase 1 Tests: Agent Command with Runtime Flags

#[test]
fn test_agent_command_help_shows_runtime_flags() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["agent", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--disk"))
        .stdout(predicate::str::contains("--memory"))
        .stdout(predicate::str::contains("--cpus"))
        .stdout(predicate::str::contains("--mount"))
        .stdout(predicate::str::contains("--verbose"))
        .stdout(predicate::str::contains("--env"))
        .stdout(predicate::str::contains("--env-file"))
        .stdout(predicate::str::contains("--inherit-env"))
        .stdout(predicate::str::contains("--runtime-script"))
        .stdout(predicate::str::contains("--forward-ssh-agent"))
        .stdout(predicate::str::contains("--no-conversations"))
        .stdout(predicate::str::contains("--auto-setup"));
}

#[test]
fn test_agent_command_accepts_flags_and_args() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["agent", "--disk", "50", "--memory", "8", "/clear"]);

    // Should not fail with CLI parse error (exit code 2)
    // May fail at runtime (no template), but flags should parse
    let result = cmd.assert();
    result.code(predicate::ne(2));
}

// Phase 1 Tests: Shell Command with Runtime Flags

#[test]
fn test_shell_command_help_shows_runtime_flags() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["shell", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--disk"))
        .stdout(predicate::str::contains("--memory"))
        .stdout(predicate::str::contains("--mount"))
        .stdout(predicate::str::contains("--verbose"));
}

#[test]
fn test_shell_command_with_runtime_flags() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["shell", "--mount", "/tmp:/tmp", "ls", "-la"]);

    // Should not fail with CLI parse error (exit code 2)
    let result = cmd.assert();
    result.code(predicate::ne(2));
}

// Phase 1 Tests: Setup Command with VM Sizing Flags

#[test]
fn test_setup_command_has_vm_sizing() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["setup", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--disk"))
        .stdout(predicate::str::contains("--memory"))
        .stdout(predicate::str::contains("--cpus"))
        // Should NOT contain runtime-only flags
        .stdout(predicate::str::contains("--env-file").not())
        .stdout(predicate::str::contains("--inherit-env").not())
        .stdout(predicate::str::contains("--forward-ssh-agent").not());
}

#[test]
fn test_setup_command_accepts_vm_flags() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["setup", "--disk", "50", "--cpus", "4", "--help"]);

    // --help with flags is valid
    cmd.assert().success();
}

// Phase 1 Tests: List and Clean Commands DO NOT have Runtime Flags

#[test]
fn test_list_help_no_runtime_flags() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["list", "--help"]);

    let result = cmd.assert().success();
    let output = result.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT contain runtime VM sizing flags (but --disk-usage is OK)
    // Check for the flags as separate arguments, not as part of other flags
    assert!(
        !stdout.contains("--disk ") && !stdout.contains("--disk\n"),
        "list help should not contain --disk flag"
    );
    assert!(
        !stdout.contains("--memory"),
        "list help should not contain --memory flag"
    );
    assert!(
        !stdout.contains("--mount"),
        "list help should not contain --mount flag"
    );
    // Note: --verbose is now a global flag and will appear in all command helps
}

#[test]
fn test_clean_help_no_runtime_flags() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["clean", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--disk").not())
        .stdout(predicate::str::contains("--memory").not())
        .stdout(predicate::str::contains("--mount").not());
    // Note: --verbose is now a global flag and will appear in all command helps
}

// Phase 1 Tests: Top-Level Help Shows Agent Subcommand

#[test]
fn test_top_level_help_shows_agent_subcommand() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("agent"));
}

#[test]
fn test_top_level_help_no_runtime_flags() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.arg("--help");

    cmd.assert()
        .success()
        // Runtime flags should NOT appear at top level
        .stdout(predicate::str::contains("--disk").not())
        .stdout(predicate::str::contains("--memory").not())
        .stdout(predicate::str::contains("--mount").not())
        .stdout(predicate::str::contains("--env").not())
        .stdout(predicate::str::contains("--runtime-script").not());
}

#[test]
fn test_no_subcommand_shows_usage_hint() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    // No arguments - should show usage hint

    // Set CLAUDE_VM_CONFIG to empty string to avoid config file errors
    cmd.env("CLAUDE_VM_CONFIG", "");

    let result = cmd.assert();
    // Before Phase 2 default routing, running with no args should show help
    // Check that it mentions "agent" or "Usage:" in output
    let output = result.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stdout.contains("agent")
            || stdout.contains("Usage:")
            || stderr.contains("agent")
            || stderr.contains("Usage:"),
        "Expected usage hint mentioning 'agent' or 'Usage:', got stdout: {}, stderr: {}",
        stdout,
        stderr
    );
}
