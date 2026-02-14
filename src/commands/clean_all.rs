use crate::error::Result;
use crate::vm::template;
use std::io::{self, Write};

pub fn execute(yes: bool) -> Result<()> {
    let templates = template::list_all()?;

    if templates.is_empty() {
        println!("No claude-vm templates found.");
        return Ok(());
    }

    // Show what will be deleted
    println!("The following templates will be deleted:");
    for template_name in &templates {
        println!("  - {}", template_name);
    }
    println!();

    // Prompt for confirmation unless --yes was provided
    if !yes {
        print!("Delete {} template(s)? [y/N] ", templates.len());
        let _ = io::stdout().flush();

        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let input = input.trim().to_lowercase();

        if input != "y" && input != "yes" {
            println!("Aborted.");
            return Ok(());
        }
    }

    println!("Cleaning all claude-vm templates...");
    for template_name in templates {
        println!("  Cleaning: {}", template_name);
        template::delete(&template_name)?;
    }

    println!("All templates cleaned successfully.");
    Ok(())
}
