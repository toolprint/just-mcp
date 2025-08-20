# rust.just - Rust development workflow recipes
# 
# This module provides comprehensive Rust development workflows including:
#
# 🏗️  BUILD COMMANDS (rust-build group):
#   • build-rust                     - Debug builds
#   • build-rust-release             - Optimized release builds  
#   • build-rust-vector              - Builds with vector search features
#   • build-rust-all-features        - Builds with all features enabled
#   • clean-rust                     - Clean build artifacts
#
# 🧪 TEST COMMANDS (rust-test group):
#   • test-rust                      - Run standard test suite
#   • test-rust-coverage             - Generate coverage reports
#   • test-rust-vector               - Test vector search features
#   • test-rust-all-features         - Test with all features
#   • test-rust-specific             - Run specific test suite
#
# ✨ CODE QUALITY (rust-quality group):
#   • format-rust                    - Auto-format code
#   • format-rust-check              - Check formatting
#   • lint-rust                      - Run Clippy linter
#   • check-rust                     - Static analysis
#   • pre-commit-rust                - Pre-commit validation
#
# 📦 INSTALLATION (rust-install group):
#   • install-rust                   - Install standard binaries
#   • install-rust-vector-search     - Install with vector features
#   • release-rust-info              - Show binary information
#
# 📚 DOCUMENTATION & INFO (rust-docs group):
#   • docs-rust                      - Generate documentation  
#   • list-rust-binaries             - List available binaries
#   • rust-toolchain-info            - Show toolchain info
#
# ⚙️  SETUP (rust-setup group):
#   • rust-setup                     - Install dev tools
#   • install-rust-tq                - Install TOML query tool
#
# USAGE EXAMPLES:
#   just build-rust-release          # Build optimized binary
#   just test-rust-coverage          # Run tests with coverage
#   just install-rust-vector-search  # Install with vector features
#   just format-rust && just lint-rust  # Format and lint code

import 'common.just'

# =====================================
# Rust Development Environment Setup
# =====================================

# Install Rust development tools and dependencies
[group('rust-setup')]
rust-setup:
    @just _info "Setting up Rust development environment"
    @just _run "Installing cargo-tarpaulin for coverage" "cargo binstall --locked cargo-tarpaulin"
    @just _success "Rust development environment ready! You can now run 'just test-rust-coverage' for coverage reports"

# Install tq (TOML query tool) for better TOML parsing
[group('rust-setup')]
install-rust-tq:
    @just _info "Installing tq (TOML query tool) for better TOML parsing"
    @just _run "Installing tq from git" "cargo install --git https://github.com/cryptaliagy/tomlq"
    @just _success "tq installed successfully"

# =====================================
# Rust Build Commands
# =====================================

# Build Rust project for development
[group('rust-build')]
build-rust:
    @just _run "Building Rust project for development" "cargo build"

# Build Rust project for release with optimizations
[group('rust-build')]
build-rust-release:
    @just _run "Building Rust project for release" "cargo build --release"

# Build Rust project with vector search features
[group('rust-build')]
build-rust-vector:
    @just _run "Building Rust project with vector search features" "cargo build --features vector-search"

# Build Rust project with all features including local embeddings
[group('rust-build')]
build-rust-all-features:
    @just _run "Building Rust project with all features" "cargo build --features \"vector-search,local-embeddings\""

# Build release with all features
[group('rust-build')]
build-rust-release-all-features:
    @just _run "Building release with all features" "cargo build --release --features \"vector-search,local-embeddings\""

# =====================================
# Rust Testing Commands  
# =====================================

# Run Rust tests
[group('rust-test')]
test-rust:
    @just _run "Running Rust tests" "cargo test"

# Run Rust tests with coverage reporting
[group('rust-test')]
test-rust-coverage:
    @just _require-command "cargo-tarpaulin"
    @just _run "Running Rust tests with coverage" "cargo tarpaulin --out Html"
    @just _success "Coverage report generated at tarpaulin-report.html"

# Run Rust tests with vector search features
[group('rust-test')]
test-rust-vector:
    @just _run "Running Rust tests with vector search features" "cargo test --features vector-search"

