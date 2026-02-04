use anyhow::Result;
use clap::Parser;

use claude_vm::cli::{Cli, Commands};
use claude_vm::config::Config;
use claude_vm::project::Project;
use claude_vm::{commands, error::ClaudeVmError};

fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Handle commands that don't need project detection first
    match &cli.command {
        Some(Commands::Version { check }) => {
            commands::version::execute(*check)?;
            return Ok(());
        }
        Some(Commands::Update {
            check,
            version,
            yes,
        }) => {
            commands::update::execute(*check, version.clone(), *yes)?;
            return Ok(());
        }
        Some(Commands::List) => {
            commands::list::execute()?;
            return Ok(());
        }
        Some(Commands::CleanAll) => {
            commands::clean_all::execute()?;
            return Ok(());
        }
        _ => {}
    }

    // Detect project for commands that need it
    let project = Project::detect().map_err(|e| match e {
        ClaudeVmError::ProjectDetection(msg) => {
            eprintln!("Error: {}", msg);
            std::process::exit(1);
        }
        _ => e,
    })?;

    // Load configuration with precedence
    let config = Config::load(project.root())?.with_cli_overrides(&cli);

    // Check for updates only on run command (default command)
    if cli.command.is_none() {
        let update_config = claude_vm::update_check::UpdateCheckConfig {
            enabled: config.update_check.enabled,
            check_interval_hours: config.update_check.interval_hours,
        };
        claude_vm::update_check::check_and_notify(&update_config);
    }

    // Execute command
    match &cli.command {
        Some(Commands::Setup { .. }) => {
            commands::setup::execute(&project, &config)?;
        }
        Some(Commands::Shell) => {
            commands::shell::execute(&project, &config)?;
        }
        Some(Commands::Clean) => {
            commands::clean::execute(&project)?;
        }
        None => {
            // Default: run Claude with provided arguments
            commands::run::execute(&project, &config, &cli.claude_args)?;
        }
        _ => unreachable!(),
    }

    Ok(())
}
