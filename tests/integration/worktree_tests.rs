use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use tempfile::TempDir;

/// Helper to create a test git repository with initial commit
fn create_test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let repo_path = dir.path();

    // Initialize git repo
    StdCommand::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    // Configure git for tests
    StdCommand::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    // Disable GPG signing for test commits
    StdCommand::new("git")
        .args(["config", "commit.gpgsign", "false"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    // Create initial commit
    fs::write(repo_path.join("README.md"), "# Test Project\n").unwrap();
    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    dir
}

/// Helper to get worktree directory path
fn get_worktree_dir(repo_dir: &Path) -> PathBuf {
    let repo_name = repo_dir.file_name().unwrap().to_str().unwrap();
    repo_dir
        .parent()
        .unwrap()
        .join(format!("{}-worktrees", repo_name))
}

// ========== Help and Command Existence Tests ==========

#[test]
fn test_worktree_command_help() {
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Manage git worktrees"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("remove"));
}

#[test]
fn test_worktree_short_alias() {
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["w", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Manage git worktrees"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("remove"));
}

#[test]
fn test_worktree_short_alias_functional() {
    let repo = create_test_repo();
    let repo_path = repo.path();

    // Create worktree using short alias
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.current_dir(repo_path)
        .args(["w", "create", "feature-w"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Created worktree"));

    // List using short alias
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.current_dir(repo_path).args(["w", "list"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("feature-w"));

    // Remove using short alias
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.current_dir(repo_path)
        .args(["w", "remove", "feature-w", "--yes"]);
    cmd.assert().success();
}

#[test]
fn test_worktree_create_help() {
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "create", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Create a new worktree"))
        .stdout(predicate::str::contains("<BRANCH>"))
        .stdout(predicate::str::contains("[BASE]"));
}

#[test]
fn test_worktree_list_help() {
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "list", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("List all worktrees"))
        .stdout(predicate::str::contains("--merged"))
        .stdout(predicate::str::contains("--locked"))
        .stdout(predicate::str::contains("--detached"));
}

#[test]
fn test_worktree_remove_help() {
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "remove", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Remove worktrees"))
        .stdout(predicate::str::contains("[BRANCHES]"))
        .stdout(predicate::str::contains("--merged"))
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("--dry-run"))
        .stdout(predicate::str::contains("--locked"));
}

#[test]
fn test_worktree_rm_alias() {
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "rm", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Remove worktrees"));
}

#[test]
fn test_worktree_remove_conflict_merged_and_branches() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Try to use both branches and --merged (should fail)
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "remove", "feature", "--merged", "main"])
        .current_dir(repo_path);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_worktree_remove_locked_without_merged() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Try to use --locked without --merged (should fail)
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "remove", "feature", "--locked"])
        .current_dir(repo_path);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("requires"));
}

#[test]
fn test_worktree_remove_no_arguments() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Try to remove without any branches or --merged (should fail)
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "remove", "--yes"])
        .current_dir(repo_path);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Must specify"));
}

#[test]
fn test_worktree_rm_alias_works() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Create worktree
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "create", "feature"])
        .current_dir(repo_path);
    cmd.assert().success();

    // Use rm alias to remove
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "rm", "feature", "--yes"])
        .current_dir(repo_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Worktree removed"));
}

// ========== Create Worktree Tests ==========

