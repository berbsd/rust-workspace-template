#!/usr/bin/env bash
# =============================================================================
# Bootstrap Script
# =============================================================================
# Sets up the complete development environment for this workspace.
# Checks for required tools, installs missing dependencies, and configures
# the Rust toolchain.
#
# Usage:
#   ./bin/bootstrap.sh
#   # Or if using direnv (recommended):
#   bootstrap.sh
#
# Supported Platforms:
#   - macOS (via Homebrew)
#   - Linux (coming soon)
#   - Windows (coming soon)
#
# What gets installed:
#   Prerequisites (via Homebrew on macOS):
#   - Homebrew (macOS package manager)
#   - Rust via rustup
#   - direnv (automatic environment setup)
#   - taplo (TOML formatter/linter)
#
#   Rust Toolchain:
#   - Nightly toolchain (for rustfmt unstable features)
#   - rustfmt, clippy components
#
#   Cargo Tools:
#   - cargo-nextest (fast test runner)
#   - cargo-llvm-cov (code coverage)
#   - cargo-deny (license/security checking)
#   - cargo-audit (security vulnerabilities)
#   - cargo-edit (add, rm, upgrade dependencies)
#   - just (task runner)
#   - cargo-watch (file watcher)
#   - cargo-outdated (dependency updates)
#   - cargo-machete (unused dependencies)
#
#   Git Hooks:
#   - lefthook install (pre-commit + commit-msg hooks from lefthook.yml)
#
# To add more tools:
#   1. Add installation in the appropriate section below
#   2. Update the justfile if the tool has commands
# =============================================================================

set -e  # Exit on any error

# =============================================================================
# Helper Functions
# =============================================================================

# Print section header
section() {
  echo ""
  echo "=============================================="
  echo "$1"
  echo "=============================================="
}

# Print step
step() {
  echo ">>> $1"
}

# Print success message
success() {
  echo "$1"
}

# Print error message and exit
error() {
  echo "ERROR: $1" >&2
  exit 1
}

# Print warning message
warn() {
  echo "WARNING: $1" >&2
}

# Check if a command exists
command_exists() {
  command -v "$1" &> /dev/null
}

# =============================================================================
# Platform Detection
# =============================================================================

detect_platform() {
  case "$(uname -s)" in
    Darwin*)
      PLATFORM="macos"
      ;;
    Linux*)
      PLATFORM="linux"
      ;;
    MINGW*|MSYS*|CYGWIN*)
      PLATFORM="windows"
      ;;
    *)
      error "Unsupported platform: $(uname -s)"
      ;;
  esac

  step "Detected platform: $PLATFORM"
}

# =============================================================================
# macOS Setup (via Homebrew)
# =============================================================================

setup_macos() {
  section "Setting up macOS environment"

  # --- Check/Install Homebrew ---
  if ! command_exists brew; then
    step "Homebrew not found. Installing..."
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

    # Add Homebrew to PATH for this session
    if [[ -f /opt/homebrew/bin/brew ]]; then
      eval "$(/opt/homebrew/bin/brew shellenv)"
    elif [[ -f /usr/local/bin/brew ]]; then
      eval "$(/usr/local/bin/brew shellenv)"
    fi

    success "Homebrew installed"
  else
    success "Homebrew already installed"
  fi

  # --- Check/Install Rust via rustup ---
  if ! command_exists rustup; then
    step "Rust not found. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

    # Source cargo environment for this session
    source "$HOME/.cargo/env"

    success "Rust installed"
  else
    success "Rust already installed"
  fi

  # --- Install direnv (optional but recommended) ---
  if ! command_exists direnv; then
    step "Installing direnv..."
    brew install direnv
    success "direnv installed"
    warn "Add 'eval \"\$(direnv hook bash)\"' or 'eval \"\$(direnv hook zsh)\"' to your shell config"
  else
    success "direnv already installed"
  fi

  # --- Install taplo (TOML formatter) ---
  if ! command_exists taplo; then
    step "Installing taplo..."
    brew install taplo
    success "taplo installed"
  else
    success "taplo already installed"
  fi

  # --- Install typos (Source code spell checker) ---
  if ! command_exists typos; then
    step "Installing typos..."
    brew install typos-cli
    success "typos installed"
  else
    success "typos already installed"
  fi

  # --- Install git-cliff (Changelog generator) ---
  if ! command_exists git-cliff; then
    step "Installing git-cliff..."
    brew install git-cliff
    success "git-cliff installed"
  else
    success "git-cliff already installed"
  fi

  # --- Install lefthook (Git hooks manager) ---
  if ! command_exists lefthook; then
    step "Installing lefthook..."
    brew install lefthook
    success "lefthook installed"
  else
    success "lefthook already installed"
  fi

  # --- Install betterleaks (Secret scanning) ---
  if ! command_exists betterleaks; then
    step "Installing betterleaks..."
    brew install betterleaks
    success "betterleaks installed"
  else
    success "betterleaks already installed"
  fi

  # --- Install cogitto (Conventional commits) ---
  if ! command_exists cog; then
    step "Installing cogitto..."
    brew install cocogitto
    success "cogitto installed"
  else
    success "cogitto already installed"
  fi
}

