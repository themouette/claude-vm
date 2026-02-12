use claude_vm::capabilities::registry::CapabilityRegistry;
use claude_vm::config::Config;

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
fn test_notifications_capability_loads() {
    let registry = CapabilityRegistry::load().expect("Failed to load registry");

    let mut config = Config::default();
    config.tools.notifications = true;

    let enabled = registry
        .get_enabled_capabilities(&config)
        .expect("Failed to get enabled capabilities");

    // Check that notifications capability is present
    let has_notifications = enabled.iter().any(|c| c.capability.id == "notifications");
    assert!(has_notifications, "Notifications capability should be enabled");

    // Check that notifications has host_setup and vm_runtime hooks
    let notifications_cap = enabled
        .iter()
        .find(|c| c.capability.id == "notifications")
        .unwrap();
    assert!(
        notifications_cap.host_setup.is_some(),
        "Notifications should have host_setup hook"
    );
    assert!(
        notifications_cap.vm_runtime.is_some(),
        "Notifications should have vm_runtime hook"
    );
}

#[test]
fn test_notifications_port_forward() {
    use claude_vm::capabilities;

    let mut config = Config::default();
    config.tools.notifications = true;

    let port_forwards = capabilities::get_port_forwards(&config)
        .expect("Failed to get port forwards");

    // Should have at least one port forward for notifications
    assert!(!port_forwards.is_empty(), "Should have port forwards");

    // Check that the notification socket is forwarded
    let has_notification_socket = port_forwards.iter().any(|pf| {
        pf.host_socket.contains("claude-vm-notifications")
            && pf.guest_socket.contains("claude-notifications")
    });
    assert!(
        has_notification_socket,
        "Should have notification socket forwarding"
    );
}

#[test]
fn test_notifications_config_enable() {
    let mut config = Config::default();

    // Initially disabled
    assert!(!config.tools.is_enabled("notifications"));

    // Enable notifications
    config.tools.enable("notifications");

    // Should now be enabled
    assert!(config.tools.is_enabled("notifications"));
    assert!(config.tools.notifications);
}
