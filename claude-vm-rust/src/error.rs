use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClaudeVmError {
    #[error("Template not found for project: {0}")]
    TemplateNotFound(String),

    #[error("Lima not installed. Install from https://lima-vm.io")]
    LimaNotInstalled,

    #[error("Script file not found: {0}")]
    ScriptNotFound(PathBuf),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Lima subprocess failed: {0}")]
    LimaExecution(String),

    #[error("Config parse error: {0}")]
    ConfigParse(#[from] toml::de::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Project detection failed: {0}")]
    ProjectDetection(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Command failed: {0}")]
    CommandFailed(String),
}

pub type Result<T> = std::result::Result<T, ClaudeVmError>;
