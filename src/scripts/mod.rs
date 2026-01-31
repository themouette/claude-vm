pub mod runner;

// Embed installation scripts
pub const INSTALL_DOCKER: &str = include_str!("../../scripts/install_docker.sh");
pub const INSTALL_NODE: &str = include_str!("../../scripts/install_node.sh");
pub const INSTALL_PYTHON: &str = include_str!("../../scripts/install_python.sh");
pub const INSTALL_CHROMIUM: &str = include_str!("../../scripts/install_chromium.sh");
