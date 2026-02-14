use std::ffi::OsString;

/// Known subcommands that should NOT trigger agent insertion.
/// These match the Commands enum variants in kebab-case.
const KNOWN_SUBCOMMANDS: &[&str] = &[
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
    "worktree",
];

/// Route CLI arguments to the appropriate command.
///
/// This function implements backward compatibility by inserting "agent" as the default
/// subcommand when the user omits it. This allows `claude-vm /clear` to work as an
/// alias for `claude-vm agent /clear`.
///
/// # Routing Logic
///
/// The router inspects only `args[1]` (the first argument after the program name):
///
/// - If `args[1]` is `--help`, `-h`, `--version`, or `-V`: unchanged (preserve main help/version)
/// - If `args[1]` is a known subcommand: unchanged
/// - If `args[1]` starts with `-` (any flag): insert "agent" after program name
/// - If `args[1]` is anything else (not a known subcommand): insert "agent" after program name
///
/// # Examples
///
/// ```text
/// claude-vm /clear              -> claude-vm agent /clear
/// claude-vm --disk 50 /clear    -> claude-vm agent --disk 50 /clear
/// claude-vm --verbose /clear    -> claude-vm agent --verbose /clear
/// claude-vm agent /clear        -> claude-vm agent /clear (unchanged)
/// claude-vm shell ls            -> claude-vm shell ls (unchanged)
/// claude-vm --help              -> claude-vm --help (unchanged)
/// ```
///
/// # Known Trade-off
///
/// `claude-vm --verbose agent /clear` will produce `claude-vm agent --verbose agent /clear`,
/// treating the literal "agent" as a trailing arg. This is acceptable because:
/// - Users can write `claude-vm agent --verbose /clear` instead
/// - This edge case is uncommon
/// - The simplicity benefit outweighs this minor issue
pub fn route_args<I, T>(args: I) -> Vec<OsString>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args: Vec<OsString> = args.into_iter().map(Into::into).collect();

    // If no args provided (just program name), default to agent
    if args.len() < 2 {
        let mut routed = Vec::with_capacity(2);
        if !args.is_empty() {
            routed.push(args[0].clone());
        }
        routed.push("agent".into());
        return routed;
    }

    let first_arg = args[1].to_string_lossy();

    // Preserve main --help and --version
    if first_arg == "--help" || first_arg == "-h" || first_arg == "--version" || first_arg == "-V" {
        return args;
    }

    // If first arg is a known subcommand, normalize and return
    if KNOWN_SUBCOMMANDS.contains(&first_arg.as_ref()) {
        return normalize_worktree_args(args);
    }

    // If first arg starts with '-' (any flag) OR is not a known subcommand,
    // insert "agent" after program name
    let mut routed = Vec::with_capacity(args.len() + 1);
    routed.push(args[0].clone());
    routed.push("agent".into());
    routed.extend_from_slice(&args[1..]);

    // Normalize --worktree arguments before passing to clap
    normalize_worktree_args(routed)
}

