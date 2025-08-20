# setup.just - Setup and installation recipes
#
# This module handles initial project setup and development environment configuration.
# It provides automated installation and configuration for all required and optional
# development tools.
#
# 🚀 MAIN SETUP COMMANDS:
#   • setup-project                  - Initial project setup (runs rust-setup)
#   • setup-all                      - Complete setup for new developers
#   • setup-brew                     - Install development tools via Homebrew
#   • setup-doppler                  - Install Doppler CLI for secrets management
#
# 🔍 VERIFICATION COMMANDS:
#   • setup-check                    - Quick setup validation
#   • setup-verify                   - Comprehensive setup verification
#
# 🛠️  UTILITY COMMANDS:
#   • setup-tools                    - Install additional development tools
#   • setup-reset                    - Reset setup state (for troubleshooting)
#
# SETUP WORKFLOW:
#   1. Run 'just setup-project' for basic setup
#   2. Run 'just setup-brew' to install development tools (macOS)
#   3. Run 'just setup-verify' to confirm everything works
#   4. Optional: 'just setup-doppler' for secrets management
#
# USAGE EXAMPLES:
#   just setup-project               # Basic project setup
#   just setup-all                   # Complete setup for new developers
#   just setup-check                 # Verify current setup status
#
# Import common utilities
import 'common.just'

# Initial project setup - development environment
_setup-project:
    @just _info "Setting up development environment..."
    @just _run "Installing Rust development tools" "just rust-setup"
    @just _success "Setup complete! You can now run 'just test-rust-coverage' for coverage reports."

# Brew installation and bundle install
_setup-brew:
    @just _require-command "brew"
    @just _run "Updating Homebrew" "brew update"
    @just _run "Installing Brewfile dependencies" "brew bundle install --file=./Brewfile"
    @just _success "Homebrew setup complete"

# Doppler CLI installation
_setup-doppler:
    @just _require-command "brew"
    @just _run "Installing GnuPG" "brew install gnupg"
    @just _run "Installing Doppler CLI" "brew install dopplerhq/cli/doppler"
    @just _success "Doppler CLI installation complete"

# Install development tools
_setup-tools:
    @just _info "Installing development tools..."
    @just _setup-brew
    @just _setup-doppler
    @just _success "All development tools installed"

# Complete project setup for new developers
_setup-all:
    @just _info "Running complete project setup..."
    @just _setup-tools
    @just _setup-project
    @just _success "Complete project setup finished!"

# Verify setup is complete
_setup-verify:
    @just _info "Verifying setup..."
    @just _require-command "brew"
    @just _require-command "doppler"
    @just _require-command "cargo"
    @just _require-command "just"
    @just _success "All required tools are installed"

# Quick setup check
_setup-check:
    #!/usr/bin/env bash
    echo "🔍 Checking development environment..."
    
    # Check essential tools
    tools=("git" "cargo" "just" "brew")
    missing_tools=()
    
    for tool in "${tools[@]}"; do
        if command -v "$tool" &> /dev/null; then
            echo "✅ $tool: $(command -v $tool)"
        else
            echo "❌ $tool: not found"
            missing_tools+=("$tool")
        fi
    done
    
    # Check optional tools
    optional_tools=("doppler" "tq" "jq")
    echo ""
    echo "📋 Optional tools:"
    for tool in "${optional_tools[@]}"; do
        if command -v "$tool" &> /dev/null; then
            echo "✅ $tool: $(command -v $tool)"
        else
            echo "⚠️  $tool: not installed (optional)"
        fi
    done
    
    if [ ${#missing_tools[@]} -eq 0 ]; then
        just _success "Development environment is ready!"
    else
        echo ""
        echo "❌ Missing required tools: ${missing_tools[*]}"
        echo "💡 Run 'just setup-all' to install missing dependencies"
        exit 1
    fi

# Reset setup (for troubleshooting)
_setup-reset:
    @echo "⚠️  This will reset your development setup"
    @echo "This is primarily for troubleshooting setup issues"
    @just _warn "This operation cannot be undone"
    @echo "Run 'just setup-all' after reset to reinstall everything"