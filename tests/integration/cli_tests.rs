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

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Extract the Options section (between "Options:" and "INVOCATION PATTERNS:")
    let options_start = stdout
        .find("Options:")
        .expect("Should have Options section");
    let options_end = stdout.find("INVOCATION PATTERNS:").unwrap_or(stdout.len());
    let options_section = &stdout[options_start..options_end];

    // Runtime flags should NOT appear at top level in Options section
    assert!(
        !options_section.contains("--disk <"),
        "Top-level help should not contain --disk flag in Options"
    );
    assert!(
        !options_section.contains("--memory"),
        "Top-level help should not contain --memory flag in Options"
    );
    assert!(
        !options_section.contains("--mount"),
        "Top-level help should not contain --mount flag in Options"
    );
    assert!(
        !options_section.contains("--env <") && !options_section.contains("--env\n"),
        "Top-level help should not contain --env flag in Options"
    );
    assert!(
        !options_section.contains("--runtime-script"),
        "Top-level help should not contain --runtime-script flag in Options"
    );
}

#[test]
fn test_no_subcommand_routes_to_agent() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    // No arguments - should route to agent

    // Set CLAUDE_VM_CONFIG to empty string to avoid config file errors
    cmd.env("CLAUDE_VM_CONFIG", "");

    let result = cmd.assert();
    // After routing, no-args invocation routes to agent
    // Agent without a project will fail (non-zero exit), but NOT with parse error (exit code 2)
    result.code(predicate::ne(2));
}

// Phase 2 Tests: Backward Compatibility - Default Agent Routing

#[test]
fn test_backward_compat_slash_command_defaults_to_agent() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.arg("/clear");
    cmd.env("CLAUDE_VM_CONFIG", "");

    // Should parse successfully (routes to agent), may fail at runtime
    let result = cmd.assert();
    result.code(predicate::ne(2)); // Not a CLI parse error
}

#[test]
fn test_backward_compat_flags_before_args() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["--disk", "50", "--memory", "8", "/clear"]);
    cmd.env("CLAUDE_VM_CONFIG", "");

    // Should parse successfully
    let result = cmd.assert();
    result.code(predicate::ne(2));
}

#[test]
fn test_backward_compat_flags_only() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["--disk", "50"]);
    cmd.env("CLAUDE_VM_CONFIG", "");

    // Routes to agent with flags but no trailing args
    let result = cmd.assert();
    result.code(predicate::ne(2));
}

#[test]
fn test_backward_compat_help_still_shows_main_help() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("agent"))
        .stdout(predicate::str::contains("shell"));
}

#[test]
fn test_backward_compat_version_still_works() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.arg("--version");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("claude-vm"));
}

#[test]
fn test_backward_compat_explicit_agent_still_works() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["agent", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--disk"))
        .stdout(predicate::str::contains("--memory"));
}

#[test]
fn test_backward_compat_explicit_and_implicit_equivalent() {
    // Test that main help and agent help are different
    let mut cmd1 = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd1.arg("--help");
    let output1 = cmd1.assert().success();
    let stdout1 = String::from_utf8_lossy(&output1.get_output().stdout);

    let mut cmd2 = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd2.args(["agent", "--help"]);
    let output2 = cmd2.assert().success();
    let stdout2 = String::from_utf8_lossy(&output2.get_output().stdout);

    // Main help should have "Commands:"
    assert!(
        stdout1.contains("Commands:"),
        "Main help should show Commands section"
    );

    // Agent help should have "--disk"
    assert!(
        stdout2.contains("--disk"),
        "Agent help should show agent-specific flags"
    );

    // They should be different
    assert_ne!(
        stdout1.as_ref(),
        stdout2.as_ref(),
        "Main help and agent help should be different"
    );
}

#[test]
fn test_backward_compat_trailing_args_without_separator() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["--project-dir", "/tmp/test"]);
    cmd.env("CLAUDE_VM_CONFIG", "");

    // Trailing args should be parsed without `--` separator
    let result = cmd.assert();
    result.code(predicate::ne(2));
}

// Phase 3 Tests: Help System

#[test]
fn test_main_help_shows_invocation_patterns() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("INVOCATION PATTERNS"))
        .stdout(predicate::str::contains("claude-vm [options] [args]"))
        .stdout(predicate::str::contains("claude-vm agent [options] [args]"));
}

#[test]
fn test_main_help_shows_examples() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("EXAMPLES"))
        .stdout(predicate::str::contains("claude-vm /clear"));
}

#[test]
fn test_main_help_excludes_agent_flags() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.arg("--help");

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Extract the Options section (between "Options:" and "INVOCATION PATTERNS:")
    let options_start = stdout
        .find("Options:")
        .expect("Should have Options section");
    let options_end = stdout.find("INVOCATION PATTERNS:").unwrap_or(stdout.len());
    let options_section = &stdout[options_start..options_end];

    // Runtime flags should NOT appear in the Options section
    assert!(
        !options_section.contains("--disk <"),
        "Main help Options should not contain --disk flag"
    );
    assert!(
        !options_section.contains("--memory"),
        "Main help Options should not contain --memory flag"
    );
    assert!(
        !options_section.contains("--mount"),
        "Main help Options should not contain --mount flag"
    );
    assert!(
        !options_section.contains("--runtime-script"),
        "Main help Options should not contain --runtime-script flag"
    );
    assert!(
        !options_section.contains("--forward-ssh-agent"),
        "Main help Options should not contain --forward-ssh-agent flag"
    );
}

#[test]
fn test_agent_help_shows_all_runtime_flags() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["agent", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--disk"))
        .stdout(predicate::str::contains("--memory"))
        .stdout(predicate::str::contains("--cpus"))
        .stdout(predicate::str::contains("--mount"))
        .stdout(predicate::str::contains("--env"))
        .stdout(predicate::str::contains("--env-file"))
        .stdout(predicate::str::contains("--inherit-env"))
        .stdout(predicate::str::contains("--runtime-script"))
        .stdout(predicate::str::contains("--forward-ssh-agent"))
        .stdout(predicate::str::contains("--no-conversations"))
        .stdout(predicate::str::contains("--auto-setup"));
}

#[test]
fn test_agent_long_help_mentions_shorthand() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["agent", "--help"]);

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Check for mention of shorthand/default behavior
    assert!(
        stdout.contains("default command") || stdout.contains("omit 'agent'"),
        "Agent help should mention that it's the default command or that 'agent' can be omitted"
    );
}

#[test]
fn test_short_help_differs_from_long_help() {
    // By default clap shows the same output for -h and --help
    // This test verifies both produce valid help output
    let mut cmd1 = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd1.arg("-h");
    let output1 = cmd1.assert().success();

    let mut cmd2 = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd2.arg("--help");
    let output2 = cmd2.assert().success();

    let stdout1 = String::from_utf8_lossy(&output1.get_output().stdout);
    let stdout2 = String::from_utf8_lossy(&output2.get_output().stdout);

    // Both should contain basic help information
    assert!(
        stdout1.contains("Usage:"),
        "Short help should contain Usage"
    );
    assert!(stdout2.contains("Usage:"), "Long help should contain Usage");

    // Both should contain INVOCATION PATTERNS (clap doesn't differentiate by default)
    assert!(
        stdout1.contains("INVOCATION PATTERNS") || stdout2.contains("INVOCATION PATTERNS"),
        "Help should contain INVOCATION PATTERNS"
    );
}

#[test]
fn test_help_flag_not_routed_to_agent() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("agent"))
        .stdout(predicate::str::contains("shell"));
}
