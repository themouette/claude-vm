use crate::error::{ClaudeVmError, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Validate an environment variable key follows POSIX naming conventions.
///
/// Valid environment variable names:
/// - Must start with a letter (A-Z, a-z) or underscore
/// - May contain letters, digits, and underscores
/// - Cannot contain special characters that could enable command injection
///
/// # Security
///
/// This validation prevents command injection through malicious environment
/// variable names. Without validation, a key like `FOO; rm -rf /` could
/// execute arbitrary commands when used in shell scripts.
///
/// # Examples
///
/// ```
/// # use claude_vm::utils::env::validate_env_key;
/// assert!(validate_env_key("MY_VAR").is_ok());
/// assert!(validate_env_key("_PRIVATE").is_ok());
/// assert!(validate_env_key("VAR123").is_ok());
/// assert!(validate_env_key("123VAR").is_err());  // Can't start with digit
/// assert!(validate_env_key("MY-VAR").is_err());  // No dashes allowed
/// assert!(validate_env_key("VAR;cmd").is_err()); // No special chars
/// ```
pub fn validate_env_key(key: &str) -> Result<()> {
    if key.is_empty() {
        return Err(ClaudeVmError::InvalidEnvKey(
            "environment variable key cannot be empty".to_string(),
        ));
    }

    // Check first character: must be letter or underscore
    let first_char = key
        .chars()
        .next()
        .expect("key is not empty due to check above");

    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        return Err(ClaudeVmError::InvalidEnvKey(format!(
            "'{key}' must start with a letter or underscore, not '{first_char}'"
        )));
    }

    // Check remaining characters: must be alphanumeric or underscore
    for (i, c) in key.chars().enumerate() {
        if !c.is_ascii_alphanumeric() && c != '_' {
            return Err(ClaudeVmError::InvalidEnvKey(format!(
                "'{key}' contains invalid character '{c}' at position {i}. \
                 Only letters, digits, and underscores are allowed"
            )));
        }
    }

    Ok(())
}

/// Build a shell export statement with proper escaping.
///
/// Validates the key and escapes the value to prevent injection attacks.
///
/// # Security
///
/// - Key is validated to prevent command injection via variable names
/// - Value is escaped using POSIX shell single-quote escaping rules
/// - Single quotes in values are replaced with '\'' (end quote, escaped quote, start quote)
///
/// # Examples
///
/// ```
/// # use claude_vm::utils::env::build_env_export;
/// assert_eq!(
///     build_env_export("MY_VAR", "hello").unwrap(),
///     "export MY_VAR='hello'"
/// );
/// assert_eq!(
///     build_env_export("MY_VAR", "it's working").unwrap(),
///     "export MY_VAR='it'\\''s working'"
/// );
/// assert!(build_env_export("123", "value").is_err());
/// ```
pub fn build_env_export(key: &str, value: &str) -> Result<String> {
    validate_env_key(key)?;

    // Escape single quotes in the value using POSIX shell escaping
    // Replace ' with '\'' (end quote, escaped quote, start quote)
    let escaped_value = value.replace('\'', "'\\''");

    Ok(format!("export {}='{}'", key, escaped_value))
}

/// Parse environment variables from CLI arguments
pub fn parse_env_args(env_args: &[String]) -> Result<HashMap<String, String>> {
    let mut env_vars = HashMap::new();

    for arg in env_args {
        if let Some((key, value)) = arg.split_once('=') {
            // Validate key before accepting it
            validate_env_key(key)?;
            env_vars.insert(key.to_string(), value.to_string());
        } else {
            return Err(ClaudeVmError::InvalidConfig(format!(
                "Invalid env format: {}. Expected KEY=VALUE",
                arg
            )));
        }
    }

    Ok(env_vars)
}

/// Load environment variables from file
pub fn load_env_file(path: &Path) -> Result<HashMap<String, String>> {
    let content = fs::read_to_string(path).map_err(|e| {
        ClaudeVmError::InvalidConfig(format!("Failed to read env file {}: {}", path.display(), e))
    })?;

    let mut env_vars = HashMap::new();
    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            // Validate key before accepting it
            validate_env_key(key).map_err(|e| {
                ClaudeVmError::InvalidConfig(format!(
                    "Invalid env key at {}:{}: {}",
                    path.display(),
                    line_num + 1,
                    e
                ))
            })?;
            env_vars.insert(key.to_string(), value.trim().to_string());
        } else {
            return Err(ClaudeVmError::InvalidConfig(format!(
                "Invalid env format at {}:{}: {}",
                path.display(),
                line_num + 1,
                line
            )));
        }
    }

    Ok(env_vars)
}

/// Get inherited environment variables from host
pub fn get_inherited_vars(vars: &[String]) -> HashMap<String, String> {
    let mut env_vars = HashMap::new();

    for var in vars {
        if let Ok(value) = std::env::var(var) {
            env_vars.insert(var.clone(), value);
        }
    }

    env_vars
}

