use crate::cli::{Cli, Commands};
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Verbose mode - show verbose output including Lima logs (not stored in config file)
    #[serde(skip)]
    pub verbose: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            vm: VmConfig::default(),
            tools: ToolsConfig::default(),
            setup: SetupConfig::default(),
            runtime: RuntimeConfig::default(),
            defaults: DefaultsConfig::default(),
            verbose: false,
        }
    }
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
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SetupConfig {
    #[serde(default)]
    pub scripts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeConfig {
    #[serde(default)]
    pub scripts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DefaultsConfig {
    #[serde(default)]
    pub claude_args: Vec<String>,
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

        // Scripts (append)
        self.setup.scripts.extend(other.setup.scripts);
        self.runtime.scripts.extend(other.runtime.scripts);

        // Default Claude args (append)
        self.defaults.claude_args.extend(other.defaults.claude_args);

        self
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
            all,
            disk,
            memory,
            setup_scripts,
        }) = &cli.command
        {
            if *all {
                self.tools.docker = true;
                self.tools.node = true;
                self.tools.python = true;
                self.tools.chromium = true;
            } else {
                if *docker {
                    self.tools.docker = true;
                }
                if *node {
                    self.tools.node = true;
                }
                if *python {
                    self.tools.python = true;
                }
                if *chromium {
                    self.tools.chromium = true;
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
