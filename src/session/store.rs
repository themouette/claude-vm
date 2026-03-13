use super::record::SessionRecord;
use crate::error::{ClaudeVmError, Result};
use crate::vm::limactl::LimaCtl;
use std::path::PathBuf;

/// Returns the directory where session records are stored: `~/.claude-vm/sessions/`
pub fn sessions_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME").map_err(|_| {
        ClaudeVmError::InvalidConfig("HOME environment variable not set".to_string())
    })?;
    Ok(PathBuf::from(home).join(".claude-vm").join("sessions"))
}

/// Persist a session record to disk.
pub fn create(record: &SessionRecord) -> Result<()> {
    let dir = sessions_dir()?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", record.id));
    let json = serde_json::to_string_pretty(record).map_err(|e| {
        ClaudeVmError::InvalidConfig(format!("Failed to serialize session record: {}", e))
    })?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Load a session record by ID.
pub fn get(id: &str) -> Result<SessionRecord> {
    let path = sessions_dir()?.join(format!("{}.json", id));
    let json = std::fs::read_to_string(&path)
        .map_err(|_| ClaudeVmError::InvalidConfig(format!("Session '{}' not found", id)))?;
    serde_json::from_str(&json).map_err(|e| {
        ClaudeVmError::InvalidConfig(format!("Failed to parse session record '{}': {}", id, e))
    })
}

/// List all session records, cross-referenced with Lima VM status.
///
/// Returns `(record, status)` pairs where `status` is the Lima VM status
/// (e.g. `"Running"`, `"Stopped"`) or `"gone"` if the VM no longer exists.
pub fn list() -> Result<Vec<(SessionRecord, String)>> {
    let dir = sessions_dir()?;
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let vms = LimaCtl::list().unwrap_or_default();

    let mut records = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let json = match std::fs::read_to_string(&path) {
            Ok(j) => j,
            Err(_) => continue,
        };
        let record: SessionRecord = match serde_json::from_str(&json) {
            Ok(r) => r,
            Err(_) => continue,
        };

        let status = vms
            .iter()
            .find(|vm| vm.name == record.vm_name)
            .map(|vm| vm.status.clone())
            .unwrap_or_else(|| "gone".to_string());

        records.push((record, status));
    }

    // Sort by created_at (oldest first)
    records.sort_by(|a, b| a.0.created_at.cmp(&b.0.created_at));

    Ok(records)
}

/// Delete a session record from disk.
pub fn delete(id: &str) -> Result<()> {
    let path = sessions_dir()?.join(format!("{}.json", id));
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

/// Remove session records whose VM no longer exists in Lima.
///
/// Returns the number of orphaned records removed.
pub fn prune_orphaned_records() -> Result<usize> {
    let dir = sessions_dir()?;
    if !dir.exists() {
        return Ok(0);
    }

    let vms = LimaCtl::list().unwrap_or_default();
    let existing_vm_names: std::collections::HashSet<String> =
        vms.into_iter().map(|vm| vm.name).collect();

    let mut removed = 0;
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let json = match std::fs::read_to_string(&path) {
            Ok(j) => j,
            Err(_) => continue,
        };
        let record: SessionRecord = match serde_json::from_str(&json) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if !existing_vm_names.contains(&record.vm_name) && std::fs::remove_file(&path).is_ok() {
            removed += 1;
        }
    }

    Ok(removed)
}

/// Remove session records matching a given template name.
///
/// Returns the number of records removed.
pub fn prune_records_for_template(template_name: &str) -> Result<usize> {
    let dir = sessions_dir()?;
    if !dir.exists() {
        return Ok(0);
    }

    let mut removed = 0;
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let json = match std::fs::read_to_string(&path) {
            Ok(j) => j,
            Err(_) => continue,
        };
        let record: SessionRecord = match serde_json::from_str(&json) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if record.template_name == template_name && std::fs::remove_file(&path).is_ok() {
            removed += 1;
        }
    }

    Ok(removed)
}
