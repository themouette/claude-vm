/// Tests to ensure CLI-specific flags are not forwarded to Claude
///
/// This test suite verifies that all claude-vm runtime flags
/// (--worktree, --env, --disk, etc.) are properly parsed and filtered
/// out before arguments are passed to the Claude CLI.
use clap::Parser;
use claude_vm::cli::router::route_args;
use claude_vm::cli::{Cli, Commands};

#[test]
fn test_worktree_single_arg() {
    // Use -- to explicitly consume only 1 arg
    let args = vec![
        "claude-vm",
        "agent",
        "--worktree",
        "feature-branch",
        "--",
        "/clear",
    ];

    let cli = Cli::parse_from(args);

    if let Some(Commands::Agent(cmd)) = cli.command {
        // Verify --worktree was parsed
        assert_eq!(cmd.runtime.worktree, vec!["feature-branch"]);

        // Verify --worktree is NOT in claude_args
        assert!(!cmd.claude_args.contains(&"--worktree".to_string()));
        assert!(!cmd.claude_args.contains(&"feature-branch".to_string()));

        // Verify only Claude args remain
        assert_eq!(cmd.claude_args, vec!["/clear"]);
    } else {
        panic!("Expected Agent command");
    }
}

#[test]
fn test_worktree_two_args() {
    let args = vec![
        "claude-vm",
        "agent",
        "--worktree",
        "feature",
        "develop",
        "/clear",
    ];

    let routed = route_args(args);
    let cli = Cli::parse_from(routed);

    if let Some(Commands::Agent(cmd)) = cli.command {
        // Verify --worktree with base was parsed
        assert_eq!(cmd.runtime.worktree, vec!["feature", "develop"]);

        // Verify neither value is in claude_args
        assert!(!cmd.claude_args.contains(&"--worktree".to_string()));
        assert!(!cmd.claude_args.contains(&"feature".to_string()));
        assert!(!cmd.claude_args.contains(&"develop".to_string()));

        // Verify only Claude args remain
        assert_eq!(cmd.claude_args, vec!["/clear"]);
    } else {
        panic!("Expected Agent command");
    }
}

#[test]
fn test_worktree_with_explicit_separator() {
    let args = vec![
        "claude-vm",
        "agent",
        "--worktree",
        "feature",
        "--",
        "/clear",
    ];

    let routed = route_args(args);
    let cli = Cli::parse_from(routed);

    if let Some(Commands::Agent(cmd)) = cli.command {
        // Verify --worktree parsed only one arg (stopped at --)
        assert_eq!(cmd.runtime.worktree, vec!["feature"]);

        // Verify only Claude args remain (-- is consumed by clap as separator)
        assert_eq!(cmd.claude_args, vec!["/clear"]);
    } else {
        panic!("Expected Agent command");
    }
}

#[test]
fn test_worktree_stops_at_known_flag() {
    let args = vec![
        "claude-vm",
        "agent",
        "--worktree",
        "feature",
        "--disk",
        "50",
        "/clear",
    ];

    let routed = route_args(args);
    let cli = Cli::parse_from(routed);

    if let Some(Commands::Agent(cmd)) = cli.command {
        // Verify --worktree parsed only one arg (stopped at --disk)
        assert_eq!(cmd.runtime.worktree, vec!["feature"]);

        // Verify --disk was parsed
        assert_eq!(cmd.runtime.disk, Some(50));

        // Verify only Claude args remain
        assert_eq!(cmd.claude_args, vec!["/clear"]);
    } else {
        panic!("Expected Agent command");
    }
}

#[test]
fn test_worktree_stops_at_unknown_flag() {
    let args = vec![
        "claude-vm",
        "agent",
        "--worktree",
        "feature",
        "--unknown-flag",
        "/clear",
    ];

    let routed = route_args(args);
    let cli = Cli::parse_from(routed);

    if let Some(Commands::Agent(cmd)) = cli.command {
        // Verify --worktree parsed only one arg (stopped at --unknown-flag)
        assert_eq!(cmd.runtime.worktree, vec!["feature"]);

        // Verify unknown flag goes to Claude
        assert_eq!(cmd.claude_args, vec!["--unknown-flag", "/clear"]);
    } else {
        panic!("Expected Agent command");
    }
}

