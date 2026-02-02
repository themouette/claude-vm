use crate::cli::{Cli, Commands};
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub vm: VmConfig,

    #[serde(default)]
    pub tools: ToolsConfig,

    #[serde(default)]
    pub setup: SetupConfig,

    #[serde(default)]
    pub runtime: RuntimeConfig,

    #[serde(default)]
    pub defaults: DefaultsConfig,

    #[serde(default)]
    pub context: ContextConfig,

    #[serde(default)]
    pub mounts: Vec<MountEntry>,

    /// Verbose mode - show verbose output including Lima logs (not stored in config file)
    #[serde(skip)]
    pub verbose: bool,

    /// Forward SSH agent to VM (not stored in config file)
    #[serde(skip)]
    pub forward_ssh_agent: bool,

    /// Mount Claude conversation folder in VM (not stored in config file)
    #[serde(skip)]
    pub mount_conversations: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    #[serde(default = "default_disk")]
    pub disk: u32,

    #[serde(default = "default_memory")]
    pub memory: u32,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            disk: default_disk(),
            memory: default_memory(),
        }
    }
}

fn default_disk() -> u32 {
    20
}

fn default_memory() -> u32 {
    8
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolsConfig {
    #[serde(default)]
    pub docker: bool,

    #[serde(default)]
    pub node: bool,

    #[serde(default)]
    pub python: bool,

    #[serde(default)]
    pub chromium: bool,

    #[serde(default)]
    pub gpg: bool,

    #[serde(default)]
    pub gh: bool,
}

impl ToolsConfig {
    /// Check if a capability is enabled by ID
    pub fn is_enabled(&self, id: &str) -> bool {
        match id {
            "docker" => self.docker,
            "node" => self.node,
            "python" => self.python,
            "chromium" => self.chromium,
            "gpg" => self.gpg,
            "gh" => self.gh,
            _ => false,
        }
    }

    /// Enable a capability by ID
    pub fn enable(&mut self, id: &str) {
        match id {
            "docker" => self.docker = true,
            "node" => self.node = true,
            "python" => self.python = true,
            "chromium" => self.chromium = true,
            "gpg" => self.gpg = true,
            "gh" => self.gh = true,
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SetupConfig {
    #[serde(default)]
    pub scripts: Vec<String>,
    #[serde(default)]
    pub mounts: Vec<MountEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeConfig {
    #[serde(default)]
    pub scripts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextConfig {
    /// User-provided instructions for Claude
    #[serde(default)]
    pub instructions: String,

    /// Path to a file containing instructions for Claude
    #[serde(default)]
    pub instructions_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsConfig {
    #[serde(default = "default_claude_args")]
    pub claude_args: Vec<String>,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            claude_args: default_claude_args(),
        }
    }
}

fn default_claude_args() -> Vec<String> {
    vec!["--dangerously-skip-permissions".to_string()]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountEntry {
    pub location: String,
    #[serde(default = "default_writable")]
    pub writable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mount_point: Option<String>,
}

fn default_writable() -> bool {
    true
}

impl Config {
    /// Load configuration with precedence:
    /// 1. CLI flags (applied later via with_cli_overrides)
    /// 2. Environment variables
    /// 3. Project config (.claude-vm.toml in project root)
    /// 4. Global config (~/.claude-vm.toml)
    /// 5. Built-in defaults
    pub fn load(project_root: &Path) -> Result<Self> {
        let mut config = Self::default();

        // 1. Load global config
        if let Some(home) = home_dir() {
            let global_config = home.join(".claude-vm.toml");
            if global_config.exists() {
                config = config.merge(Self::from_file(&global_config)?);
            }
        }

        // 2. Load project config
        let project_config = project_root.join(".claude-vm.toml");
        if project_config.exists() {
            config = config.merge(Self::from_file(&project_config)?);
        }

        // 3. Apply environment variables
        config = config.merge_env();

        // 4. Resolve context file if needed
        config = config.resolve_context_file()?;

        Ok(config)
    }

    /// Load configuration from a TOML file
    fn from_file(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Merge another config into this one (other takes precedence)
    fn merge(mut self, other: Self) -> Self {
        // VM settings
        if other.vm.disk != default_disk() {
            self.vm.disk = other.vm.disk;
        }
        if other.vm.memory != default_memory() {
            self.vm.memory = other.vm.memory;
        }

        // Tools
        self.tools.docker = self.tools.docker || other.tools.docker;
        self.tools.node = self.tools.node || other.tools.node;
        self.tools.python = self.tools.python || other.tools.python;
        self.tools.chromium = self.tools.chromium || other.tools.chromium;
        self.tools.gpg = self.tools.gpg || other.tools.gpg;
        self.tools.gh = self.tools.gh || other.tools.gh;

        // Scripts (append)
        self.setup.scripts.extend(other.setup.scripts);
        self.runtime.scripts.extend(other.runtime.scripts);

        // Default Claude args (append)
        self.defaults.claude_args.extend(other.defaults.claude_args);

        // Context (replace if not empty)
        if !other.context.instructions.is_empty() {
            self.context.instructions = other.context.instructions;
        }
        if !other.context.instructions_file.is_empty() {
            self.context.instructions_file = other.context.instructions_file;
        }

        self
    }

    /// Load context from file if instructions_file is set and instructions is empty
    fn resolve_context_file(mut self) -> Result<Self> {
        // If instructions is already set, don't load from file
        if !self.context.instructions.is_empty() {
            return Ok(self);
        }

        // If instructions_file is set, load from file
        if !self.context.instructions_file.is_empty() {
            let file_path = if self.context.instructions_file.starts_with('~') {
                // Expand ~ to home directory
                if let Some(home) = home_dir() {
                    let path_without_tilde = self.context.instructions_file.trim_start_matches('~');
                    home.join(path_without_tilde.trim_start_matches('/'))
                } else {
                    PathBuf::from(&self.context.instructions_file)
                }
            } else {
                PathBuf::from(&self.context.instructions_file)
            };

            // Read file content
            match std::fs::read_to_string(&file_path) {
                Ok(content) => {
                    self.context.instructions = content;
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to read context file '{}': {}",
                        file_path.display(),
                        e
                    );
                }
            }
        }

        Ok(self)
    }

    /// Apply environment variable overrides
    fn merge_env(mut self) -> Self {
        if let Ok(disk) = std::env::var("CLAUDE_VM_DISK") {
            if let Ok(disk) = disk.parse::<u32>() {
                self.vm.disk = disk;
            }
        }

        if let Ok(memory) = std::env::var("CLAUDE_VM_MEMORY") {
            if let Ok(memory) = memory.parse::<u32>() {
                self.vm.memory = memory;
            }
        }

        self
    }

    /// Apply CLI overrides (highest precedence)
    pub fn with_cli_overrides(mut self, cli: &Cli) -> Self {
        // Verbose flag
        self.verbose = cli.verbose;

        // SSH agent forwarding
        self.forward_ssh_agent = cli.forward_ssh_agent;

        // Mount conversations (inverted: --no-conversations means mount_conversations = false)
        self.mount_conversations = !cli.no_conversations;

        // Custom mounts from CLI (accumulate with config mounts)
        // Parse CLI mount specs immediately to validate and extract values
        for mount_spec in &cli.mounts {
            // Parse the mount spec to extract location, mount_point, and writable
            match crate::vm::mount::Mount::from_spec(mount_spec) {
                Ok(mount) => {
                    self.mounts.push(MountEntry {
                        location: mount.location.to_string_lossy().to_string(),
                        writable: mount.writable,
                        mount_point: mount.mount_point.map(|p| p.to_string_lossy().to_string()),
                    });
                }
                Err(e) => {
                    eprintln!("Warning: Invalid mount spec '{}': {}", mount_spec, e);
                }
            }
        }

        // Global CLI overrides
        if let Some(disk) = cli.disk {
            self.vm.disk = disk;
        }
        if let Some(memory) = cli.memory {
            self.vm.memory = memory;
        }

        // Command-specific overrides
        if let Some(Commands::Setup {
            docker,
            node,
            python,
            chromium,
            gpg,
            gh,
            all,
            disk,
            memory,
            setup_scripts,
            mounts,
        }) = &cli.command
        {
            if *all {
                self.tools.enable("docker");
                self.tools.enable("node");
                self.tools.enable("python");
                self.tools.enable("chromium");
                self.tools.enable("gpg");
                self.tools.enable("gh");
            } else {
                if *docker {
                    self.tools.enable("docker");
                }
                if *node {
                    self.tools.enable("node");
                }
                if *python {
                    self.tools.enable("python");
                }
                if *chromium {
                    self.tools.enable("chromium");
                }
                if *gpg {
                    self.tools.enable("gpg");
                }
                if *gh {
                    self.tools.enable("gh");
                }
            }

            if let Some(d) = disk {
                self.vm.disk = *d;
            }
            if let Some(m) = memory {
                self.vm.memory = *m;
            }

            // Add setup scripts from CLI
            for script in setup_scripts {
                if let Some(script_str) = script.to_str() {
                    self.setup.scripts.push(script_str.to_string());
                }
            }

            // Add setup mounts from CLI (parse immediately like runtime mounts)
            for mount_spec in mounts {
                match crate::vm::mount::Mount::from_spec(mount_spec) {
                    Ok(mount) => {
                        self.setup.mounts.push(MountEntry {
                            location: mount.location.to_string_lossy().to_string(),
                            writable: mount.writable,
                            mount_point: mount.mount_point.map(|p| p.to_string_lossy().to_string()),
                        });
                    }
                    Err(e) => {
                        eprintln!("Warning: Invalid setup mount spec '{}': {}", mount_spec, e);
                    }
                }
            }
        }

        // Add runtime scripts from CLI
        for script in &cli.runtime_scripts {
            if let Some(script_str) = script.to_str() {
                self.runtime.scripts.push(script_str.to_string());
            }
        }

        self
    }
}

/// Get the home directory
fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.vm.disk, 20);
        assert_eq!(config.vm.memory, 8);
        assert!(!config.tools.docker);
    }

    #[test]
    fn test_merge_config() {
        let mut base = Config::default();
        base.vm.disk = 30;

        let mut override_cfg = Config::default();
        override_cfg.vm.memory = 16;
        override_cfg.tools.docker = true;

        let merged = base.merge(override_cfg);
        assert_eq!(merged.vm.disk, 30); // Kept from base
        assert_eq!(merged.vm.memory, 16); // From override
        assert!(merged.tools.docker); // From override
    }
}