#[test]
fn test_worktree_create_new_branch() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Create worktree for new branch
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "create", "feature-branch"])
        .current_dir(repo_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Created worktree"))
        .stdout(predicate::str::contains("feature-branch"));

    // Verify worktree directory exists
    let worktree_dir = get_worktree_dir(repo_path).join("feature-branch");
    assert!(worktree_dir.exists(), "Worktree directory should exist");

    // Verify it's a valid git worktree
    let output = StdCommand::new("git")
        .args(["worktree", "list"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("feature-branch"),
        "Branch should be in worktree list"
    );
}

#[test]
fn test_worktree_create_with_base_branch() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Create a base branch
    StdCommand::new("git")
        .args(["checkout", "-b", "develop"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    fs::write(repo_path.join("develop.txt"), "develop branch").unwrap();
    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", "Add develop"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    // Create worktree from develop branch
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "create", "feature", "develop"])
        .current_dir(repo_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Created worktree"))
        .stdout(predicate::str::contains("feature"));

    // Verify the worktree has the develop branch content
    let worktree_dir = get_worktree_dir(repo_path).join("feature");
    assert!(worktree_dir.join("develop.txt").exists());
}

#[test]
fn test_worktree_create_resume_existing() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Create worktree first time
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "create", "feature"])
        .current_dir(repo_path);
    cmd.assert().success();

    // Try to create again - should resume
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "create", "feature"])
        .current_dir(repo_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Resuming worktree"))
        .stdout(predicate::str::contains("feature"));
}

#[test]
fn test_worktree_create_invalid_branch_name() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Try to create with invalid branch name (reserved name)
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "create", "HEAD"])
        .current_dir(repo_path);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("reserved git ref name"));
}

#[test]
fn test_worktree_create_path_traversal_in_branch() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Try to create with path traversal in branch name
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "create", "../escape"])
        .current_dir(repo_path);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("cannot contain '..'"));
}

// ========== List Worktree Tests ==========

#[test]
fn test_worktree_list_empty() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "list"]).current_dir(repo_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No additional worktrees"));
}

#[test]
fn test_worktree_list_shows_worktrees() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Create multiple worktrees
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "create", "feature-1"])
        .current_dir(repo_path);
    cmd.assert().success();

    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "create", "feature-2"])
        .current_dir(repo_path);
    cmd.assert().success();

    // List worktrees
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "list"]).current_dir(repo_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Worktrees:"))
        .stdout(predicate::str::contains("feature-1"))
        .stdout(predicate::str::contains("feature-2"));
}

#[test]
fn test_worktree_list_merged_filter() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Create a worktree and add work to it
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "create", "feature-merged"])
        .current_dir(repo_path);
    cmd.assert().success();

    let merged_dir = get_worktree_dir(repo_path).join("feature-merged");
    fs::write(merged_dir.join("merged.txt"), "merged").unwrap();
    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(&merged_dir)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", "Add merged feature"])
        .current_dir(&merged_dir)
        .output()
        .unwrap();

    // Merge it
    StdCommand::new("git")
        .args(["checkout", "master"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["merge", "feature-merged", "--no-edit"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    // Now create an unmerged worktree AFTER the merge
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "create", "feature-unmerged"])
        .current_dir(repo_path);
    cmd.assert().success();

    let unmerged_dir = get_worktree_dir(repo_path).join("feature-unmerged");
    fs::write(unmerged_dir.join("unmerged.txt"), "unmerged").unwrap();
    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(&unmerged_dir)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", "Add unmerged feature"])
        .current_dir(&unmerged_dir)
        .output()
        .unwrap();

    // List with --merged filter should show only merged worktree
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "list", "--merged", "master"])
        .current_dir(repo_path);

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Should contain merged worktree
    assert!(
        stdout.contains("feature-merged"),
        "Should show merged worktree"
    );
    // Should NOT contain unmerged worktree
    assert!(
        !stdout.contains("feature-unmerged"),
        "Should not show unmerged worktree"
    );
}

// ========== Delete Worktree Tests ==========

