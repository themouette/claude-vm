use clap::Parser;
use claude_vm::capabilities::registry::CapabilityRegistry;
use claude_vm::cli::Cli;
use claude_vm::config::{Config, PolicyMode, ToolsConfig};

#[test]
fn test_capability_registry_loads() {
    let _registry = CapabilityRegistry::load().expect("Failed to load capability registry");
    // If we got here, all TOML files parsed successfully
}

#[test]
fn test_get_enabled_capabilities() {
    let registry = CapabilityRegistry::load().expect("Failed to load registry");

    let mut config = Config::default();
    config.tools.chromium = true;
    config.tools.node = true;

    let enabled = registry
        .get_enabled_capabilities(&config)
        .expect("Failed to get enabled capabilities");

    // Should have at least chromium and node
    assert!(enabled.len() >= 2);

    // Check that chromium capability is present
    let has_chromium = enabled.iter().any(|c| c.capability.id == "chromium");
    assert!(has_chromium, "Chromium capability should be enabled");

    // Check that node capability is present
    let has_node = enabled.iter().any(|c| c.capability.id == "node");
    assert!(has_node, "Node capability should be enabled");
}

#[test]
fn test_mcp_servers() {
    let registry = CapabilityRegistry::load().expect("Failed to load registry");

    let mut config = Config::default();
    config.tools.chromium = true;
    config.tools.node = true;

    let mcp_servers = registry
        .get_mcp_servers(&config)
        .expect("Failed to get MCP servers");

    // Chromium capability should register chrome-devtools MCP when node is enabled
    let has_chrome_devtools = mcp_servers.iter().any(|s| s.id == "chrome-devtools");
    assert!(
        has_chrome_devtools,
        "Chrome DevTools MCP should be registered when both chromium and node are enabled"
    );
}

#[test]
fn test_mcp_conditional_enable() {
    let registry = CapabilityRegistry::load().expect("Failed to load registry");

    // Enable chromium but NOT node capability (user may install node manually)
    let mut config = Config::default();
    config.tools.chromium = true;
    config.tools.node = false;

    let mcp_servers = registry
        .get_mcp_servers(&config)
        .expect("Failed to get MCP servers");

    // Chrome DevTools MCP should be registered even without node capability
    // (user may install node manually in setup scripts)
    let has_chrome_devtools = mcp_servers.iter().any(|s| s.id == "chrome-devtools");
    assert!(
        has_chrome_devtools,
        "Chrome DevTools MCP should be registered when chromium is enabled (node installed manually)"
    );
}

#[test]
fn test_gpg_capability_loads() {
    let registry = CapabilityRegistry::load().expect("Failed to load registry");

    let mut config = Config::default();
    config.tools.gpg = true;

    let enabled = registry
        .get_enabled_capabilities(&config)
        .expect("Failed to get enabled capabilities");

    // Check that GPG capability is present
    let has_gpg = enabled.iter().any(|c| c.capability.id == "gpg");
    assert!(has_gpg, "GPG capability should be enabled");

    // Check that GPG has all three hooks
    let gpg_cap = enabled.iter().find(|c| c.capability.id == "gpg").unwrap();
    assert!(
        gpg_cap.host_setup.is_some(),
        "GPG should have host_setup hook"
    );
    assert!(gpg_cap.vm_setup.is_some(), "GPG should have vm_setup hook");
    assert!(
        gpg_cap.vm_runtime.is_some(),
        "GPG should have vm_runtime hook"
    );
}

#[test]
fn test_all_capabilities_load() {
    let registry = CapabilityRegistry::load().expect("Failed to load registry");

    let mut config = Config::default();
    config.tools.docker = true;
    config.tools.node = true;
    config.tools.python = true;
    config.tools.chromium = true;
    config.tools.gpg = true;

    let enabled = registry
        .get_enabled_capabilities(&config)
        .expect("Failed to get enabled capabilities");

    // Should have all 5 capabilities
    assert_eq!(enabled.len(), 5, "Should have all 5 capabilities enabled");

    let ids: Vec<_> = enabled.iter().map(|c| c.capability.id.as_str()).collect();
    assert!(ids.contains(&"docker"));
    assert!(ids.contains(&"node"));
    assert!(ids.contains(&"python"));
    assert!(ids.contains(&"chromium"));
    assert!(ids.contains(&"gpg"));
}

