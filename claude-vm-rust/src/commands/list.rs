use crate::error::Result;
use crate::vm::template;

pub fn execute() -> Result<()> {
    let templates = template::list_all()?;

    if templates.is_empty() {
        println!("No claude-vm templates found.");
    } else {
        println!("Claude VM templates:");
        for template in templates {
            println!("  {}", template);
        }
    }

    Ok(())
}