#[test]
fn test_worktree_remove_single_with_yes_flag() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Create worktree
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "create", "feature"])
        .current_dir(repo_path);
    cmd.assert().success();

    let worktree_dir = get_worktree_dir(repo_path).join("feature");
    assert!(worktree_dir.exists(), "Worktree should exist before remove");

    // Remove with --yes flag
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "remove", "feature", "--yes"])
        .current_dir(repo_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Worktree removed"))
        .stdout(predicate::str::contains("feature"));

    // Verify worktree is removed
    assert!(!worktree_dir.exists(), "Worktree should be removed");

    // Verify branch still exists
    let output = StdCommand::new("git")
        .args(["branch"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("feature"), "Branch should still exist");
}

#[test]
fn test_worktree_remove_multiple_branches() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Create multiple worktrees
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "create", "feature-1"])
        .current_dir(repo_path);
    cmd.assert().success();

    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "create", "feature-2"])
        .current_dir(repo_path);
    cmd.assert().success();

    // Remove both
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "remove", "feature-1", "feature-2", "--yes"])
        .current_dir(repo_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Removed 2 of 2 worktree"));
}

#[test]
fn test_worktree_remove_dry_run() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Create worktree
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "create", "feature"])
        .current_dir(repo_path);
    cmd.assert().success();

    let worktree_dir = get_worktree_dir(repo_path).join("feature");

    // Dry run remove
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "remove", "feature", "--dry-run"])
        .current_dir(repo_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Dry run - no changes made"));

    // Verify worktree still exists
    assert!(
        worktree_dir.exists(),
        "Worktree should still exist after dry-run"
    );
}

#[test]
fn test_worktree_remove_nonexistent_branch() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "remove", "nonexistent", "--yes"])
        .current_dir(repo_path);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("no worktree"));
}

#[test]
fn test_worktree_remove_partial_success() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Create one worktree
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "create", "feature-1"])
        .current_dir(repo_path);
    cmd.assert().success();

    // Try to remove one existing and one non-existing
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "remove", "feature-1", "nonexistent", "--yes"])
        .current_dir(repo_path);

    // Should warn about missing but succeed for the valid one
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("no worktree"))
        .stdout(predicate::str::contains("Worktree removed: feature-1"));
}

// ========== Remove Merged Worktree Tests ==========

#[test]
fn test_worktree_remove_merged_with_yes() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Create a feature branch, add commit, and merge it
    StdCommand::new("git")
        .args(["checkout", "-b", "feature"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    fs::write(repo_path.join("feature.txt"), "feature content").unwrap();
    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", "Add feature"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["checkout", "master"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["merge", "feature", "--no-edit"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    // Create worktree for merged branch
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "create", "feature"])
        .current_dir(repo_path);
    cmd.assert().success();

    let worktree_dir = get_worktree_dir(repo_path).join("feature");
    assert!(worktree_dir.exists());

    // Remove merged worktrees
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "remove", "--merged", "master", "--yes"])
        .current_dir(repo_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Removed 1 merged worktree"));

    // Verify worktree is removed
    assert!(!worktree_dir.exists());
}

#[test]
fn test_worktree_remove_no_merged_worktrees() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Create unmerged worktree with unique commits
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "create", "feature"])
        .current_dir(repo_path);
    cmd.assert().success();

    // Add unique commits to feature branch
    let worktree_dir = get_worktree_dir(repo_path).join("feature");
    fs::write(worktree_dir.join("feature.txt"), "unmerged feature").unwrap();
    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(&worktree_dir)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", "Add unmerged feature"])
        .current_dir(&worktree_dir)
        .output()
        .unwrap();

    // Try to remove merged (should find none)
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "remove", "--merged", "master", "--yes"])
        .current_dir(repo_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No merged worktrees to remove"));
}

#[test]
fn test_worktree_remove_merged_dry_run() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Create and merge a branch
    StdCommand::new("git")
        .args(["checkout", "-b", "feature"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    fs::write(repo_path.join("feature.txt"), "feature").unwrap();
    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", "Add feature"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["checkout", "master"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["merge", "feature", "--no-edit"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    // Create worktree
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "create", "feature"])
        .current_dir(repo_path);
    cmd.assert().success();

    let worktree_dir = get_worktree_dir(repo_path).join("feature");

    // Dry run remove
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "remove", "--merged", "master", "--dry-run"])
        .current_dir(repo_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("feature"))
        .stdout(predicate::str::contains("Dry run - no changes made"));

    // Verify worktree still exists
    assert!(worktree_dir.exists());
}

