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
        let _ = io::stdout().flush();

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

    // Clean up persistent vscode-server data for this template
    if let Ok(home) = std::env::var("HOME") {
        let vscode_persist = std::path::PathBuf::from(home)
            .join(".claude-vm")
            .join("vscode-server")
            .join(project.template_name());
        let _ = std::fs::remove_dir_all(vscode_persist);
    }

    println!("Template cleaned successfully: {}", project.template_name());

    Ok(())
}
