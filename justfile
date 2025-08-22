#!/usr/bin/env -S just --justfile

# Recommend installing completion scripts: https://just.systems/man/en/shell-completion-scripts.html
# Recommend installing vscode extension: https://just.systems/man/en/visual-studio-code.html

# Import modular justfiles
import 'just/common.just'
import 'just/rust.just'
import 'just/setup.just'
import 'just/vector.just'
import 'just/docker.just'
import 'just/release.just'

# Common commands
doppler_run := "doppler run --"
doppler_run_preserve := "doppler run --preserve-env --"

# Default recipe - comprehensive help system
default:
    @just help

# Comprehensive help system
[group('help')]
help:
    #!/usr/bin/env bash
    echo "🚀 just-mcp Development Commands"
    echo "═══════════════════════════════════════════════════════════"
    echo ""
    echo "🎯 QUICK START:"
    echo "  just quickstart         - Complete setup for new developers"
    echo "  just dev-setup          - Comprehensive development environment setup"
    echo "  just ci                 - Run all CI checks"
    echo "  just workflow [target]  - Run development workflow (quick/full/all/commit)"
    echo "  just dev                - Start development environment"
    echo ""
    echo "▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬"
    echo "🔥 MOST USED COMMANDS:"
    echo "  just build              - Build project for development"
    echo "  just test               - Run all tests"
    echo "  just format             - Format all code (Rust + JSON + Markdown)"
    echo "  just lint               - Lint all code with fixes"
    echo "  just check              - Run format + lint + test (full validation)"
    echo "  just install            - Install with default features"
    echo "  just install-all-features - Install with all possible features"
    echo "  just install-tq         - Install tq for env support"
    echo "  just dev                - Start development environment"
    echo "  just ci                 - Run all CI checks"
    echo "  just clean              - Clean all build artifacts"
    echo "  just pre-commit         - Validate before committing"
    echo ""
    echo "▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬"
    echo "🔧 DETAILED DEVELOPMENT:"
    echo "  just build [debug|release] - Build project with specific mode"
    echo "  just test [true|false]     - Run tests with optional coverage"
    echo "  just format [rust|json|markdown|all] - Format specific code types"
    echo "  just lint [target] [fix]   - Lint code (target: rust|json|markdown|all, fix: true|false)"
    echo "  just workflow [quick|full|all|commit] - Run development workflows"
    echo ""
    echo "▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬"
    echo "🏗️  BUILD & RELEASE:"
    echo "  just release <version> [zigbuild|dagger] - Create release for all platforms"
    echo "  just zigbuild-release [version]          - Cross-compile for all platforms"
    echo "  just dagger-ci                           - Run CI pipeline with Dagger"
    echo ""
    echo "▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬"
    echo "🔍 DISCOVERY & NAVIGATION:"
    echo "  just --list             - List all recipes organized by groups"
    echo "  just --groups           - List all recipe groups"
    echo "  just --summary          - Compact list of recipe names only"
    echo "  just --show <recipe>    - Show recipe source code"
    echo "  just --choose           - Interactive recipe picker (requires fzf)"
    echo ""
    echo "📚 MODULE-SPECIFIC HELP:"
    echo "  just rust-help          - Rust development commands"
    echo "  just docker-help        - Docker/Dagger CI/CD operations"
    echo "  just vector-help        - Vector search demos and utilities"
    echo "  just release-help       - Release and deployment operations"
    echo ""
    echo "💡 TIPS:"
    echo "  • Use 'just --show <recipe>' to see implementation details"
    echo "  • Commands are organized by logical groups and functionality"
    echo "  • Try 'just --choose' for interactive command selection"
    echo "  • Most commands work without parameters (sensible defaults)"
    echo "  • Vector search: try 'just demo-search' or 'just demo-local'"
    echo "  • Docker/CI: try 'just dagger-ci' or 'just dagger-test'"
    echo ""
    echo "🆘 GETTING HELP:"
    echo "  • Documentation: docs/refactor/justfile-refactor.md"
    echo "  • Best practices: docs/guides/justfile-best-practices.md"
    echo "  • Module help: just <module>-help (e.g., just docker-help)"

