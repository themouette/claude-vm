use crate::error::Result;
use crate::project::Project;
use crate::vm::template;

pub fn execute(project: &Project) -> Result<()> {
    println!("Cleaning template: {}", project.template_name());

    if !template::exists(project.template_name())? {
        println!("Template does not exist: {}", project.template_name());
        return Ok(());
    }

    template::delete(project.template_name())?;
    println!("Template cleaned successfully: {}", project.template_name());

    Ok(())
}
