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

# Download and install
install_claude_vm() {
  local platform=$(detect_platform)
  local version="${1:-$(get_latest_version)}"
  local install_dir="${2:-/usr/local/bin}"

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
    exit 1
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

  # Verify installation
  if command -v claude-vm &> /dev/null; then
    log_success "Version: $(claude-vm --version)"
  else
    log_error "Installation succeeded but claude-vm not found in PATH"
    log_info "You may need to add ${install_dir} to your PATH"
    exit 1
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
