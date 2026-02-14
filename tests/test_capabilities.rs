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

    // Check that GPG has host_setup and phases
    let gpg_cap = enabled.iter().find(|c| c.capability.id == "gpg").unwrap();
    assert!(
        gpg_cap.host_setup.is_some(),
        "GPG should have host_setup hook"
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