#[test]
fn test_network_security_capability_loads() {
    let registry = CapabilityRegistry::load().expect("Failed to load registry");

    let mut config = Config::default();
    config.security.network.enabled = true;

    let enabled = registry
        .get_enabled_capabilities(&config)
        .expect("Failed to get enabled capabilities");

    // Check that network-security capability is present
    let has_network_security = enabled
        .iter()
        .any(|c| c.capability.id == "network-security");
    assert!(
        has_network_security,
        "Network security capability should be enabled"
    );

    // Verify it has the expected hooks
    let net_sec_cap = enabled
        .iter()
        .find(|c| c.capability.id == "network-security")
        .unwrap();
    assert!(
        net_sec_cap.host_setup.is_some(),
        "Network security should have host_setup hook"
    );
    assert!(
        net_sec_cap.vm_setup.is_some(),
        "Network security should have vm_setup hook"
    );
    assert!(
        net_sec_cap.vm_runtime.is_some(),
        "Network security should have vm_runtime hook"
    );
}

#[test]
fn test_network_security_cli_enable() {
    // Test --network-security flag
    let cli = Cli::parse_from(["claude-vm", "setup", "--network-security"]);

    let config = Config::default().with_cli_overrides(&cli);

    assert!(
        config.security.network.enabled,
        "CLI flag --network-security should enable security.network.enabled"
    );
}

#[test]
fn test_network_security_config_enable() {
    // Test TOML config
    let toml = r#"
        [security.network]
        enabled = true
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse config");

    assert!(
        config.security.network.enabled,
        "TOML config should enable security.network.enabled"
    );
}

#[test]
fn test_network_security_all_flag() {
    // Test --all includes network-security
    let cli = Cli::parse_from(["claude-vm", "setup", "--all"]);

    let config = Config::default().with_cli_overrides(&cli);

    assert!(
        config.security.network.enabled,
        "CLI flag --all should enable security.network.enabled"
    );
    assert!(config.tools.docker, "--all should enable docker");
    assert!(config.tools.node, "--all should enable node");
    assert!(config.tools.python, "--all should enable python");
}

#[test]
fn test_network_security_registry_enable_check() {
    let registry = CapabilityRegistry::load().expect("Failed to load registry");

    // Test that network-security is checked via security.network.enabled
    let mut config = Config::default();

    // Should be disabled by default
    config.security.network.enabled = false;
    let enabled = registry
        .get_enabled_capabilities(&config)
        .expect("Failed to get capabilities");
    assert!(!enabled
        .iter()
        .any(|c| c.capability.id == "network-security"));

    // Should be enabled when security.network.enabled = true
    config.security.network.enabled = true;
    let enabled = registry
        .get_enabled_capabilities(&config)
        .expect("Failed to get capabilities");
    assert!(enabled
        .iter()
        .any(|c| c.capability.id == "network-security"));
}

#[test]
fn test_network_security_with_full_config() {
    // Test full configuration with security.network section
    let toml = r#"
        [security.network]
        enabled = true
        mode = "allowlist"
        allowed_domains = ["api.github.com", "*.npmjs.org"]
        blocked_domains = ["evil.com"]
        bypass_domains = ["localhost"]
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse config");

    // Verify security.network section
    assert!(
        config.security.network.enabled,
        "Security network should be enabled"
    );
    assert_eq!(
        config.security.network.mode,
        PolicyMode::Allowlist,
        "Policy mode should be Allowlist"
    );
    assert_eq!(
        config.security.network.allowed_domains,
        vec!["api.github.com", "*.npmjs.org"],
        "Allowed domains should match"
    );
    assert_eq!(
        config.security.network.blocked_domains,
        vec!["evil.com"],
        "Blocked domains should match"
    );
    assert_eq!(
        config.security.network.bypass_domains,
        vec!["localhost"],
        "Bypass domains should match"
    );
}
