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

    #[error("Command exited with status {0}")]
    CommandExitCode(i32),

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

    #[error("Update error: {0}")]
    UpdateError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Permission denied: {0}. Try running with sudo.")]
    PermissionDenied(String),

    #[error("Git worktree is locked: {reason}\nTo unlock, run: git worktree unlock {path}")]
    WorktreeLocked { reason: String, path: String },

    #[error("Git version {version} is too old. Worktrees require Git 2.5+.\nDownload the latest version: https://git-scm.com/downloads")]
    GitVersionTooOld { version: String },

    #[error("Repository uses submodules. Git worktree support for submodules is experimental.\nSee: https://git-scm.com/docs/git-worktree#_bugs")]
    SubmodulesDetected,

    #[error("Git worktree error: {0}")]
    Worktree(String),
}

impl From<self_update::errors::Error> for ClaudeVmError {
    fn from(err: self_update::errors::Error) -> Self {
        ClaudeVmError::UpdateError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ClaudeVmError>;
