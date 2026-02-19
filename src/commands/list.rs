use crate::error::Result;
use crate::project::Project;
use crate::vm::template;

/// Returns a green `"• "` when the VM is running, `"  "` otherwise.
/// Both variants are 2 characters wide so names stay column-aligned.
fn running_indicator(is_running: bool) -> &'static str {
    if is_running {
        "\x1b[32m•\x1b[0m "
    } else {
        "  "
    }
}

pub fn execute(project: Option<&Project>, unused: bool, disk_usage: bool) -> Result<()> {
    // Fetch all VMs once — used for both status look-ups and session discovery.
    let all_vms = template::list_all_vms()?;

    // Determine which templates to display.
    let template_names: Vec<String> = if let Some(proj) = project {
        let name = proj.template_name().to_string();
        if all_vms.iter().any(|vm| vm.name == name) {
            vec![name]
        } else {
            vec![]
        }
    } else {
        template::list_all()?
    };

    if template_names.is_empty() {
        if project.is_some() {
            println!("No template found for this project.");
        } else {
            println!("No claude-vm templates found.");
        }
        return Ok(());
    }

    // Apply --unused filter to templates.
    let template_names: Vec<String> = if unused {
        template_names
            .into_iter()
            .filter(|name| template::is_unused(name))
            .collect()
    } else {
        template_names
    };

    if unused && template_names.is_empty() {
        println!("No unused templates found.");
        return Ok(());
    }

    if disk_usage {
        println!(
            "{:<52} {:>10} {:>15}",
            "TEMPLATE / SESSION", "SIZE", "LAST USED"
        );
        println!("{}", "-".repeat(79));
    }

    for template_name in &template_names {
        let is_running = all_vms
            .iter()
            .find(|vm| vm.name == *template_name)
            .map(|vm| vm.status == "Running")
            .unwrap_or(false);

        let indicator = running_indicator(is_running);

        if disk_usage {
            let size = template::get_disk_usage(template_name);
            let last_used = template::format_last_used(template_name);
            println!(
                "{}{:<50} {:>10} {:>15}",
                indicator, template_name, size, last_used
            );
        } else {
            println!("{}{}", indicator, template_name);
        }

        // Session VMs belonging to this template, indented one level.
        let sessions = template::list_sessions_for(template_name, &all_vms);
        for session in &sessions {
            let session_indicator = running_indicator(session.status == "Running");
            if disk_usage {
                let size = template::get_disk_usage(&session.name);
                let last_used = template::format_last_used(&session.name);
                println!(
                    "  {}{:<48} {:>10} {:>15}",
                    session_indicator, session.name, size, last_used
                );
            } else {
                println!("  {}{}", session_indicator, session.name);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_function_signature() {
        let _execute_fn: fn(Option<&Project>, bool, bool) -> Result<()> = execute;
    }

    #[test]
    fn test_running_indicator() {
        assert_eq!(running_indicator(true), "\x1b[32m•\x1b[0m ");
        assert_eq!(running_indicator(false), "  ");
    }

    #[test]
    fn test_list_flags_combinations() {
        let _unused_flag: bool = true;
        let _disk_usage_flag: bool = true;
        let _both_flags = (_unused_flag, _disk_usage_flag);
    }

    #[test]
    fn test_list_uses_template_module() {
        let template_name = "test-template";
        let _unused = template::is_unused(template_name);
        let _disk = template::get_disk_usage(template_name);
        let _last_used = template::format_last_used(template_name);
    }
}
