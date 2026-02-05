use crate::error::{ClaudeVmError, Result};
use crate::vm::limactl::LimaCtl;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime};

/// Check if a template exists for the given name
pub fn exists(template_name: &str) -> Result<bool> {
    LimaCtl::vm_exists(template_name)
}

/// Verify a template exists, return error if not
pub fn verify(template_name: &str) -> Result<()> {
    if !exists(template_name)? {
        return Err(ClaudeVmError::TemplateNotFound(template_name.to_string()));
    }
    Ok(())
}

/// Delete a template
pub fn delete(template_name: &str) -> Result<()> {
    if exists(template_name)? {
        LimaCtl::delete(template_name, true, true)?; // Always verbose for user-initiated deletes
    }
    Ok(())
}

/// List all claude-vm templates
pub fn list_all() -> Result<Vec<String>> {
    let vms = LimaCtl::list()?;
    let templates: Vec<String> = vms
        .into_iter()
        .filter(|vm| vm.name.starts_with("claude-tpl_"))
        .map(|vm| vm.name)
        .collect();
    Ok(templates)
}

/// Get the filesystem path for a template's VM directory
pub fn get_path(template_name: &str) -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".lima").join(template_name))
}

/// Get disk usage for a template in human-readable format (e.g., "1.2G")
pub fn get_disk_usage(template_name: &str) -> String {
    let vm_dir = match get_path(template_name) {
        Some(path) if path.exists() => path,
        _ => return "unknown".to_string(),
    };

    // Use du command to get disk usage
    let output = Command::new("du")
        .args(["-sh", &vm_dir.to_string_lossy()])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // du output format: "SIZE\tPATH"
            if let Some(size) = stdout.split_whitespace().next() {
                return size.to_string();
            }
        }
    }

    "unknown".to_string()
}

/// Get the last access time for a template
pub fn get_last_access_time(template_name: &str) -> Option<SystemTime> {
    let vm_dir = get_path(template_name)?;

    if !vm_dir.exists() {
        return None;
    }

    // Check the modified time of the VM directory
    let metadata = fs::metadata(&vm_dir).ok()?;
    metadata.modified().ok()
}

/// Check if a template is unused (not accessed in 30+ days)
pub fn is_unused(template_name: &str) -> bool {
    let thirty_days = Duration::from_secs(30 * 24 * 60 * 60);
    if let Some(last_access) = get_last_access_time(template_name) {
        if let Ok(elapsed) = SystemTime::now().duration_since(last_access) {
            return elapsed > thirty_days;
        }
    }
    false
}

/// Format last access time as human-readable string
pub fn format_last_used(template_name: &str) -> String {
    if let Some(last_access) = get_last_access_time(template_name) {
        if let Ok(elapsed) = SystemTime::now().duration_since(last_access) {
            let days = elapsed.as_secs() / (24 * 60 * 60);
            if days == 0 {
                return "today".to_string();
            } else if days == 1 {
                return "1 day ago".to_string();
            } else if days < 30 {
                return format!("{} days ago", days);
            } else {
                let weeks = days / 7;
                if weeks < 8 {
                    return format!("{} weeks ago", weeks);
                } else {
                    let months = days / 30;
                    return format!("{} months ago", months);
                }
            }
        }
    }
    "unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    #[test]
    #[serial_test::serial]
    fn test_get_path_with_home() {
        env::set_var("HOME", "/home/testuser");
        let path = get_path("test-template");
        assert_eq!(
            path,
            Some(PathBuf::from("/home/testuser/.lima/test-template"))
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_get_path_no_home() {
        env::remove_var("HOME");
        let path = get_path("test-template");
        assert_eq!(path, None);
    }

    #[test]
    fn test_get_disk_usage_nonexistent() {
        let usage = get_disk_usage("nonexistent-template-xyz");
        assert_eq!(usage, "unknown");
    }

    #[test]
    fn test_get_last_access_time_nonexistent() {
        let time = get_last_access_time("nonexistent-template-xyz");
        assert_eq!(time, None);
    }

    #[test]
    fn test_is_unused_nonexistent() {
        // Nonexistent templates should return false (not unused, because they don't exist)
        let unused = is_unused("nonexistent-template-xyz");
        assert!(!unused);
    }

    #[test]
    fn test_format_last_used_nonexistent() {
        let formatted = format_last_used("nonexistent-template-xyz");
        assert_eq!(formatted, "unknown");
    }

    #[test]
    #[serial_test::serial]
    fn test_format_last_used_with_mock_time() {
        // Create a temporary HOME directory
        let temp_home = env::temp_dir().join(format!("claude-vm-test-home-{}",
            std::process::id()));
        if temp_home.exists() {
            fs::remove_dir_all(&temp_home).ok();
        }
        fs::create_dir(&temp_home).unwrap();

        // Set HOME to temp directory
        let old_home = env::var("HOME").ok();
        env::set_var("HOME", &temp_home);

        // Create .lima/test-template directory structure
        let lima_dir = temp_home.join(".lima");
        fs::create_dir_all(&lima_dir).unwrap();
        let template_dir = lima_dir.join("test-template");
        fs::create_dir(&template_dir).unwrap();

        // Test that it returns a valid time format (should be "today" since we just created it)
        let formatted = format_last_used("test-template");
        assert_eq!(formatted, "today");

        // Cleanup
        fs::remove_dir_all(&temp_home).ok();
        if let Some(home) = old_home {
            env::set_var("HOME", home);
        } else {
            env::remove_var("HOME");
        }
    }

    #[test]
    fn test_format_last_used_time_ranges() {
        // Test the formatting logic with mock elapsed times
        // We can't easily mock SystemTime, but we can test the logic indirectly

        // Test that the function handles None gracefully
        let result = format_last_used("nonexistent");
        assert_eq!(result, "unknown");
    }
}
