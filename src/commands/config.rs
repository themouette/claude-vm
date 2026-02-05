use crate::cli::ConfigCommands;
use crate::config::Config;
use crate::error::Result;
use crate::project::Project;
use std::path::PathBuf;

pub fn execute(command: &ConfigCommands) -> Result<()> {
    match command {
        ConfigCommands::Validate => validate(),
        ConfigCommands::Show => show(),
    }
}

fn validate() -> Result<()> {
    let project = Project::detect()?;
    let project_config = project.root().join(".claude-vm.toml");
    let global_config = std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".claude-vm.toml"))
        .unwrap_or_else(|| PathBuf::from("~/.claude-vm.toml"));

    println!("Validating configuration files...\n");

    // Check if files exist
    if global_config.exists() {
        println!("  Global config: {}", global_config.display());
    } else {
        println!(
            "  Global config: {} - not found (optional)",
            global_config.display()
        );
    }

    if project_config.exists() {
        println!("  Project config: {}", project_config.display());
    } else {
        println!(
            "  Project config: {} - not found (optional)",
            project_config.display()
        );
    }

    // Try to load merged config - this will validate all files
    println!("\nLoading and validating configuration...");
    match Config::load(project.root()) {
        Ok(_) => {
            println!("✓ Configuration is valid!");
            Ok(())
        }
        Err(e) => {
            println!("✗ Configuration is invalid!");
            println!("  Error: {}", e);
            Err(e)
        }
    }
}

fn show() -> Result<()> {
    let project = Project::detect()?;
    let config = Config::load(project.root())?;

    println!("Effective Configuration:");
    println!("(CLI > Project config > Global config > Defaults)\n");

    println!("VM:");
    println!("  disk: {}GB", config.vm.disk);
    println!("  memory: {}GB", config.vm.memory);

    println!("\nTools:");
    println!("  docker: {}", config.tools.docker);
    println!("  node: {}", config.tools.node);
    println!("  python: {}", config.tools.python);
    println!("  chromium: {}", config.tools.chromium);
    println!("  gpg: {}", config.tools.gpg);
    println!("  gh: {}", config.tools.gh);
    println!("  git: {}", config.tools.git);

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

    if !config.runtime.scripts.is_empty() {
        println!("\nRuntime Scripts:");
        for script in &config.runtime.scripts {
            println!("  - {}", script);
        }
    }

    if !config.setup.scripts.is_empty() {
        println!("\nSetup Scripts:");
        for script in &config.setup.scripts {
            println!("  - {}", script);
        }
    }

    if !config.context.instructions.is_empty() {
        println!("\nContext Instructions:");
        println!("  {}", config.context.instructions);
    }

    if !config.context.instructions_file.is_empty() {
        println!("\nContext Instructions File:");
        println!("  {}", config.context.instructions_file);
    }

    println!("\nUpdate Check:");
    println!("  enabled: {}", config.update_check.enabled);
    println!("  interval: {} hours", config.update_check.interval_hours);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::ConfigCommands;

    #[test]
    fn test_config_commands_dispatch() {
        // Test that both command variants are handled
        // This verifies the execute() function has all match arms

        // We can't actually run these without a project setup,
        // but we can verify the match statement compiles correctly
        let _validate = ConfigCommands::Validate;
        let _show = ConfigCommands::Show;
    }

    #[test]
    fn test_config_module_exports() {
        // Verify the execute function is accessible
        // This ensures the public API is stable
        let _execute_fn: fn(&ConfigCommands) -> Result<()> = execute;
    }
}
