use crate::config::Config;
use crate::error::Result;
use crate::project::Project;
use crate::vm::template;
use crate::worktree::{
    operations,
    validation,
};
use std::io::{self, Write};
use std::path::PathBuf;

/// Ensure template exists, prompting user to create it if missing
///
/// This function checks if a template exists for the given project.
/// If the template doesn't exist:
/// - If auto_setup is enabled, automatically creates the template
/// - Otherwise, prompts the user to confirm template creation
/// - If user declines, returns an error
pub fn ensure_template_exists(project: &Project, config: &Config) -> Result<()> {
    // Check if template exists
    if template::exists(project.template_name())? {
        return Ok(());
    }

    // Template doesn't exist
    if config.auto_setup {
        // Auto-create template without prompting
        println!("Template not found. Auto-creating template...");
        create_template(project, config)?;
        return Ok(());
    }

    // Prompt user
    println!(
        "No template found for project: {}",
        project.root().display()
    );
    println!("Template name: {}", project.template_name());
    println!();
    print!("Would you like to create it now? [Y/n]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input.is_empty() || input == "y" || input == "yes" {
        println!();
        create_template(project, config)?;
        Ok(())
    } else {
        Err(crate::error::ClaudeVmError::TemplateNotFound(
            project.template_name().to_string(),
        ))
    }
}

/// Create a template for the project
fn create_template(project: &Project, config: &Config) -> Result<()> {
    // Auto-setup always installs the agent (no_agent_install = false)
    crate::commands::setup::execute(project, config, false)
}

/// Resolve worktree from command-line arguments
///
/// This function handles the --worktree flag for agent and shell commands.
/// It validates git version and submodules, then creates or resumes a worktree.
///
/// Returns the path to the worktree directory.
pub fn resolve_worktree(
    worktree_args: &[String],
    config: &Config,
    project: &Project,
) -> Result<PathBuf> {
    let repo_root = project.root();

    // Validate git version and check for submodules
    validation::check_git_version()?;
    validation::check_submodules_and_warn(repo_root);

    // Parse arguments: length 1 = (branch, None), length 2 = (branch, Some(base))
    let (branch, base) = match worktree_args.len() {
        1 => (worktree_args[0].as_str(), None),
        2 => (worktree_args[0].as_str(), Some(worktree_args[1].as_str())),
        _ => {
            return Err(crate::error::ClaudeVmError::Worktree(
                "Invalid worktree arguments".to_string(),
            ))
        }
    };

    // Create or resume worktree
    let result = operations::create_worktree(&config.worktree, repo_root, branch, base)?;

    // Print user-facing message
    eprintln!("{}", result.message(branch));

    // Return the path
    Ok(result.path().clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_template_exists_function_signature() {
        // Verify the function signature is stable
        let _fn: fn(&Project, &Config) -> Result<()> = ensure_template_exists;
    }

    #[test]
    fn test_create_template_function_signature() {
        // Verify internal function signature
        let _fn: fn(&Project, &Config) -> Result<()> = create_template;
    }

    #[test]
    fn test_module_exports() {
        // Ensure the public API is accessible
        // This test verifies that ensure_template_exists is public
        // and can be called from other modules
        use crate::commands::helpers::ensure_template_exists;
        let _fn = ensure_template_exists;
    }
}
