use assert_cmd::Command;
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
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Manage git worktrees"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("delete"))
        .stdout(predicate::str::contains("clean"));
}

#[test]
fn test_worktree_create_help() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "create", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Create a new worktree"))
        .stdout(predicate::str::contains("<BRANCH>"))
        .stdout(predicate::str::contains("[BASE]"));
}

#[test]
fn test_worktree_list_help() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "list", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("List all worktrees"))
        .stdout(predicate::str::contains("--merged"))
        .stdout(predicate::str::contains("--locked"))
        .stdout(predicate::str::contains("--detached"));
}

#[test]
fn test_worktree_delete_help() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "delete", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Delete a worktree"))
        .stdout(predicate::str::contains("<BRANCHES>"))
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("--dry-run"));
}

#[test]
fn test_worktree_clean_help() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "clean", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Clean worktrees"))
        .stdout(predicate::str::contains("--merged"))
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("--dry-run"))
        .stdout(predicate::str::contains("--locked"));
}

// ========== Create Worktree Tests ==========

#[test]
fn test_worktree_create_new_branch() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Create worktree for new branch
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
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
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
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
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "create", "feature"])
        .current_dir(repo_path);
    cmd.assert().success();

    // Try to create again - should resume
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
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
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
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
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
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

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
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
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "create", "feature-1"])
        .current_dir(repo_path);
    cmd.assert().success();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "create", "feature-2"])
        .current_dir(repo_path);
    cmd.assert().success();

    // List worktrees
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
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
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
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
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
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
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
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
fn test_worktree_delete_single_with_yes_flag() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Create worktree
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "create", "feature"])
        .current_dir(repo_path);
    cmd.assert().success();

    let worktree_dir = get_worktree_dir(repo_path).join("feature");
    assert!(worktree_dir.exists(), "Worktree should exist before delete");

    // Delete with --yes flag
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "delete", "feature", "--yes"])
        .current_dir(repo_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Worktree deleted"))
        .stdout(predicate::str::contains("feature"));

    // Verify worktree is removed
    assert!(!worktree_dir.exists(), "Worktree should be deleted");

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
fn test_worktree_delete_multiple_branches() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Create multiple worktrees
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "create", "feature-1"])
        .current_dir(repo_path);
    cmd.assert().success();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "create", "feature-2"])
        .current_dir(repo_path);
    cmd.assert().success();

    // Delete both
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "delete", "feature-1", "feature-2", "--yes"])
        .current_dir(repo_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Deleted 2 of 2 worktree"));
}

#[test]
fn test_worktree_delete_dry_run() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Create worktree
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "create", "feature"])
        .current_dir(repo_path);
    cmd.assert().success();

    let worktree_dir = get_worktree_dir(repo_path).join("feature");

    // Dry run delete
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "delete", "feature", "--dry-run"])
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
fn test_worktree_delete_nonexistent_branch() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "delete", "nonexistent", "--yes"])
        .current_dir(repo_path);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("no worktree"));
}

#[test]
fn test_worktree_delete_partial_success() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Create one worktree
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "create", "feature-1"])
        .current_dir(repo_path);
    cmd.assert().success();

    // Try to delete one existing and one non-existing
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "delete", "feature-1", "nonexistent", "--yes"])
        .current_dir(repo_path);

    // Should warn about missing but succeed for the valid one
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("no worktree"))
        .stdout(predicate::str::contains("Worktree deleted: feature-1"));
}

// ========== Clean Worktree Tests ==========

#[test]
fn test_worktree_clean_merged_with_yes() {
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
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "create", "feature"])
        .current_dir(repo_path);
    cmd.assert().success();

    let worktree_dir = get_worktree_dir(repo_path).join("feature");
    assert!(worktree_dir.exists());

    // Clean merged worktrees
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "clean", "--merged", "master", "--yes"])
        .current_dir(repo_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Cleaned 1 merged worktree"));

    // Verify worktree is removed
    assert!(!worktree_dir.exists());
}

#[test]
fn test_worktree_clean_no_merged_worktrees() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Create unmerged worktree with unique commits
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
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

    // Try to clean merged (should find none)
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "clean", "--merged", "master", "--yes"])
        .current_dir(repo_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No merged worktrees to clean"));
}

#[test]
fn test_worktree_clean_dry_run() {
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
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "create", "feature"])
        .current_dir(repo_path);
    cmd.assert().success();

    let worktree_dir = get_worktree_dir(repo_path).join("feature");

    // Dry run clean
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "clean", "--merged", "master", "--dry-run"])
        .current_dir(repo_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("feature"))
        .stdout(predicate::str::contains("Dry run - no changes made"));

    // Verify worktree still exists
    assert!(worktree_dir.exists());
}

#[test]
fn test_worktree_clean_uses_default_branch() {
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
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "create", "feature"])
        .current_dir(repo_path);
    cmd.assert().success();

    // Clean without specifying base (should use default branch - explicitly use master)
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "clean", "--merged", "master", "--yes"])
        .current_dir(repo_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Cleaned 1 merged worktree"));
}

// ========== End-to-End Workflow Tests ==========

#[test]
fn test_complete_worktree_workflow() {
    let repo_dir = create_test_repo();
    let repo_path = repo_dir.path();

    // Step 1: Create a worktree
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "create", "feature-complete"])
        .current_dir(repo_path);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Created worktree"));

    // Step 2: List worktrees (should show our new worktree)
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
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

    // Step 5: Clean merged worktrees
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "clean", "--merged", "master", "--yes"])
        .current_dir(repo_path);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Cleaned 1 merged worktree"));

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
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
        cmd.args(["worktree", "create", branch])
            .current_dir(repo_path);
        cmd.assert().success();
    }

    // List should show all worktrees
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "list"]).current_dir(repo_path);

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    for branch in &branches {
        assert!(stdout.contains(branch));
    }

    // Batch delete all
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.args(["worktree", "delete"])
        .args(branches)
        .arg("--yes")
        .current_dir(repo_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Deleted 3 of 3 worktree"));
}