# Setup recipes
[group('setup')]
brew:
    just _setup-brew



# Complete development environment setup
[group('setup')]
setup:
    @just _info "Setting up complete development environment..."
    @just _setup-all
    @just _success "Setup complete! Development environment is ready."

# ===========================================
# Unified Delegation Commands
# These provide consistent interfaces across modules
# ===========================================

# Run development mode
[group('dev')]
dev:
    @echo "🚀 Starting development environment..."
    @echo "💡 Tip: Use 'just build' to build, 'just test' to test"
    @echo "📚 Run 'just help' for all available commands"

# Unified test command
[group('dev')]
test coverage="false":
    #!/usr/bin/env bash
    just _validate "{{coverage}}" "true false"
    if [ "{{coverage}}" = "true" ]; then
        just test-rust-coverage
    else
        just test-rust
    fi

# Unified build command
[group('dev')]
build mode="debug":
    #!/usr/bin/env bash
    just _validate "{{mode}}" "debug release"
    if [ "{{mode}}" = "release" ]; then
        just build-rust-release
    else
        just build-rust
    fi

# Build development tools (separate workspace member)
[group('dev')]
build-dev-tools:
    @echo "🔧 Building development tools..."
    @cargo build --manifest-path dev-tools/Cargo.toml
    @just _success "Development tools built successfully"

# Install tq (TOML query tool) for env support
[group('dev')]
install-tq:
    just install-rust-tq

# Show information about release binaries
[group('dev')]
release-info:
    just release-rust-info


# Clean build artifacts and dependencies
[group('dev')]
clean:
    just clean-rust

# Unified format command (Rust + JSON + Markdown)
[group('dev')]
format target="all":
    #!/usr/bin/env bash
    just _validate "{{target}}" "rust json markdown all"
    case "{{target}}" in
        rust)
            just format-rust
            ;;
        json)
            echo "🔄 Formatting JSON files..."
            prettier --write "**/*.json" --ignore-path .gitignore || true
            echo "✅ JSON formatting complete"
            ;;
        markdown)
            echo "🔄 Formatting Markdown files..."
            markdownlint-cli2 --fix "**/*.md" "#node_modules" "#.git" "#target" || true
            echo "✅ Markdown formatting complete"
            ;;
        all)
            echo "🔄 Formatting all code..."
            just format-rust
            echo "🔄 Formatting JSON files..."
            prettier --write "**/*.json" --ignore-path .gitignore || true
            echo "🔄 Formatting Markdown files..."
            markdownlint-cli2 --fix "**/*.md" "#node_modules" "#.git" "#target" || true
            echo "✅ All formatting complete!"
            ;;
    esac

# Unified lint command (Rust + JSON + Markdown)
[group('dev')]
lint target="all" fix="false":
    #!/usr/bin/env bash
    just _validate "{{target}}" "rust json markdown all"
    just _validate "{{fix}}" "true false"
    case "{{target}}" in
        rust)
            if [ "{{fix}}" = "true" ]; then
                just lint-rust-fix
            else
                just lint-rust
            fi
            ;;
        json)
            echo "🔄 Linting JSON files..."
            if [ "{{fix}}" = "true" ]; then
                prettier --write "**/*.json" --ignore-path .gitignore || true
            else
                prettier --check "**/*.json" --ignore-path .gitignore || exit 1
            fi
            echo "✅ JSON linting complete"
            ;;
        markdown)
            echo "🔄 Linting Markdown files..."
            if [ "{{fix}}" = "true" ]; then
                markdownlint-cli2 --fix "**/*.md" "#node_modules" "#.git" "#target" || true
            else
                markdownlint-cli2 "**/*.md" "#node_modules" "#.git" "#target" || exit 1
            fi
            echo "✅ Markdown linting complete"
            ;;
        all)
            echo "🔄 Linting all code..."
            if [ "{{fix}}" = "true" ]; then
                just lint-rust-fix
                prettier --write "**/*.json" --ignore-path .gitignore || true
                markdownlint-cli2 --fix "**/*.md" "#node_modules" "#.git" "#target" || true
            else
                just format-rust-check
                just lint-rust
                prettier --check "**/*.json" --ignore-path .gitignore || exit 1
                markdownlint-cli2 "**/*.md" "#node_modules" "#.git" "#target" || exit 1
            fi
            echo "✅ All linting complete!"
            ;;
    esac

