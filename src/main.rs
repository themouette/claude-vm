#![forbid(unsafe_code)]

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
        Some(Commands::List { unused, disk_usage }) => {
            commands::list::execute(*unused, *disk_usage)?;
            return Ok(());
        }
        Some(Commands::Config { command }) => {
            commands::config::execute(command)?;
            return Ok(());
        }
        Some(Commands::CleanAll { yes }) => {
            commands::clean_all::execute(*yes)?;
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
            commands::shell::execute(&project, &config, &cli)?;
        }
        Some(Commands::Exec { command }) => {
            commands::exec::execute(&project, &config, &cli, command)?;
        }
        Some(Commands::Info) => {
            commands::info::execute()?;
        }
        Some(Commands::Clean { yes }) => {
            commands::clean::execute(&project, *yes)?;
        }
        Some(Commands::Logs { follow }) => {
            commands::logs::execute(&project, *follow)?;
        }
        None => {
            // Default: run Claude with provided arguments
            commands::run::execute(&project, &config, &cli, &cli.claude_args)?;
        }
        _ => unreachable!(),
    }

    Ok(())
}
