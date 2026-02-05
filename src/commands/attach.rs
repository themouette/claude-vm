use crate::config::Config;
use crate::error::Result;
use crate::project::Project;
use crate::vm::limactl::LimaCtl;
use crate::vm::template;

pub fn execute(project: &Project, config: &Config) -> Result<()> {
    // Verify template exists
    template::verify(project.template_name())?;

    println!("Attaching to VM: {}", project.template_name());

    // Open interactive shell
    LimaCtl::shell(
        project.template_name(),
        Some(project.root()),
        "bash",
        &[],
        config.forward_ssh_agent,
    )?;

    Ok(())
}
