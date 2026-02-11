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
    pub security: SecurityConfig,

    #[serde(default)]
    pub mounts: Vec<MountEntry>,

    #[serde(default)]
    pub update_check: UpdateCheckSettings,

    /// Automatically create template if missing (default: false)
    #[serde(default)]
    pub auto_setup: bool,

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
    pub rust: bool,

    #[serde(default)]
    pub chromium: bool,

    #[serde(default)]
    pub gpg: bool,

    #[serde(default)]
    pub gh: bool,

    #[serde(default)]
    pub git: bool,

    #[serde(default)]
    pub network_isolation: bool,
}

impl ToolsConfig {
    /// Check if a capability is enabled by ID
    pub fn is_enabled(&self, id: &str) -> bool {
        match id {
            "docker" => self.docker,
            "node" => self.node,
            "python" => self.python,
            "rust" => self.rust,
            "chromium" => self.chromium,
            "gpg" => self.gpg,
            "gh" => self.gh,
            "git" => self.git,
            "network-isolation" => self.network_isolation,
            _ => false,
        }
    }

    /// Enable a capability by ID
    pub fn enable(&mut self, id: &str) {
        match id {
            "docker" => self.docker = true,
            "node" => self.node = true,
            "python" => self.python = true,
            "rust" => self.rust = true,
            "chromium" => self.chromium = true,
            "gpg" => self.gpg = true,
            "gh" => self.gh = true,
            "git" => self.git = true,
            "network-isolation" => self.network_isolation = true,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityConfig {
    #[serde(default)]
    pub network: NetworkIsolationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkIsolationConfig {
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

impl Default for NetworkIsolationConfig {
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

impl NetworkIsolationConfig {
    /// Validate configuration and return warnings (not errors - config is still usable)
    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        // Skip validation if network isolation is disabled
        if !self.enabled {
            return warnings;
        }

        // 1. Check for empty allowlist in allowlist mode
        if self.mode == PolicyMode::Allowlist && self.allowed_domains.is_empty() {
            warnings.push(
                "Network isolation is in 'allowlist' mode but no domains are allowed. \
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

        // Check for invalid characters (only alphanumeric, dots, hyphens, underscores, asterisk allowed)
        if domain
            .chars()
            .any(|c| !c.is_alphanumeric() && c != '.' && c != '-' && c != '_' && c != '*')
        {
            return Some(
                "domain contains invalid characters (only alphanumeric, '.', '-', '_', '*' allowed)"
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
                    "wildcard pattern '*.' must be followed by a domain (e.g., '*.example.com')"
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
    ///
    /// For worktrees, this method checks both the worktree and main repo.
    /// Pass the worktree root as project_root and main repo root as main_repo_root.
    pub fn load(project_root: &Path) -> Result<Self> {
        Self::load_with_main_repo(project_root, project_root)
    }

    /// Load configuration with support for worktrees
    /// - main_repo_root: Main repository root (for fallback config)
    /// - project_root: Current project root (worktree if in worktree)
    pub fn load_with_main_repo(project_root: &Path, main_repo_root: &Path) -> Result<Self> {
        let mut config = Self::default();

        // 1. Load global config
        if let Some(home) = home_dir() {
            let global_config = home.join(".claude-vm.toml");
            if global_config.exists() {
                config = config.merge(Self::from_file(&global_config)?);
            }
        }

        // 2. Load main repo config (if different from project root)
        if main_repo_root != project_root {
            let main_config = main_repo_root.join(".claude-vm.toml");
            if main_config.exists() {
                config = config.merge(Self::from_file(&main_config)?);
            }
        }

        // 3. Load project config (worktree config if in worktree)
        let project_config = project_root.join(".claude-vm.toml");
        if project_config.exists() {
            config = config.merge(Self::from_file(&project_config)?);
        }

        // 4. Apply environment variables
        config = config.merge_env();

        // 5. Resolve context file if needed
        config = config.resolve_context_file()?;

        Ok(config)
    }

    /// Load configuration from a TOML file
    pub fn from_file(path: &Path) -> Result<Self> {
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
        self.tools.rust = self.tools.rust || other.tools.rust;
        self.tools.chromium = self.tools.chromium || other.tools.chromium;
        self.tools.gpg = self.tools.gpg || other.tools.gpg;
        self.tools.gh = self.tools.gh || other.tools.gh;
        self.tools.git = self.tools.git || other.tools.git;
        self.tools.network_isolation =
            self.tools.network_isolation || other.tools.network_isolation;

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

        // Security config
        // Enable if other enables it
        self.security.network.enabled =
            self.security.network.enabled || other.security.network.enabled;

        // Mode: other takes precedence if network isolation is enabled in other
        if other.security.network.enabled {
            self.security.network.mode = other.security.network.mode;
            self.security.network.block_private_networks =
                other.security.network.block_private_networks;
            self.security.network.block_metadata_services =
                other.security.network.block_metadata_services;
            self.security.network.block_tcp_udp = other.security.network.block_tcp_udp;
        }

        // Domain lists: accumulate (extend)
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
                    // In test mode, fail immediately without prompting
                    #[cfg(test)]
                    {
                        return Err(crate::error::ClaudeVmError::InvalidConfig(format!(
                            "Failed to read context file '{}': {}",
                            file_path.display(),
                            e
                        )));
                    }

                    #[cfg(not(test))]
                    {
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
                                        "Context file load failed and user chose to abort"
                                            .to_string(),
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

        // Network isolation environment variables
        if let Ok(enabled) = std::env::var("NETWORK_ISOLATION_ENABLED") {
            if let Ok(enabled) = enabled.parse::<bool>() {
                self.security.network.enabled = enabled;
            }
        }

        if let Ok(mode) = std::env::var("POLICY_MODE") {
            match mode.to_lowercase().as_str() {
                "allowlist" => self.security.network.mode = PolicyMode::Allowlist,
                "denylist" => self.security.network.mode = PolicyMode::Denylist,
                _ => {}
            }
        }

        if let Ok(domains) = std::env::var("ALLOWED_DOMAINS") {
            let domains: Vec<String> = domains
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            self.security.network.allowed_domains.extend(domains);
        }

        if let Ok(domains) = std::env::var("BLOCKED_DOMAINS") {
            let domains: Vec<String> = domains
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            self.security.network.blocked_domains.extend(domains);
        }

        if let Ok(domains) = std::env::var("BYPASS_DOMAINS") {
            let domains: Vec<String> = domains
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            self.security.network.bypass_domains.extend(domains);
        }

        if let Ok(block) = std::env::var("BLOCK_TCP_UDP") {
            if let Ok(block) = block.parse::<bool>() {
                self.security.network.block_tcp_udp = block;
            }
        }

        if let Ok(block) = std::env::var("BLOCK_PRIVATE_NETWORKS") {
            if let Ok(block) = block.parse::<bool>() {
                self.security.network.block_private_networks = block;
            }
        }

        if let Ok(block) = std::env::var("BLOCK_METADATA_SERVICES") {
            if let Ok(block) = block.parse::<bool>() {
                self.security.network.block_metadata_services = block;
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

        // Auto setup
        if cli.auto_setup {
            self.auto_setup = true;
        }

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
            rust,
            chromium,
            gpg,
            gh,
            git,
            network_isolation,
            all,
            disk,
            memory,
            setup_scripts,
            mounts,
            #[cfg(debug_assertions)]
                no_agent_install: _,
        }) = &cli.command
        {
            if *all {
                self.tools.enable("docker");
                self.tools.enable("node");
                self.tools.enable("python");
                self.tools.enable("rust");
                self.tools.enable("chromium");
                self.tools.enable("gpg");
                self.tools.enable("gh");
                self.tools.enable("git");
                self.tools.enable("network-isolation");
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
                if *rust {
                    self.tools.enable("rust");
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
                if *network_isolation {
                    self.tools.enable("network-isolation");
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

        // Should error immediately in test mode (no interactive prompt)
        let result = config.resolve_context_file();
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Failed to read context file"));
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

    // Network isolation configuration tests
    #[test]
    fn test_network_isolation_default_config() {
        let config = NetworkIsolationConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.mode, PolicyMode::Denylist);
        assert!(config.block_private_networks);
        assert!(config.block_metadata_services);
        assert!(config.block_tcp_udp);
        assert!(config.allowed_domains.is_empty());
        assert!(config.blocked_domains.is_empty());
        assert!(config.bypass_domains.is_empty());
    }

    #[test]
    fn test_network_isolation_validate_disabled() {
        let config = NetworkIsolationConfig::default();
        let warnings = config.validate();
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_network_isolation_validate_empty_allowlist() {
        let config = NetworkIsolationConfig {
            enabled: true,
            mode: PolicyMode::Allowlist,
            ..Default::default()
        };
        // No domains in allowlist

        let warnings = config.validate();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("allowlist"));
        assert!(warnings[0].contains("no domains are allowed"));
    }

    #[test]
    fn test_network_isolation_domain_validation_valid() {
        assert!(NetworkIsolationConfig::validate_domain_pattern("example.com").is_none());
        assert!(NetworkIsolationConfig::validate_domain_pattern("api.example.com").is_none());
        assert!(NetworkIsolationConfig::validate_domain_pattern("*.example.com").is_none());
        assert!(NetworkIsolationConfig::validate_domain_pattern("my-api.example.com").is_none());
        assert!(NetworkIsolationConfig::validate_domain_pattern("api2.example.com").is_none());
        // Underscores are valid in DNS (RFC allows them)
        assert!(NetworkIsolationConfig::validate_domain_pattern("my_api.example.com").is_none());
        assert!(NetworkIsolationConfig::validate_domain_pattern("_service.example.com").is_none());
    }

    #[test]
    fn test_network_isolation_domain_validation_invalid() {
        // Empty domain
        assert!(NetworkIsolationConfig::validate_domain_pattern("").is_some());

        // Invalid characters
        assert!(NetworkIsolationConfig::validate_domain_pattern("example$.com").is_some());
        assert!(NetworkIsolationConfig::validate_domain_pattern("example com").is_some());

        // Invalid wildcard usage
        assert!(NetworkIsolationConfig::validate_domain_pattern("*example.com").is_some());
        assert!(NetworkIsolationConfig::validate_domain_pattern("example.*.com").is_some());
        assert!(NetworkIsolationConfig::validate_domain_pattern("*.*.example.com").is_some());
        assert!(NetworkIsolationConfig::validate_domain_pattern("*.").is_some());

        // Consecutive dots
        assert!(NetworkIsolationConfig::validate_domain_pattern("example..com").is_some());

        // Leading/trailing dots
        assert!(NetworkIsolationConfig::validate_domain_pattern(".example.com").is_some());
        assert!(NetworkIsolationConfig::validate_domain_pattern("example.com.").is_some());

        // Leading/trailing hyphens
        assert!(NetworkIsolationConfig::validate_domain_pattern("-example.com").is_some());
        assert!(NetworkIsolationConfig::validate_domain_pattern("example.com-").is_some());
    }

    #[test]
    fn test_network_isolation_domain_conflict_warning() {
        let config = NetworkIsolationConfig {
            enabled: true,
            allowed_domains: vec!["example.com".to_string()],
            blocked_domains: vec!["example.com".to_string()],
            ..Default::default()
        };

        let warnings = config.validate();
        assert!(warnings
            .iter()
            .any(|w| w.contains("both allowed_domains and blocked_domains")));
    }

    #[test]
    fn test_network_isolation_merge_enabled() {
        let base = Config::default();
        let mut override_cfg = Config::default();
        override_cfg.security.network.enabled = true;

        let merged = base.merge(override_cfg);
        assert!(merged.security.network.enabled);
    }

    #[test]
    fn test_network_isolation_merge_domains() {
        let mut base = Config::default();
        base.security.network.allowed_domains = vec!["example.com".to_string()];
        base.security.network.blocked_domains = vec!["bad.com".to_string()];

        let mut override_cfg = Config::default();
        override_cfg.security.network.allowed_domains = vec!["api.example.com".to_string()];
        override_cfg.security.network.blocked_domains = vec!["evil.com".to_string()];

        let merged = base.merge(override_cfg);
        assert_eq!(merged.security.network.allowed_domains.len(), 2);
        assert!(merged
            .security
            .network
            .allowed_domains
            .contains(&"example.com".to_string()));
        assert!(merged
            .security
            .network
            .allowed_domains
            .contains(&"api.example.com".to_string()));
        assert_eq!(merged.security.network.blocked_domains.len(), 2);
        assert!(merged
            .security
            .network
            .blocked_domains
            .contains(&"bad.com".to_string()));
        assert!(merged
            .security
            .network
            .blocked_domains
            .contains(&"evil.com".to_string()));
    }

    #[test]
    fn test_network_isolation_merge_mode() {
        let mut base = Config::default();
        base.security.network.mode = PolicyMode::Denylist;

        let mut override_cfg = Config::default();
        override_cfg.security.network.enabled = true;
        override_cfg.security.network.mode = PolicyMode::Allowlist;

        let merged = base.merge(override_cfg);
        assert_eq!(merged.security.network.mode, PolicyMode::Allowlist);
    }

    #[test]
    fn test_network_isolation_merge_blocks() {
        let mut base = Config::default();
        base.security.network.block_tcp_udp = false;

        let mut override_cfg = Config::default();
        override_cfg.security.network.enabled = true;
        override_cfg.security.network.block_tcp_udp = true;
        override_cfg.security.network.block_private_networks = false;

        let merged = base.merge(override_cfg);
        assert!(merged.security.network.block_tcp_udp);
        assert!(!merged.security.network.block_private_networks);
    }

    #[test]
    fn test_policy_mode_as_str() {
        assert_eq!(PolicyMode::Allowlist.as_str(), "allowlist");
        assert_eq!(PolicyMode::Denylist.as_str(), "denylist");
    }

    #[test]
    fn test_tools_config_network_isolation() {
        let mut tools = ToolsConfig::default();
        assert!(!tools.is_enabled("network-isolation"));

        tools.enable("network-isolation");
        assert!(tools.is_enabled("network-isolation"));
        assert!(tools.network_isolation);
    }
}
