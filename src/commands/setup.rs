use crate::capabilities;
use crate::config::Config;
use crate::error::{ClaudeVmError, Result};
use crate::project::Project;
use crate::scripts::runner;
use crate::vm::{limactl::LimaCtl, mount, template};
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

    // Install agent (Claude by default)
    let agent = &config.defaults.agent;
    install_claude(project)?; // TODO: Make this agent-agnostic

    // Configure agent infrastructure
    store_agent_metadata(project, agent)?;
    create_agent_deployment_script(project)?;
    create_agent_wrapper(project)?;

    // Authenticate agent
    authenticate_claude(project)?; // TODO: Make this agent-agnostic

    // Register all MCP servers from capabilities to registry
    capabilities::register_all_mcp_servers(project, config)?;

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

    LimaCtl::shell(
        project.template_name(),
        None,
        "sudo",
        &["DEBIAN_FRONTEND=noninteractive", "apt-get", "update"],
        false,
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

fn store_agent_metadata(project: &Project, agent: &str) -> Result<()> {
    println!("Storing agent metadata...");

    let metadata = format!(
        r#"#!/bin/bash
# Agent metadata - auto-generated by claude-vm setup
export CLAUDE_VM_AGENT="{}"
"#,
        agent
    );

    let script = format!(
        r#"
cat > /tmp/agent-metadata << 'EOF'
{}
EOF
sudo mkdir -p /usr/local/share/claude-vm
sudo mv /tmp/agent-metadata /usr/local/share/claude-vm/agent
sudo chmod +r /usr/local/share/claude-vm/agent
echo "Agent metadata stored: {}"
"#,
        metadata, agent
    );

    LimaCtl::shell(
        project.template_name(),
        None,
        "bash",
        &["-c", &script],
        false,
    )?;

    Ok(())
}

fn create_agent_deployment_script(project: &Project) -> Result<()> {
    println!("Creating agent deployment script...");

    let script = r#"#!/bin/bash
# Agent deployment functions - auto-generated by claude-vm setup

deploy_context() {
    local context_file="$1"

    case "$CLAUDE_VM_AGENT" in
        claude)
            echo "Deploying context for Claude..."
            mkdir -p ~/.claude
            mv "$context_file" ~/.claude/CLAUDE.md
            ;;
        opencode)
            echo "Deploying context for Opencode..."
            mkdir -p ~/.config/opencode
            mv "$context_file" ~/.config/opencode/AGENTS.md
            ;;
        *)
            echo "ERROR: Unknown agent: $CLAUDE_VM_AGENT"
            return 1
            ;;
    esac
}

deploy_mcp() {
    local mcp_file="$1"

    # Skip if no MCP config provided
    if [ ! -f "$mcp_file" ]; then
        echo "No MCP configuration to deploy"
        return 0
    fi

    case "$CLAUDE_VM_AGENT" in
        claude)
            echo "Deploying MCP config for Claude..."
            mkdir -p ~/.claude
            mv "$mcp_file" ~/.claude.json
            ;;
        opencode)
            echo "Deploying MCP config for Opencode..."
            mkdir -p ~/.config/opencode

            # OpenCode uses opencode.json with mcpServers section
            if [ -f ~/.config/opencode/opencode.json ]; then
                # Merge MCP servers into existing config
                jq -s '.[0] * {mcpServers: .[1].mcpServers}' \
                    ~/.config/opencode/opencode.json "$mcp_file" \
                    > ~/.config/opencode/opencode.json.tmp
                mv ~/.config/opencode/opencode.json.tmp ~/.config/opencode/opencode.json
                rm "$mcp_file"
            else
                # Create new config with MCP servers
                echo '{}' | jq --slurpfile mcp "$mcp_file" \
                    '. + {mcpServers: $mcp[0].mcpServers}' \
                    > ~/.config/opencode/opencode.json
                rm "$mcp_file"
            fi
            ;;
        *)
            echo "ERROR: Unknown agent for MCP deployment: $CLAUDE_VM_AGENT"
            return 1
            ;;
    esac
}
"#;

    let install_script = format!(
        r#"
cat > /tmp/agent-deploy.sh << 'EOF'
{}
EOF
sudo mkdir -p /usr/local/share/claude-vm
sudo mv /tmp/agent-deploy.sh /usr/local/share/claude-vm/agent-deploy.sh
sudo chmod +rx /usr/local/share/claude-vm/agent-deploy.sh
echo "Agent deployment script installed"
"#,
        script
    );

    LimaCtl::shell(
        project.template_name(),
        None,
        "bash",
        &["-c", &install_script],
        false,
    )?;

    Ok(())
}

fn create_agent_wrapper(project: &Project) -> Result<()> {
    println!("Creating agent wrapper...");

    let wrapper = r#"#!/bin/bash
# Agent wrapper - auto-generated by claude-vm setup

# Load agent metadata
if [ -f /usr/local/share/claude-vm/agent ]; then
    source /usr/local/share/claude-vm/agent
else
    echo "ERROR: Agent metadata not found"
    exit 1
fi

# Execute the configured agent
exec "$CLAUDE_VM_AGENT" "$@"
"#;

    let install_script = format!(
        r#"
cat > /tmp/agent << 'EOF'
{}
EOF
sudo mv /tmp/agent /usr/local/bin/agent
sudo chmod +x /usr/local/bin/agent
echo "Agent wrapper installed at /usr/local/bin/agent"
"#,
        wrapper
    );

    LimaCtl::shell(
        project.template_name(),
        None,
        "bash",
        &["-c", &install_script],
        false,
    )?;

    Ok(())
}
