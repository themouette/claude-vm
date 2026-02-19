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

/// Prints one tree row.
///
/// `prefix`    — characters printed before the connector (`│   ` or `    `)
/// `connector` — `├── ` or `└── `
/// `indicator` — running indicator (2 chars)
/// `name`      — VM name
/// `suffix`    — optional right-aligned columns (disk usage mode)
fn print_row(prefix: &str, connector: &str, indicator: &str, name: &str, suffix: &str) {
    if suffix.is_empty() {
        println!("{}{}{}{}", prefix, connector, indicator, name);
    } else {
        // Name column is 50 chars wide minus the prefix/connector overhead so
        // the right-hand columns stay aligned regardless of nesting depth.
        let name_width = 50usize.saturating_sub(prefix.len() + connector.len());
        println!(
            "{}{}{}{:<width$}{}",
            prefix,
            connector,
            indicator,
            name,
            suffix,
            width = name_width
        );
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
            "{:<50} {:>10} {:>15}",
            "TEMPLATE / SESSION", "SIZE", "LAST USED"
        );
        println!("{}", "-".repeat(77));
    }

    let tpl_last = template_names.len().saturating_sub(1);

    for (ti, template_name) in template_names.iter().enumerate() {
        let is_last_tpl = ti == tpl_last;
        let tpl_connector = if is_last_tpl {
            "└── "
        } else {
            "├── "
        };

        let is_running = all_vms
            .iter()
            .find(|vm| vm.name == *template_name)
            .map(|vm| vm.status == "Running")
            .unwrap_or(false);

        let suffix = if disk_usage {
            let size = template::get_disk_usage(template_name);
            let last_used = template::format_last_used(template_name);
            format!(" {:>10} {:>15}", size, last_used)
        } else {
            String::new()
        };

        print_row(
            "",
            tpl_connector,
            running_indicator(is_running),
            template_name,
            &suffix,
        );

        // Session VMs — vertical bar continues only if this template is not last.
        let session_prefix = if is_last_tpl { "    " } else { "│   " };
        let sessions = template::list_sessions_for(template_name, &all_vms);
        let ses_last = sessions.len().saturating_sub(1);

        for (si, session) in sessions.iter().enumerate() {
            let ses_connector = if si == ses_last {
                "└── "
            } else {
                "├── "
            };
            let ses_suffix = if disk_usage {
                let size = template::get_disk_usage(&session.name);
                let last_used = template::format_last_used(&session.name);
                format!(" {:>10} {:>15}", size, last_used)
            } else {
                String::new()
            };
            print_row(
                session_prefix,
                ses_connector,
                running_indicator(session.status == "Running"),
                &session.name,
                &ses_suffix,
            );
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
    fn test_print_row_no_suffix() {
        // smoke-test: just ensure it doesn't panic
        print_row("│   ", "├── ", "  ", "claude-tpl_proj_abc1234", "");
        print_row(
            "    ",
            "└── ",
            "\x1b[32m•\x1b[0m ",
            "claude-tpl_proj_abc1234",
            "",
        );
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
