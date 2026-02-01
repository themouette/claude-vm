use crate::capabilities;
use crate::config::Config;
use crate::error::{ClaudeVmError, Result};
use crate::project::Project;
use crate::scripts::runner;
use crate::vm::{limactl::LimaCtl, template};
use std::path::Path;

pub fn execute(project: &Project, config: &Config) -> Result<()> {
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

    // Install optional tools via capability system (vm_setup hooks)
    capabilities::execute_vm_setup(project, config)?;

    // Install vm_runtime scripts into template
    capabilities::install_vm_runtime_scripts(project, config)?;

    // Install Claude Code
    install_claude(project)?;

    // Authenticate Claude
    authenticate_claude(project)?;

    // Configure all MCP servers from capabilities
    capabilities::configure_mcp_servers(project, config)?;

    // Run user-defined setup scripts
    run_setup_scripts(project, config)?;

    // Stop template
    println!("Stopping template VM...");
    LimaCtl::stop(project.template_name(), true)?; // Always verbose for setup

    println!("\nTemplate ready for project: {}", project.root().display());
    println!("Run 'claude-vm' in this project directory to use it.");

    Ok(())
}

fn create_base_template(project: &Project, config: &Config) -> Result<()> {
    println!("Creating base template VM...");

    // Collect port forwards from enabled capabilities
    let port_forwards = capabilities::get_port_forwards(config)?;

    if !port_forwards.is_empty() {
        println!("Configuring {} port forward(s)...", port_forwards.len());
    }

    // Use Debian 13 template
    LimaCtl::create(
        project.template_name(),
        "debian-13",
        config.vm.disk,
        config.vm.memory,
        &port_forwards,
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

    LimaCtl::shell(project.template_name(), None, "bash", &["-c", &cmd])?;

    Ok(())
}

fn disable_needrestart(project: &Project) -> Result<()> {
    println!("Configuring system...");

    let cmd = r#"mkdir -p /etc/needrestart/conf.d && echo '$nrconf{restart} = '"'"'a'"'"';' > /etc/needrestart/conf.d/no-prompt.conf"#;

    LimaCtl::shell(project.template_name(), None, "sudo", &["bash", "-c", cmd])?;

    Ok(())
}

fn install_base_packages(project: &Project) -> Result<()> {
    println!("Installing base packages...");

    LimaCtl::shell(
        project.template_name(),
        None,
        "sudo",
        &["DEBIAN_FRONTEND=noninteractive", "apt-get", "update"],
    )?;

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
    )?;

    // Add to PATH
    let cmd = r#"echo "export PATH=$HOME/.local/bin:$HOME/.claude/local/bin:$PATH" >> ~/.bashrc"#;
    LimaCtl::shell(project.template_name(), None, "bash", &["-c", cmd])?;

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
    )?;

    Ok(())
}

// Removed: configure_chrome_mcp - now handled by capability system

fn run_setup_scripts(project: &Project, config: &Config) -> Result<()> {
    // Standard setup script locations
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
            runner::execute_script_file(project.template_name(), script_path)?;
        }
    }

    // Custom setup scripts from config
    for script_path_str in &config.setup.scripts {
        let script_path = Path::new(script_path_str);
        if script_path.exists() {
            println!("Running custom setup script: {}", script_path.display());
            runner::execute_script_file(project.template_name(), script_path)?;
        } else {
            eprintln!("Warning: Setup script not found: {}", script_path_str);
        }
    }

    Ok(())
}
