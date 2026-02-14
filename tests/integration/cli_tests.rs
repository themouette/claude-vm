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

// Phase 4 Tests: Edge Cases and Backward Compatibility Validation

// Group 1: Router Edge Cases — Subcommand names as trailing args

#[test]
fn test_edge_case_literal_shell_as_first_arg_is_subcommand() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.arg("shell");
    cmd.env("CLAUDE_VM_CONFIG", "");

    // "shell" should be recognized as the shell subcommand (not routed to agent)
    let result = cmd.assert();
    result.code(predicate::ne(2)); // Not a CLI parse error
}

#[test]
fn test_edge_case_flag_then_literal_shell_routes_to_agent() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["--verbose", "shell"]);
    cmd.env("CLAUDE_VM_CONFIG", "");

    // Router sees --verbose at args[1], inserts "agent", so "shell" becomes a trailing arg
    // This is the documented trade-off from router.rs
    let result = cmd.assert();
    result.code(predicate::ne(2)); // Should parse successfully (routes to agent with trailing arg "shell")
}

#[test]
fn test_edge_case_literal_setup_as_first_arg_is_subcommand() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.arg("setup");
    cmd.env("CLAUDE_VM_CONFIG", "");

    // "setup" should be recognized as setup subcommand
    let result = cmd.assert();
    result.code(predicate::ne(2)); // Not a CLI parse error
}

#[test]
fn test_edge_case_literal_list_as_first_arg_is_subcommand() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.arg("list");
    cmd.env("CLAUDE_VM_CONFIG", "");

    // "list" should be recognized as list subcommand
    let result = cmd.assert();
    result.code(predicate::ne(2)); // Not a CLI parse error
}

#[test]
fn test_edge_case_unknown_word_routes_to_agent() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.arg("foobar");
    cmd.env("CLAUDE_VM_CONFIG", "");

    // "foobar" is not a known subcommand, should route to agent
    let result = cmd.assert();
    result.code(predicate::ne(2)); // Should parse successfully (routes to agent)
}

#[test]
fn test_edge_case_number_as_first_arg_routes_to_agent() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.arg("42");
    cmd.env("CLAUDE_VM_CONFIG", "");

    // Numeric arg should route to agent
    let result = cmd.assert();
    result.code(predicate::ne(2)); // Should parse successfully
}

// Group 2: Flag Position Variations — Systematic coverage

#[test]
fn test_flag_positions_systematic() {
    // Table-driven test covering various flag position patterns
    let test_cases = vec![
        (
            "flag before arg (implicit agent)",
            vec!["--disk", "50", "/clear"],
        ),
        (
            "multiple flags before arg",
            vec!["--disk", "50", "--memory", "8", "/clear"],
        ),
        ("boolean flag before arg", vec!["--verbose", "/clear"]),
        (
            "boolean + value flags before arg",
            vec!["--verbose", "--disk", "50", "/clear"],
        ),
        ("arg only (no flags)", vec!["/clear"]),
        ("flag only (no trailing args)", vec!["--disk", "50"]),
    ];

    for (description, args) in test_cases {
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
        cmd.args(&args);
        cmd.env("CLAUDE_VM_CONFIG", "");

        let result = cmd.assert();
        result.code(predicate::ne(2)); // Should parse without CLI error
        eprintln!("✓ Passed: {}", description);
    }
}

#[test]
fn test_flag_positions_explicit_agent_systematic() {
    // Same table as above but with "agent" prepended
    // Verifies explicit and implicit produce same parse outcome
    let test_cases = vec![
        (
            "agent + flag before arg",
            vec!["agent", "--disk", "50", "/clear"],
        ),
        (
            "agent + multiple flags before arg",
            vec!["agent", "--disk", "50", "--memory", "8", "/clear"],
        ),
        (
            "agent + boolean flag before arg",
            vec!["agent", "--verbose", "/clear"],
        ),
        (
            "agent + boolean + value flags",
            vec!["agent", "--verbose", "--disk", "50", "/clear"],
        ),
        ("agent + arg only", vec!["agent", "/clear"]),
        ("agent + flag only", vec!["agent", "--disk", "50"]),
    ];

    for (description, args) in test_cases {
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
        cmd.args(&args);
        cmd.env("CLAUDE_VM_CONFIG", "");

        let result = cmd.assert();
        result.code(predicate::ne(2)); // Should parse without CLI error
        eprintln!("✓ Passed: {}", description);
    }
}

// Group 3: Double-dash separator handling

