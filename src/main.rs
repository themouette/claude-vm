#![forbid(unsafe_code)]

use anyhow::Result;
use clap::Parser;

use claude_vm::cli::{Cli, Commands, NetworkCommands};
use claude_vm::config::Config;
use claude_vm::project::Project;
use claude_vm::{commands, error::ClaudeVmError};

fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Handle commands that truly don't need project or config
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
        _ => {}
    }

    // Try to detect project (most commands need it)
    // If we're in a project, load config to validate it (even if command doesn't use it)
    let project_result = Project::detect();

    // For commands that must have a project, fail if not found
    let requires_project = matches!(
        &cli.command,
        Some(Commands::Setup { .. })
            | Some(Commands::Shell { .. })
            | Some(Commands::Info)
            | Some(Commands::Clean { .. })
            | Some(Commands::Network { .. })
            | None // run command
    );

    let (project, config) = if requires_project {
        // Must have project
        let proj = project_result.map_err(|e| match e {
            ClaudeVmError::ProjectDetection(msg) => {
                eprintln!("Error: {}", msg);
                std::process::exit(1);
            }
            _ => e,
        })?;
        let cfg = Config::load_with_main_repo(proj.root(), proj.main_repo_root())?
            .with_cli_overrides(&cli);
        (Some(proj), Some(cfg))
    } else if let Ok(proj) = project_result {
        // Optional project, but if we have one, validate config
        match Config::load_with_main_repo(proj.root(), proj.main_repo_root()) {
            Ok(cfg) => {
                let cfg = cfg.with_cli_overrides(&cli);
                (Some(proj), Some(cfg))
            }
            Err(e) => {
                // Config is invalid - fail even for optional-project commands
                return Err(e.into());
            }
        }
    } else {
        // No project, and that's OK for these commands
        (None, None)
    };

    // Handle commands that don't strictly need project but benefit from config validation
    match &cli.command {
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

    // At this point, we must have project and config
    let project = project.unwrap();
    let config = config.unwrap();

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
        Some(Commands::Setup {
            #[cfg(debug_assertions)]
            no_agent_install,
            ..
        }) => {
            #[cfg(debug_assertions)]
            let skip_install = *no_agent_install;
            #[cfg(not(debug_assertions))]
            let skip_install = false;

            commands::setup::execute(&project, &config, skip_install)?;
        }
        Some(Commands::Shell { command }) => {
            commands::shell::execute(&project, &config, &cli, command)?;
        }
        Some(Commands::Info) => {
            commands::info::execute()?;
        }
        Some(Commands::Clean { yes }) => {
            commands::clean::execute(&project, *yes)?;
        }
        Some(Commands::Network { command }) => match command {
            NetworkCommands::Status => {
                commands::network::status::execute(&project, &config)?;
            }
            NetworkCommands::Logs {
                lines,
                filter,
                all,
                follow,
            } => {
                commands::network::logs::execute(
                    &project,
                    *lines,
                    filter.as_deref(),
                    *all,
                    *follow,
                )?;
            }
            NetworkCommands::Test { domain } => {
                commands::network::test::execute(&config, domain)?;
            }
        },
        None => {
            // Default: run Claude with provided arguments
            commands::run::execute(&project, &config, &cli, &cli.claude_args)?;
        }
        _ => unreachable!(),
    }

    Ok(())
}
