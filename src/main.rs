#![forbid(unsafe_code)]

use clap::Parser;

use claude_vm::cli::{router, Cli, Commands, NetworkCommands, SessionSubcommand, WorktreeCommands};
use claude_vm::commands;
use claude_vm::config::Config;
use claude_vm::error::{ClaudeVmError, Result};
use claude_vm::project::Project;

fn main() {
    match run() {
        Ok(()) => {}
        Err(ClaudeVmError::CommandExitCode(code)) => {
            std::process::exit(code);
        }
        Err(e) => {
            eprintln!("❌ {}", e);
            std::process::exit(1);
        }
    }
}

fn run() -> Result<()> {
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
    let requires_project = cli.command.as_ref().is_some_and(Commands::needs_project);

    let (project, config) = if requires_project {
        // Must have project
        let proj = project_result?;

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
            Some(Commands::Session(cmd)) if matches!(cmd.subcommand, SessionSubcommand::Start) => {
                let mut cfg = Config::load_with_main_repo(proj.root(), proj.main_repo_root())?;
                cfg.verbose = cli.verbose;
                cfg
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
                return Err(e);
            }
        }
    } else {
        // No project, and that's OK for these commands
        (None, None)
    };

    // Handle commands that don't strictly need project but benefit from config validation
    match &cli.command {
        Some(Commands::List {
            unused,
            disk_usage,
            all,
        }) => {
            let effective_project = if *all { None } else { project.as_ref() };
            commands::list::execute(effective_project, *unused, *disk_usage)?;
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
        Some(Commands::Prune { yes }) => {
            commands::prune::execute(*yes)?;
            return Ok(());
        }
        Some(Commands::Session(cmd)) => match &cmd.subcommand {
            SessionSubcommand::Stop { id } => {
                commands::session::execute_stop(id)?;
                return Ok(());
            }
            SessionSubcommand::List => {
                commands::session::execute_list()?;
                return Ok(());
            }
            SessionSubcommand::Start => {
                // Handled below (needs project)
            }
        },
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
        Some(Commands::Setup(_cmd)) => {
            #[cfg(debug_assertions)]
            let skip_install = _cmd.no_agent_install;
            #[cfg(not(debug_assertions))]
            let skip_install = false;

            commands::setup::execute(&project, &config, skip_install)?;
        }
        Some(Commands::Info) => {
            commands::info::execute()?;
        }
        Some(Commands::Session(cmd)) => {
            // Only Start reaches here (Stop/List handled above)
            commands::session::execute(&project, &config, cmd)?;
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
            WorktreeCommands::List {
                merged,
                locked,
                detached,
            } => {
                commands::worktree::list::execute(merged.as_deref(), *locked, *detached)?;
            }
            WorktreeCommands::Remove {
                branches,
                merged,
                yes,
                dry_run,
                locked,
            } => {
                let branches_opt = if branches.is_empty() {
                    None
                } else {
                    Some(branches.as_slice())
                };
                commands::worktree::remove::execute(
                    branches_opt,
                    merged.as_deref(),
                    *yes,
                    *dry_run,
                    *locked,
                )?;
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
