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

    // If fewer than 2 args (just program name or empty), return unchanged
    if args.len() < 2 {
        return args;
    }

    let first_arg = args[1].to_string_lossy();

    // Preserve main --help and --version
    if first_arg == "--help" || first_arg == "-h" || first_arg == "--version" || first_arg == "-V" {
        return args;
    }

    // If first arg is a known subcommand, return unchanged
    if KNOWN_SUBCOMMANDS.contains(&first_arg.as_ref()) {
        return args;
    }

    // If first arg starts with '-' (any flag) OR is not a known subcommand,
    // insert "agent" after program name
    let mut routed = Vec::with_capacity(args.len() + 1);
    routed.push(args[0].clone());
    routed.push("agent".into());
    routed.extend_from_slice(&args[1..]);

    routed
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
    fn test_empty_args() {
        let input = args(&["claude-vm"]);
        let output = route_args(input.clone());
        assert_eq!(output, input);
    }

    #[test]
    fn test_no_args_at_all() {
        let input: Vec<OsString> = vec![];
        let output = route_args(input.clone());
        assert_eq!(output, input);
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
}
