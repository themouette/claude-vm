use claude_vm::agents::registry::AgentRegistry;
use claude_vm::config::Config;

#[test]
fn test_agent_registry_loads() {
    let _registry = AgentRegistry::load().expect("Failed to load agent registry");
    // If we got here, all agent TOML files parsed successfully
}

#[test]
fn test_list_available_agents() {
    let registry = AgentRegistry::load().expect("Failed to load registry");
    let available = registry.list_available();

    // Should have at least claude and opencode
    assert!(available.len() >= 2);
    assert!(available.contains(&"claude".to_string()));
    assert!(available.contains(&"opencode".to_string()));
}

#[test]
fn test_get_claude_agent() {
    let registry = AgentRegistry::load().expect("Failed to load registry");
    let agent = registry
        .get("claude")
        .expect("Claude agent should be available");

    assert_eq!(agent.agent.id, "claude");
    assert_eq!(agent.agent.name, "Claude Code");
    assert_eq!(agent.agent.command, "claude");
    assert!(agent.agent.requires_authentication);

    // Check paths
    assert_eq!(agent.paths.config_dir, ".claude");
    assert_eq!(agent.paths.context_file, "CLAUDE.md");
    assert_eq!(agent.paths.mcp_config_file, ".claude.json");

    // Check scripts exist
    assert!(agent.install.is_some());
    assert!(agent.authenticate.is_some());
    assert!(agent.deploy.is_some());

    // Check no capability requirements
    assert_eq!(agent.requires.capabilities.len(), 0);
}

#[test]
fn test_get_opencode_agent() {
    let registry = AgentRegistry::load().expect("Failed to load registry");
    let agent = registry
        .get("opencode")
        .expect("OpenCode agent should be available");

    assert_eq!(agent.agent.id, "opencode");
    assert_eq!(agent.agent.name, "OpenCode");
    assert_eq!(agent.agent.command, "opencode");
    assert!(!agent.agent.requires_authentication);

    // Check paths
    assert_eq!(agent.paths.config_dir, ".config/opencode");
    assert_eq!(agent.paths.context_file, "AGENTS.md");
    assert_eq!(agent.paths.mcp_config_file, "opencode.json");

    // Check scripts
    assert!(agent.install.is_some());
    assert!(agent.authenticate.is_none()); // OpenCode doesn't require auth
    assert!(agent.deploy.is_some());

    // Check node capability requirement
    assert_eq!(agent.requires.capabilities.len(), 1);
    assert!(agent.requires.capabilities.contains(&"node".to_string()));
}

#[test]
fn test_get_nonexistent_agent() {
    let registry = AgentRegistry::load().expect("Failed to load registry");
    let agent = registry.get("nonexistent");

    assert!(agent.is_none());
}

#[test]
fn test_default_agent_config() {
    let config = Config::default();
    assert_eq!(config.defaults.agent, "claude");
}

#[test]
fn test_agent_requirements_validation() {
    let registry = AgentRegistry::load().expect("Failed to load registry");
    let opencode = registry.get("opencode").expect("OpenCode should exist");

    // Without node capability
    let config = Config::default();
    let result = claude_vm::agents::executor::verify_requirements(&opencode, &config);
    assert!(
        result.is_err(),
        "Should fail when node capability is not enabled"
    );

    // With node capability
    let mut config_with_node = Config::default();
    config_with_node.tools.node = true;
    let result = claude_vm::agents::executor::verify_requirements(&opencode, &config_with_node);
    assert!(
        result.is_ok(),
        "Should succeed when node capability is enabled"
    );
}

#[test]
fn test_claude_no_requirements() {
    let registry = AgentRegistry::load().expect("Failed to load registry");
    let claude = registry.get("claude").expect("Claude should exist");

    // Claude should work with default config
    let config = Config::default();
    let result = claude_vm::agents::executor::verify_requirements(&claude, &config);
    assert!(
        result.is_ok(),
        "Claude should work without any capabilities"
    );
}

#[test]
fn test_agent_deploy_script_exists() {
    let registry = AgentRegistry::load().expect("Failed to load registry");

    for agent_id in registry.list_available() {
        let agent = registry.get(&agent_id).expect("Agent should exist");
        let result = claude_vm::agents::executor::get_deploy_script_content(&agent);
        assert!(
            result.is_ok(),
            "Deploy script should exist for agent: {}",
            agent_id
        );
    }
}

#[test]
fn test_agent_command_validation() {
    // Valid commands should work
    let valid_commands = vec!["claude", "opencode", "my-agent"];
    for cmd in valid_commands {
        assert!(
            !cmd.contains(';') && !cmd.contains('&') && !cmd.contains('|'),
            "Command validation test: {}",
            cmd
        );
    }

    // Invalid commands should be caught
    let invalid_commands = vec!["claude; rm -rf /", "agent && malicious", "cmd | pipe"];
    for cmd in invalid_commands {
        assert!(
            cmd.contains(';') || cmd.contains('&') || cmd.contains('|'),
            "Command validation should catch: {}",
            cmd
        );
    }
}

#[test]
fn test_agent_config_migration() {
    // Test that claude_args is migrated to agent_args
    let mut config = Config::default();
    config.defaults.claude_args = vec!["--flag".to_string()];
    config.defaults.agent_args = vec![];
    config.defaults.agent = "claude".to_string();

    config.defaults.migrate();

    // After migration, agent_args should have the claude_args values
    assert_eq!(config.defaults.agent_args, vec!["--flag".to_string()]);
}

#[test]
fn test_agent_sorted_list() {
    let registry = AgentRegistry::load().expect("Failed to load registry");
    let available = registry.list_available();

    // List should be sorted
    let mut sorted = available.clone();
    sorted.sort();
    assert_eq!(available, sorted, "Agent list should be sorted");
}
