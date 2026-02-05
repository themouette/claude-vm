use crate::error::{ClaudeVmError, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Parse environment variables from CLI arguments
pub fn parse_env_args(env_args: &[String]) -> Result<HashMap<String, String>> {
    let mut env_vars = HashMap::new();

    for arg in env_args {
        if let Some((key, value)) = arg.split_once('=') {
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
            env_vars.insert(key.trim().to_string(), value.trim().to_string());
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
pub fn build_export_commands(env_vars: &HashMap<String, String>) -> String {
    let mut exports = Vec::new();

    for (key, value) in env_vars {
        // Escape single quotes in the value
        let escaped_value = value.replace('\'', "'\\''");
        exports.push(format!("export {}='{}'", key, escaped_value));
    }

    exports.join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_build_export_commands() {
        let mut vars = HashMap::new();
        vars.insert("KEY1".to_string(), "value1".to_string());
        vars.insert("KEY2".to_string(), "value's".to_string());

        let exports = build_export_commands(&vars);
        assert!(exports.contains("export KEY1='value1'"));
        assert!(exports.contains("export KEY2='value'\\''s'"));
    }
}