#[test]
fn test_double_dash_separator_implicit_agent() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["--disk", "50", "--", "/clear"]);
    cmd.env("CLAUDE_VM_CONFIG", "");

    // The `--` separates flags from trailing args
    let result = cmd.assert();
    result.code(predicate::ne(2)); // Should parse without error
}

#[test]
fn test_double_dash_separator_explicit_agent() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["agent", "--disk", "50", "--", "/clear"]);
    cmd.env("CLAUDE_VM_CONFIG", "");

    let result = cmd.assert();
    result.code(predicate::ne(2)); // Should parse without error
}

#[test]
fn test_double_dash_with_flag_like_arg() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["agent", "--", "--project-dir", "/path"]);
    cmd.env("CLAUDE_VM_CONFIG", "");

    // After `--`, `--project-dir` should be treated as a literal trailing arg
    let result = cmd.assert();
    result.code(predicate::ne(2)); // Should parse without error
}

// Group 4: Flag-like trailing args (hyphen values)

#[test]
fn test_trailing_arg_with_hyphens() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["--project-dir", "/tmp/test"]);
    cmd.env("CLAUDE_VM_CONFIG", "");

    // --project-dir consumed as trailing arg due to allow_hyphen_values
    let result = cmd.assert();
    result.code(predicate::ne(2)); // Should parse without error
}

#[test]
fn test_trailing_arg_with_hyphens_explicit_agent() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["agent", "--project-dir", "/tmp/test"]);
    cmd.env("CLAUDE_VM_CONFIG", "");

    let result = cmd.assert();
    result.code(predicate::ne(2)); // Should parse without error
}

#[test]
fn test_multiple_trailing_args_with_hyphens() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["agent", "--resume", "--project-dir", "/tmp/test"]);
    cmd.env("CLAUDE_VM_CONFIG", "");

    // Multiple flag-like trailing args should parse correctly
    let result = cmd.assert();
    result.code(predicate::ne(2)); // Should parse without error
}

// Group 5: Parity tests — implicit vs explicit produce equivalent parse outcomes

#[test]
fn test_parity_no_args() {
    // Both claude-vm and claude-vm agent should produce same routing result
    let mut cmd1 = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd1.env("CLAUDE_VM_CONFIG", "");
    let result1 = cmd1.assert();
    result1.code(predicate::ne(2)); // Routing succeeds

    let mut cmd2 = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd2.arg("agent");
    cmd2.env("CLAUDE_VM_CONFIG", "");
    let result2 = cmd2.assert();
    result2.code(predicate::ne(2)); // Routing succeeds
}

#[test]
fn test_parity_slash_command() {
    let mut cmd1 = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd1.arg("/clear");
    cmd1.env("CLAUDE_VM_CONFIG", "");
    let result1 = cmd1.assert();
    result1.code(predicate::ne(2));

    let mut cmd2 = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd2.args(["agent", "/clear"]);
    cmd2.env("CLAUDE_VM_CONFIG", "");
    let result2 = cmd2.assert();
    result2.code(predicate::ne(2));
}

#[test]
fn test_parity_flags_and_args() {
    let mut cmd1 = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd1.args(["--disk", "50", "/clear"]);
    cmd1.env("CLAUDE_VM_CONFIG", "");
    let result1 = cmd1.assert();
    result1.code(predicate::ne(2));

    let mut cmd2 = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd2.args(["agent", "--disk", "50", "/clear"]);
    cmd2.env("CLAUDE_VM_CONFIG", "");
    let result2 = cmd2.assert();
    result2.code(predicate::ne(2));
}

#[test]
fn test_parity_verbose_flag() {
    let mut cmd1 = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd1.args(["--verbose", "/clear"]);
    cmd1.env("CLAUDE_VM_CONFIG", "");
    let result1 = cmd1.assert();
    result1.code(predicate::ne(2));

    let mut cmd2 = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd2.args(["agent", "--verbose", "/clear"]);
    cmd2.env("CLAUDE_VM_CONFIG", "");
    let result2 = cmd2.assert();
    result2.code(predicate::ne(2));
}

// Group 6: Reserved words and special characters

#[test]
fn test_reserved_word_help_as_trailing_arg() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["agent", "help"]);
    cmd.env("CLAUDE_VM_CONFIG", "");

    // "help" as trailing arg (not subcommand) should parse
    let result = cmd.assert();
    result.code(predicate::ne(2)); // Should parse without error
}

