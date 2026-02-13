use crate::config::Config;
use crate::error::Result;
use crate::project::Project;
use crate::worktree::operations::{self, CreateResult};
use crate::worktree::validation::{check_git_version, check_submodules_and_warn};

/// Execute the create worktree command
///
/// Creates a new worktree for the specified branch, or resumes an existing one
pub fn execute(config: &Config, project: &Project, branch: &str, base: Option<&str>) -> Result<()> {
    let repo_root = project.root();

    // Validate git version supports worktrees
    check_git_version()?;

    // Warn about submodule limitations
    check_submodules_and_warn(repo_root);

    // Create or resume the worktree
    let result = operations::create_worktree(&config.worktree, repo_root, branch, base)?;

    // Print user-facing message based on result
    match result {
        CreateResult::Resumed(path) => {
            println!(
                "Resuming worktree for branch '{}' at {}",
                branch,
                path.display()
            );
        }
        CreateResult::Created(path) => {
            println!(
                "Created worktree for branch '{}' at {}",
                branch,
                path.display()
            );
        }
    }

    Ok(())
}