# Install with default features
[group('dev')]
install:
    just install-rust

# Install with all possible features
[group('dev')]
install-all-features:
    #!/usr/bin/env bash
    just _info "Installing with all possible features"
    echo "Features: vector-search, local-embeddings, ultrafast-framework"
    echo ""
    
    # Build release with all features first
    if just build-rust-release-all-features; then
        # Install using cargo install with all features
        echo "🚀 Running: cargo install --path . --force --all-features"
        if cargo install --path . --force --all-features; then
            echo ""
            just _success "Installation with all features completed successfully!"
            echo ""
            
            # Show what was installed
            binaries=$(just _get-binaries)
            echo "📦 Installed binaries: $binaries"
            echo "🔬 Enabled features: vector-search, local-embeddings, ultrafast-framework"
            echo ""
            echo "🎯 You can now use the full feature set including:"
            echo "   • Vector search and semantic indexing"
            echo "   • Local embedding models"
            echo "   • Ultra-fast MCP framework"
        else
            just _error "Installation" "Installation with all features failed!"
        fi
    else
        just _error "Installation" "Build with all features failed!"
    fi

# Unified check command (format + lint + test)
[group('dev')]
check target="all":
    #!/usr/bin/env bash
    just _validate "{{target}}" "quick full all"
    case "{{target}}" in
        quick)
            echo "🔄 Running quick checks..."
            just format
            just lint
            ;;
        full)
            echo "🔄 Running full checks..."
            just format
            just lint
            just test
            ;;
        all)
            echo "🔄 Running all checks..."
            just format
            just lint
            just test
            ;;
    esac

# Pre-commit validation
[group('dev')]
pre-commit:
    just pre-commit-rust

# Docker/Dagger recipes are now available through docker.just import
# Available: dagger-ci, dagger-format, dagger-lint, dagger-test, dagger-coverage
# Available: dagger-build, dagger-build-release, docker-help, docker-check, docker-clean

# Release recipes are now available through release.just import
# Available: zigbuild-release, zigbuild-test, zigbuild-target, dagger-release
# Available: dagger-release-platform, release-clean, release-targets, release-check, release

# ===========================================
# Workflow Recipes
# These combine multiple tasks for common developer scenarios
# ===========================================

# Complete setup for new developers
[group('workflow')]
quickstart:
    #!/usr/bin/env bash
    echo "🚀 just-mcp Quickstart Setup"
    echo "=============================="
    echo ""
    
    # 1. Setup development environment
    echo "1️⃣  Setting up development environment..."
    just setup-project
    echo ""
    
    # 2. Check requirements and install dependencies  
    echo "2️⃣  Checking and installing optional tools..."
    just setup-brew || true  # Non-fatal if homebrew not available
    echo ""
    
    # 3. Build the project
    echo "3️⃣  Building the project..."
    just build
    echo ""
    
    # 4. Run tests to verify everything works
    echo "4️⃣  Running tests to verify setup..."
    just test
    echo ""
    
    # 5. Show helpful next steps
    echo "✅ Quickstart complete!"
    echo ""
    echo "🎯 NEXT STEPS:"
    echo "  • Run 'just dev' to start development environment"
    echo "  • Run 'just help' to see all available commands"
    echo "  • Edit code and use 'just check' before committing"
    echo "  • Try vector search demos: 'just demo-vector-search'"
    echo "  • Use 'just install' for default features or 'just install-all-features' for full capabilities"
    echo ""
    echo "📚 DOCUMENTATION:"
    echo "  • Project docs: docs/refactor/justfile-refactor.md"
    echo "  • Best practices: Available through MCP resource"
    echo "  • Module help: Use 'just <module>-help' (e.g., 'just docker-help')"

