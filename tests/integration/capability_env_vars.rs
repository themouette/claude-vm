/// Integration tests for capability environment variable injection
///
/// These tests verify that capability phases receive the expected environment
/// variables (CAPABILITY_ID, TEMPLATE_NAME, PROJECT_NAME, etc.) when executed.
use claude_vm::config::ScriptPhase;
use claude_vm::phase_executor::build_phase_env_setup;
use claude_vm::project::Project;
use std::collections::HashMap;
use tempfile::TempDir;

/// Create a temporary test project
fn create_test_project() -> (TempDir, Project) {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path().to_path_buf();

    // Create a minimal git repository
    std::fs::create_dir(project_path.join(".git")).unwrap();

    let project = Project::new_for_test(project_path);
    (temp_dir, project)
}

#[test]
fn test_user_phase_without_capability_id() {
    let (_temp, project) = create_test_project();

    let mut env = HashMap::new();
    env.insert("MY_VAR".to_string(), "value".to_string());

    let phase = ScriptPhase {
        name: "user-phase".to_string(),
        env,
        ..Default::default()
    };

    let result = build_phase_env_setup(&phase, &project, "test-vm").unwrap();

    // Should only contain the user-defined variable
    assert!(result.contains("export MY_VAR='value'"));
    // Should NOT inject capability vars for user phases
    assert!(!result.contains("CAPABILITY_ID"));
    assert!(!result.contains("TEMPLATE_NAME"));
}

#[test]
fn test_capability_phase_injects_env_vars() {
    let (_temp, project) = create_test_project();

    let mut env = HashMap::new();
    // Mark as capability phase
    env.insert("CAPABILITY_ID".to_string(), "test-capability".to_string());
    env.insert("CLAUDE_VM_PHASE".to_string(), "setup".to_string());

    let phase = ScriptPhase {
        name: "capability-setup".to_string(),
        env,
        ..Default::default()
    };

    let result = build_phase_env_setup(&phase, &project, "test-vm").unwrap();

    // Should contain all capability environment variables
    assert!(result.contains("export CAPABILITY_ID='test-capability'"));
    assert!(result.contains("export CLAUDE_VM_PHASE='setup'"));
    assert!(result.contains("export CLAUDE_VM_VERSION="));
    assert!(result.contains("export TEMPLATE_NAME="));
    assert!(result.contains("export LIMA_INSTANCE='test-vm'"));
    assert!(result.contains("export PROJECT_ROOT="));
    assert!(result.contains("export PROJECT_NAME="));
    // Worktree vars should be empty for non-worktree projects
    assert!(result.contains("export PROJECT_WORKTREE_ROOT=''"));
    assert!(result.contains("export PROJECT_WORKTREE=''"));
}

#[test]
fn test_capability_phase_with_additional_env_vars() {
    let (_temp, project) = create_test_project();

    let mut env = HashMap::new();
    env.insert("CAPABILITY_ID".to_string(), "gh".to_string());
    env.insert("CLAUDE_VM_PHASE".to_string(), "runtime".to_string());
    // Additional capability-specific vars
    env.insert("CUSTOM_VAR".to_string(), "custom_value".to_string());

    let phase = ScriptPhase {
        name: "gh-runtime".to_string(),
        env,
        ..Default::default()
    };

    let result = build_phase_env_setup(&phase, &project, "ephemeral-vm").unwrap();

    // Should contain both capability vars and custom vars
    assert!(result.contains("export CAPABILITY_ID='gh'"));
    assert!(result.contains("export CLAUDE_VM_PHASE='runtime'"));
    assert!(result.contains("export LIMA_INSTANCE='ephemeral-vm'"));
    assert!(result.contains("export CUSTOM_VAR='custom_value'"));
}

#[test]
fn test_project_name_extraction() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path().join("my-awesome-project");
    std::fs::create_dir(&project_path).unwrap();
    std::fs::create_dir(project_path.join(".git")).unwrap();

    let project = Project::new_for_test(project_path);

    let mut env = HashMap::new();
    env.insert("CAPABILITY_ID".to_string(), "test".to_string());

    let phase = ScriptPhase {
        name: "test".to_string(),
        env,
        ..Default::default()
    };

    let result = build_phase_env_setup(&phase, &project, "test-vm").unwrap();

    // Should extract project name from directory
    assert!(result.contains("export PROJECT_NAME='my-awesome-project'"));
}

#[test]
fn test_git_worktree_detection() {
    let temp_dir = TempDir::new().unwrap();
    let main_repo = temp_dir.path().join("main-repo");
    let worktree = temp_dir.path().join("feature-branch");

    // Create main repo
    std::fs::create_dir_all(&main_repo).unwrap();
    std::fs::create_dir(main_repo.join(".git")).unwrap();
    std::fs::create_dir(main_repo.join(".git").join("worktrees")).unwrap();

    // Create worktree
    std::fs::create_dir_all(&worktree).unwrap();

    // Create .git file pointing to worktree
    let gitdir_path = main_repo.join(".git/worktrees/feature-branch");
    std::fs::create_dir_all(&gitdir_path).unwrap();

    let git_file_content = format!("gitdir: {}", gitdir_path.display());
    std::fs::write(worktree.join(".git"), git_file_content).unwrap();

    let project = Project::new_for_test(worktree.clone());

    let mut env = HashMap::new();
    env.insert("CAPABILITY_ID".to_string(), "test".to_string());

    let phase = ScriptPhase {
        name: "test".to_string(),
        env,
        ..Default::default()
    };

    let result = build_phase_env_setup(&phase, &project, "test-vm").unwrap();

    // Should detect worktree and set appropriate vars
    assert!(result.contains(&format!(
        "export PROJECT_WORKTREE_ROOT='{}'",
        main_repo.display()
    )));
    assert!(result.contains(&format!("export PROJECT_WORKTREE='{}'", worktree.display())));
}