#[test]
fn test_worktree_stops_at_short_flag() {
    let args = vec![
        "claude-vm",
        "agent",
        "--worktree",
        "feature",
        "-v",
        "/clear",
    ];

    let routed = route_args(args);
    let cli = Cli::parse_from(routed);

    // -v should go to global verbose, not worktree
    assert!(cli.verbose);

    if let Some(Commands::Agent(cmd)) = cli.command {
        // Verify --worktree parsed only one arg (stopped at -v)
        assert_eq!(cmd.runtime.worktree, vec!["feature"]);

        // Verify only Claude args remain
        assert_eq!(cmd.claude_args, vec!["/clear"]);
    } else {
        panic!("Expected Agent command");
    }
}

#[test]
fn test_worktree_with_shell_command() {
    // Use -- to separate worktree args from shell command
    let args = vec![
        "claude-vm",
        "shell",
        "--worktree",
        "feature",
        "--",
        "ls",
        "-la",
    ];

    let routed = route_args(args);
    let cli = Cli::parse_from(routed);

    if let Some(Commands::Shell(cmd)) = cli.command {
        // Verify --worktree was parsed
        assert_eq!(cmd.runtime.worktree, vec!["feature"]);

        // Verify shell command remains
        assert_eq!(cmd.command, vec!["ls", "-la"]);
    } else {
        panic!("Expected Shell command");
    }
}

#[test]
fn test_worktree_equals_syntax_backward_compat() {
    // Equals syntax should still work (used internally by normalization)
    let args = vec!["claude-vm", "agent", "--worktree=feature,main", "/clear"];

    let routed = route_args(args);
    let cli = Cli::parse_from(routed);

    if let Some(Commands::Agent(cmd)) = cli.command {
        // Verify --worktree was parsed
        assert_eq!(cmd.runtime.worktree, vec!["feature", "main"]);

        // Verify only Claude args remain
        assert_eq!(cmd.claude_args, vec!["/clear"]);
    } else {
        panic!("Expected Agent command");
    }
}

#[test]
fn test_no_worktree_backward_compat() {
    // Without --worktree, everything should work as before
    let args = vec!["claude-vm", "/clear"];

    // Route args first (simulates what main.rs does)
    let routed = route_args(args);
    let cli = Cli::parse_from(routed);

    if let Some(Commands::Agent(cmd)) = cli.command {
        // Verify no worktree
        assert!(cmd.runtime.worktree.is_empty());

        // Verify Claude args passed through
        assert_eq!(cmd.claude_args, vec!["/clear"]);
    } else {
        panic!("Expected Agent command");
    }
}

#[test]
fn test_env_flag_not_forwarded() {
    let args = vec![
        "claude-vm",
        "agent",
        "--env",
        "FOO=bar",
        "--env",
        "BAZ=qux",
        "/clear",
    ];

    let cli = Cli::parse_from(args);

    if let Some(Commands::Agent(cmd)) = cli.command {
        // Verify --env was parsed
        assert_eq!(cmd.runtime.env, vec!["FOO=bar", "BAZ=qux"]);

        // Verify --env is NOT in claude_args
        assert!(!cmd.claude_args.contains(&"--env".to_string()));
        assert!(!cmd.claude_args.contains(&"FOO=bar".to_string()));
        assert!(!cmd.claude_args.contains(&"BAZ=qux".to_string()));

        assert_eq!(cmd.claude_args, vec!["/clear"]);
    } else {
        panic!("Expected Agent command");
    }
}

#[test]
fn test_runtime_script_not_forwarded() {
    let args = vec![
        "claude-vm",
        "agent",
        "--runtime-script",
        "/path/to/script.sh",
        "/clear",
    ];

    let cli = Cli::parse_from(args);

    if let Some(Commands::Agent(cmd)) = cli.command {
        // Verify --runtime-script was parsed
        assert_eq!(cmd.runtime.runtime_scripts.len(), 1);

        // Verify it's NOT in claude_args
        assert!(!cmd.claude_args.contains(&"--runtime-script".to_string()));
        assert!(!cmd.claude_args.contains(&"/path/to/script.sh".to_string()));

        assert_eq!(cmd.claude_args, vec!["/clear"]);
    } else {
        panic!("Expected Agent command");
    }
}

