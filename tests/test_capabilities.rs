use claude_vm::capabilities::registry::CapabilityRegistry;
use claude_vm::config::Config;
use serial_test::serial;
use std::path::PathBuf;
use std::process::Command;

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

    // Check that GPG has host phases
    let gpg_cap = enabled.iter().find(|c| c.capability.id == "gpg").unwrap();
    assert!(
        !gpg_cap.phase.before_setup.is_empty(),
        "GPG should have before_setup phases"
    );
    assert!(
        !gpg_cap.phase.setup.is_empty(),
        "GPG should have setup phases"
    );
    assert!(
        !gpg_cap.phase.runtime.is_empty(),
        "GPG should have runtime phases"
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

/// Helper function to create a temporary git repository for testing
fn create_test_git_repo(base_dir: &std::path::Path, repo_name: &str) -> PathBuf {
    let repo_path = base_dir.join(repo_name);
    std::fs::create_dir_all(&repo_path).expect("Failed to create test repo directory");

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to init git repo");

    // Configure git user (required for commits)
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to set git email");

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to set git name");

    // Create initial commit
    std::fs::write(repo_path.join("README.md"), "# Test Repo\n").expect("Failed to write README");

    Command::new("git")
        .args(["add", "README.md"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to add file");

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to create commit");

    repo_path
}

/// Integration test for Project worktree detection
/// This test verifies that Project correctly identifies git worktrees
#[test]
#[serial]
fn test_project_worktree_detection() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let base_path = temp_dir.path();

    // Create main repository
    let main_repo = create_test_git_repo(base_path, "main-repo");

    // Create a worktree
    let worktree_path = base_path.join("test-worktree");
    Command::new("git")
        .args([
            "worktree",
            "add",
            worktree_path.to_str().unwrap(),
            "-b",
            "feature-branch",
        ])
        .current_dir(&main_repo)
        .output()
        .expect("Failed to create worktree");

    // Change to worktree directory and detect project
    let original_dir = std::env::current_dir().expect("Failed to get current dir");
    std::env::set_current_dir(&worktree_path).expect("Failed to change to worktree dir");

    // Detect project from worktree
    let project = claude_vm::project::Project::detect().expect("Failed to detect project");

    // Restore original directory
    std::env::set_current_dir(original_dir).expect("Failed to restore directory");

    // Verify worktree detection
    assert!(
        project.is_worktree(),
        "Project should be detected as a worktree"
    );

    // Verify project root points to worktree
    assert_eq!(
        project.root().canonicalize().unwrap(),
        worktree_path.canonicalize().unwrap(),
        "Project root should be the worktree path"
    );

    // Verify main repo root points to main repository
    assert_eq!(
        project.main_repo_root().canonicalize().unwrap(),
        main_repo.canonicalize().unwrap(),
        "Main repo root should be the main repository path"
    );

    // Template name should be based on main repo, not worktree
    let template_name = project.template_name();
    assert!(
        template_name.contains("main-repo"),
        "Template name should contain main repo name: {}",
        template_name
    );
}

/// Integration test for regular (non-worktree) project detection
#[test]
#[serial]
fn test_project_regular_repo_detection() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let base_path = temp_dir.path();

    // Create main repository
    let main_repo = create_test_git_repo(base_path, "regular-repo");

    // Change to repo directory and detect project
    let original_dir = std::env::current_dir().expect("Failed to get current dir");
    std::env::set_current_dir(&main_repo).expect("Failed to change to repo dir");

    // Detect project
    let project = claude_vm::project::Project::detect().expect("Failed to detect project");

    // Restore original directory
    std::env::set_current_dir(original_dir).expect("Failed to restore directory");

    // Verify it's NOT a worktree
    assert!(
        !project.is_worktree(),
        "Project should NOT be detected as a worktree"
    );

    // Verify both roots point to the same location
    assert_eq!(
        project.root().canonicalize().unwrap(),
        main_repo.canonicalize().unwrap(),
        "Project root should be the main repo path"
    );

    assert_eq!(
        project.main_repo_root().canonicalize().unwrap(),
        main_repo.canonicalize().unwrap(),
        "Main repo root should be the same as project root for regular repos"
    );
}

#[test]
fn test_capability_phases_run_before_user_phases() {
    use claude_vm::capabilities::merge_capability_phases;
    use claude_vm::config::ScriptPhase;

    let mut config = Config::default();

    // Add user-defined setup and runtime phases
    config.phase.setup.push(ScriptPhase {
        name: "user-setup".to_string(),
        script: Some("echo 'user setup'".to_string()),
        ..Default::default()
    });

    config.phase.runtime.push(ScriptPhase {
        name: "user-runtime".to_string(),
        script: Some("echo 'user runtime'".to_string()),
        ..Default::default()
    });

    // Enable a capability that has both setup and runtime phases
    config.tools.gh = true;

    // Merge capability phases
    merge_capability_phases(&mut config).expect("Failed to merge capability phases");

    // Verify setup phases: capability phases should come first
    assert!(!config.phase.setup.is_empty(), "Should have setup phases");
    let first_setup = &config.phase.setup[0];
    // First phase should be a capability phase (has CAPABILITY_ID env var)
    assert!(
        first_setup.env.contains_key("CAPABILITY_ID"),
        "First setup phase should be a capability phase with CAPABILITY_ID"
    );

    // User phase should come after capability phases
    let last_setup = config.phase.setup.last().unwrap();
    assert_eq!(
        &last_setup.name, "user-setup",
        "User setup phase should come after capability phases"
    );

    // Verify runtime phases: capability phases should come first
    assert!(
        !config.phase.runtime.is_empty(),
        "Should have runtime phases"
    );
    let first_runtime = &config.phase.runtime[0];
    // First phase should be a capability phase (has CAPABILITY_ID env var)
    assert!(
        first_runtime.env.contains_key("CAPABILITY_ID"),
        "First runtime phase should be a capability phase with CAPABILITY_ID"
    );

    // User phase should come after capability phases
    let last_runtime = config.phase.runtime.last().unwrap();
    assert_eq!(
        &last_runtime.name, "user-runtime",
        "User runtime phase should come after capability phases"
    );
}

#[test]
fn test_rtk_capability_loads() {
    use claude_vm::config::{RtkConfig, RtkToolConfig};

    let registry = CapabilityRegistry::load().expect("Failed to load registry");

    let mut config = Config::default();
    config.tools.rtk = Some(RtkConfig::Detailed(RtkToolConfig { hook_mode: true }));

    let enabled = registry
        .get_enabled_capabilities(&config)
        .expect("Failed to get enabled capabilities");

    let has_rtk = enabled.iter().any(|c| c.capability.id == "rtk");
    assert!(has_rtk, "RTK capability should be enabled");

    let rtk_cap = enabled.iter().find(|c| c.capability.id == "rtk").unwrap();
    assert!(
        !rtk_cap.phase.setup.is_empty(),
        "RTK should have setup phases"
    );
    assert!(
        !rtk_cap.phase.runtime.is_empty(),
        "RTK should have runtime phases"
    );
}

#[test]
fn test_rtk_config_defaults() {
    use claude_vm::config::RtkConfig;

    let config = Config::default();
    assert!(
        config.tools.rtk.is_none(),
        "RTK should be disabled by default"
    );

    // When enabled with Simple(true), hook mode should default to true
    let rtk_config = RtkConfig::Simple(true);
    assert!(
        rtk_config.get_config().hook_mode,
        "RTK hook mode should be enabled by default with Simple(true)"
    );
}

#[test]
fn test_rtk_hook_mode_opt_out() {
    use claude_vm::config::{RtkConfig, RtkToolConfig};

    let mut config = Config::default();
    config.tools.rtk = Some(RtkConfig::Detailed(RtkToolConfig { hook_mode: false }));

    assert!(config.tools.is_enabled("rtk"), "RTK should be enabled");
    assert!(
        !config.tools.rtk.unwrap().get_config().hook_mode,
        "Hook mode should be disabled when opted out"
    );
}

#[test]
fn test_rtk_with_other_capabilities() {
    use claude_vm::config::RtkConfig;

    let registry = CapabilityRegistry::load().expect("Failed to load registry");

    let mut config = Config::default();
    config.tools.rtk = Some(RtkConfig::Simple(true));
    config.tools.rust = true;
    config.tools.git = true;

    let enabled = registry
        .get_enabled_capabilities(&config)
        .expect("Failed to get enabled capabilities");

    let ids: Vec<_> = enabled.iter().map(|c| c.capability.id.as_str()).collect();
    assert!(ids.contains(&"rtk"), "Should include RTK");
    assert!(ids.contains(&"rust"), "Should include Rust");
    assert!(ids.contains(&"git"), "Should include Git");
}

#[test]
fn test_rtk_is_enabled_check() {
    use claude_vm::config::RtkConfig;

    let mut config = Config::default();

    // RTK should not be enabled by default
    assert!(
        !config.tools.is_enabled("rtk"),
        "RTK should not be enabled by default"
    );

    // Enable RTK with simple syntax
    config.tools.rtk = Some(RtkConfig::Simple(true));

    // RTK should now be enabled
    assert!(
        config.tools.is_enabled("rtk"),
        "RTK should be enabled when configured"
    );
}

#[test]
fn test_rtk_enable_method() {
    let mut config = Config::default();

    // RTK should not be enabled by default
    assert!(!config.tools.is_enabled("rtk"));

    // Enable RTK via enable() method
    config.tools.enable("rtk");

    // RTK should now be enabled with default hook_mode = true
    assert!(config.tools.is_enabled("rtk"));
    assert!(config.tools.rtk.is_some());
    assert!(config.tools.rtk.as_ref().unwrap().get_config().hook_mode);
}

#[test]
fn test_rtk_simple_syntax() {
    use claude_vm::config::RtkConfig;

    // Test rtk = true
    let mut config = Config::default();
    config.tools.rtk = Some(RtkConfig::Simple(true));

    assert!(
        config.tools.is_enabled("rtk"),
        "RTK should be enabled with Simple(true)"
    );
    assert!(
        config.tools.rtk.as_ref().unwrap().get_config().hook_mode,
        "Hook mode should be enabled by default with Simple(true)"
    );

    // Test rtk = false
    config.tools.rtk = Some(RtkConfig::Simple(false));
    assert!(
        !config.tools.is_enabled("rtk"),
        "RTK should be disabled with Simple(false)"
    );
}

#[test]
fn test_rtk_detailed_syntax() {
    use claude_vm::config::{RtkConfig, RtkToolConfig};

    let mut config = Config::default();

    // Test [tools.rtk] with hook_mode = true
    config.tools.rtk = Some(RtkConfig::Detailed(RtkToolConfig { hook_mode: true }));
    assert!(
        config.tools.is_enabled("rtk"),
        "RTK should be enabled with Detailed config"
    );
    assert!(
        config.tools.rtk.as_ref().unwrap().get_config().hook_mode,
        "Hook mode should be enabled when set to true"
    );

    // Test [tools.rtk] with hook_mode = false
    config.tools.rtk = Some(RtkConfig::Detailed(RtkToolConfig { hook_mode: false }));
    assert!(
        config.tools.is_enabled("rtk"),
        "RTK should still be enabled with Detailed config (presence = enabled)"
    );
    assert!(
        !config.tools.rtk.as_ref().unwrap().get_config().hook_mode,
        "Hook mode should be disabled when set to false"
    );
}
