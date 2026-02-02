use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "claude-vm")]
#[command(about = "Run Claude Code inside sandboxed Lima VMs", long_about = None)]
#[command(version)]
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
    },

    /// Open a shell in the template VM
    Shell,

    /// List all claude-vm templates
    List,

    /// Clean the template for this project
    Clean,

    /// Clean all claude-vm templates
    CleanAll,

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
