use crate::config::Config;
use crate::error::Result;
use crate::project::Project;
use crate::vm::limactl::LimaCtl;
use crate::vm::template;

pub fn execute() -> Result<()> {
    let project = Project::detect()?;
    let config = Config::load(project.root())?;

    println!("Project Information:");
    println!("  Path: {}", project.root().display());
    println!("  Template: {}", project.template_name());

    // Check if template exists
    let exists = template::exists(project.template_name())?;
    if !exists {
        println!("  Status: Not created");
        println!("\nRun 'claude-vm setup' to create the template.");
        return Ok(());
    }

    // Get VM status
    let vms = LimaCtl::list()?;
    let vm_info = vms.iter().find(|vm| vm.name == project.template_name());

    if let Some(info) = vm_info {
        println!("  Status: {}", info.status);
    } else {
        println!("  Status: Unknown");
    }

    // Show configuration
    println!("\nConfiguration:");
    println!("  Disk: {}GB", config.vm.disk);
    println!("  Memory: {}GB", config.vm.memory);

    // Show enabled capabilities
    let enabled_capabilities: Vec<String> = vec![
        ("docker", config.tools.docker),
        ("node", config.tools.node),
        ("python", config.tools.python),
        ("chromium", config.tools.chromium),
        ("gpg", config.tools.gpg),
        ("gh", config.tools.gh),
        ("git", config.tools.git),
    ]
    .into_iter()
    .filter_map(|(name, enabled)| {
        if enabled {
            Some(name.to_string())
        } else {
            None
        }
    })
    .collect();

    if !enabled_capabilities.is_empty() {
        println!("  Capabilities: {}", enabled_capabilities.join(", "));
    }

    // Show mounts
    if !config.mounts.is_empty() {
        println!("\nMounts:");
        for mount in &config.mounts {
            let mode = if mount.writable { "rw" } else { "ro" };
            if let Some(ref mount_point) = mount.mount_point {
                println!("  - {} -> {} ({})", mount.location, mount_point, mode);
            } else {
                println!("  - {} ({})", mount.location, mode);
            }
        }
    }

    // Show runtime scripts
    if !config.runtime.scripts.is_empty() {
        println!("\nRuntime Scripts:");
        for script in &config.runtime.scripts {
            println!("  - {}", script);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_info_function_signature() {
        // Verify the execute function has the correct signature
        // This ensures the public API is stable
        let _execute_fn: fn() -> Result<()> = execute;
    }

    #[test]
    fn test_info_uses_project_and_config() {
        // This test verifies that the info module has access to
        // the required types and modules
        // Actual execution would require a project setup and Lima

        // Verify imports compile correctly
        fn _check_types() {
            let _project_type: Option<Project> = None;
            let _config_type: Option<Config> = None;
        }
    }
}
