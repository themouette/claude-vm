use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeConfig {
    /// Custom worktree base directory (default: None, meaning sibling directory `{repo_root}-worktrees/`)
    #[serde(default)]
    pub location: Option<String>,

    /// Path template for worktree naming (default: "{branch}")
    #[serde(default = "default_template")]
    pub template: String,
}

fn default_template() -> String {
    "{branch}".to_string()
}

impl Default for WorktreeConfig {
    fn default() -> Self {
        Self {
            location: None,
            template: default_template(),
        }
    }
}

impl WorktreeConfig {
    /// Validate configuration and return warnings (not errors - config is still usable)
    /// Following NetworkIsolationConfig::validate() pattern
    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        // Check if location path exists (if specified)
        if let Some(location) = &self.location {
            let path = std::path::Path::new(location);
            if !path.exists() {
                warnings.push(format!(
                    "Worktree location '{}' does not exist. It will be created when first worktree is added.",
                    location
                ));
            }
        }

        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WorktreeConfig::default();
        assert_eq!(config.location, None);
        assert_eq!(config.template, "{branch}");
    }

    #[test]
    fn test_deserialize_empty_table() {
        // Empty TOML (no fields set) should use defaults
        let toml = r#""#;

        let config: WorktreeConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.location, None);
        assert_eq!(config.template, "{branch}");
    }

    #[test]
    fn test_deserialize_with_location() {
        let toml = r#"
        location = "/tmp/worktrees"
        "#;

        let config: WorktreeConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.location, Some("/tmp/worktrees".to_string()));
        assert_eq!(config.template, "{branch}");
    }

    #[test]
    fn test_deserialize_with_template() {
        let toml = r#"
        template = "{date}-{branch}"
        "#;

        let config: WorktreeConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.location, None);
        assert_eq!(config.template, "{date}-{branch}");
    }

    #[test]
    fn test_deserialize_full() {
        let toml = r#"
        location = "/custom/path"
        template = "{feature}/{branch}"
        "#;

        let config: WorktreeConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.location, Some("/custom/path".to_string()));
        assert_eq!(config.template, "{feature}/{branch}");
    }

    #[test]
    fn test_validate_nonexistent_location_warns() {
        // Use a path that definitely doesn't exist
        let nonexistent = "/tmp/nonexistent-worktree-path-12345";
        let config = WorktreeConfig {
            location: Some(nonexistent.to_string()),
            template: "{branch}".to_string(),
        };

        let warnings = config.validate();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("does not exist"));
        assert!(warnings[0].contains(nonexistent));
    }

    #[test]
    fn test_validate_no_warnings_default() {
        let config = WorktreeConfig::default();
        let warnings = config.validate();
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_validate_existing_location_no_warning() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let config = WorktreeConfig {
            location: Some(temp_dir.path().to_string_lossy().to_string()),
            template: "{branch}".to_string(),
        };

        let warnings = config.validate();
        assert!(warnings.is_empty());
    }
}
