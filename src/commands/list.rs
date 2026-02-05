use crate::error::Result;
use crate::vm::template;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime};

pub fn execute(unused: bool, disk_usage: bool) -> Result<()> {
    let templates = template::list_all()?;

    if templates.is_empty() {
        println!("No claude-vm templates found.");
        return Ok(());
    }

    // Filter unused templates if requested
    let templates: Vec<String> = if unused {
        templates
            .into_iter()
            .filter(|name| is_unused(name))
            .collect()
    } else {
        templates
    };

    if unused && templates.is_empty() {
        println!("No unused templates found.");
        return Ok(());
    }

    // Display templates
    if disk_usage {
        println!("{:<50} {:>10} {:>15}", "TEMPLATE", "SIZE", "LAST USED");
        println!("{}", "-".repeat(77));
        for template in templates {
            let size = get_disk_usage(&template);
            let last_used = get_last_used(&template);
            println!("{:<50} {:>10} {:>15}", template, size, last_used);
        }
    } else {
        println!("Claude VM templates:");
        for template in templates {
            println!("  {}", template);
        }
    }

    Ok(())
}

fn is_unused(template_name: &str) -> bool {
    let thirty_days = Duration::from_secs(30 * 24 * 60 * 60);
    if let Some(last_access) = get_last_access_time(template_name) {
        if let Ok(elapsed) = SystemTime::now().duration_since(last_access) {
            return elapsed > thirty_days;
        }
    }
    false
}

fn get_last_access_time(template_name: &str) -> Option<SystemTime> {
    let home = std::env::var("HOME").ok()?;
    let vm_dir = PathBuf::from(home).join(".lima").join(template_name);

    if !vm_dir.exists() {
        return None;
    }

    // Check the modified time of the VM directory
    let metadata = fs::metadata(&vm_dir).ok()?;
    metadata.modified().ok()
}

fn get_last_used(template_name: &str) -> String {
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

fn get_disk_usage(template_name: &str) -> String {
    let home = match std::env::var("HOME") {
        Ok(h) => h,
        Err(_) => return "unknown".to_string(),
    };

    let vm_dir = PathBuf::from(home).join(".lima").join(template_name);

    if !vm_dir.exists() {
        return "unknown".to_string();
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_commands_exist() {
        // This test just verifies the module compiles
        assert!(true);
    }
}
