pub mod logs;
pub mod status;
pub mod test;

use crate::error::{ClaudeVmError, Result};
use crate::project::Project;
use crate::vm::limactl::LimaCtl;
use std::io::{self, Write};

/// Find running ephemeral VMs for a project
pub fn find_running_vms(project: &Project) -> Result<Vec<String>> {
    let template_prefix = format!("{}-", project.template_name());
    let all_vms = LimaCtl::list()?;

    let running_vms: Vec<String> = all_vms
        .into_iter()
        .filter(|vm| vm.status == "Running" && vm.name.starts_with(&template_prefix))
        .map(|vm| vm.name)
        .collect();

    Ok(running_vms)
}

/// Select a VM from the list, prompting the user if there are multiple
pub fn select_vm(vms: &[String]) -> Result<String> {
    match vms.len() {
        0 => Err(ClaudeVmError::CommandFailed(
            "No running VMs found".to_string(),
        )),
        1 => Ok(vms[0].clone()),
        _ => {
            // Multiple VMs - prompt user to select
            println!("Multiple running VMs found:");
            for (i, vm) in vms.iter().enumerate() {
                println!("  {}. {}", i + 1, vm);
            }
            println!();
            print!("Select VM (1-{}): ", vms.len());
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            let selection = input.trim().parse::<usize>().ok().and_then(|n| {
                if n > 0 && n <= vms.len() {
                    Some(n - 1)
                } else {
                    None
                }
            });

            match selection {
                Some(idx) => Ok(vms[idx].clone()),
                None => Err(ClaudeVmError::CommandFailed(
                    "Invalid selection".to_string(),
                )),
            }
        }
    }
}
