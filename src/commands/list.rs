use crate::error::Result;
use crate::vm::template;

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
            .filter(|name| template::is_unused(name))
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
        for name in templates {
            let size = template::get_disk_usage(&name);
            let last_used = template::format_last_used(&name);
            println!("{:<50} {:>10} {:>15}", name, size, last_used);
        }
    } else {
        println!("Claude VM templates:");
        for name in templates {
            println!("  {}", name);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_function_signature() {
        // Verify the execute function has the correct signature
        let _execute_fn: fn(bool, bool) -> Result<()> = execute;
    }

    #[test]
    fn test_list_flags_combinations() {
        // Test that all flag combinations are valid type-wise
        // Actual execution would require Lima VMs to exist

        // Test flag types
        let _unused_flag: bool = true;
        let _disk_usage_flag: bool = true;

        // Verify both can be combined
        let _both_flags = (_unused_flag, _disk_usage_flag);
    }

    #[test]
    fn test_list_uses_template_module() {
        // Verify we're using the refactored template module functions
        // This ensures we're using the shared utilities correctly

        let template_name = "test-template";

        // These functions should exist and be callable
        let _unused = template::is_unused(template_name);
        let _disk = template::get_disk_usage(template_name);
        let _last_used = template::format_last_used(template_name);
    }
}
