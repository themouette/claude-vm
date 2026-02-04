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

    #[serde(default)]
    pub security: SecurityConfig,

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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityConfig {
    #[serde(default)]
    pub network: NetworkSecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSecurityConfig {
    /// Network policy mode: allowlist or denylist
    #[serde(default = "default_policy_mode")]
    pub mode: PolicyMode,

    /// Block private networks (10.0.0.0/8, 192.168.0.0/16, etc.)
    #[serde(default = "default_true")]
    pub block_private_networks: bool,

    /// Block cloud metadata services (169.254.169.254)
    #[serde(default = "default_true")]
    pub block_metadata_services: bool,

    /// Block non-HTTP protocols (raw TCP, UDP)
    #[serde(default = "default_true")]
    pub block_tcp_udp: bool,

    /// Allowed domains (for denylist mode)
    #[serde(default)]
    pub allowed_domains: Vec<String>,

    /// Blocked domains (for allowlist mode)
    #[serde(default)]
    pub blocked_domains: Vec<String>,

    /// Bypass HTTPS inspection for these domains (certificate pinning)
    #[serde(default)]
    pub bypass_domains: Vec<String>,

    /// Enable network filtering
    #[serde(default)]
    pub enabled: bool,
}

impl Default for NetworkSecurityConfig {
    fn default() -> Self {
        Self {
            mode: default_policy_mode(),
            block_private_networks: true,
            block_metadata_services: true,
            block_tcp_udp: true,
            allowed_domains: vec![],
            blocked_domains: vec![],
            bypass_domains: vec![],
            enabled: false, // Opt-in for backward compatibility
        }
    }
}

impl NetworkSecurityConfig {
    /// Validate configuration and return warnings (not errors - config is still usable)
    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        // Skip validation if network security is disabled
        if !self.enabled {
            return warnings;
        }

        // 1. Check for empty allowlist in allowlist mode
        if self.mode == PolicyMode::Allowlist && self.allowed_domains.is_empty() {
            warnings.push(
                "Network security is in 'allowlist' mode but no domains are allowed. \
                This will block ALL network access (only DNS and localhost allowed)."
                    .to_string(),
            );
        }

        // 2. Validate domain patterns
        let all_domains: Vec<(&str, &str)> = self
            .allowed_domains
            .iter()
            .map(|d| (d.as_str(), "allowed_domains"))
            .chain(
                self.blocked_domains
                    .iter()
                    .map(|d| (d.as_str(), "blocked_domains")),
            )
            .chain(
                self.bypass_domains
                    .iter()
                    .map(|d| (d.as_str(), "bypass_domains")),
            )
            .collect();

        for (domain, list_name) in all_domains {
            if let Some(warning) = Self::validate_domain_pattern(domain) {
                warnings.push(format!(
                    "Invalid domain in {}: '{}' - {}",
                    list_name, domain, warning
                ));
            }
        }

        // 3. Check for conflicting domains
        for allowed in &self.allowed_domains {
            if self.blocked_domains.contains(allowed) {
                warnings.push(format!(
                    "Domain '{}' appears in both allowed_domains and blocked_domains. \
                    It will be treated as ALLOWED (allowed_domains takes precedence).",
                    allowed
                ));
            }
        }

        warnings
    }

    /// Validate a single domain pattern
    fn validate_domain_pattern(domain: &str) -> Option<String> {
        if domain.is_empty() {
            return Some("domain cannot be empty".to_string());
        }

        // Check for invalid characters (only alphanumeric, dots, hyphens, asterisk allowed)
        if domain
            .chars()
            .any(|c| !c.is_alphanumeric() && c != '.' && c != '-' && c != '*')
        {
            return Some(
                "domain contains invalid characters (only alphanumeric, '.', '-', '*' allowed)"
                    .to_string(),
            );
        }

        // Check wildcard usage
        if domain.contains('*') {
            // Wildcard must be at the beginning as "*."
            if !domain.starts_with("*.") {
                return Some(
                    "wildcard (*) can only be used as a prefix in the form '*.domain.com'"
                        .to_string(),
                );
            }

            // Only one wildcard allowed
            if domain.matches('*').count() > 1 {
                return Some("only one wildcard (*) is allowed per domain".to_string());
            }

            // Check the part after "*."
            let suffix = &domain[2..];
            if suffix.is_empty() {
                return Some(
                    "wildcard pattern '*.` must be followed by a domain (e.g., '*.example.com')"
                        .to_string(),
                );
            }

            // The suffix should be a valid domain (no wildcards)
            if suffix.contains('*') {
                return Some("only one wildcard (*) is allowed per domain".to_string());
            }
        }

        // Check for consecutive dots
        if domain.contains("..") {
            return Some("domain cannot contain consecutive dots".to_string());
        }

        // Check if domain starts or ends with dot (except after wildcard)
        let check_domain = domain.strip_prefix("*.").unwrap_or(domain);

        if check_domain.starts_with('.') || check_domain.ends_with('.') {
            return Some("domain cannot start or end with a dot".to_string());
        }

        // Check for consecutive hyphens or invalid hyphen placement
        if check_domain.starts_with('-') || check_domain.ends_with('-') {
            return Some("domain cannot start or end with a hyphen".to_string());
        }

        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PolicyMode {
    /// Block all except explicitly allowed domains
    Allowlist,
    /// Allow all except explicitly blocked domains (default)
    Denylist,
}

impl PolicyMode {
    /// Get the string representation for environment variables
    pub fn as_str(&self) -> &'static str {
        match self {
            PolicyMode::Allowlist => "allowlist",
            PolicyMode::Denylist => "denylist",
        }
    }
}

fn default_policy_mode() -> PolicyMode {
    PolicyMode::Denylist
}

fn default_true() -> bool {
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

        // 5. Validate network security configuration
        let warnings = config.security.network.validate();
        if !warnings.is_empty() {
            use std::io::{self, Write};
            eprintln!();
            eprintln!("⚠️  Network Security Configuration Warnings:");
            for warning in warnings {
                eprintln!("   - {}", warning);
            }
            eprintln!();
            io::stderr().flush().ok();
        }

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

        // Security (merge network policies)
        if other.security.network.enabled {
            self.security.network.enabled = true;
        }
        if other.security.network.mode != default_policy_mode() {
            self.security.network.mode = other.security.network.mode;
        }
        self.security.network.block_private_networks =
            other.security.network.block_private_networks;
        self.security.network.block_metadata_services =
            other.security.network.block_metadata_services;
        self.security.network.block_tcp_udp = other.security.network.block_tcp_udp;

        // Merge domain lists (append)
        self.security
            .network
            .allowed_domains
            .extend(other.security.network.allowed_domains);
        self.security
            .network
            .blocked_domains
            .extend(other.security.network.blocked_domains);
        self.security
            .network
            .bypass_domains
            .extend(other.security.network.bypass_domains);

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
            network_security,
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
                self.security.network.enabled = true;
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
                if *network_security {
                    self.security.network.enabled = true;
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
    fn test_network_security_config_parsing() {
        let toml = r#"
            [security.network]
            enabled = true
            mode = "allowlist"
            allowed_domains = ["api.github.com", "*.npmjs.org"]
            blocked_domains = ["evil.com"]
            bypass_domains = ["localhost"]
            block_tcp_udp = true
            block_private_networks = true
            block_metadata_services = true
        "#;

        let config: Config = toml::from_str(toml).expect("Failed to parse config");

        assert!(config.security.network.enabled);
        assert_eq!(config.security.network.mode, PolicyMode::Allowlist);
        assert_eq!(
            config.security.network.allowed_domains,
            vec!["api.github.com", "*.npmjs.org"]
        );
        assert_eq!(config.security.network.blocked_domains, vec!["evil.com"]);
        assert_eq!(config.security.network.bypass_domains, vec!["localhost"]);
        assert!(config.security.network.block_tcp_udp);
        assert!(config.security.network.block_private_networks);
        assert!(config.security.network.block_metadata_services);
    }

    #[test]
    fn test_policy_mode_as_str() {
        assert_eq!(PolicyMode::Allowlist.as_str(), "allowlist");
        assert_eq!(PolicyMode::Denylist.as_str(), "denylist");
    }

    #[test]
    fn test_policy_mode_serde() {
        // Test serialization
        let allowlist = PolicyMode::Allowlist;
        let json = serde_json::to_string(&allowlist).unwrap();
        assert_eq!(json, "\"allowlist\"");

        let denylist = PolicyMode::Denylist;
        let json = serde_json::to_string(&denylist).unwrap();
        assert_eq!(json, "\"denylist\"");

        // Test deserialization
        let parsed: PolicyMode = serde_json::from_str("\"allowlist\"").unwrap();
        assert_eq!(parsed, PolicyMode::Allowlist);

        let parsed: PolicyMode = serde_json::from_str("\"denylist\"").unwrap();
        assert_eq!(parsed, PolicyMode::Denylist);
    }

    #[test]
    fn test_network_security_config_defaults() {
        let config = NetworkSecurityConfig::default();

        assert!(!config.enabled); // Disabled by default
        assert_eq!(config.mode, PolicyMode::Denylist); // Denylist is default
        assert!(config.block_tcp_udp);
        assert!(config.block_private_networks);
        assert!(config.block_metadata_services);
        assert!(config.allowed_domains.is_empty());
        assert!(config.blocked_domains.is_empty());
        assert!(config.bypass_domains.is_empty());
    }

    #[test]
    fn test_network_security_domain_list_merging() {
        // Test that domain lists are accumulated (not replaced) during merge
        let mut base = Config::default();
        base.security.network.allowed_domains = vec!["api.github.com".to_string()];
        base.security.network.blocked_domains = vec!["evil.com".to_string()];

        let mut override_cfg = Config::default();
        override_cfg.security.network.allowed_domains = vec!["api.npmjs.org".to_string()];
        override_cfg.security.network.blocked_domains = vec!["bad.com".to_string()];

        let merged = base.merge(override_cfg);

        // Domain lists should be accumulated
        assert_eq!(
            merged.security.network.allowed_domains,
            vec!["api.github.com", "api.npmjs.org"]
        );
        assert_eq!(
            merged.security.network.blocked_domains,
            vec!["evil.com", "bad.com"]
        );
    }

    #[test]
    fn test_network_security_enabled_flag_merge() {
        let mut base = Config::default();
        base.security.network.enabled = false;

        let mut override_cfg = Config::default();
        override_cfg.security.network.enabled = true;

        let merged = base.merge(override_cfg);

        // enabled flag should be set to true if any config enables it
        assert!(merged.security.network.enabled);
    }

    #[test]
    fn test_network_security_mode_override() {
        let mut base = Config::default();
        base.security.network.mode = PolicyMode::Denylist;

        let mut override_cfg = Config::default();
        override_cfg.security.network.mode = PolicyMode::Allowlist;

        let merged = base.merge(override_cfg);

        // Mode should be overridden if different from default
        assert_eq!(merged.security.network.mode, PolicyMode::Allowlist);
    }

    // === Validation Tests ===

    #[test]
    fn test_validation_disabled_network_security() {
        let config = NetworkSecurityConfig {
            enabled: false,
            mode: PolicyMode::Allowlist,
            allowed_domains: vec![], // Would normally warn
            ..Default::default()
        };

        let warnings = config.validate();
        assert!(warnings.is_empty(), "Should not validate when disabled");
    }

    #[test]
    fn test_validation_empty_allowlist() {
        let config = NetworkSecurityConfig {
            enabled: true,
            mode: PolicyMode::Allowlist,
            allowed_domains: vec![],
            ..Default::default()
        };

        let warnings = config.validate();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("no domains are allowed"));
        assert!(warnings[0].contains("block ALL network access"));
    }

    #[test]
    fn test_validation_empty_denylist_ok() {
        let config = NetworkSecurityConfig {
            enabled: true,
            mode: PolicyMode::Denylist,
            blocked_domains: vec![], // OK in denylist mode
            ..Default::default()
        };

        let warnings = config.validate();
        assert!(warnings.is_empty(), "Empty denylist should be valid");
    }

    #[test]
    fn test_validation_valid_domains() {
        let config = NetworkSecurityConfig {
            enabled: true,
            mode: PolicyMode::Allowlist,
            allowed_domains: vec![
                "api.github.com".to_string(),
                "example.com".to_string(),
                "sub.example.co.uk".to_string(),
                "*.npmjs.org".to_string(),
                "*.cdn.example.com".to_string(),
            ],
            ..Default::default()
        };

        let warnings = config.validate();
        assert!(
            warnings.is_empty(),
            "Valid domains should pass: {:?}",
            warnings
        );
    }

    #[test]
    fn test_validation_invalid_domain_empty() {
        let config = NetworkSecurityConfig {
            enabled: true,
            mode: PolicyMode::Allowlist,
            allowed_domains: vec!["".to_string()],
            ..Default::default()
        };

        let warnings = config.validate();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("cannot be empty"));
    }

    #[test]
    fn test_validation_invalid_domain_characters() {
        let config = NetworkSecurityConfig {
            enabled: true,
            mode: PolicyMode::Allowlist,
            allowed_domains: vec![
                "api.github.com/path".to_string(), // slash not allowed
                "example@.com".to_string(),        // @ not allowed
                "test:8080".to_string(),           // colon not allowed
            ],
            ..Default::default()
        };

        let warnings = config.validate();
        assert_eq!(warnings.len(), 3);
        for warning in &warnings {
            assert!(warning.contains("invalid characters"));
        }
    }

    #[test]
    fn test_validation_invalid_wildcard_usage() {
        let config = NetworkSecurityConfig {
            enabled: true,
            mode: PolicyMode::Allowlist,
            allowed_domains: vec![
                "example.*.com".to_string(),   // wildcard in middle
                "example.*".to_string(),       // wildcard at end
                "*example.com".to_string(),    // wildcard without dot
                "*.*.example.com".to_string(), // multiple wildcards
                "*.".to_string(),              // wildcard without domain
            ],
            ..Default::default()
        };

        let warnings = config.validate();
        assert!(
            warnings.len() >= 5,
            "Should have at least 5 warnings for invalid wildcards"
        );

        // Check specific error messages
        assert!(warnings
            .iter()
            .any(|w| w.contains("example.*.com") && w.contains("only be used as a prefix")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("example.*") && w.contains("only be used as a prefix")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("*example.com") && w.contains("only be used as a prefix")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("*.*.example.com") && w.contains("only one wildcard")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("*.") && w.contains("must be followed by a domain")));
    }

    #[test]
    fn test_validation_consecutive_dots() {
        let config = NetworkSecurityConfig {
            enabled: true,
            mode: PolicyMode::Allowlist,
            allowed_domains: vec!["example..com".to_string(), "test...example.com".to_string()],
            ..Default::default()
        };

        let warnings = config.validate();
        assert_eq!(warnings.len(), 2);
        for warning in &warnings {
            assert!(warning.contains("consecutive dots"));
        }
    }

    #[test]
    fn test_validation_domain_starts_ends_with_dot() {
        let config = NetworkSecurityConfig {
            enabled: true,
            mode: PolicyMode::Allowlist,
            allowed_domains: vec![
                ".example.com".to_string(),
                "example.com.".to_string(),
                "*.example.com.".to_string(), // wildcard doesn't excuse trailing dot
            ],
            ..Default::default()
        };

        let warnings = config.validate();
        assert_eq!(warnings.len(), 3);
        for warning in &warnings {
            assert!(warning.contains("cannot start or end with a dot"));
        }
    }

    #[test]
    fn test_validation_domain_starts_ends_with_hyphen() {
        let config = NetworkSecurityConfig {
            enabled: true,
            mode: PolicyMode::Allowlist,
            allowed_domains: vec![
                "-example.com".to_string(),
                "example-.com".to_string(),
                "*.--test.com".to_string(),
            ],
            ..Default::default()
        };

        let warnings = config.validate();
        assert!(warnings.len() >= 2); // At least the first two should fail
        assert!(warnings
            .iter()
            .any(|w| w.contains("cannot start or end with a hyphen")));
    }

    #[test]
    fn test_validation_conflicting_domains() {
        let config = NetworkSecurityConfig {
            enabled: true,
            mode: PolicyMode::Denylist,
            allowed_domains: vec!["api.github.com".to_string(), "example.com".to_string()],
            blocked_domains: vec![
                "evil.com".to_string(),
                "api.github.com".to_string(), // Conflict!
            ],
            ..Default::default()
        };

        let warnings = config.validate();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("api.github.com"));
        assert!(warnings[0].contains("both allowed_domains and blocked_domains"));
        assert!(warnings[0].contains("ALLOWED"));
    }

    #[test]
    fn test_validation_multiple_conflicts() {
        let config = NetworkSecurityConfig {
            enabled: true,
            mode: PolicyMode::Allowlist,
            allowed_domains: vec!["example.com".to_string(), "test.com".to_string()],
            blocked_domains: vec!["example.com".to_string(), "test.com".to_string()],
            ..Default::default()
        };

        let warnings = config.validate();
        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].contains("example.com"));
        assert!(warnings[1].contains("test.com"));
    }

    #[test]
    fn test_validation_multiple_issues() {
        let config = NetworkSecurityConfig {
            enabled: true,
            mode: PolicyMode::Allowlist,
            allowed_domains: vec![
                "".to_string(),                // Empty
                "example..com".to_string(),    // Consecutive dots
                "api/test.com".to_string(),    // Invalid character
                "*.example.*.com".to_string(), // Multiple wildcards
            ],
            blocked_domains: vec![],
            ..Default::default()
        };

        let warnings = config.validate();
        // Should have 4 domain validation warnings (no empty allowlist warning since list is not empty)
        assert_eq!(warnings.len(), 4);
    }

    #[test]
    fn test_validation_bypass_domains() {
        let config = NetworkSecurityConfig {
            enabled: true,
            mode: PolicyMode::Allowlist,
            allowed_domains: vec!["api.github.com".to_string()],
            bypass_domains: vec![
                "localhost".to_string(),         // Valid
                "example..com".to_string(),      // Invalid
                "*.pinned-cert.com".to_string(), // Valid wildcard
            ],
            ..Default::default()
        };

        let warnings = config.validate();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("bypass_domains"));
        assert!(warnings[0].contains("example..com"));
        assert!(warnings[0].contains("consecutive dots"));
    }

    #[test]
    fn test_validation_all_domain_lists() {
        // Test that validation checks all three domain lists
        let config = NetworkSecurityConfig {
            enabled: true,
            mode: PolicyMode::Allowlist,
            allowed_domains: vec!["valid.com".to_string(), "invalid..com".to_string()],
            blocked_domains: vec![
                "also-valid.com".to_string(),
                "also..invalid.com".to_string(),
            ],
            bypass_domains: vec![
                "another-valid.com".to_string(),
                "another..invalid.com".to_string(),
            ],
            ..Default::default()
        };

        let warnings = config.validate();
        // Should have 3 warnings (one for each invalid domain)
        assert_eq!(warnings.len(), 3);

        // Verify each list is checked
        assert!(warnings.iter().any(|w| w.contains("allowed_domains")));
        assert!(warnings.iter().any(|w| w.contains("blocked_domains")));
        assert!(warnings.iter().any(|w| w.contains("bypass_domains")));
    }
}