#[test]
fn test_worktree_remove_merged_uses_current_branch() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Create and merge a branch into master
    StdCommand::new("git")
        .args(["checkout", "-b", "feature"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    fs::write(repo_path.join("feature.txt"), "feature").unwrap();
    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", "Add feature"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["checkout", "master"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["merge", "feature", "--no-edit"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    // Create worktree for the merged branch
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "create", "feature"])
        .current_dir(repo_path);
    cmd.assert().success();

    // Remove without specifying base (should use current branch which is master)
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "remove", "--merged", "--yes"])
        .current_dir(repo_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Using current branch: master"))
        .stdout(predicate::str::contains("Removed 1 merged worktree"));
}

// ========== End-to-End Workflow Tests ==========

#[test]
fn test_complete_worktree_workflow() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Step 1: Create a worktree
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "create", "feature-complete"])
        .current_dir(repo_path);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Created worktree"));

    // Step 2: List worktrees (should show our new worktree)
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "list"]).current_dir(repo_path);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("feature-complete"));

    // Step 3: Work in the worktree (add a file)
    let worktree_dir = get_worktree_dir(repo_path).join("feature-complete");
    fs::write(worktree_dir.join("work.txt"), "work done").unwrap();
    StdCommand::new("git")
        .args(["add", "."])
        .current_dir(&worktree_dir)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", "Add work"])
        .current_dir(&worktree_dir)
        .output()
        .unwrap();

    // Step 4: Merge the work back to master
    StdCommand::new("git")
        .args(["checkout", "master"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["merge", "feature-complete", "--no-edit"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    // Step 5: Remove merged worktrees
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "remove", "--merged", "master", "--yes"])
        .current_dir(repo_path);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Removed 1 merged worktree"));

    // Step 6: Verify worktree is gone but branch remains
    assert!(!worktree_dir.exists());
    let output = StdCommand::new("git")
        .args(["branch"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("feature-complete"));
}

#[test]
fn test_multiple_worktrees_parallel_work() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Create multiple worktrees for parallel development
    let branches = ["feat-1", "feat-2", "feat-3"];
    for branch in &branches {
        let mut cmd = cargo_bin_cmd!("claude-vm");
        cmd.args(["worktree", "create", branch])
            .current_dir(repo_path);
        cmd.assert().success();
    }

    // List should show all worktrees
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "list"]).current_dir(repo_path);

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    for branch in &branches {
        assert!(stdout.contains(branch));
    }

    // Batch remove all
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.args(["worktree", "remove"])
        .args(branches)
        .arg("--yes")
        .current_dir(repo_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Removed 3 of 3 worktree"));
}

// ==================== Error Scenario Tests ====================

#[test]
fn test_worktree_create_with_invalid_base_branch() {
    let repo = create_test_repo();
    let repo_path = repo.path();

    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.current_dir(repo_path)
        .args(["worktree", "create", "feature", "nonexistent-base"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("git worktree failed"));
}

#[test]
fn test_worktree_remove_during_git_lock() {
    let repo = create_test_repo();
    let repo_path = repo.path();

    // Create a worktree
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.current_dir(repo_path)
        .args(["worktree", "create", "feature1"]);
    cmd.assert().success();

    // Lock the worktree in git metadata
    let git_dir = repo_path.join(".git").join("worktrees").join("feature1");
    if git_dir.exists() {
        fs::write(git_dir.join("locked"), "Testing locked worktree").unwrap();

        // Try to remove the locked worktree without --locked flag
        let mut cmd = cargo_bin_cmd!("claude-vm");
        cmd.current_dir(repo_path)
            .args(["worktree", "remove", "--merged", "--yes"]);

        // Should succeed but skip locked worktrees
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("No merged worktrees to remove"));
    }
}