#[test]
fn test_vm_sizing_flags_not_forwarded() {
    let args = vec![
        "claude-vm",
        "agent",
        "--disk",
        "50",
        "--memory",
        "16",
        "--cpus",
        "8",
        "/clear",
    ];

    let cli = Cli::parse_from(args);

    if let Some(Commands::Agent(cmd)) = cli.command {
        // Verify VM sizing flags were parsed
        assert_eq!(cmd.runtime.disk, Some(50));
        assert_eq!(cmd.runtime.memory, Some(16));
        assert_eq!(cmd.runtime.cpus, Some(8));

        // Verify they're NOT in claude_args
        assert!(!cmd.claude_args.contains(&"--disk".to_string()));
        assert!(!cmd.claude_args.contains(&"50".to_string()));
        assert!(!cmd.claude_args.contains(&"--memory".to_string()));
        assert!(!cmd.claude_args.contains(&"16".to_string()));
        assert!(!cmd.claude_args.contains(&"--cpus".to_string()));
        assert!(!cmd.claude_args.contains(&"8".to_string()));

        assert_eq!(cmd.claude_args, vec!["/clear"]);
    } else {
        panic!("Expected Agent command");
    }
}

#[test]
fn test_mount_flag_not_forwarded() {
    let args = vec![
        "claude-vm",
        "agent",
        "--mount",
        "/host/path:/vm/path:ro",
        "/clear",
    ];

    let cli = Cli::parse_from(args);

    if let Some(Commands::Agent(cmd)) = cli.command {
        // Verify --mount was parsed
        assert_eq!(cmd.runtime.mounts, vec!["/host/path:/vm/path:ro"]);

        // Verify it's NOT in claude_args
        assert!(!cmd.claude_args.contains(&"--mount".to_string()));
        assert!(!cmd
            .claude_args
            .contains(&"/host/path:/vm/path:ro".to_string()));

        assert_eq!(cmd.claude_args, vec!["/clear"]);
    } else {
        panic!("Expected Agent command");
    }
}

#[test]
fn test_auto_setup_flag_not_forwarded() {
    let args = vec!["claude-vm", "agent", "--auto-setup", "/clear"];

    let cli = Cli::parse_from(args);

    if let Some(Commands::Agent(cmd)) = cli.command {
        // Verify --auto-setup was parsed
        assert!(cmd.runtime.auto_setup);

        // Verify it's NOT in claude_args
        assert!(!cmd.claude_args.contains(&"--auto-setup".to_string()));

        assert_eq!(cmd.claude_args, vec!["/clear"]);
    } else {
        panic!("Expected Agent command");
    }
}

#[test]
fn test_verbose_flag_not_forwarded() {
    let args = vec!["claude-vm", "--verbose", "agent", "/clear"];

    let cli = Cli::parse_from(args);

    // Verify --verbose was parsed
    assert!(cli.verbose);

    if let Some(Commands::Agent(cmd)) = cli.command {
        // Verify it's NOT in claude_args
        assert!(!cmd.claude_args.contains(&"--verbose".to_string()));
        assert!(!cmd.claude_args.contains(&"-v".to_string()));

        assert_eq!(cmd.claude_args, vec!["/clear"]);
    } else {
        panic!("Expected Agent command");
    }
}

#[test]
fn test_no_conversations_flag_not_forwarded() {
    let args = vec!["claude-vm", "agent", "--no-conversations", "/clear"];

    let cli = Cli::parse_from(args);

    if let Some(Commands::Agent(cmd)) = cli.command {
        // Verify --no-conversations was parsed
        assert!(cmd.no_conversations);

        // Verify it's NOT in claude_args
        assert!(!cmd.claude_args.contains(&"--no-conversations".to_string()));

        assert_eq!(cmd.claude_args, vec!["/clear"]);
    } else {
        panic!("Expected Agent command");
    }
}

#[test]
fn test_forward_ssh_agent_not_forwarded() {
    let args = vec!["claude-vm", "agent", "--forward-ssh-agent", "/clear"];

    let cli = Cli::parse_from(args);

    if let Some(Commands::Agent(cmd)) = cli.command {
        // Verify flag was parsed
        assert!(cmd.runtime.forward_ssh_agent);

        // Verify it's NOT in claude_args
        assert!(!cmd.claude_args.contains(&"--forward-ssh-agent".to_string()));
        assert!(!cmd.claude_args.contains(&"-A".to_string()));

        assert_eq!(cmd.claude_args, vec!["/clear"]);
    } else {
        panic!("Expected Agent command");
    }
}

