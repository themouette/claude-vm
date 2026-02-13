use clap::Parser;
use std::path::PathBuf;

/// Runtime flags shared by agent and shell commands.
/// These flags configure the ephemeral VM session.
#[derive(Parser, Debug, Clone, Default)]
pub struct RuntimeFlags {
    /// VM disk size in GB
    #[arg(long)]
    pub disk: Option<u32>,

    /// VM memory size in GB
    #[arg(long)]
    pub memory: Option<u32>,

    /// Number of CPUs for the VM
    #[arg(long)]
    pub cpus: Option<u32>,

    /// Forward SSH agent to VM
    #[arg(short = 'A', long = "forward-ssh-agent")]
    pub forward_ssh_agent: bool,

    /// Custom mount in docker-style format: /host/path[:vm/path][:ro|rw]
    #[arg(long = "mount")]
    pub mounts: Vec<String>,

    /// Set environment variable (KEY=VALUE)
    #[arg(long = "env")]
    pub env: Vec<String>,

    /// Load environment variables from file
    #[arg(long = "env-file")]
    pub env_file: Vec<PathBuf>,

    /// Inherit specific environment variables from host
    #[arg(long = "inherit-env")]
    pub inherit_env: Vec<String>,

    /// Runtime script to execute before starting
    #[arg(long = "runtime-script")]
    pub runtime_scripts: Vec<PathBuf>,

    /// Automatically create template if missing
    #[arg(long = "auto-setup")]
    pub auto_setup: bool,
}

/// VM sizing flags for the setup command.
/// Setup only needs disk, memory, and cpus — not runtime-specific flags.
#[derive(Parser, Debug, Clone, Default)]
pub struct SetupVmFlags {
    /// VM disk size in GB
    #[arg(long)]
    pub disk: Option<u32>,

    /// VM memory size in GB
    #[arg(long)]
    pub memory: Option<u32>,

    /// Number of CPUs for the VM
    #[arg(long)]
    pub cpus: Option<u32>,
}