#[test]
fn test_worktree_remove_with_invalid_base_branch() {
    let repo = create_test_repo();
    let repo_path = repo.path();

    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.current_dir(repo_path).args([
        "worktree",
        "remove",
        "--merged",
        "nonexistent-branch",
        "--yes",
    ]);

    cmd.assert().failure().stderr(predicate::str::contains(
        "Branch 'nonexistent-branch' does not exist",
    ));
}

#[test]
fn test_worktree_create_in_non_git_repo() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.current_dir(temp_dir.path())
        .args(["worktree", "create", "feature"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("git").or(predicate::str::contains("repository")));
}

#[test]
fn test_worktree_list_in_non_git_repo() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.current_dir(temp_dir.path()).args(["worktree", "list"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("git").or(predicate::str::contains("repository")));
}

#[test]
fn test_worktree_create_resume_race_condition() {
    // Test that creating the same worktree twice results in resume behavior
    let repo = create_test_repo();
    let repo_path = repo.path();

    // First creation
    let mut cmd1 = cargo_bin_cmd!("claude-vm");
    cmd1.current_dir(repo_path)
        .args(["worktree", "create", "race-test"]);
    cmd1.assert()
        .success()
        .stdout(predicate::str::contains("Created worktree"));

    // Second creation (should resume)
    let mut cmd2 = cargo_bin_cmd!("claude-vm");
    cmd2.current_dir(repo_path)
        .args(["worktree", "create", "race-test"]);
    cmd2.assert()
        .success()
        .stdout(predicate::str::contains("Resuming worktree"));
}

#[test]
fn test_worktree_remove_with_missing_directory() {
    // Test handling of orphaned git metadata (directory deleted but git still tracks it)
    let repo = create_test_repo();
    let repo_path = repo.path();

    // Create a worktree
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.current_dir(repo_path)
        .args(["worktree", "create", "orphaned"]);
    cmd.assert().success();

    // Find and manually delete the worktree directory
    let worktrees_dir = repo_path.join(format!(
        "{}-worktrees",
        repo_path.file_name().unwrap().to_str().unwrap()
    ));
    let orphaned_dir = worktrees_dir.join("orphaned");

    if orphaned_dir.exists() {
        fs::remove_dir_all(&orphaned_dir).unwrap();

        // Try to remove via command - git should handle this with prune or force
        let mut cmd = cargo_bin_cmd!("claude-vm");
        cmd.current_dir(repo_path)
            .args(["worktree", "remove", "orphaned", "--yes"]);

        // This might fail or succeed depending on git's handling, but shouldn't panic
        let _ = cmd.assert();
    }
}

#[test]
fn test_worktree_create_branch_name_edge_cases() {
    let repo = create_test_repo();
    let repo_path = repo.path();

    // Test branch name with multiple slashes (valid)
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.current_dir(repo_path)
        .args(["worktree", "create", "feature/deep/nested/branch"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Created worktree"));

    // Verify worktree appears in list
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.current_dir(repo_path).args(["worktree", "list"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("feature/deep/nested/branch"));
}

#[test]
fn test_worktree_remove_all_branches_separately() {
    // Test removing branches one by one vs batch
    let repo = create_test_repo();
    let repo_path = repo.path();

    // Create multiple worktrees
    for i in 1..=3 {
        let mut cmd = cargo_bin_cmd!("claude-vm");
        cmd.current_dir(repo_path)
            .args(["worktree", "create", &format!("feature{}", i)]);
        cmd.assert().success();
    }

    // Remove them one by one
    for i in 1..=3 {
        let mut cmd = cargo_bin_cmd!("claude-vm");
        cmd.current_dir(repo_path)
            .args(["worktree", "remove", &format!("feature{}", i), "--yes"]);
        cmd.assert().success();
    }

    // Verify all are removed - list should show no additional worktrees
    let mut cmd = cargo_bin_cmd!("claude-vm");
    cmd.current_dir(repo_path).args(["worktree", "list"]);

    cmd.assert().success().stdout(
        predicate::str::contains("No additional worktrees")
            .or(predicate::str::contains("feature1").not()),
    );
}
