#![forbid(unsafe_code)]

use anyhow::Result;
use clap::Parser;

use claude_vm::cli::{router, Cli, Commands, NetworkCommands, WorktreeCommands};
use claude_vm::config::Config;
use claude_vm::project::Project;
use claude_vm::{commands, error::ClaudeVmError};

fn main() -> Result<()> {
    // Route arguments to default to agent command when appropriate
    let args = std::env::args_os();
    let routed_args = router::route_args(args);
    let cli = Cli::parse_from(routed_args);

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
        Some(Commands::Agent(..))
            | Some(Commands::Setup(..))
            | Some(Commands::Shell(..))
            | Some(Commands::Info)
            | Some(Commands::Clean { .. })
            | Some(Commands::Network { .. })
            | Some(Commands::Worktree { .. })
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

        // Load config and apply command-specific overrides
        let cfg = match &cli.command {
            Some(Commands::Agent(cmd)) => {
                Config::load_with_main_repo(proj.root(), proj.main_repo_root())?
                    .with_runtime_overrides(&cmd.runtime, cli.verbose)
                    .with_conversations(!cmd.no_conversations)
            }
            Some(Commands::Shell(cmd)) => {
                Config::load_with_main_repo(proj.root(), proj.main_repo_root())?
                    .with_runtime_overrides(&cmd.runtime, cli.verbose)
            }
            Some(Commands::Setup(cmd)) => {
                Config::load_with_main_repo(proj.root(), proj.main_repo_root())?
                    .with_setup_overrides(cmd, cli.verbose)
            }
            _ => {
                let mut cfg = Config::load_with_main_repo(proj.root(), proj.main_repo_root())?;
                cfg.verbose = cli.verbose;
                cfg
            }
        };

        (Some(proj), Some(cfg))
    } else if let Ok(proj) = project_result {
        // Optional project, but if we have one, validate config
        match Config::load_with_main_repo(proj.root(), proj.main_repo_root()) {
            Ok(cfg) => (Some(proj), Some(cfg)),
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

    // Check for updates only on agent command (replaces old default run behavior)
    if matches!(&cli.command, Some(Commands::Agent(..))) {
        let update_config = claude_vm::update_check::UpdateCheckConfig {
            enabled: config.update_check.enabled,
            check_interval_hours: config.update_check.interval_hours,
        };
        claude_vm::update_check::check_and_notify(&update_config);
    }

    // Execute command
    match &cli.command {
        Some(Commands::Agent(cmd)) => {
            commands::agent::execute(&project, &config, cmd)?;
        }
        Some(Commands::Shell(cmd)) => {
            commands::shell::execute(&project, &config, cmd)?;
        }
        Some(Commands::Setup(cmd)) => {
            #[cfg(debug_assertions)]
            let skip_install = cmd.no_agent_install;
            #[cfg(not(debug_assertions))]
            let skip_install = false;

            commands::setup::execute(&project, &config, skip_install)?;
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
        Some(Commands::Worktree { command }) => match command {
            WorktreeCommands::Create { branch, base } => {
                commands::worktree::create::execute(&config, &project, branch, base.as_deref())?;
            }
            WorktreeCommands::List => {
                commands::worktree::list::execute()?;
            }
            WorktreeCommands::Delete { branch, yes } => {
                commands::worktree::delete::execute(branch, *yes)?;
            }
            WorktreeCommands::Clean { merged, yes } => {
                commands::worktree::clean::execute(merged, *yes)?;
            }
        },
        None => {
            // Router should always insert a subcommand; this is a safety net
            eprintln!(
                "Internal error: no command after routing. Run 'claude-vm --help' for usage."
            );
            std::process::exit(1);
        }
        _ => unreachable!(),
    }

    Ok(())
}
