//! Agent system for managing different AI coding assistants.
//!
//! This module provides a declarative, TOML-based system for defining and managing
//! AI agents (like Claude, OpenCode, etc.) that can be used with claude-vm.
//!
//! # Architecture
//!
//! Agents define:
//! - **Metadata**: ID, name, description, command
//! - **Paths**: Config directory, context file, MCP config file
//! - **Scripts**: Install, authenticate (optional), deploy
//! - **Requirements**: Required capabilities
//!
//! # Example
//!
//! ```toml
//! [agent]
//! id = "claude"
//! name = "Claude Code"
//! command = "claude"
//! requires_authentication = true
//!
//! [paths]
//! config_dir = ".claude"
//! context_file = "CLAUDE.md"
//! mcp_config_file = ".claude.json"
//!
//! [install]
//! script_file = "install.sh"
//! ```

pub mod definition;
pub mod executor;
pub mod registry;

pub use definition::{Agent, AgentMeta, AgentPaths};
pub use executor::{authenticate_agent, install_agent};
pub use registry::AgentRegistry;
