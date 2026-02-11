use crate::capabilities;
use crate::config::Config;
use crate::error::{ClaudeVmError, Result};
use crate::project::Project;
use crate::scripts::runner;
use crate::vm::{limactl::LimaCtl, mount, template};
use std::path::Path;

pub fn execute(project: &Project, config: &Config, no_agent_install: bool) -> Result<()> {
    // Check if Lima is installed
    if !LimaCtl::is_installed() {
        return Err(ClaudeVmError::LimaNotInstalled);
    }

    println!(
        "Setting up template for project: {}",
        project.root().display()
    );
    println!("Template name: {}", project.template_name());

    // Clean old template if it exists
    if template::exists(project.template_name())? {
        println!("Removing existing template...");
        template::delete(project.template_name())?;
    }

    // Create base template
    create_base_template(project, config)?;

    // Run the setup process and clean up on failure
    match run_setup_process(project, config, no_agent_install) {
        Ok(()) => {
            println!("\nTemplate ready for project: {}", project.root().display());
            println!("Run 'claude-vm' in this project directory to use it.");
            Ok(())
        }
        Err(e) => {
            eprintln!("\nSetup failed: {}", e);
            eprintln!("Cleaning up template...");

            // Try to stop the VM if it's running
            if let Err(stop_err) = LimaCtl::stop(project.template_name(), false) {
                eprintln!("Warning: Failed to stop template VM: {}", stop_err);
            }

            // Delete the template
            if let Err(del_err) = template::delete(project.template_name()) {
                eprintln!("Warning: Failed to delete template: {}", del_err);
            } else {
                eprintln!("Template cleaned up successfully.");
            }

            Err(e)
        }
    }
}

fn run_setup_process(project: &Project, config: &Config, no_agent_install: bool) -> Result<()> {
    // Start the VM
    println!("Starting template VM...");
    LimaCtl::start(project.template_name(), true)?; // Always verbose for setup

    // Run host setup hooks for capabilities
    capabilities::execute_host_setup(project, config)?;

    // Store project metadata
    store_project_metadata(project)?;

    // Disable needrestart interactive prompts
    disable_needrestart(project)?;

    // Install base packages
    install_base_packages(project)?;

    // === THREE-PHASE PACKAGE MANAGEMENT ===

    // Phase 1: Setup custom repositories (Docker, Node, gh, etc.)
    capabilities::setup_repositories(project, config)?;

    // Phase 2: Batch install all packages in SINGLE apt-get call
    capabilities::install_system_packages(project, config)?;

    // === END PACKAGE MANAGEMENT ===

    // Execute vm_setup hooks (now primarily for post-install configuration)
    capabilities::execute_vm_setup(project, config)?;

    // Install vm_runtime scripts into template
    capabilities::install_vm_runtime_scripts(project, config)?;

    // Install Claude Code (skip if --no-agent-install flag is set)
    if !no_agent_install {
        install_claude(project)?;

        // Authenticate Claude
        authenticate_claude(project)?;

        // Configure all MCP servers from capabilities
        capabilities::configure_mcp_servers(project, config)?;
    } else {
        println!("Skipping Claude Code installation (--no-agent-install flag set)");
    }

    // Run user-defined setup scripts
    run_setup_scripts(project, config)?;

    // Stop template
    println!("Stopping template VM...");
    LimaCtl::stop(project.template_name(), true)?; // Always verbose for setup

    Ok(())
}

fn create_base_template(project: &Project, config: &Config) -> Result<()> {
    println!("Creating base template VM...");

    // Collect port forwards from enabled capabilities
    let port_forwards = capabilities::get_port_forwards(config)?;

    if !port_forwards.is_empty() {
        println!("Configuring {} port forward(s)...", port_forwards.len());
    }

    // Convert setup mounts from config using shared helper
    let setup_mounts = mount::convert_mount_entries(&config.setup.mounts)?;

    if !setup_mounts.is_empty() {
        println!("Configuring {} setup mount(s)...", setup_mounts.len());
    }

    // Use Debian 13 template with setup mounts
    LimaCtl::create(
        project.template_name(),
        "debian-13",
        config.vm.disk,
        config.vm.memory,
        &port_forwards,
        &setup_mounts,
        true, // Always verbose for setup
    )?;

    Ok(())
}

fn store_project_metadata(project: &Project) -> Result<()> {
    println!("Storing project metadata...");

    let project_root = project.root().to_string_lossy();
    let cmd = format!(
        "mkdir -p ~/.claude-vm && echo '{}' > ~/.claude-vm/project-root",
        project_root
    );

    LimaCtl::shell(project.template_name(), None, "bash", &["-c", &cmd], false)?;

    Ok(())
}

fn disable_needrestart(project: &Project) -> Result<()> {
    println!("Configuring system...");

    let cmd = r#"mkdir -p /etc/needrestart/conf.d && echo '$nrconf{restart} = '"'"'a'"'"';' > /etc/needrestart/conf.d/no-prompt.conf"#;

    LimaCtl::shell(
        project.template_name(),
        None,
        "sudo",
        &["bash", "-c", cmd],
        false,
    )?;

    Ok(())
}