# =============================================================================
# Linux Setup (Coming Soon)
# =============================================================================

setup_linux() {
  error "Linux setup is not yet implemented.

To set up manually:
  1. Install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  2. Install direnv via your package manager
  3. Run this script again to install Rust tools

Linux support coming soon!"
}

# =============================================================================
# Windows Setup (Coming Soon)
# =============================================================================

setup_windows() {
  error "Windows setup is not yet implemented.

To set up manually:
  1. Install Rust from https://rustup.rs
  2. Install Visual Studio Build Tools
  3. Run this script again in Git Bash to install Rust tools

Windows support coming soon!"
}

# =============================================================================
# Rust Toolchain Setup (Cross-Platform)
# =============================================================================

setup_rust_toolchain() {
  section "Setting up Rust toolchain"

  # Ensure cargo is in PATH
  if [[ -f "$HOME/.cargo/env" ]]; then
    source "$HOME/.cargo/env"
  fi

  if ! command_exists rustup; then
    error "rustup not found. Please install Rust first: https://rustup.rs"
  fi

  if ! command_exists cargo; then
    error "cargo not found. Please ensure Rust is properly installed."
  fi

  step "Updating Rust toolchain..."
  rustup update

  step "Installing nightly toolchain (for rustfmt)..."
  rustup toolchain install nightly

  step "Adding components..."
  rustup component add rustfmt clippy

  success "Rust toolchain configured"
}

# =============================================================================
# Cargo Tools Installation (Cross-Platform)
# =============================================================================

install_cargo_tools() {
  section "Installing Cargo tools"

  # --- Testing Tools ---
  step "Installing testing tools..."
  cargo install cargo-nextest --locked    # Fast test runner
  cargo install cargo-llvm-cov --locked   # Code coverage

  # --- Security Tools ---
  step "Installing security tools..."
  cargo install cargo-deny --locked       # License/security checking
  cargo install cargo-audit --locked      # Security vulnerabilities

  # --- Development Tools ---
  step "Installing development tools..."
  cargo install cargo-edit --locked       # Add, rm, upgrade dependencies
  cargo install just --locked             # Task runner
  cargo install cargo-watch --locked      # File watcher
  cargo install cargo-outdated --locked   # Dependency updates
  cargo install cargo-machete --locked    # Unused dependencies (fast)
  cargo install cargo-udeps --locked      # Unused dependencies (thorough, nightly)
  cargo install cargo-release --locked    # Release automation
  cargo install cargo-semver-checks --locked  # Semver violation linting

  success "All Cargo tools installed"
}

# =============================================================================
# Git Hooks (Cross-Platform)
# =============================================================================

# Point git at the hooks lefthook.yml declares (pre-commit secret/typo/format
# scanning, commit-msg conventional-commit check). Rewrites .git/hooks shims
# from scratch every run, so it is safe to repeat.
install_git_hooks() {
  section "Installing Git hooks"

  if ! command_exists lefthook; then
    warn "lefthook is not installed - skipping git hooks. Run 'just hooks' after installing it."
    return 0
  fi

  if ! git rev-parse --git-dir > /dev/null 2>&1; then
    warn "Not inside a git repository - skipping git hooks."
    return 0
  fi

  step "Installing lefthook hooks..."
  lefthook install
  success "Git hooks installed (pre-commit, commit-msg)"
}

# =============================================================================
# Main
# =============================================================================

main() {
  section "Bootstrap: Development Environment Setup"

  # Detect platform
  detect_platform

  # Platform-specific setup
  case "$PLATFORM" in
    macos)
      setup_macos
      ;;
    linux)
      setup_linux
      ;;
    windows)
      setup_windows
      ;;
  esac

  # Cross-platform Rust setup
  setup_rust_toolchain
  install_cargo_tools

  # Git hooks (needs lefthook from the platform setup above)
  install_git_hooks

  # --- Done ---
  section "Bootstrap Complete!"
  echo ""
  echo "Quick start commands:"
  echo "  just         - Show all available tasks"
  echo "  just check   - Run all checks (format, lint, test, security)"
  echo "  just fmt     - Format code"
  echo "  just test    - Run tests"
  echo "  just watch   - Watch for changes and run checks"
  echo ""
  echo "Run 'just' to see all available commands."

  # Remind about direnv if installed
  if command_exists direnv; then
    echo ""
    echo "direnv is installed. Run 'direnv allow' to enable automatic environment setup."
  fi
}

# Run main
main "$@"