# Development environment setup (comprehensive)
[group('workflow')]
dev-setup:
    #!/usr/bin/env bash
    echo "🔧 Development Environment Setup"
    echo "================================="
    echo ""
    
    # 1. Core project setup
    echo "1️⃣  Core project setup..."
    just setup-project
    echo ""
    
    # 2. Install all optional development tools
    echo "2️⃣  Installing development tools..."
    just setup-brew
    echo ""
    
    # 3. Install tq for better TOML parsing
    echo "3️⃣  Installing TOML query tool..."
    just install-tq
    echo ""
    
    # 4. Format and lint all code
    echo "4️⃣  Formatting and linting codebase..."
    just format
    echo ""
    
    # 5. Build in both debug and release modes
    echo "5️⃣  Building project (debug and release)..."
    just build debug
    just build release
    echo ""
    
    # 6. Run comprehensive tests
    echo "6️⃣  Running comprehensive test suite..."
    just test coverage=true
    echo ""
    
    # 7. Verify Docker/Dagger setup if available
    echo "7️⃣  Checking Docker/Dagger setup..."
    just docker-check || echo "⚠️  Docker/Dagger not available (optional for development)"
    echo ""
    
    # 8. Install release binaries
    echo "8️⃣  Installing release binaries..."
    just install
    echo ""
    
    echo "✅ Development environment fully configured!"
    echo ""
    echo "🛠️  DEVELOPMENT WORKFLOW:"
    echo "  • Use 'just dev' to start development session"
    echo "  • Use 'just check' before committing changes"  
    echo "  • Use 'just pre-commit' for full validation"
    echo "  • Use 'just ci' to run CI checks locally"

# Run all CI checks (equivalent to what runs in CI/CD)
[group('workflow')]
ci:
    #!/usr/bin/env bash
    echo "🔍 Running CI/CD Checks"
    echo "======================="
    echo ""
    
    # 1. Format check (fail if not formatted)
    echo "1️⃣  Checking code formatting..."
    just format-rust-check || just _error "CI checks" "Code is not properly formatted. Run 'just format' to fix."
    echo ""
    
    # 2. Lint with clippy (strict mode)
    echo "2️⃣  Running Clippy linter..."
    just lint-rust || just _error "CI checks" "Clippy found issues. Fix them and try again."
    echo ""
    
    # 3. Run all tests with coverage
    echo "3️⃣  Running comprehensive test suite..."
    just test coverage=true || just _error "CI checks" "Tests failed. Fix them and try again."
    echo ""
    
    # 4. Build release (ensure it compiles cleanly)
    echo "4️⃣  Building release version..."
    just build release || just _error "CI checks" "Release build failed."
    echo ""
    
    # 5. Validate JSON and Markdown
    echo "5️⃣  Validating JSON and Markdown..."
    just lint json || just _error "CI checks" "JSON validation failed."
    just lint markdown || just _error "CI checks" "Markdown validation failed."
    echo ""
    
    # 6. Security audit if cargo-audit is available
    echo "6️⃣  Security audit (if available)..."
    if command -v cargo-audit >/dev/null 2>&1; then
        cargo audit || just _warn "Security audit found potential issues"
    else
        echo "   cargo-audit not installed (optional)"
    fi
    echo ""
    
    echo "✅ All CI checks passed!"
    echo ""
    echo "🚀 READY FOR:"
    echo "  • Git commit and push"
    echo "  • Pull request creation"
    echo "  • Release deployment"

# Complete development workflow (code → test → check → commit ready)
[group('workflow')]
workflow target="all":
    #!/usr/bin/env bash
    just _validate "{{target}}" "quick full all commit"
    echo "🔄 Development Workflow"
    echo "======================"
    echo ""
    
    case "{{target}}" in
        quick)
            echo "Running quick development workflow..."
            just format
            just lint
            just test
            ;;
        full)
            echo "Running full development workflow..."
            just format
            just lint
            just test coverage=true
            just build release
            ;;
        all)
            echo "Running complete development workflow..."
            just ci
            ;;
        commit)
            echo "Running commit-ready workflow..."
            just pre-commit
            ;;
    esac
    
    echo ""
    echo "✅ Workflow complete!"