/// Build shell export commands from environment variables
///
/// Note: This function assumes keys are already validated. If keys come from
/// untrusted sources, use `build_env_export` for each key-value pair instead.
pub fn build_export_commands(env_vars: &HashMap<String, String>) -> String {
    let mut exports = Vec::new();

    for (key, value) in env_vars {
        // Keys should be pre-validated, but we validate here as defense-in-depth
        if validate_env_key(key).is_ok() {
            let escaped_value = value.replace('\'', "'\\''");
            exports.push(format!("export {}='{}'", key, escaped_value));
        }
        // Silently skip invalid keys to avoid breaking existing functionality
        // Keys should have been validated when added to the HashMap
    }

    exports.join("; ")
}

/// Collect all environment variables from CLI flags
pub fn collect_env_vars(
    env_args: &[String],
    env_files: &[std::path::PathBuf],
    inherit_vars: &[String],
) -> Result<HashMap<String, String>> {
    let mut env_vars = HashMap::new();

    // Load from env files (lowest priority)
    for file in env_files {
        env_vars.extend(load_env_file(file)?);
    }

    // Add --env args (medium priority)
    env_vars.extend(parse_env_args(env_args)?);

    // Add inherited vars (highest priority)
    env_vars.extend(get_inherited_vars(inherit_vars));

    Ok(env_vars)
}

/// Prepend environment exports to a command string
pub fn prepend_env_to_command(env_vars: &HashMap<String, String>, command: &str) -> String {
    if env_vars.is_empty() {
        command.to_string()
    } else {
        format!("{}; {}", build_export_commands(env_vars), command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_env_key_valid() {
        assert!(validate_env_key("VAR").is_ok());
        assert!(validate_env_key("MY_VAR").is_ok());
        assert!(validate_env_key("_PRIVATE").is_ok());
        assert!(validate_env_key("VAR123").is_ok());
        assert!(validate_env_key("VAR_123_ABC").is_ok());
        assert!(validate_env_key("a").is_ok());
        assert!(validate_env_key("_").is_ok());
    }

    #[test]
    fn test_validate_env_key_invalid_start() {
        assert!(validate_env_key("123VAR").is_err());
        assert!(validate_env_key("9VAR").is_err());
        assert!(validate_env_key("-VAR").is_err());
    }

    #[test]
    fn test_validate_env_key_invalid_chars() {
        assert!(validate_env_key("MY-VAR").is_err());
        assert!(validate_env_key("MY.VAR").is_err());
        assert!(validate_env_key("MY VAR").is_err());
        assert!(validate_env_key("MY$VAR").is_err());
        assert!(validate_env_key("MY;VAR").is_err());
        assert!(validate_env_key("MY=VAR").is_err());
        assert!(validate_env_key("MY|VAR").is_err());
        assert!(validate_env_key("MY&VAR").is_err());
        assert!(validate_env_key("MY`VAR").is_err());
    }

    #[test]
    fn test_validate_env_key_empty() {
        assert!(validate_env_key("").is_err());
    }

    #[test]
    fn test_build_env_export_simple() {
        let result = build_env_export("MY_VAR", "hello").unwrap();
        assert_eq!(result, "export MY_VAR='hello'");
    }

    #[test]
    fn test_build_env_export_with_quotes() {
        let result = build_env_export("MY_VAR", "it's working").unwrap();
        assert_eq!(result, "export MY_VAR='it'\\''s working'");
    }

    #[test]
    fn test_build_env_export_with_multiple_quotes() {
        let result = build_env_export("MY_VAR", "it's a 'test'").unwrap();
        assert_eq!(result, "export MY_VAR='it'\\''s a '\\''test'\\'''");
    }

    #[test]
    fn test_build_env_export_invalid_key() {
        assert!(build_env_export("123", "value").is_err());
        assert!(build_env_export("MY-VAR", "value").is_err());
        assert!(build_env_export("MY;rm -rf /", "value").is_err());
    }

    #[test]
    fn test_build_env_export_special_values() {
        // Test various special characters in values
        assert!(build_env_export("VAR", "a b c").is_ok());
        assert!(build_env_export("VAR", "a\nb").is_ok());
        assert!(build_env_export("VAR", "a$b").is_ok());
        assert!(build_env_export("VAR", "a`b`").is_ok());
        assert!(build_env_export("VAR", "a;b").is_ok());
    }

    #[test]
    fn test_parse_env_args() {
        let args = vec!["KEY1=value1".to_string(), "KEY2=value2".to_string()];
        let vars = parse_env_args(&args).unwrap();
        assert_eq!(vars.get("KEY1"), Some(&"value1".to_string()));
        assert_eq!(vars.get("KEY2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_parse_env_args_invalid() {
        let args = vec!["INVALID".to_string()];
        assert!(parse_env_args(&args).is_err());
    }

    #[test]
    fn test_parse_env_args_invalid_key() {
        let args = vec!["123=value".to_string()];
        assert!(parse_env_args(&args).is_err());

        let args = vec!["MY-VAR=value".to_string()];
        assert!(parse_env_args(&args).is_err());
    }

    #[test]
    fn test_build_export_commands() {
        let mut vars = HashMap::new();
        vars.insert("KEY1".to_string(), "value1".to_string());
        vars.insert("KEY2".to_string(), "value's".to_string());

        let exports = build_export_commands(&vars);
        assert!(exports.contains("export KEY1='value1'"));
        assert!(exports.contains("export KEY2='value'\\''s'"));
    }
}
