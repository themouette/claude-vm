/// Escape a string for safe use in shell single quotes
/// Converts: foo'bar -> 'foo'\''bar'
///
/// This ensures that arguments with spaces, special characters, or quotes
/// are properly escaped when building shell commands.
///
/// # Examples
///
/// ```
/// use claude_vm::utils::shell::escape;
///
/// assert_eq!(escape("hello"), "'hello'");
/// assert_eq!(escape("hello world"), "'hello world'");
/// assert_eq!(escape("foo'bar"), "'foo'\\''bar'");
/// ```
pub fn escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// Join command arguments with proper shell escaping
///
/// Each argument is escaped using single quotes to prevent word splitting
/// and special character interpretation.
///
/// # Examples
///
/// ```
/// use claude_vm::utils::shell::join_args;
///
/// let args = vec!["echo", "hello world"];
/// assert_eq!(join_args(&args), "'echo' 'hello world'");
///
/// let args = vec!["rm", "file with spaces.txt"];
/// assert_eq!(join_args(&args), "'rm' 'file with spaces.txt'");
/// ```
pub fn join_args(args: &[impl AsRef<str>]) -> String {
    args.iter()
        .map(|arg| escape(arg.as_ref()))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_simple() {
        assert_eq!(escape("hello"), "'hello'");
    }

    #[test]
    fn test_escape_with_spaces() {
        assert_eq!(escape("hello world"), "'hello world'");
    }

    #[test]
    fn test_escape_with_single_quote() {
        assert_eq!(escape("foo'bar"), "'foo'\\''bar'");
    }

    #[test]
    fn test_escape_with_special_chars() {
        assert_eq!(escape("$(whoami)"), "'$(whoami)'");
    }

    #[test]
    fn test_join_args_simple() {
        let args = vec!["echo", "hello"];
        assert_eq!(join_args(&args), "'echo' 'hello'");
    }

    #[test]
    fn test_join_args_with_spaces() {
        let args = vec!["echo", "hello world"];
        assert_eq!(join_args(&args), "'echo' 'hello world'");
    }

    #[test]
    fn test_join_args_with_quotes() {
        let args = vec!["echo", "it's"];
        assert_eq!(join_args(&args), "'echo' 'it'\\''s'");
    }

    #[test]
    fn test_join_args_injection_protection() {
        let args = vec!["rm", "$(rm -rf /)"];
        assert_eq!(join_args(&args), "'rm' '$(rm -rf /)'");
    }
}
