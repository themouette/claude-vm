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
    fn test_list_commands_exist() {
        // This test just verifies the module compiles
        assert!(true);
    }
}