# ===========================================
# Convenience Wrapper Recipes
# These provide easy access to discovery and navigation features
# ===========================================

# List all recipes organized by groups
[group('help')]
list:
    @just --list

# List all recipe groups
[group('help')]
groups:
    @just --groups

# Show recipe names in compact format
[group('help')]
summary:
    @just --summary

# Interactive recipe picker (requires fzf)
[group('help')]
choose:
    @just --choose

# Show source code for a specific recipe
[group('help')]
show recipe:
    @just --show {{recipe}}

# List all available variables and their values
[group('help')]
variables:
    @just --variables

# Dump all recipes as JSON (useful for tooling)
[group('help')]
dump:
    @just --dump

# Evaluate a justfile expression
[group('help')]
evaluate expression:
    @just --evaluate {{expression}}

# ===========================================
# Module-Specific Help Wrappers
# ===========================================

# Show help for release operations
[group('help')]
release-help:
    @just release-targets

# Show help for vector search operations
[group('help')]
vector-help:
    #!/usr/bin/env bash
    echo "🔍 Vector Search Commands:"
    echo ""
    echo "Main Demos:"
    echo "  just demo-search           - Full vector search demo with mock embeddings"
    echo "  just demo-local            - Local embeddings demo (downloads ~80MB model)"
    echo "  just demo-quick [query]    - Quick search test"
    echo "  just demo-compare          - Compare mock vs local embeddings"
    echo "  just demo-nlp              - Natural language processing tests"
    echo "  just demo-benchmark        - Performance benchmarking"
    echo ""
    echo "Utilities:"
    echo "  just search-query <query>           - Run custom search query"
    echo "  just index-directory <dir>          - Index a directory"
    echo "  just stats [database]               - Show database statistics"
    echo "  just vector-clean [database]        - Clean specific database"
    echo "  just vector-clean-all               - Clean all demo databases"
    echo ""
    echo "Features available: vector-search, local-embeddings"
    echo "Build with features: cargo build --features 'vector-search,local-embeddings'"



# Show help for Rust development operations
[group('help')]
rust-help:
    #!/usr/bin/env bash
    echo "🦀 Rust Development Help:"
    echo ""
    echo "Available from rust.just module:"
    echo "  just build-rust [--release]        - Build Rust project"
    echo "  just test-rust [--coverage]        - Run Rust tests"
    echo "  just format-rust [--check]         - Format Rust code"
    echo "  just lint-rust [--fix]             - Lint Rust code with Clippy"
    echo "  just install                        - Install with default features"
    echo "  just install-all-features           - Install with all possible features"
    echo "  just clean-rust                    - Clean Rust artifacts"
    echo "  just pre-commit-rust               - Run pre-commit validation"
    echo ""
    echo "Unified commands:"
    echo "  just build, just test, just format, just lint, just install, just install-all-features"

# Show all available help topics
[group('help')]
help-topics:
    #!/usr/bin/env bash
    echo "📚 Available Help Topics:"
    echo ""
    echo "General Help:"
    echo "  just help              - Main help system"
    echo "  just list              - List all recipes by groups"
    echo "  just groups            - List all recipe groups"
    echo "  just summary           - Compact recipe list"
    echo "  just choose            - Interactive recipe picker (requires fzf)"
    echo ""
    echo "Module-Specific Help:"
    echo "  just docker-help       - Docker/Dagger operations (from docker.just)"
    echo "  just release-help      - Release and deployment"
    echo "  just vector-help       - Vector search functionality"
    echo "  just rust-help         - Rust development commands"
    echo ""
    echo "Discovery Commands:"
    echo "  just variables         - Show all variables"
    echo "  just show <recipe>     - Show recipe source code"
    echo "  just dump              - Export all recipes as JSON"
    echo "  just evaluate <expr>   - Evaluate justfile expression"


