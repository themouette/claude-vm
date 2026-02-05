use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Validate configuration files
    Validate,

    /// Show effective configuration after merging all sources
    Show,
}

#[derive(Parser, Debug)]
#[command(name = "claude-vm")]
#[command(about = "Run Claude Code inside sandboxed Lima VMs", long_about = None)]
#[command(version = env!("CLAUDE_VM_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Arguments to pass to Claude (when no subcommand)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub claude_args: Vec<String>,

    /// Runtime script to execute before starting
    #[arg(long = "runtime-script", global = true)]
    pub runtime_scripts: Vec<PathBuf>,

    /// VM disk size in GB
    #[arg(long, global = true)]
    pub disk: Option<u32>,

    /// VM memory size in GB
    #[arg(long, global = true)]
    pub memory: Option<u32>,

    /// Show verbose output including Lima logs
    #[arg(short = 'v', long = "verbose", global = true)]
    pub verbose: bool,

    /// Forward SSH agent to VM
    #[arg(short = 'A', long = "forward-ssh-agent", global = true)]
    pub forward_ssh_agent: bool,

    /// Don't mount Claude conversation folder in VM
    #[arg(long = "no-conversations", global = true)]
    pub no_conversations: bool,

    /// Custom mount in docker-style format: /host/path[:vm/path][:ro|rw]
    #[arg(long = "mount", global = true)]
    pub mounts: Vec<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Set up a new template VM for this project
    Setup {
        /// Install Docker
        #[arg(long)]
        docker: bool,

        /// Install Node.js
        #[arg(long)]
        node: bool,

        /// Install Python
        #[arg(long)]
        python: bool,

        /// Install Chromium for debugging
        #[arg(long)]
        chromium: bool,

        /// Enable GPG agent forwarding
        #[arg(long)]
        gpg: bool,

        /// Install GitHub CLI
        #[arg(long)]
        gh: bool,

        /// Configure git from host
        #[arg(long)]
        git: bool,

        /// Install all tools
        #[arg(long)]
        all: bool,

        /// VM disk size in GB
        #[arg(long)]
        disk: Option<u32>,

        /// VM memory size in GB
        #[arg(long)]
        memory: Option<u32>,

        /// Setup scripts to execute
        #[arg(long = "setup-script")]
        setup_scripts: Vec<PathBuf>,

        /// Setup-only mounts (available during template creation only)
        #[arg(long = "mount")]
        mounts: Vec<String>,
    },

    /// Open a shell in the template VM
    Shell,

    /// Show information about the current project's template
    Info,

    /// Configuration management commands
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// List all claude-vm templates
    List {
        /// Show only unused templates (not used in 30 days)
        #[arg(long)]
        unused: bool,

        /// Show disk usage information
        #[arg(long)]
        disk_usage: bool,
    },

    /// Clean the template for this project
    Clean {
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Clean all claude-vm templates
    CleanAll {
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Check claude-vm version and updates
    Version {
        /// Check for available updates
        #[arg(long)]
        check: bool,
    },

    /// Update claude-vm to the latest version
    Update {
        /// Check for updates without installing
        #[arg(long)]
        check: bool,

        /// Update to specific version
        #[arg(long)]
        version: Option<String>,

        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
}