# Run Rust tests with all features
[group('rust-test')]
test-rust-all-features:
    @just _run "Running Rust tests with all features" "cargo test --features \"vector-search,local-embeddings\""

# Run specific Rust test suite with output
[group('rust-test')]
test-rust-specific test-name:
    @just _run "Running specific test suite: {{test-name}}" "cargo test --test {{test-name}} -- --nocapture"

# =====================================
# Rust Code Quality Commands
# =====================================

# Format Rust code automatically
[group('rust-quality')]
format-rust:
    @just _run "Formatting Rust code" "cargo fmt"

# Check Rust code formatting without changes
[group('rust-quality')]
format-rust-check:
    @just _run "Checking Rust code formatting" "cargo fmt -- --check"

# Run Rust linter (clippy) with warnings as errors
[group('rust-quality')]
lint-rust:
    @just _run "Running Rust clippy linter" "cargo clippy -- -D warnings"

# Run Rust linter (clippy) with automatic fixes
[group('rust-quality')]
lint-rust-fix:
    @just _run "Running Rust clippy with fixes" "cargo clippy --fix --allow-dirty -- -D warnings"

# Run Rust static analysis check
[group('rust-quality')]
check-rust:
    @just _run "Running Rust static check" "cargo check"

# Run comprehensive Rust code quality checks
[group('rust-quality')]
check-rust-all: format-rust-check lint-rust test-rust

# =====================================
# Rust Pre-commit Validation
# =====================================

# Run pre-commit validation for Rust code
[group('rust-quality')]
pre-commit-rust:
    #!/usr/bin/env bash
    just _info "Running Rust pre-commit validation"
    echo "=================================="
    echo ""
    
    # Track success for checks and linters
    checks_success=true
    
    # 1. Static check (cargo check)
    echo "1️⃣  Static code check..."
    if cargo check; then
        echo "   ✅ Static check passed"
    else
        echo "   ❌ Static check failed"
        checks_success=false
    fi
    echo ""
    
    # 2. Code formatting check
    echo "2️⃣  Code formatting check..."
    if cargo fmt --check; then
        echo "   ✅ Code formatting is correct"
    else
        echo "   ❌ Code formatting issues found"
        echo "   💡 Run 'just format-rust' to fix formatting"
        checks_success=false
    fi
    echo ""
    
    # 3. Clippy linter
    echo "3️⃣  Clippy linter check..."
    if cargo clippy -- -D warnings; then
        echo "   ✅ Clippy linter passed"
    else
        echo "   ❌ Clippy linter found issues"
        checks_success=false
    fi
    echo ""
    
    # Check if we should proceed to tests
    if [ "$checks_success" = false ]; then
        echo "=================================="
        echo "❌ FAILURE: Code checks and linters failed"
        echo "🔧 Please fix the above issues before running tests"
        echo "💡 Once fixed, run 'just pre-commit-rust' again to include tests"
        exit 1
    fi
    
    # 4. Tests (only run if all checks passed)
    echo "4️⃣  Running tests..."
    if cargo test; then
        echo "   ✅ All tests passed"
    else
        echo "   ❌ Some tests failed"
        echo ""
        echo "=================================="
        echo "❌ FAILURE: Tests failed"
        echo "🔧 Please fix the failing tests before committing"
        exit 1
    fi
    echo ""
    
    # Final success message
    echo "=================================="
    echo "🎉 SUCCESS: All Rust pre-commit checks passed!"
    echo "✅ Code is ready for commit"

# =====================================
# Rust Installation Commands
# =====================================

