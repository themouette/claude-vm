use crate::error::{ClaudeVmError, Result};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

const INCLUDE_LINE: &str = "Include ~/.claude-vm/ssh/config";

fn home_dir() -> Result<PathBuf> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| ClaudeVmError::Io(io::Error::new(io::ErrorKind::NotFound, "HOME not set")))
}

fn managed_config_dir() -> Result<PathBuf> {
    Ok(home_dir()?.join(".claude-vm").join("ssh"))
}

fn managed_config_path() -> Result<PathBuf> {
    Ok(managed_config_dir()?.join("config"))
}

fn ssh_config_path() -> Result<PathBuf> {
    Ok(home_dir()?.join(".ssh").join("config"))
}

/// Idempotently add `Include ~/.claude-vm/ssh/config` to ~/.ssh/config.
/// Prompts user for consent if the Include is not already present.
pub fn ensure_ssh_include() -> Result<()> {
    let ssh_config = ssh_config_path()?;
    let ssh_dir = ssh_config.parent().unwrap();

    // Create ~/.ssh if missing
    if !ssh_dir.exists() {
        fs::create_dir_all(ssh_dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(ssh_dir, fs::Permissions::from_mode(0o700))?;
        }
    }

    // Check if Include already present
    if ssh_config.exists() {
        let content = fs::read_to_string(&ssh_config)?;
        if content.contains(INCLUDE_LINE) {
            return Ok(());
        }
    }

    // Prompt user
    eprint!(
        "claude-vm needs to add an Include directive to {}.\n\
         This allows VSCode Remote-SSH to connect to claude-vm sessions.\n\
         Proceed? [Y/n] ",
        ssh_config.display()
    );
    io::stderr().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();
    if !input.is_empty() && input != "y" && input != "yes" {
        return Err(ClaudeVmError::CommandFailed(
            "SSH config modification declined by user".to_string(),
        ));
    }

    // Prepend Include to ssh config (must be at top to take effect)
    let existing = if ssh_config.exists() {
        fs::read_to_string(&ssh_config)?
    } else {
        String::new()
    };

    let new_content = format!("{}\n\n{}", INCLUDE_LINE, existing);
    fs::write(&ssh_config, new_content)?;

    // Ensure managed config dir exists
    let managed_dir = managed_config_dir()?;
    if !managed_dir.exists() {
        fs::create_dir_all(&managed_dir)?;
    }

    Ok(())
}

/// Write SSH config block for a session VM. Returns the SSH host alias.
pub fn write_session_config(vm_name: &str) -> Result<String> {
    let ssh_config = super::limactl::LimaCtl::show_ssh_config(vm_name)?;

    // Extract host alias from the config block (line starting with "Host ")
    let host_alias = ssh_config
        .lines()
        .find(|line| line.starts_with("Host "))
        .map(|line| line.trim_start_matches("Host ").trim().to_string())
        .ok_or_else(|| {
            ClaudeVmError::LimaExecution(
                "Could not extract Host alias from SSH config".to_string(),
            )
        })?;

    let managed_dir = managed_config_dir()?;
    if !managed_dir.exists() {
        fs::create_dir_all(&managed_dir)?;
    }

    fs::write(managed_config_path()?, &ssh_config)?;

    Ok(host_alias)
}

/// Remove session SSH config (truncate managed file).
pub fn remove_session_config() {
    if let Ok(path) = managed_config_path() {
        let _ = fs::write(path, "");
    }
}