#[test]
fn test_multiple_flags_with_claude_args() {
    let args = vec![
        "claude-vm",
        "--verbose",
        "agent",
        "--worktree",
        "feature",
        "--env",
        "DEBUG=1",
        "--disk",
        "50",
        "/clear",
        "--model",
        "opus",
    ];

    let cli = Cli::parse_from(args);

    assert!(cli.verbose);

    if let Some(Commands::Agent(cmd)) = cli.command {
        // Verify all CLI flags were parsed
        assert_eq!(cmd.runtime.worktree, vec!["feature"]);
        assert_eq!(cmd.runtime.env, vec!["DEBUG=1"]);
        assert_eq!(cmd.runtime.disk, Some(50));

        // Verify ONLY Claude args remain
        assert_eq!(cmd.claude_args, vec!["/clear", "--model", "opus"]);

        // Verify NO CLI flags in claude_args
        assert!(!cmd.claude_args.contains(&"--worktree".to_string()));
        assert!(!cmd.claude_args.contains(&"feature".to_string()));
        assert!(!cmd.claude_args.contains(&"--env".to_string()));
        assert!(!cmd.claude_args.contains(&"DEBUG=1".to_string()));
        assert!(!cmd.claude_args.contains(&"--disk".to_string()));
        assert!(!cmd.claude_args.contains(&"50".to_string()));
        assert!(!cmd.claude_args.contains(&"--verbose".to_string()));
    } else {
        panic!("Expected Agent command");
    }
}

#[test]
fn test_all_runtime_flags_comprehensive() {
    // Test every single RuntimeFlag to ensure none leak into claude_args
    let args = vec![
        "claude-vm",
        "--verbose",
        "agent",
        "--disk",
        "50",
        "--memory",
        "16",
        "--cpus",
        "8",
        "--forward-ssh-agent",
        "--mount",
        "/host:/vm",
        "--env",
        "VAR=value",
        "--env-file",
        ".env",
        "--inherit-env",
        "PATH",
        "--runtime-script",
        "script.sh",
        "--auto-setup",
        "--worktree",
        "branch",
        "base",
        "--no-conversations",
        "/clear",
    ];

    let cli = Cli::parse_from(args);

    assert!(cli.verbose);

    if let Some(Commands::Agent(cmd)) = cli.command {
        // Verify ALL flags were parsed correctly
        assert_eq!(cmd.runtime.disk, Some(50));
        assert_eq!(cmd.runtime.memory, Some(16));
        assert_eq!(cmd.runtime.cpus, Some(8));
        assert!(cmd.runtime.forward_ssh_agent);
        assert_eq!(cmd.runtime.mounts, vec!["/host:/vm"]);
        assert_eq!(cmd.runtime.env, vec!["VAR=value"]);
        assert_eq!(cmd.runtime.env_file.len(), 1);
        assert_eq!(cmd.runtime.inherit_env, vec!["PATH"]);
        assert_eq!(cmd.runtime.runtime_scripts.len(), 1);
        assert!(cmd.runtime.auto_setup);
        assert_eq!(cmd.runtime.worktree, vec!["branch", "base"]);
        assert!(cmd.no_conversations);

        // Verify ONLY Claude args remain
        assert_eq!(cmd.claude_args, vec!["/clear"]);

        // Verify NO CLI flags leaked into claude_args
        let cli_flags = vec![
            "--verbose",
            "-v",
            "--disk",
            "--memory",
            "--cpus",
            "--forward-ssh-agent",
            "-A",
            "--mount",
            "--env",
            "--env-file",
            "--inherit-env",
            "--runtime-script",
            "--auto-setup",
            "--worktree",
            "--no-conversations",
        ];

        for flag in cli_flags {
            assert!(
                !cmd.claude_args.contains(&flag.to_string()),
                "Flag {} should not be in claude_args",
                flag
            );
        }
    } else {
        panic!("Expected Agent command");
    }
}
