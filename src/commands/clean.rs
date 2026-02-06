use crate::error::Result;
use crate::project::Project;
use crate::vm::template;
use std::io::{self, Write};

pub fn execute(project: &Project, yes: bool) -> Result<()> {
    if !template::exists(project.template_name())? {
        println!("Template does not exist: {}", project.template_name());
        return Ok(());
    }

    println!("Template: {}", project.template_name());
    println!("This will delete the template VM.");
    println!();

    // Prompt for confirmation unless --yes was provided
    if !yes {
        print!("Delete template? [y/N] ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let input = input.trim().to_lowercase();

        if input != "y" && input != "yes" {
            println!("Aborted.");
            return Ok(());
        }
    }

    println!("Cleaning template: {}", project.template_name());
    template::delete(project.template_name())?;
    println!("Template cleaned successfully: {}", project.template_name());

    Ok(())
}
