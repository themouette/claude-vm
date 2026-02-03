//! Executor for agent lifecycle scripts.

use super::definition::{Agent, ScriptConfig};
use crate::config::Config;
use crate::error::{ClaudeVmError, Result};
use crate::project::Project;
use crate::scripts::runner;
use crate::vm::limactl::LimaCtl;
use std::sync::Arc;

/// Execute an agent's installation script
pub fn install_agent(project: &Project, agent: &Arc<Agent>) -> Result<()> {
    let Some(install) = &agent.install else {
        return Ok(());
    };

    println!("Installing {}...", agent.agent.name);

    let script_content = get_script_content(install, &agent.agent.id)?;
    let filename = format!("{}_install.sh", agent.agent.id);

    runner::execute_script(project.template_name(), &script_content, &filename)?;

    Ok(())
}

/// Execute an agent's authentication script
pub fn authenticate_agent(project: &Project, agent: &Arc<Agent>) -> Result<()> {
    if !agent.agent.requires_authentication {
        return Ok(());
    }

    let Some(authenticate) = &agent.authenticate else {
        println!(
            "Note: {} may require authentication on first use",
            agent.agent.name
        );
        return Ok(());
    };

    println!("Setting up {} authentication...", agent.agent.name);

    let script_content = get_script_content(authenticate, &agent.agent.id)?;
    let filename = format!("{}_authenticate.sh", agent.agent.id);

    runner::execute_script(project.template_name(), &script_content, &filename)?;

    Ok(())
}

/// Get deployment script content from agent
pub fn get_deploy_script_content(agent: &Arc<Agent>) -> Result<String> {
    let Some(deploy) = &agent.deploy else {
        return Err(ClaudeVmError::InvalidConfig(format!(
            "Agent {} missing deployment script",
            agent.agent.id
        )));
    };

    get_script_content(deploy, &agent.agent.id)
}

/// Verify that all required capabilities are enabled
pub fn verify_requirements(agent: &Arc<Agent>, config: &Config) -> Result<()> {
    for cap in &agent.requires.capabilities {
        if !config.tools.is_enabled(cap) {
            return Err(ClaudeVmError::InvalidConfig(format!(
                "Agent '{}' requires capability '{}' to be enabled\n\
                 Enable it with: claude-vm setup --{}",
                agent.agent.id, cap, cap
            )));
        }
    }
    Ok(())
}

/// Get script content from config (either inline or from embedded file)
fn get_script_content(script_config: &ScriptConfig, agent_id: &str) -> Result<String> {
    if let Some(inline) = &script_config.script {
        return Ok(inline.clone());
    }

    if let Some(file) = &script_config.script_file {
        return get_embedded_script(agent_id, file);
    }

    Err(ClaudeVmError::InvalidConfig(
        "Script config must have either 'script' or 'script_file'".to_string(),
    ))
}

/// Get embedded script content by agent ID and script filename
fn get_embedded_script(agent_id: &str, script_name: &str) -> Result<String> {
    let content = match (agent_id, script_name) {
        ("claude", "install.sh") => include_str!("../../agents/claude/install.sh"),
        ("claude", "authenticate.sh") => include_str!("../../agents/claude/authenticate.sh"),
        ("claude", "deploy.sh") => include_str!("../../agents/claude/deploy.sh"),
        ("opencode", "install.sh") => include_str!("../../agents/opencode/install.sh"),
        ("opencode", "deploy.sh") => include_str!("../../agents/opencode/deploy.sh"),
        _ => {
            return Err(ClaudeVmError::InvalidConfig(format!(
                "Embedded script '{}' not found for agent '{}'",
                script_name, agent_id
            )))
        }
    };

    Ok(content.to_string())
}