# Show information about Rust release binaries  
[group('rust-install')]
release-rust-info:
    #!/usr/bin/env bash
    echo "============================="
    echo "📦 Rust Release Binary Information"
    echo "============================="
    echo ""
    
    if [ ! -d "target/release" ]; then
        just _error "Release Info" "Release directory not found. Run 'just build-rust-release' first"
    fi
    
    echo "🗂️  Release Directory: target/release/"
    echo ""
    
    # Get binaries using common utility
    binaries=$(just _get-binaries)
    
    if [ -z "$binaries" ]; then
        just _error "Release Info" "No [[bin]] sections found in Cargo.toml"
    fi
    
    echo "🔍 Binaries defined in Cargo.toml: $binaries"
    echo ""
    
    found_any=false
    for binary in $binaries; do
        if [ -f "target/release/$binary" ]; then
            echo "🔧 Binary: $binary"
            echo "   📍 Path: target/release/$binary"
            echo "   📏 Size: $(du -h target/release/$binary | cut -f1)"
            echo "   🏗️  Platform: $(uname -m)-$(uname -s | tr '[:upper:]' '[:lower:]')"
            echo "   📅 Modified: $(stat -f '%Sm' -t '%Y-%m-%d %H:%M:%S' target/release/$binary 2>/dev/null || stat -c '%y' target/release/$binary 2>/dev/null | cut -d'.' -f1)"
            if command -v file >/dev/null 2>&1; then
                echo "   🔍 Type: $(file target/release/$binary | cut -d':' -f2 | sed 's/^ *//')"
            fi
            echo ""
            found_any=true
        else
            echo "❌ Binary $binary not found in target/release/"
            echo ""
        fi
    done
    
    if [ "$found_any" = false ]; then
        just _error "Release Info" "No binaries found in target/release/. Run 'just build-rust-release' first"
    fi

# Install Rust release binaries locally
[group('rust-install')]
install-rust: build-rust-release
    #!/usr/bin/env bash
    just _info "Installing Rust Release Binaries"
    echo "=============================="
    echo ""
    
    # Get binaries using common utility
    binaries=$(just _get-binaries)
    
    if [ -z "$binaries" ]; then
        just _error "Installation" "No [[bin]] sections found in Cargo.toml"
    fi
    
    echo "🔍 Installing binaries: $binaries"
    echo ""
    
    # Install using cargo install
    echo "🚀 Running: cargo install --path . --force"
    if cargo install --path . --force; then
        echo ""
        just _success "Installation completed successfully!"
        echo ""
        
        # Show installation information using common utility
        cargo_bin_dir=$(just _get-cargo-bin-dir)
        
        echo "📂 Installation Directory: $cargo_bin_dir"
        echo ""
        
        for binary in $binaries; do
            if [ -f "$cargo_bin_dir/$binary" ]; then
                echo "🔧 Binary: $binary"
                echo "   📍 Path: $cargo_bin_dir/$binary"
                echo "   📏 Size: $(du -h $cargo_bin_dir/$binary | cut -f1)"
                echo "   🏗️  Platform: $(uname -m)-$(uname -s | tr '[:upper:]' '[:lower:]')"
                echo "   📅 Installed: $(stat -f '%Sm' -t '%Y-%m-%d %H:%M:%S' $cargo_bin_dir/$binary 2>/dev/null || stat -c '%y' $cargo_bin_dir/$binary 2>/dev/null | cut -d'.' -f1)"
                if command -v file >/dev/null 2>&1; then
                    echo "   🔍 Type: $(file $cargo_bin_dir/$binary | cut -d':' -f2 | sed 's/^ *//')"
                fi
                echo ""
            else
                echo "❌ Binary $binary not found at $cargo_bin_dir/$binary"
                echo ""
            fi
        done
        
        echo "💡 Usage:"
        for binary in $binaries; do
            echo "   Run directly: $binary --help"
        done
        echo "   Or ensure ~/.cargo/bin is in your PATH"
        echo ""
        
    else
        just _error "Installation" "Installation failed!"
    fi