fn install_base_packages(project: &Project) -> Result<()> {
    println!("Installing base packages...");

    // Note: No apt-get update needed here. Base packages are in default Debian repos
    // and Lima templates come with current package lists. We do a single apt-get update
    // later after repository setup scripts add custom sources.
    LimaCtl::shell(
        project.template_name(),
        None,
        "sudo",
        &[
            "DEBIAN_FRONTEND=noninteractive",
            "apt-get",
            "install",
            "-y",
            "git",
            "curl",
            "jq",
            "wget",
            "build-essential",
            "ripgrep",
            "fd-find",
            "htop",
            "unzip",
            "zip",
            "ca-certificates",
        ],
        false,
    )?;

    Ok(())
}

// Removed: install_optional_tools - now handled by capability system

fn install_claude(project: &Project) -> Result<()> {
    println!("Installing Claude Code...");

    LimaCtl::shell(
        project.template_name(),
        None,
        "bash",
        &["-c", "curl -fsSL https://claude.ai/install.sh | bash"],
        false,
    )?;

    // Add to PATH
    let cmd = r#"echo "export PATH=$HOME/.local/bin:$HOME/.claude/local/bin:$PATH" >> ~/.bashrc"#;
    LimaCtl::shell(project.template_name(), None, "bash", &["-c", cmd], false)?;

    Ok(())
}

fn authenticate_claude(project: &Project) -> Result<()> {
    println!("Setting up Claude authentication...");
    println!("(This will open a browser window for authentication)");

    LimaCtl::shell(
        project.template_name(),
        None,
        "bash",
        &["-lc", "claude 'Ok I am logged in, I can exit now.'"],
        false,
    )?;

    Ok(())
}

// Removed: configure_chrome_mcp - now handled by capability system

fn run_setup_scripts(project: &Project, config: &Config) -> Result<()> {
    let vm_name = project.template_name();

    // 1. Auto-detected file-based scripts (unchanged)
    let standard_scripts = vec![
        format!(
            "{}/.claude-vm.setup.sh",
            std::env::var("HOME").unwrap_or_default()
        ),
        format!("{}/.claude-vm.setup.sh", project.root().display()),
    ];

    for script_path_str in standard_scripts {
        let script_path = Path::new(&script_path_str);
        if script_path.exists() {
            println!("Running setup script: {}", script_path.display());
            runner::execute_script_file(vm_name, script_path)?;
        }
    }

    // 2. Legacy scripts (with deprecation warning)
    if !config.setup.scripts.is_empty() {
        eprintln!(
            "⚠ Warning: [setup] scripts array is deprecated. Please migrate to [[phase.setup]]"
        );
        eprintln!("   See: docs/configuration.md");

        for script_path_str in &config.setup.scripts {
            let script_path = Path::new(script_path_str);
            if !script_path.exists() {
                eprintln!("⚠ Warning: Setup script not found: {}", script_path_str);
                continue;
            }
            println!("Running custom setup script: {}", script_path.display());
            runner::execute_script_file(vm_name, script_path)?;
        }
    }

    // 3. New phase-based scripts
    for phase in &config.phase.setup {
        println!("\n━━━ Setup Phase: {} ━━━", phase.name);

        // Check conditional execution
        if !phase.should_execute(vm_name)? {
            println!("⊘ Skipped (condition not met: {:?})", phase.when);
            continue;
        }

        // Get all scripts for this phase
        let scripts = match phase.get_scripts(project.root()) {
            Ok(s) => s,
            Err(e) => {
                if phase.continue_on_error {
                    eprintln!("⚠ Warning: Phase '{}' failed: {}", phase.name, e);
                    eprintln!("  Continuing due to continue_on_error=true");
                    continue;
                } else {
                    return Err(e);
                }
            }
        };

        // Execute scripts in this phase
        for (script_name, content) in scripts {
            println!("  Running: {}", script_name);

            // Create environment with phase-specific vars
            let env_setup = phase
                .env
                .iter()
                .map(|(k, v)| format!("export {}='{}'", k, v.replace('\'', "'\\''")))
                .collect::<Vec<_>>()
                .join("\n");

            let full_script = if env_setup.is_empty() {
                content
            } else {
                format!("{}\n\n{}", env_setup, content)
            };

            match runner::execute_script(vm_name, &full_script, &script_name) {
                Ok(_) => println!("  ✓ Completed: {}", script_name),
                Err(e) => {
                    if phase.continue_on_error {
                        eprintln!("  ⚠ Warning: Script '{}' failed: {}", script_name, e);
                        eprintln!("    Continuing due to continue_on_error=true");
                    } else {
                        return Err(e);
                    }
                }
            }
        }
    }

    Ok(())
}
