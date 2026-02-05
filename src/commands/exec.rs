use crate::config::Config;
use crate::error::Result;
use crate::project::Project;
use crate::vm::limactl::LimaCtl;
use crate::vm::template;

pub fn execute(project: &Project, config: &Config, command: &[String]) -> Result<()> {
    // Verify template exists
    template::verify(project.template_name())?;

    if command.is_empty() {
        eprintln!("Error: No command specified");
        std::process::exit(1);
    }

    // Build command string
    let cmd_str = if command.len() == 1 {
        command[0].clone()
    } else {
        command.join(" ")
    };

    // Execute command in VM
    LimaCtl::shell(
        project.template_name(),
        Some(project.root()),
        "bash",
        &["-c", &cmd_str],
        config.forward_ssh_agent,
    )?;

    Ok(())
}
