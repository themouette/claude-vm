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

    /// User-defined packages to install
    #[serde(default)]
    pub packages: PackagesConfig,

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

    #[serde(default)]
    pub update_check: UpdateCheckSettings,

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

    #[serde(default)]
    pub git: bool,
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
            "git" => self.git,
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
            "git" => self.git = true,
            _ => {}
        }
    }
}

/// User-defined package specifications.
///
/// Users can specify additional packages to install in their .claude-vm.toml files.
/// These are merged with capability-defined packages and installed together in a
/// single batch operation to minimize setup time.
///
/// ## Version Pinning Example
///
/// ```toml
/// [packages]
/// system = [
///     "postgresql-client",     # Latest version
///     "redis-tools=7.0.*",     # Redis 7.0.x
///     "jq",                    # Latest version
///     "htop",                  # Latest version
/// ]
/// ```
///
/// ## Custom Repository Setup
///
/// ⚠️  **SECURITY WARNING**: `setup_script` executes arbitrary bash code with sudo privileges
/// in the VM during setup. Only use scripts from trusted sources. Malicious scripts can
/// compromise the VM.
///
/// ```toml
/// [packages]
/// system = ["my-custom-package"]
/// setup_script = """
/// #!/bin/bash
/// set -e
/// # Add custom repository
/// sudo add-apt-repository ppa:my-ppa/custom
/// # Or manually add repository and key
/// curl -fsSL https://example.com/key.gpg | sudo tee /etc/apt/keyrings/custom.gpg > /dev/null
/// echo "deb [signed-by=/etc/apt/keyrings/custom.gpg] https://example.com/debian stable main" | sudo tee /etc/apt/sources.list.d/custom.list
/// """
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackagesConfig {
    /// System packages to install via apt.
    ///
    /// Supports version pinning:
    /// - "package" - latest version
    /// - "package=1.2.3" - exact version
    /// - "package=1.2.*" - version wildcard
    /// - "package:amd64" - specific architecture
    #[serde(default)]
    pub system: Vec<String>,

    /// Optional script to run before apt-get update (adds custom repositories, GPG keys).
    ///
    /// ⚠️  **SECURITY WARNING**: This script runs with sudo privileges. Only use
    /// trusted scripts. Review any script before adding it to your configuration.
    ///
    /// This script runs in the same phase as capability repository setup scripts,
    /// before the single apt-get update call.
    ///
    /// Example: Add a PPA
    /// ```bash
    /// #!/bin/bash
    /// set -e
    /// sudo add-apt-repository -y ppa:deadsnakes/ppa
    /// ```
    #[serde(default)]
    pub setup_script: Option<String>,
    // Future extensions: npm, pip, cargo, etc.
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckSettings {
    #[serde(default = "default_update_check_enabled")]
    pub enabled: bool,

    #[serde(default = "default_update_check_interval")]
    pub interval_hours: u64,
}

impl Default for UpdateCheckSettings {
    fn default() -> Self {
        Self {
            enabled: default_update_check_enabled(),
            interval_hours: default_update_check_interval(),
        }
    }
}

fn default_update_check_enabled() -> bool {
    true
}

fn default_update_check_interval() -> u64 {
    72 // 3 days
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
        self.tools.git = self.tools.git || other.tools.git;

        // Packages (extend/append)
        self.packages.system.extend(other.packages.system);
        // Merge setup_script (other takes precedence if present)
        if other.packages.setup_script.is_some() {
            self.packages.setup_script = other.packages.setup_script;
        }

        // Scripts (append)
        self.setup.scripts.extend(other.setup.scripts);
        self.runtime.scripts.extend(other.runtime.scripts);

        // Mounts (append)
        self.mounts.extend(other.mounts);
        self.setup.mounts.extend(other.setup.mounts);

        // Default Claude args (append)
        self.defaults.claude_args.extend(other.defaults.claude_args);

        // Context (replace if not empty)
        if !other.context.instructions.is_empty() {
            self.context.instructions = other.context.instructions;
        }
        if !other.context.instructions_file.is_empty() {
            self.context.instructions_file = other.context.instructions_file;
        }

        // Update check settings (other takes precedence)
        self.update_check = other.update_check;

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
            // Expand ~ in the path (supports both ~ and ~user syntax)
            let file_path = crate::utils::path::expand_tilde(&self.context.instructions_file)
                .unwrap_or_else(|| PathBuf::from(&self.context.instructions_file));

            // Read file content
            match std::fs::read_to_string(&file_path) {
                Ok(content) => {
                    self.context.instructions = content;
                }
                Err(e) => {
                    use std::io::{self, Write};

                    // Print highly visible warning
                    eprintln!();
                    eprintln!("╔═══════════════════════════════════════════════════════╗");
                    eprintln!("║ ⚠️  WARNING: Failed to load context file            ║");
                    eprintln!("╚═══════════════════════════════════════════════════════╝");
                    eprintln!("  File: {}", file_path.display());
                    eprintln!("  Error: {}", e);
                    eprintln!();
                    eprintln!("  Claude will start WITHOUT your custom instructions.");
                    eprintln!();

                    // Prompt user to continue
                    eprint!("Continue anyway? [y/N]: ");
                    io::stderr().flush().ok();

                    let mut input = String::new();
                    match io::stdin().read_line(&mut input) {
                        Ok(_) => {
                            if !input.trim().eq_ignore_ascii_case("y") {
                                return Err(crate::error::ClaudeVmError::InvalidConfig(
                                    "Context file load failed and user chose to abort".to_string(),
                                ));
                            }
                        }
                        Err(_) => {
                            // If stdin is not available (non-interactive), abort
                            return Err(crate::error::ClaudeVmError::InvalidConfig(format!(
                                "Failed to read context file '{}': {}",
                                file_path.display(),
                                e
                            )));
                        }
                    }
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

        if let Ok(enabled) = std::env::var("CLAUDE_VM_UPDATE_CHECK") {
            if let Ok(enabled) = enabled.parse::<bool>() {
                self.update_check.enabled = enabled;
            }
        }

        if let Ok(interval) = std::env::var("CLAUDE_VM_UPDATE_INTERVAL") {
            if let Ok(interval) = interval.parse::<u64>() {
                self.update_check.interval_hours = interval;
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
            git,
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
                self.tools.enable("git");
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
                if *git {
                    self.tools.enable("git");
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

    #[test]
    fn test_merge_git_capability() {
        let base = Config::default();
        let mut override_cfg = Config::default();
        override_cfg.tools.git = true;
        override_cfg.tools.gpg = true;
        override_cfg.tools.gh = true;

        let merged = base.merge(override_cfg);
        assert!(merged.tools.git);
        assert!(merged.tools.gpg);
        assert!(merged.tools.gh);
    }

    #[test]
    fn test_context_instructions_inline() {
        let mut config = Config::default();
        config.context.instructions = "Test instructions".to_string();

        assert_eq!(config.context.instructions, "Test instructions");
    }

    #[test]
    fn test_context_merge() {
        let mut base = Config::default();
        base.context.instructions = "Base instructions".to_string();

        let mut override_cfg = Config::default();
        override_cfg.context.instructions = "Override instructions".to_string();

        let merged = base.merge(override_cfg);
        assert_eq!(merged.context.instructions, "Override instructions");
    }

    #[test]
    fn test_context_file_loading() {
        use std::io::Write;

        // Create a temporary context file
        let temp_dir = std::env::temp_dir();
        let context_file = temp_dir.join("test-context.md");
        let mut file = std::fs::File::create(&context_file).unwrap();
        writeln!(file, "# Test Context\nThis is test content.").unwrap();
        drop(file);

        // Create config with context file
        let mut config = Config::default();
        config.context.instructions_file = context_file.to_string_lossy().to_string();

        // Resolve context file
        let config = config.resolve_context_file().unwrap();

        // Verify content was loaded
        assert!(config.context.instructions.contains("Test Context"));
        assert!(config.context.instructions.contains("This is test content"));

        // Cleanup
        std::fs::remove_file(&context_file).unwrap();
    }

    #[test]
    fn test_context_instructions_precedence() {
        use std::io::Write;

        // Create a temporary context file
        let temp_dir = std::env::temp_dir();
        let context_file = temp_dir.join("test-context-precedence.md");
        let mut file = std::fs::File::create(&context_file).unwrap();
        writeln!(file, "File content").unwrap();
        drop(file);

        // Create config with both instructions and file
        let mut config = Config::default();
        config.context.instructions = "Inline content".to_string();
        config.context.instructions_file = context_file.to_string_lossy().to_string();

        // Resolve context file
        let config = config.resolve_context_file().unwrap();

        // Verify inline instructions take precedence
        assert_eq!(config.context.instructions, "Inline content");

        // Cleanup
        std::fs::remove_file(&context_file).unwrap();
    }

    #[test]
    fn test_context_file_not_found() {
        // Create config with non-existent file
        let mut config = Config::default();
        config.context.instructions_file = "/nonexistent/path/to/file.md".to_string();

        // Should error in non-interactive mode (tests have no user input)
        let result = config.resolve_context_file();
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        // Either stdin read fails, or user doesn't confirm
        assert!(
            error_msg.contains("Failed to read context file")
                || error_msg.contains("Context file load failed")
        );
    }

    #[test]
    fn test_context_file_empty_when_no_file() {
        let config = Config::default();
        let config = config.resolve_context_file().unwrap();

        // Should remain empty
        assert!(config.context.instructions.is_empty());
    }

    #[test]
    #[serial_test::serial]
    fn test_context_tilde_expansion() {
        use std::io::Write;

        // Create a temporary home directory
        let temp_home =
            std::env::temp_dir().join(format!("claude-vm-test-home-{}", std::process::id()));
        std::fs::create_dir_all(&temp_home).unwrap();

        // Create context file in temp home
        let context_file = temp_home.join(".test-context-tilde.md");
        let mut file = std::fs::File::create(&context_file).unwrap();
        writeln!(file, "Tilde test content").unwrap();
        drop(file);

        // Temporarily set HOME
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &temp_home);

        // Create config with ~ path
        let mut config = Config::default();
        config.context.instructions_file = "~/.test-context-tilde.md".to_string();

        // Resolve context file
        let config = config.resolve_context_file().unwrap();

        // Verify content was loaded
        assert!(config.context.instructions.contains("Tilde test content"));

        // Restore original HOME
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }

        // Cleanup
        std::fs::remove_dir_all(&temp_home).unwrap();
    }

    #[test]
    fn test_update_check_defaults() {
        let config = Config::default();
        assert!(config.update_check.enabled);
        assert_eq!(config.update_check.interval_hours, 72);
    }

    #[test]
    fn test_update_check_merge() {
        let base = Config::default();
        let mut override_cfg = Config::default();
        override_cfg.update_check.enabled = false;
        override_cfg.update_check.interval_hours = 168;

        let merged = base.merge(override_cfg);
        assert!(!merged.update_check.enabled);
        assert_eq!(merged.update_check.interval_hours, 168);
    }

    #[test]
    fn test_mounts_merge() {
        // Create base config with one mount
        let mut base = Config::default();
        base.mounts.push(MountEntry {
            location: "/host/path1".to_string(),
            writable: true,
            mount_point: None,
        });

        // Create override config with another mount
        let mut override_cfg = Config::default();
        override_cfg.mounts.push(MountEntry {
            location: "/host/path2".to_string(),
            writable: false,
            mount_point: Some("/vm/path2".to_string()),
        });

        // Merge configs
        let merged = base.merge(override_cfg);

        // Verify both mounts are present
        assert_eq!(merged.mounts.len(), 2);
        assert_eq!(merged.mounts[0].location, "/host/path1");
        assert!(merged.mounts[0].writable);
        assert_eq!(merged.mounts[1].location, "/host/path2");
        assert!(!merged.mounts[1].writable);
        assert_eq!(merged.mounts[1].mount_point, Some("/vm/path2".to_string()));
    }

    #[test]
    fn test_setup_mounts_merge() {
        // Create base config with one setup mount
        let mut base = Config::default();
        base.setup.mounts.push(MountEntry {
            location: "/setup/path1".to_string(),
            writable: true,
            mount_point: None,
        });

        // Create override config with another setup mount
        let mut override_cfg = Config::default();
        override_cfg.setup.mounts.push(MountEntry {
            location: "/setup/path2".to_string(),
            writable: true,
            mount_point: None,
        });

        // Merge configs
        let merged = base.merge(override_cfg);

        // Verify both setup mounts are present
        assert_eq!(merged.setup.mounts.len(), 2);
        assert_eq!(merged.setup.mounts[0].location, "/setup/path1");
        assert_eq!(merged.setup.mounts[1].location, "/setup/path2");
    }

    #[test]
    fn test_tools_merge() {
        // Create base config with some tools enabled
        let mut base = Config::default();
        base.tools.docker = true;
        base.tools.node = true;

        // Create override config with different tools enabled
        let mut override_cfg = Config::default();
        override_cfg.tools.python = true;
        override_cfg.tools.chromium = true;
        override_cfg.tools.gpg = true;

        // Merge configs
        let merged = base.merge(override_cfg);

        // Verify all tools are enabled (ORed together)
        assert!(merged.tools.docker);
        assert!(merged.tools.node);
        assert!(merged.tools.python);
        assert!(merged.tools.chromium);
        assert!(merged.tools.gpg);
        assert!(!merged.tools.gh); // Not enabled in either
    }

    #[test]
    fn test_packages_system_merge() {
        // Create base config with system packages
        let mut base = Config::default();
        base.packages.system.push("htop".to_string());
        base.packages.system.push("curl".to_string());

        // Create override config with additional system packages
        let mut override_cfg = Config::default();
        override_cfg.packages.system.push("jq".to_string());
        override_cfg.packages.system.push("vim".to_string());

        // Merge configs
        let merged = base.merge(override_cfg);

        // Verify all packages are present (extended)
        assert_eq!(merged.packages.system.len(), 4);
        assert!(merged.packages.system.contains(&"htop".to_string()));
        assert!(merged.packages.system.contains(&"curl".to_string()));
        assert!(merged.packages.system.contains(&"jq".to_string()));
        assert!(merged.packages.system.contains(&"vim".to_string()));
    }

    #[test]
    fn test_packages_setup_script_merge() {
        // Create base config with setup_script
        let mut base = Config::default();
        base.packages.setup_script = Some("echo 'global setup'".to_string());

        // Create override config with different setup_script
        let mut override_cfg = Config::default();
        override_cfg.packages.setup_script = Some("echo 'project setup'".to_string());

        // Merge configs
        let merged = base.merge(override_cfg);

        // Verify override takes precedence
        assert_eq!(
            merged.packages.setup_script,
            Some("echo 'project setup'".to_string())
        );
    }

    #[test]
    fn test_packages_setup_script_merge_none() {
        // Create base config with setup_script
        let mut base = Config::default();
        base.packages.setup_script = Some("echo 'global setup'".to_string());

        // Create override config with no setup_script
        let override_cfg = Config::default();

        // Merge configs
        let merged = base.merge(override_cfg);

        // Verify base setup_script is preserved when override is None
        assert_eq!(
            merged.packages.setup_script,
            Some("echo 'global setup'".to_string())
        );
    }

    #[test]
    fn test_setup_scripts_merge() {
        // Create base config with setup scripts
        let mut base = Config::default();
        base.setup.scripts.push("script1.sh".to_string());
        base.setup.scripts.push("script2.sh".to_string());

        // Create override config with additional setup scripts
        let mut override_cfg = Config::default();
        override_cfg.setup.scripts.push("script3.sh".to_string());

        // Merge configs
        let merged = base.merge(override_cfg);

        // Verify all scripts are present (extended)
        assert_eq!(merged.setup.scripts.len(), 3);
        assert_eq!(merged.setup.scripts[0], "script1.sh");
        assert_eq!(merged.setup.scripts[1], "script2.sh");
        assert_eq!(merged.setup.scripts[2], "script3.sh");
    }

    #[test]
    fn test_runtime_scripts_merge() {
        // Create base config with runtime scripts
        let mut base = Config::default();
        base.runtime.scripts.push("runtime1.sh".to_string());

        // Create override config with additional runtime scripts
        let mut override_cfg = Config::default();
        override_cfg.runtime.scripts.push("runtime2.sh".to_string());
        override_cfg.runtime.scripts.push("runtime3.sh".to_string());

        // Merge configs
        let merged = base.merge(override_cfg);

        // Verify all scripts are present (extended)
        assert_eq!(merged.runtime.scripts.len(), 3);
        assert_eq!(merged.runtime.scripts[0], "runtime1.sh");
        assert_eq!(merged.runtime.scripts[1], "runtime2.sh");
        assert_eq!(merged.runtime.scripts[2], "runtime3.sh");
    }

    #[test]
    fn test_defaults_claude_args_merge() {
        // Create base config with claude args
        let mut base = Config::default();
        base.defaults.claude_args = vec!["--arg1".to_string()];

        // Create override config with additional claude args
        let mut override_cfg = Config::default();
        override_cfg.defaults.claude_args = vec!["--arg2".to_string(), "--arg3".to_string()];

        // Merge configs
        let merged = base.merge(override_cfg);

        // Verify all args are present (extended)
        assert_eq!(merged.defaults.claude_args.len(), 3);
        assert_eq!(merged.defaults.claude_args[0], "--arg1");
        assert_eq!(merged.defaults.claude_args[1], "--arg2");
        assert_eq!(merged.defaults.claude_args[2], "--arg3");
    }

    #[test]
    fn test_context_instructions_file_merge() {
        // Create base config with instructions_file
        let mut base = Config::default();
        base.context.instructions_file = "~/.global-context.md".to_string();

        // Create override config with different instructions_file
        let mut override_cfg = Config::default();
        override_cfg.context.instructions_file = "./.local-context.md".to_string();

        // Merge configs
        let merged = base.merge(override_cfg);

        // Verify override takes precedence
        assert_eq!(merged.context.instructions_file, "./.local-context.md");
    }

    #[test]
    fn test_context_instructions_file_merge_empty() {
        // Create base config with instructions_file
        let mut base = Config::default();
        base.context.instructions_file = "~/.global-context.md".to_string();

        // Create override config with empty instructions_file
        let override_cfg = Config::default();

        // Merge configs
        let merged = base.merge(override_cfg);

        // Verify base instructions_file is preserved when override is empty
        assert_eq!(merged.context.instructions_file, "~/.global-context.md");
    }
}
