use anyhow::Result;
use clap::Parser;

use claude_vm::cli::{Cli, Commands};
use claude_vm::config::Config;
use claude_vm::project::Project;
use claude_vm::{commands, error::ClaudeVmError};

fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Detect project
    let project = Project::detect().map_err(|e| match e {
        ClaudeVmError::ProjectDetection(msg) => {
            eprintln!("Error: {}", msg);
            std::process::exit(1);
        }
        _ => e,
    })?;

    // Load configuration with precedence
    let config = Config::load(project.root())?.with_cli_overrides(&cli);

    // Execute command
    match &cli.command {
        Some(Commands::Setup { .. }) => {
            commands::setup::execute(&project, &config)?;
        }
        Some(Commands::Shell) => {
            commands::shell::execute(&project, &config)?;
        }
        Some(Commands::List) => {
            commands::list::execute()?;
        }
        Some(Commands::Clean) => {
            commands::clean::execute(&project)?;
        }
        Some(Commands::CleanAll) => {
            commands::clean_all::execute()?;
        }
        None => {
            // Default: run Claude with provided arguments
            commands::run::execute(&project, &config, &cli.claude_args)?;
        }
    }

    Ok(())
}
