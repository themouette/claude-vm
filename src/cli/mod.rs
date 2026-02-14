use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub mod flags;
pub mod router;
pub use flags::{RuntimeFlags, SetupVmFlags};

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Validate configuration files
    Validate {
        /// Optional path to a specific config file to validate
        file: Option<PathBuf>,
    },

    /// Show effective configuration after merging all sources
    Show,
}

#[derive(Subcommand, Debug)]
pub enum NetworkCommands {
    /// Show network isolation status
    Status,

    /// View network isolation logs
    Logs {
        /// Number of lines to show (default: 50)
        #[arg(short = 'n', long, default_value = "50")]
        lines: usize,

        /// Filter logs by pattern
        #[arg(short = 'f', long)]
        filter: Option<String>,

        /// Show all logs (no line limit)
        #[arg(long)]
        all: bool,

        /// Follow log output in real-time (like tail -f)
        #[arg(long)]
        follow: bool,
    },

    /// Test if a domain would be allowed or blocked
    Test {
        /// Domain to test (e.g., example.com or *.example.com)
        domain: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum WorktreeCommands {
    /// Create a new worktree for a branch
    Create {
        /// Branch name for the worktree
        branch: String,

        /// Base branch or commit to create from (default: current HEAD)
        base: Option<String>,
    },

    /// List all worktrees
    List {
        /// Show only worktrees for branches merged into base
        #[arg(long)]
        merged: Option<String>,

        /// Show only locked worktrees
        #[arg(long)]
        locked: bool,

        /// Show only detached HEAD worktrees
        #[arg(long)]
        detached: bool,
    },

    /// Remove worktrees (by name or merged status)
    #[command(alias = "rm")]
    Remove {
        /// Branch name(s) of the worktree(s) to remove
        branches: Vec<String>,

        /// Remove worktrees for branches merged into base (defaults to current branch)
        #[arg(long, conflicts_with = "branches", num_args(0..=1), default_missing_value = "")]
        merged: Option<String>,

        /// Include locked worktrees when using --merged
        #[arg(long, requires = "merged")]
        locked: bool,

        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,

        /// Show what would be removed without making changes
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Parser, Debug)]
#[command(name = "claude-vm")]
#[command(about = "Run Claude Code inside sandboxed Lima VMs", long_about = None)]
#[command(version = env!("CLAUDE_VM_VERSION"))]
#[command(after_help = "\
INVOCATION PATTERNS:
  The 'agent' command is the default. These are equivalent:

  claude-vm [options] [args]         Shorthand for 'claude-vm agent'
  claude-vm agent [options] [args]   Explicit agent command

EXAMPLES:
  claude-vm /clear                   Start Claude with /clear command
  claude-vm --disk 50 /clear         Use 50GB disk (shorthand)
  claude-vm agent --disk 50 /clear   Same as above (explicit form)
  claude-vm shell                    Open an interactive VM shell

For details about a specific command, use:
  claude-vm <command> --help")]
pub struct Cli {
    /// Show verbose output including Lima logs
    #[arg(short = 'v', long = "verbose", global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run Claude Code agent in an ephemeral VM
    #[command(long_about = "Run Claude Code agent in an ephemeral VM.\n\n\
        Creates a fresh VM from your project's template, runs Claude Code,\n\
        and destroys the VM when done. This is the default command - you can\n\
        omit 'agent' and use 'claude-vm [options] [args]' as a shorthand.")]
    Agent(AgentCmd),

    /// Open a shell or execute a command in an ephemeral VM
    #[command(
        long_about = "Open a shell or execute a command in an ephemeral VM.\n\n\
        Without arguments: Opens an interactive shell in a fresh VM.\n\
        With arguments: Executes the command in the VM and exits."
    )]
    Shell(ShellCmd),

    /// Set up a new template VM for this project
    Setup(SetupCmd),

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

    /// Network isolation commands
    Network {
        #[command(subcommand)]
        command: NetworkCommands,
    },

    /// Manage git worktrees for parallel development
    #[command(alias = "w")]
    Worktree {
        #[command(subcommand)]
        command: WorktreeCommands,
    },
}

#[derive(Parser, Debug)]
pub struct AgentCmd {
    /// Runtime configuration flags
    #[command(flatten)]
    pub runtime: RuntimeFlags,

    /// Don't mount Claude conversation folder in VM
    #[arg(long = "no-conversations")]
    pub no_conversations: bool,

    /// Arguments to pass to Claude
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub claude_args: Vec<String>,
}

#[derive(Parser, Debug)]
pub struct ShellCmd {
    /// Runtime configuration flags
    #[command(flatten)]
    pub runtime: RuntimeFlags,

    /// Command to execute (optional, opens interactive shell if not provided)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub command: Vec<String>,
}

#[derive(Parser, Debug)]
pub struct SetupCmd {
    /// VM sizing flags
    #[command(flatten)]
    pub vm_flags: SetupVmFlags,

    /// Install Docker
    #[arg(long)]
    pub docker: bool,

    /// Install Node.js
    #[arg(long)]
    pub node: bool,

    /// Install Python
    #[arg(long)]
    pub python: bool,

    /// Install Rust toolchain
    #[arg(long)]
    pub rust: bool,

    /// Install Chromium for debugging
    #[arg(long)]
    pub chromium: bool,

    /// Enable GPG agent forwarding
    #[arg(long)]
    pub gpg: bool,

    /// Install GitHub CLI
    #[arg(long)]
    pub gh: bool,

    /// Configure git from host
    #[arg(long)]
    pub git: bool,

    /// Enable network isolation
    #[arg(long)]
    pub network_isolation: bool,

    /// Install all tools
    #[arg(long)]
    pub all: bool,

    /// Setup scripts to execute
    #[arg(long = "setup-script")]
    pub setup_scripts: Vec<PathBuf>,

    /// Setup-only mounts (available during template creation only)
    #[arg(long = "mount")]
    pub mounts: Vec<String>,

    /// Skip Claude Code agent installation (dev builds only)
    #[cfg(debug_assertions)]
    #[arg(long)]
    pub no_agent_install: bool,
}
