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
    cmd.args(["--runtime-script", "/tmp/test.sh", "setup", "--help"]);

    // Should accept the flag
    cmd.assert().success();
}

#[test]
fn test_disk_memory_flags() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["setup", "--help", "--disk", "30", "--memory", "16"]);

    // Should accept the flags
    cmd.assert().success();
}