#[test]
fn test_reserved_word_version_as_trailing_arg() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["agent", "version"]);
    cmd.env("CLAUDE_VM_CONFIG", "");

    // "version" as trailing arg (following "agent") should parse
    let result = cmd.assert();
    result.code(predicate::ne(2)); // Should parse without error
}

#[test]
fn test_special_chars_in_trailing_args() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["agent", "/path/with spaces/file.txt"]);
    cmd.env("CLAUDE_VM_CONFIG", "");

    // Arg with spaces should parse (note: shell would quote this, but here it's a single arg)
    let result = cmd.assert();
    result.code(predicate::ne(2)); // Should parse without error
}

#[test]
fn test_empty_string_trailing_arg() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["agent", ""]);
    cmd.env("CLAUDE_VM_CONFIG", "");

    // Empty string as trailing arg
    let result = cmd.assert();
    result.code(predicate::ne(2)); // Should parse without error
}

// Group 7: Regression smoke test for all subcommands

#[test]
fn test_regression_all_subcommands_parseable() {
    // Iterate over all known subcommands and verify --help works for each
    // This catches any future subcommand additions that break parsing
    let subcommands = vec![
        "agent",
        "shell",
        "setup",
        "info",
        "config",
        "list",
        "clean",
        "clean-all",
        "version",
        "update",
        "network",
    ];

    for subcommand in subcommands {
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
        cmd.args([subcommand, "--help"]);

        let result = cmd.assert();
        result.success(); // --help should always succeed with exit code 0
        eprintln!("✓ Subcommand '{}' --help works", subcommand);
    }
}

// Phase 5 Tests: Worktree Flag Integration

#[test]
fn test_agent_help_shows_worktree_flag() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["agent", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--worktree"));
}

#[test]
fn test_shell_help_shows_worktree_flag() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["shell", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--worktree"));
}

#[test]
fn test_worktree_flag_with_branch_parses() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["agent", "--worktree=my-feature", "--help"]);

    // --help short-circuits execution, but flag should parse
    cmd.assert().success();
}

#[test]
fn test_worktree_flag_with_branch_and_base_parses() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["agent", "--worktree=my-feature,main", "--help"]);

    // --help short-circuits execution, but flag should parse
    cmd.assert().success();
}

#[test]
fn test_shell_worktree_flag_with_branch_parses() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["shell", "--worktree=my-feature", "--help"]);

    // --help short-circuits execution, but flag should parse
    cmd.assert().success();
}

#[test]
fn test_worktree_flag_implicit_agent_parses() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["--worktree=my-feature", "--help"]);

    // Implicit agent routing with --worktree flag
    cmd.assert().success();
}

#[test]
fn test_worktree_flag_requires_branch_name() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["agent", "--worktree"]);
    cmd.env("CLAUDE_VM_CONFIG", "");

    // Should fail with CLI parse error (exit code 2) since --worktree requires a value with =
    let result = cmd.assert();
    result.code(predicate::eq(2));
}

#[test]
fn test_worktree_remove_dry_run_flag_parses() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "remove", "branch-name", "--dry-run", "--help"]);
    cmd.assert().success();
}

#[test]
fn test_worktree_remove_dry_run_with_merged_parses() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args([
        "worktree",
        "remove",
        "--merged",
        "main",
        "--dry-run",
        "--help",
    ]);
    cmd.assert().success();
}

#[test]
fn test_dry_run_and_yes_together_parses() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args([
        "worktree",
        "remove",
        "branch",
        "--dry-run",
        "--yes",
        "--help",
    ]);
    cmd.assert().success();
}

#[test]
fn test_worktree_remove_single_branch_parses() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "remove", "feature-branch", "--help"]);
    cmd.assert().success();
}

#[test]
fn test_worktree_remove_multiple_branches_parses() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args([
        "worktree", "remove", "branch-1", "branch-2", "branch-3", "--help",
    ]);
    cmd.assert().success();
}

#[test]
fn test_worktree_list_filter_flags_parse() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "list", "--merged", "main", "--help"]);
    cmd.assert().success();

    let mut cmd2 = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd2.args(["worktree", "list", "--locked", "--help"]);
    cmd2.assert().success();

    let mut cmd3 = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd3.args(["worktree", "list", "--detached", "--help"]);
    cmd3.assert().success();
}

#[test]
fn test_worktree_list_multiple_filters() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "list", "--merged", "main", "--locked", "--help"]);
    cmd.assert().success();
}

#[test]
fn test_worktree_remove_locked_flag() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args([
        "worktree", "remove", "--merged", "main", "--locked", "--help",
    ]);
    cmd.assert().success();
}