/// Normalize --worktree arguments to --worktree=value format.
///
/// This function processes `--worktree` flags that don't use `=` syntax and converts
/// them to the `--worktree=branch[,base]` format that clap expects.
///
/// # Argument Consumption Rules
///
/// After `--worktree`, the function consumes 1-2 arguments as worktree parameters:
/// - Stops consuming if it encounters `--` (double dash separator)
/// - Stops consuming if it encounters any flag starting with `-`
/// - Consumes at most 2 non-flag arguments (branch and optional base)
///
/// This matches the behavior of `git worktree create` and allows users to:
/// - Use `--worktree branch` for simple cases
/// - Use `--worktree branch base` to specify a base ref
/// - Use `--worktree branch --` to pass additional args without specifying base
///
/// # Examples
///
/// ```text
/// --worktree feature              -> --worktree=feature
/// --worktree feature main         -> --worktree=feature,main
/// --worktree feature -- /clear    -> --worktree=feature -- /clear
/// --worktree feature --disk 50    -> --worktree=feature --disk 50
/// --worktree=feature,main         -> --worktree=feature,main (unchanged)
/// ```
fn normalize_worktree_args(args: Vec<OsString>) -> Vec<OsString> {
    let mut result = Vec::new();
    let mut i = 0;

    while i < args.len() {
        let arg = args[i].to_string_lossy();

        // Only process --worktree without = (with = is already in the right format)
        if arg == "--worktree" {
            let mut worktree_parts = Vec::new();

            // Look ahead for worktree arguments (max 2: branch and optional base)
            let mut j = i + 1;
            while j < args.len() && worktree_parts.len() < 2 {
                let next_arg = args[j].to_string_lossy();

                // Stop if we hit -- or any flag
                if next_arg == "--" || next_arg.starts_with('-') {
                    break;
                }

                worktree_parts.push(next_arg.to_string());
                j += 1;
            }

            if worktree_parts.is_empty() {
                // No arguments after --worktree, keep as-is (will error in clap)
                result.push(args[i].clone());
            } else {
                // Convert to --worktree=value format
                let worktree_value = worktree_parts.join(",");
                result.push(format!("--worktree={}", worktree_value).into());
                i = j - 1; // Skip consumed arguments (-1 because loop will increment)
            }
        } else {
            // Not a bare --worktree flag, keep as-is
            result.push(args[i].clone());
        }

        i += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to convert string slices to Vec<OsString>
    fn args(items: &[&str]) -> Vec<OsString> {
        items.iter().map(|s| (*s).into()).collect()
    }

    // Core routing tests: unchanged args

    #[test]
    fn test_empty_args_defaults_to_agent() {
        let input = args(&["claude-vm"]);
        let output = route_args(input);
        assert_eq!(output, args(&["claude-vm", "agent"]));
    }

    #[test]
    fn test_no_args_at_all_defaults_to_agent() {
        let input: Vec<OsString> = vec![];
        let output = route_args(input);
        assert_eq!(output, args(&["agent"]));
    }

    #[test]
    fn test_help_flag_not_routed() {
        let input = args(&["claude-vm", "--help"]);
        let output = route_args(input.clone());
        assert_eq!(output, input);
    }

    #[test]
    fn test_short_help_not_routed() {
        let input = args(&["claude-vm", "-h"]);
        let output = route_args(input.clone());
        assert_eq!(output, input);
    }

    #[test]
    fn test_version_flag_not_routed() {
        let input = args(&["claude-vm", "--version"]);
        let output = route_args(input.clone());
        assert_eq!(output, input);
    }

    #[test]
    fn test_short_version_not_routed() {
        let input = args(&["claude-vm", "-V"]);
        let output = route_args(input.clone());
        assert_eq!(output, input);
    }

    #[test]
    fn test_explicit_agent_unchanged() {
        let input = args(&["claude-vm", "agent", "/clear"]);
        let output = route_args(input.clone());
        assert_eq!(output, input);
    }

    #[test]
    fn test_explicit_shell_unchanged() {
        let input = args(&["claude-vm", "shell", "ls"]);
        let output = route_args(input.clone());
        assert_eq!(output, input);
    }

    #[test]
    fn test_explicit_setup_unchanged() {
        let input = args(&["claude-vm", "setup"]);
        let output = route_args(input.clone());
        assert_eq!(output, input);
    }

    #[test]
    fn test_all_known_subcommands_unchanged() {
        for subcommand in KNOWN_SUBCOMMANDS {
            let input = args(&["claude-vm", subcommand]);
            let output = route_args(input.clone());
            assert_eq!(
                output, input,
                "Subcommand '{}' should not be modified",
                subcommand
            );
        }
    }

    // Core routing tests: agent inserted

    #[test]
    fn test_trailing_arg_routes_to_agent() {
        let input = args(&["claude-vm", "/clear"]);
        let expected = args(&["claude-vm", "agent", "/clear"]);
        let output = route_args(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_path_arg_routes_to_agent() {
        let input = args(&["claude-vm", "/tmp/myproject"]);
        let expected = args(&["claude-vm", "agent", "/tmp/myproject"]);
        let output = route_args(input);
        assert_eq!(output, expected);
    }

    // Flag-triggered routing tests

    #[test]
    fn test_boolean_flag_routes_to_agent() {
        let input = args(&["claude-vm", "--verbose", "/clear"]);
        let expected = args(&["claude-vm", "agent", "--verbose", "/clear"]);
        let output = route_args(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_short_flag_routes_to_agent() {
        let input = args(&["claude-vm", "-v", "/clear"]);
        let expected = args(&["claude-vm", "agent", "-v", "/clear"]);
        let output = route_args(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_value_flag_routes_to_agent() {
        let input = args(&["claude-vm", "--disk", "50", "/clear"]);
        let expected = args(&["claude-vm", "agent", "--disk", "50", "/clear"]);
        let output = route_args(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_multiple_flags_route_to_agent() {
        let input = args(&["claude-vm", "--verbose", "--disk", "50", "/clear"]);
        let expected = args(&["claude-vm", "agent", "--verbose", "--disk", "50", "/clear"]);
        let output = route_args(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_flags_only_route_to_agent() {
        let input = args(&["claude-vm", "--disk", "50"]);
        let expected = args(&["claude-vm", "agent", "--disk", "50"]);
        let output = route_args(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_hyphen_value_arg_routes_to_agent() {
        let input = args(&["claude-vm", "--project-dir", "/path"]);
        let expected = args(&["claude-vm", "agent", "--project-dir", "/path"]);
        let output = route_args(input);
        assert_eq!(output, expected);
    }

    // Edge case / known trade-off tests

    #[test]
    fn test_global_flag_before_explicit_subcommand_inserts_agent() {
        let input = args(&["claude-vm", "--verbose", "agent", "/clear"]);
        let expected = args(&["claude-vm", "agent", "--verbose", "agent", "/clear"]);
        let output = route_args(input);
        assert_eq!(
            output, expected,
            "Known trade-off: --verbose before explicit subcommand causes agent insertion"
        );
    }

    // Synchronization test

    #[test]
    fn test_known_subcommands_match_commands_enum() {
        use crate::cli::Cli;
        use clap::CommandFactory;

        let cli_cmd = Cli::command();
        let subcommands: Vec<&str> = cli_cmd.get_subcommands().map(|c| c.get_name()).collect();

        // Verify every subcommand from Commands enum is in KNOWN_SUBCOMMANDS
        for name in &subcommands {
            assert!(
                KNOWN_SUBCOMMANDS.contains(name),
                "Commands enum has '{}' but KNOWN_SUBCOMMANDS does not",
                name
            );
        }

        // Verify every entry in KNOWN_SUBCOMMANDS exists in Commands enum
        for name in KNOWN_SUBCOMMANDS {
            assert!(
                subcommands.contains(name),
                "KNOWN_SUBCOMMANDS has '{}' but Commands enum does not",
                name
            );
        }
    }

    // Worktree normalization tests

    #[test]
    fn test_worktree_single_arg_with_separator() {
        // Use -- to explicitly stop at 1 arg
        let input = args(&[
            "claude-vm",
            "agent",
            "--worktree",
            "feature",
            "--",
            "/clear",
        ]);
        let expected = args(&["claude-vm", "agent", "--worktree=feature", "--", "/clear"]);
        let output = route_args(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_worktree_two_args_normalized() {
        // Without --, consumes up to 2 non-flag args
        let input = args(&[
            "claude-vm",
            "agent",
            "--worktree",
            "feature",
            "main",
            "/clear",
        ]);
        let expected = args(&["claude-vm", "agent", "--worktree=feature,main", "/clear"]);
        let output = route_args(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_worktree_single_arg_behavior() {
        // Without -- and with only 2 args total, both get consumed
        // Users should use -- if they want only 1 arg for worktree
        let input = args(&["claude-vm", "agent", "--worktree", "feature", "/clear"]);
        let expected = args(&["claude-vm", "agent", "--worktree=feature,/clear"]);
        let output = route_args(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_worktree_stops_at_double_dash() {
        let input = args(&[
            "claude-vm",
            "agent",
            "--worktree",
            "feature",
            "--",
            "/clear",
        ]);
        let expected = args(&["claude-vm", "agent", "--worktree=feature", "--", "/clear"]);
        let output = route_args(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_worktree_stops_at_flag() {
        let input = args(&[
            "claude-vm",
            "agent",
            "--worktree",
            "feature",
            "--disk",
            "50",
            "/clear",
        ]);
        let expected = args(&[
            "claude-vm",
            "agent",
            "--worktree=feature",
            "--disk",
            "50",
            "/clear",
        ]);
        let output = route_args(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_worktree_stops_at_short_flag() {
        let input = args(&[
            "claude-vm",
            "agent",
            "--worktree",
            "feature",
            "-v",
            "/clear",
        ]);
        let expected = args(&["claude-vm", "agent", "--worktree=feature", "-v", "/clear"]);
        let output = route_args(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_worktree_equals_syntax_unchanged() {
        let input = args(&["claude-vm", "agent", "--worktree=feature,main", "/clear"]);
        let output = route_args(input.clone());
        // Should be unchanged (except potential agent routing)
        assert!(
            output
                .iter()
                .any(|arg| arg.to_string_lossy() == "--worktree=feature,main"),
            "Equals syntax should be preserved"
        );
    }

    #[test]
    fn test_worktree_with_shell_command_and_separator() {
        let input = args(&["claude-vm", "shell", "--worktree", "feature", "--", "ls"]);
        let expected = args(&["claude-vm", "shell", "--worktree=feature", "--", "ls"]);
        let output = route_args(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_worktree_with_shell_command_no_separator() {
        // Without --, both args are consumed (feature and ls would be branch and base)
        // This is probably not what users want, but it's the trade-off of not using --
        let input = args(&["claude-vm", "shell", "--worktree", "feature", "ls"]);
        let expected = args(&["claude-vm", "shell", "--worktree=feature,ls"]);
        let output = route_args(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_worktree_no_args_kept_as_is() {
        let input = args(&["claude-vm", "agent", "--worktree"]);
        let expected = args(&["claude-vm", "agent", "--worktree"]);
        let output = route_args(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_worktree_before_routing_with_separator() {
        // Test that worktree normalization works with routing
        let input = args(&["claude-vm", "--worktree", "feature", "--", "/clear"]);
        let expected = args(&["claude-vm", "agent", "--worktree=feature", "--", "/clear"]);
        let output = route_args(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_worktree_two_args_before_routing() {
        let input = args(&["claude-vm", "--worktree", "feature", "main", "/clear"]);
        let expected = args(&["claude-vm", "agent", "--worktree=feature,main", "/clear"]);
        let output = route_args(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_multiple_worktree_flags() {
        // Edge case: multiple --worktree flags (though not useful in practice)
        let input = args(&[
            "claude-vm",
            "agent",
            "--worktree",
            "feature1",
            "--worktree",
            "feature2",
        ]);
        let expected = args(&[
            "claude-vm",
            "agent",
            "--worktree=feature1",
            "--worktree=feature2",
        ]);
        let output = route_args(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_worktree_consumes_max_two_args() {
        // Even with more non-flag args, should only consume 2
        let input = args(&[
            "claude-vm",
            "agent",
            "--worktree",
            "feature",
            "main",
            "extra",
            "/clear",
        ]);
        let expected = args(&[
            "claude-vm",
            "agent",
            "--worktree=feature,main",
            "extra",
            "/clear",
        ]);
        let output = route_args(input);
        assert_eq!(output, expected);
    }
}