# Install Rust release binaries with vector search features
[group('rust-install')]
install-rust-vector-search:
    #!/usr/bin/env bash
    just _info "Installing Rust Release Binaries with Vector Search Features"
    echo "========================================================"
    echo ""
    
    # Get binaries using common utility
    binaries=$(just _get-binaries)
    
    if [ -z "$binaries" ]; then
        just _error "Installation" "No [[bin]] sections found in Cargo.toml"
    fi
    
    echo "🔍 Installing binaries: $binaries"
    echo "🔬 Features: vector-search, local-embeddings"
    echo ""
    
    # Build release with vector search features first
    echo "🏗️  Building release with vector search features..."
    if ! cargo build --release --features "vector-search,local-embeddings"; then
        just _error "Build" "Build failed!"
    fi
    echo ""
    
    # Install using cargo install with features
    echo "🚀 Running: cargo install --path . --force --features \"vector-search,local-embeddings\""
    if cargo install --path . --force --features "vector-search,local-embeddings"; then
        echo ""
        just _success "Installation completed successfully!"
        echo ""
        
        # Show installation information using common utility
        cargo_bin_dir=$(just _get-cargo-bin-dir)
        
        echo "📂 Installation Directory: $cargo_bin_dir"
        echo ""
        
        for binary in $binaries; do
            if [ -f "$cargo_bin_dir/$binary" ]; then
                echo "🔧 Binary: $binary"
                echo "   📍 Path: $cargo_bin_dir/$binary"
                echo "   📏 Size: $(du -h $cargo_bin_dir/$binary | cut -f1)"
                echo "   🏗️  Platform: $(uname -m)-$(uname -s | tr '[:upper:]' '[:lower:]')"
                echo "   📅 Installed: $(stat -f '%Sm' -t '%Y-%m-%d %H:%M:%S' $cargo_bin_dir/$binary 2>/dev/null || stat -c '%y' $cargo_bin_dir/$binary 2>/dev/null | cut -d'.' -f1)"
                if command -v file >/dev/null 2>&1; then
                    echo "   🔍 Type: $(file $cargo_bin_dir/$binary | cut -d':' -f2 | sed 's/^ *//')"
                fi
                echo "   ✨ Features: vector-search, local-embeddings"
                echo ""
            else
                echo "❌ Binary $binary not found at $cargo_bin_dir/$binary"
                echo ""
            fi
        done
        
        echo "💡 Usage with Vector Search:"
        for binary in $binaries; do
            echo "   Test installation: $binary search -h"
            echo "   Cache information: $binary search cache-info"
            echo "   Index justfiles: $binary search index --directory . --local-embeddings"
            echo "   Semantic search: $binary search query --query \"your search\" --local-embeddings"
        done
        echo ""
        echo "🤖 Local Embeddings:"
        echo "   • Model cache: ~/.cache/just-mcp/models/ (or custom via --cache-dir)"
        echo "   • Model: sentence-transformers/all-MiniLM-L6-v2 (~80MB)"
        echo "   • First run downloads model automatically"
        echo "   • Fully offline after initial download"
        echo ""
        
    else
        just _error "Installation" "Installation failed!"
    fi

# =====================================
# Rust Utility Commands
# =====================================

# Clean Rust build artifacts
[group('rust-build')]
clean-rust:
    @just _run "Cleaning Rust build artifacts" "cargo clean"

# Check which Rust binaries are defined in Cargo.toml
[group('rust-docs')]
list-rust-binaries:
    #!/usr/bin/env bash
    just _info "Rust binaries defined in Cargo.toml"
    binaries=$(just _get-binaries)
    if [ -z "$binaries" ]; then
        just _warn "No [[bin]] sections found in Cargo.toml"
    else
        echo "📦 Binaries: $binaries"
    fi

# Show Rust toolchain information
[group('rust-docs')]
rust-toolchain-info:
    @just _info "Rust toolchain information"
    @echo "🦀 Rust version: $(rustc --version)"
    @echo "📦 Cargo version: $(cargo --version)"
    @echo "🔧 Rustup version: $(rustup --version)"
    @if [ -f "rust-toolchain.toml" ]; then echo "📋 Toolchain file: rust-toolchain.toml"; fi

# =====================================
# Rust Documentation
# =====================================

# Generate and open Rust documentation
[group('rust-docs')]
docs-rust:
    @just _run "Generating Rust documentation" "cargo doc --open"

# Generate Rust documentation with all features
[group('rust-docs')]
docs-rust-all-features:
    @just _run "Generating Rust documentation with all features" "cargo doc --features \"vector-search,local-embeddings\" --open"