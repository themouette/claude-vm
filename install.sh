#!/usr/bin/env bash
set -e

# Installation script for claude-vm (Rust version)
# Usage: curl -fsSL https://raw.githubusercontent.com/themouette/claude-vm/main/install.sh | bash

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
  echo -e "${BLUE}==>${NC} $1"
}

log_success() {
  echo -e "${GREEN}✓${NC} $1"
}

log_error() {
  echo -e "${RED}✗${NC} $1"
}

log_warning() {
  echo -e "${YELLOW}!${NC} $1"
}

# Show help message
show_help() {
  cat << EOF
claude-vm installer

USAGE:
    curl -fsSL https://raw.githubusercontent.com/themouette/claude-vm/main/install.sh | bash
    curl -fsSL https://raw.githubusercontent.com/themouette/claude-vm/main/install.sh | bash -s -- [OPTIONS]

OPTIONS:
    --version <VERSION>        Version to install (default: latest)
                               Examples: v0.3.0, latest

    --destination <PATH>       Custom installation directory
                               Overrides --local and --global

    --global                   Install to /usr/local/bin (requires sudo)
                               Default is ~/.local/bin (no sudo required)

    --help                     Show this help message

EXAMPLES:
    # Install latest version to ~/.local/bin (default)
    curl ... | bash

    # Install specific version
    curl ... | bash -s -- --version v0.3.0

    # Install to system directory
    curl ... | bash -s -- --global

    # Install to custom directory
    curl ... | bash -s -- --destination /opt/bin

DOCUMENTATION:
    https://github.com/themouette/claude-vm

EOF
  exit 0
}

# Detect OS and architecture
detect_platform() {
  local os=""
  local arch=""

  case "$OSTYPE" in
    darwin*)
      os="macos"
      ;;
    linux*)
      os="linux"
      ;;
    *)
      log_error "Unsupported OS: $OSTYPE"
      exit 1
      ;;
  esac

  case "$(uname -m)" in
    x86_64|amd64)
      arch="x86_64"
      ;;
    aarch64|arm64)
      arch="aarch64"
      ;;
    *)
      log_error "Unsupported architecture: $(uname -m)"
      exit 1
      ;;
  esac

  echo "${os}-${arch}"
}

# Get latest release version from GitHub
get_latest_version() {
  curl -fsSL https://api.github.com/repos/themouette/claude-vm/releases/latest \
    | grep '"tag_name":' \
    | sed -E 's/.*"([^"]+)".*/\1/'
}

# Check if directory is in PATH
check_path() {
  local dir="$1"
  if echo "$PATH" | tr ':' '\n' | grep -q "^${dir}$"; then
    return 0
  else
    return 1
  fi
}

# Download and install
install_claude_vm() {
  local platform=$(detect_platform)
  local version="latest"
  local install_dir="$HOME/.local/bin"
  local use_global=false

  # Parse arguments
  while [[ $# -gt 0 ]]; do
    case $1 in
      --version)
        version="$2"
        shift 2
        ;;
      --destination)
        install_dir="$2"
        shift 2
        ;;
      --global)
        use_global=true
        install_dir="/usr/local/bin"
        shift
        ;;
      --help)
        show_help
        ;;
      *)
        log_error "Unknown option: $1"
        echo "Use --help for usage information"
        exit 1
        ;;
    esac
  done

  # Resolve version
  if [ "$version" = "latest" ]; then
    log_info "Fetching latest version..."
    version=$(get_latest_version)
    if [ -z "$version" ]; then
      log_error "Failed to determine latest version"
      exit 1
    fi
  fi

  log_info "Installing claude-vm ${version} for ${platform}"

  # Download URL
  local download_url="https://github.com/themouette/claude-vm/releases/download/${version}/claude-vm-${platform}.tar.gz"

  log_info "Downloading from ${download_url}"

  # Create temp directory
  local tmp_dir=$(mktemp -d)
  trap "rm -rf $tmp_dir" EXIT

  # Download and extract
  if ! curl -fsSL "$download_url" | tar -xz -C "$tmp_dir"; then
    log_error "Failed to download claude-vm"
    log_error "Please check that version ${version} exists"
    exit 1
  fi

  # Create install directory if it doesn't exist
  if [ ! -d "$install_dir" ]; then
    log_info "Creating directory ${install_dir}"
    mkdir -p "$install_dir" 2>/dev/null || sudo mkdir -p "$install_dir"
  fi

  # Install binary
  log_info "Installing to ${install_dir}/claude-vm"

  if [ -w "$install_dir" ]; then
    mv "$tmp_dir/claude-vm" "$install_dir/claude-vm"
    chmod +x "$install_dir/claude-vm"
  else
    sudo mv "$tmp_dir/claude-vm" "$install_dir/claude-vm"
    sudo chmod +x "$install_dir/claude-vm"
  fi

  log_success "claude-vm installed successfully!"

  # Check if installation directory is in PATH
  if ! check_path "$install_dir"; then
    log_warning "Installation directory is not in your PATH"
    echo ""
    echo "Add this to your shell configuration file (~/.bashrc, ~/.zshrc, etc.):"
    echo ""
    echo "  export PATH=\"${install_dir}:\$PATH\""
    echo ""
  fi

  # Verify installation
  if command -v claude-vm &> /dev/null; then
    log_success "Version: $(claude-vm --version)"
  else
    log_warning "claude-vm installed but not found in PATH"
    log_info "You may need to add ${install_dir} to your PATH or restart your shell"
  fi

  echo ""
  echo "Get started:"
  echo "  claude-vm setup --docker --node"
  echo "  claude-vm \"help me code\""
  echo ""
  echo "Documentation: https://github.com/themouette/claude-vm"
}

# Main
main() {
  # Check for help flag first
  if [[ "$*" == *"--help"* ]]; then
    show_help
  fi

  echo ""
  log_info "claude-vm installer"
  echo ""

  # Check for required commands
  for cmd in curl tar; do
    if ! command -v $cmd &> /dev/null; then
      log_error "$cmd is required but not installed"
      exit 1
    fi
  done

  install_claude_vm "$@"
}

main "$@"
