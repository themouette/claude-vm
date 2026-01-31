use crate::error::Result;
use crate::vm::template;

pub fn execute() -> Result<()> {
    let templates = template::list_all()?;

    if templates.is_empty() {
        println!("No claude-vm templates found.");
        return Ok(());
    }

    println!("Cleaning all claude-vm templates...");
    for template_name in templates {
        println!("  Cleaning: {}", template_name);
        template::delete(&template_name)?;
    }

    println!("All templates cleaned successfully.");
    Ok(())
}
