#!/usr/bin/env -S just --justfile

# Recommend installing completion scripts: https://just.systems/man/en/shell-completion-scripts.html
# Recommend installing vscode extension: https://just.systems/man/en/visual-studio-code.html

# Common commands
doppler_run := "doppler run --"
doppler_run_preserve := "doppler run --preserve-env --"

# Default recipe - show available commands
_default:
    @just -l -u

# Brew installation
[group('setup')]
brew:
    brew update & brew bundle install --file=./Brewfile

[group('setup')]
doppler-install:
    brew install gnupg
    brew install dopplerhq/cli/doppler

# Recursively sync git submodules
[group('git')]
sync-submodules:
    git submodule update --init --recursive

# Show git status
[group('git')]
git-status:
    git status

# Create a new git branch
[group('git')]
git-branch name:
    git checkout -b {{ name }}

# Initial project setup
[group('setup')]
setup:
    @echo "📦 Setting up development environment..."
    @echo "Installing Rust development tools..."
    cargo binstall --locked cargo-tarpaulin
    @echo "✅ Setup complete! You can now run 'just test-coverage' for coverage reports."

# Run development mode
[group('dev')]
dev:
    @echo "TODO: Add your dev command here"

# Run tests
[group('test')]
test:
    @echo "Running tests..."
    cargo test

# Run tests with coverage
[group('test')]
test-coverage:
    @echo "Running tests with coverage..."
    cargo tarpaulin --out Html

# Build for development
[group('build')]
build:
    @echo "Building for development..."
    cargo build

# Build the project for release
[group('build')]
build-release:
    @echo "Building project for release..."
    cargo build --release

# Install tq (TOML query tool) for better TOML parsing
[group('rust')]
install-tq:
    @echo "📦 Installing tq (TOML query tool)..."
    cargo install --git https://github.com/cryptaliagy/tomlq

# Show information about release binaries
[group('rust')]
release-info:
    #!/usr/bin/env bash
    echo "============================="
    echo "📦 Release Binary Information"
    echo "============================="
    echo ""
    
    if [ ! -d "target/release" ]; then
        echo "❌ Release directory not found"
        echo "   Run 'just build-release' first"
        exit 0
    fi
    
    echo "🗂️  Release Directory: target/release/"
    echo ""
    
    # Parse TOML to get binary names
    if command -v tq >/dev/null 2>&1 && command -v jq >/dev/null 2>&1; then
        echo "🔍 Using tq + jq to parse Cargo.toml"
        binaries=$(tq -o json -f Cargo.toml 'bin' 2>/dev/null | jq -r '.[].name' 2>/dev/null | tr '\n' ' ')
    elif command -v tq >/dev/null 2>&1; then
        echo "🔍 Using tq to parse Cargo.toml (install jq for better parsing)"
        bin_json=$(tq -o json -f Cargo.toml 'bin' 2>/dev/null)
        # Extract names from JSON manually
        binaries=$(echo "$bin_json" | sed 's/.*"name":"\([^"]*\)".*/\1/g' | tr '\n' ' ')
    else
        echo "🔍 Using AWK to parse Cargo.toml (fallback - install tq for better parsing)"
        echo "   Install with: just install-tq"
        binaries=$(awk '
        /^\[\[bin\]\]/ { in_bin=1; next }
        /^\[/ { in_bin=0 }
        in_bin && /^name = / {
            gsub(/^name = "|"$/, "")
            print
        }
        ' Cargo.toml | tr '\n' ' ')
    fi
    
    if [ -z "$binaries" ]; then
        echo "❌ No [[bin]] sections found in Cargo.toml"
        echo "   Check Cargo.toml configuration"
        exit 0
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
        echo "❌ No binaries found in target/release/"
        echo "   Run 'just build-release' first"
    fi

# Install release binaries locally and show installation info
[group('rust')]
install: build-release
    #!/usr/bin/env bash
    echo "📦 Installing Release Binaries"
    echo "=============================="
    echo ""
    
    # Parse TOML to get binary names (same logic as release-info)
    if command -v tq >/dev/null 2>&1 && command -v jq >/dev/null 2>&1; then
        echo "🔍 Using tq + jq to parse Cargo.toml"
        binaries=$(tq -o json -f Cargo.toml 'bin' 2>/dev/null | jq -r '.[].name' 2>/dev/null | tr '\n' ' ')
    elif command -v tq >/dev/null 2>&1; then
        echo "🔍 Using tq to parse Cargo.toml"
        bin_json=$(tq -o json -f Cargo.toml 'bin' 2>/dev/null)
        binaries=$(echo "$bin_json" | sed 's/.*"name":"\([^"]*\)".*/\1/g' | tr '\n' ' ')
    else
        echo "🔍 Using AWK to parse Cargo.toml"
        binaries=$(awk '
        /^\[\[bin\]\]/ { in_bin=1; next }
        /^\[/ { in_bin=0 }
        in_bin && /^name = / {
            gsub(/^name = "|"$/, "")
            print
        }
        ' Cargo.toml | tr '\n' ' ')
    fi
    
    if [ -z "$binaries" ]; then
        echo "❌ No [[bin]] sections found in Cargo.toml"
        exit 1
    fi
    
    echo "🔍 Installing binaries: $binaries"
    echo ""
    
    # Install using cargo install
    echo "🚀 Running: cargo install --path . --force"
    if cargo install --path . --force; then
        echo ""
        echo "✅ Installation completed successfully!"
        echo ""
        
        # Show installation information  
        if [ -n "$CARGO_HOME" ]; then
            cargo_bin_dir="$CARGO_HOME/bin"
        else
            cargo_bin_dir="$HOME/.cargo/bin"
        fi
        
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
        echo ""
        echo "❌ Installation failed!"
        exit 1
    fi

# Install release binaries with vector search and local embeddings features
[group('rust')]
install-with-vector-search:
    #!/usr/bin/env bash
    echo "📦 Installing Release Binaries with Vector Search Features"
    echo "========================================================"
    echo ""
    
    # Parse TOML to get binary names (same logic as release-info)
    if command -v tq >/dev/null 2>&1 && command -v jq >/dev/null 2>&1; then
        echo "🔍 Using tq + jq to parse Cargo.toml"
        binaries=$(tq -o json -f Cargo.toml 'bin' 2>/dev/null | jq -r '.[].name' 2>/dev/null | tr '\n' ' ')
    elif command -v tq >/dev/null 2>&1; then
        echo "🔍 Using tq to parse Cargo.toml"
        bin_json=$(tq -o json -f Cargo.toml 'bin' 2>/dev/null)
        binaries=$(echo "$bin_json" | sed 's/.*"name":"\([^"]*\)".*/\1/g' | tr '\n' ' ')
    else
        echo "🔍 Using AWK to parse Cargo.toml"
        binaries=$(awk '
        /^\[\[bin\]\]/ { in_bin=1; next }
        /^\[/ { in_bin=0 }
        in_bin && /^name = / {
            gsub(/^name = "|"$/, "")
            print
        }
        ' Cargo.toml | tr '\n' ' ')
    fi
    
    if [ -z "$binaries" ]; then
        echo "❌ No [[bin]] sections found in Cargo.toml"
        exit 1
    fi
    
    echo "🔍 Installing binaries: $binaries"
    echo "🔬 Features: vector-search, local-embeddings"
    echo ""
    
    # Build release with vector search features first
    echo "🏗️  Building release with vector search features..."
    if ! cargo build --release --features "vector-search,local-embeddings"; then
        echo "❌ Build failed!"
        exit 1
    fi
    echo ""
    
    # Install using cargo install with features
    echo "🚀 Running: cargo install --path . --force --features \"vector-search,local-embeddings\""
    if cargo install --path . --force --features "vector-search,local-embeddings"; then
        echo ""
        echo "✅ Installation completed successfully!"
        echo ""
        
        # Show installation information  
        if [ -n "$CARGO_HOME" ]; then
            cargo_bin_dir="$CARGO_HOME/bin"
        else
            cargo_bin_dir="$HOME/.cargo/bin"
        fi
        
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
        echo ""
        echo "❌ Installation failed!"
        exit 1
    fi

# Clean build artifacts and dependencies
[group('clean')]
clean:
    @echo "Cleaning build artifacts..."
    cargo clean

# Format code
[group('lint')]
format:
    @echo "Formatting Rust code..."
    cargo fmt
    @echo "Formatting JSON files..."
    @prettier --write "**/*.json" --ignore-path .gitignore || true
    @echo "Formatting Markdown files..."
    @markdownlint-cli2 --fix "**/*.md" "#node_modules" "#.git" "#target" || true
    @echo "Formatting complete!"

# Lint code
[group('lint')]
lint:
    @echo "Checking Rust formatting..."
    cargo fmt -- --check
    @echo "Running clippy..."
    cargo clippy -- -D warnings
    @echo "Linting JSON files..."
    @prettier --check "**/*.json" --ignore-path .gitignore || exit 1
    @echo "Linting Markdown files..."
    @markdownlint-cli2 "**/*.md" "#node_modules" "#.git" "#target" || exit 1
    @echo "Linting complete!"

# Check code (format + lint + test)
[group('lint')]
check: format lint test

# Pre-commit validation - runs all checks required before committing
[group('format')]
pre-commit:
    #!/usr/bin/env bash
    echo "🔄 Running pre-commit validation..."
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
        echo "   💡 Run 'just fmt' to fix formatting"
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
        echo "💡 Once fixed, run 'just pre-commit' again to include tests"
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
    echo "🎉 SUCCESS: All pre-commit checks passed!"
    echo "✅ Code is ready for commit"

# =====================================
# Dagger CI/CD Commands
# =====================================

# Run Dagger CI pipeline locally
[group('dagger')]
dagger-ci:
    @echo "🚀 Running Dagger CI pipeline..."
    dagger call ci --source .

# Run Dagger format check
[group('dagger')]
dagger-format:
    @echo "🔍 Checking code formatting with Dagger..."
    dagger call format --source .

# Run Dagger lint
[group('dagger')]
dagger-lint:
    @echo "📋 Running clippy with Dagger..."
    dagger call lint --source .

# Run Dagger tests
[group('dagger')]
dagger-test platform="linux/amd64":
    @echo "🧪 Running tests on {{ platform }} with Dagger..."
    dagger call test --source . --platform {{ platform }}

# Run Dagger coverage
[group('dagger')]
dagger-coverage:
    @echo "📊 Generating coverage report with Dagger..."
    dagger call coverage --source . export --path ./tarpaulin-report.html
    @echo "✅ Coverage report saved to tarpaulin-report.html"

# Build with Dagger
[group('dagger')]
dagger-build platform="linux/amd64":
    @echo "🔨 Building for {{ platform }} with Dagger..."
    @mkdir -p ./build
    dagger call build --source . --platform {{ platform }} export --path ./build/just-mcp-debug-{{ replace(platform, "/", "-") }}

# Build release with Dagger
[group('dagger')]
dagger-build-release platform="linux/amd64":
    @echo "📦 Building release for {{ platform }} with Dagger..."
    @mkdir -p ./build
    dagger call build-release --source . --platform {{ platform }} export --path ./build/just-mcp-release-{{ replace(platform, "/", "-") }}

# Build releases for all platforms using Dagger with zigbuild (parallel execution)
[group('dagger')]
dagger-release version="v0.1.0":
    @echo "🚀 Building all platform releases in parallel with Dagger + zigbuild..."
    @mkdir -p ./release-artifacts
    dagger call release-zigbuild --source . --version {{ version }} export --path ./release-artifacts/
    @echo "✅ All platform releases built successfully!"
    @echo "📦 Release artifacts:"
    @ls -la ./release-artifacts/


# =====================================
# Zigbuild Cross-Compilation Commands
# =====================================

# Build all platforms using cargo-zigbuild Docker image
[group('zigbuild')]
zigbuild-release version="v0.1.0":
    #!/usr/bin/env bash
    echo "🚀 Building releases for all platforms using cargo-zigbuild..."
    mkdir -p ./release-artifacts
    
    # Build all platforms in a single container to maintain state
    docker run --rm -v $(pwd):/io -w /io ghcr.io/rust-cross/cargo-zigbuild:latest \
        sh -c '
            echo "📦 Adding Rust targets..." && \
            rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu x86_64-apple-darwin aarch64-apple-darwin && \
            echo "🔨 Building Linux x86_64..." && \
            cargo zigbuild --release --target x86_64-unknown-linux-gnu && \
            echo "🔨 Building Linux ARM64..." && \
            cargo zigbuild --release --target aarch64-unknown-linux-gnu && \
            echo "🔨 Building macOS x86_64..." && \
            cargo zigbuild --release --target x86_64-apple-darwin && \
            echo "🔨 Building macOS ARM64..." && \
            cargo zigbuild --release --target aarch64-apple-darwin && \
            echo "🔨 Building macOS Universal Binary..." && \
            cargo zigbuild --release --target universal2-apple-darwin
        '
    
    # Package all builds
    echo "📦 Packaging release artifacts..."
    
    # Linux x86_64
    tar czf ./release-artifacts/just-mcp-{{ version }}-x86_64-unknown-linux-gnu.tar.gz \
        -C target/x86_64-unknown-linux-gnu/release just-mcp \
        -C "$(pwd)" README.md LICENSE
    
    # Linux ARM64
    tar czf ./release-artifacts/just-mcp-{{ version }}-aarch64-unknown-linux-gnu.tar.gz \
        -C target/aarch64-unknown-linux-gnu/release just-mcp \
        -C "$(pwd)" README.md LICENSE
    
    # macOS x86_64
    tar czf ./release-artifacts/just-mcp-{{ version }}-x86_64-apple-darwin.tar.gz \
        -C target/x86_64-apple-darwin/release just-mcp \
        -C "$(pwd)" README.md LICENSE
    
    # macOS ARM64
    tar czf ./release-artifacts/just-mcp-{{ version }}-aarch64-apple-darwin.tar.gz \
        -C target/aarch64-apple-darwin/release just-mcp \
        -C "$(pwd)" README.md LICENSE
    
    # macOS Universal
    tar czf ./release-artifacts/just-mcp-{{ version }}-universal2-apple-darwin.tar.gz \
        -C target/universal2-apple-darwin/release just-mcp \
        -C "$(pwd)" README.md LICENSE
    
    echo "✅ All platform releases built successfully!"
    echo "📦 Release artifacts:"
    ls -la ./release-artifacts/

# Test zigbuild setup for a single platform
[group('zigbuild')]
zigbuild-test target="x86_64-apple-darwin":
    #!/usr/bin/env bash
    echo "🧪 Testing cargo-zigbuild for {{ target }}..."
    docker run --rm -v $(pwd):/io -w /io ghcr.io/rust-cross/cargo-zigbuild:latest \
        sh -c "rustup target add {{ target }} && cargo zigbuild --release --target {{ target }}"
    echo "✅ Build successful! Binary at: target/{{ target }}/release/just-mcp"

# =====================================
# Vector Search Demo Commands
# =====================================

# Vector search demo - index demo justfile and test search functionality
[group('demo')]
demo-vector-search:
    #!/usr/bin/env bash
    echo "🔍 Vector Search Demo"
    echo "===================="
    echo ""
    
    # Build with vector search feature
    echo "1. Building with vector-search feature..."
    cargo build --features vector-search
    echo ""
    
    # Index the demo justfile
    echo "2. Indexing demo/justfile..."
    target/debug/just-mcp search index --directory demo --mock-embeddings --batch-size 10
    echo ""
    
    # Show database stats
    echo "3. Database statistics:"
    target/debug/just-mcp search stats
    echo ""
    
    # Demonstrate various search scenarios
    echo "4. Search demonstrations:"
    echo ""
    
    echo "🔸 Searching for 'build docker image':"
    target/debug/just-mcp search query --query "build docker image" --mock-embeddings --limit 3
    echo ""
    
    echo "🔸 Searching for 'database operations':"
    target/debug/just-mcp search query --query "database operations" --mock-embeddings --limit 3
    echo ""
    
    echo "🔸 Searching for 'testing and quality':"
    target/debug/just-mcp search query --query "testing and quality" --mock-embeddings --limit 3
    echo ""
    
    echo "🔸 Finding tasks similar to 'deploy':"
    target/debug/just-mcp search similar --task "deploy to production environment" --mock-embeddings --limit 3
    echo ""
    
    echo "🔸 Text search for 'docker':"
    target/debug/just-mcp search text --text "docker" --limit 5
    echo ""
    
    echo "🔸 Filter by task type:"
    target/debug/just-mcp search filter --filter type=justfile_task --limit 5
    echo ""
    
    echo "✅ Demo complete! Database saved as 'vector_search.db'"

# Quick vector search test - build and run a simple search
[group('demo')]
demo-vector-quick:
    @echo "🚀 Quick Vector Search Test"
    @echo "=========================="
    @echo "Building with vector-search feature..."
    cargo build --features vector-search
    @echo "Indexing demo justfile..."
    target/debug/just-mcp search index --directory demo --mock-embeddings --batch-size 20
    @echo "Running sample search..."
    target/debug/just-mcp search query --query "docker deployment" --mock-embeddings --limit 5

# Clean vector search database
[group('demo')]
demo-vector-clean:
    @echo "🧹 Cleaning vector search database..."
    @rm -f vector_search.db
    @echo "✅ Database cleaned"

# Local embeddings demo - index and search using offline models
[group('demo')]
demo-vector-local:
    #!/usr/bin/env bash
    echo "🤖 Local Embeddings Demo"
    echo "========================"
    echo ""
    
    # Build with local embeddings feature
    echo "1. Building with local-embeddings feature..."
    cargo build --features "vector-search,local-embeddings"
    echo ""
    
    # Index the demo justfile with local embeddings
    echo "2. Indexing demo/justfile with local embeddings (first run downloads model ~80MB)..."
    echo "   Note: This may take a moment on first run while the model downloads..."
    target/debug/just-mcp search index --directory demo --local-embeddings --batch-size 10 --database vector_search_local.db
    echo ""
    
    # Show database stats
    echo "3. Database statistics:"
    target/debug/just-mcp search stats --database vector_search_local.db
    echo ""
    
    # Demonstrate semantic search with local embeddings
    echo "4. Semantic search demonstrations with local embeddings:"
    echo ""
    
    echo "🔸 Natural language query: 'How do I build a container image?':"
    target/debug/just-mcp search query --query "How do I build a container image?" --local-embeddings --limit 3 --database vector_search_local.db
    echo ""
    
    echo "🔸 Natural language query: 'I need to run tests':"
    target/debug/just-mcp search query --query "I need to run tests" --local-embeddings --limit 3 --database vector_search_local.db
    echo ""
    
    echo "🔸 Natural language query: 'deploy my application to production':"
    target/debug/just-mcp search query --query "deploy my application to production" --local-embeddings --limit 3 --database vector_search_local.db
    echo ""
    
    echo "🔸 Natural language query: 'check system health and status':"
    target/debug/just-mcp search query --query "check system health and status" --local-embeddings --limit 3 --database vector_search_local.db
    echo ""
    
    echo "🔸 Finding tasks similar to 'backup database':"
    target/debug/just-mcp search similar --task "backup database with compression" --local-embeddings --limit 3 --database vector_search_local.db
    echo ""
    
    echo "✅ Local embeddings demo complete! Database saved as 'vector_search_local.db'"
    echo "💡 Model cached at: ~/.cache/just-mcp/models/ for future use"

# Compare local vs mock embeddings - side-by-side comparison
[group('demo')]
demo-vector-compare:
    #!/usr/bin/env bash
    echo "⚖️  Local vs Mock Embeddings Comparison"
    echo "======================================"
    echo ""
    
    # Build with both features
    echo "1. Building with all vector search features..."
    cargo build --features "vector-search,local-embeddings"
    echo ""
    
    # Clean databases first
    echo "2. Cleaning previous databases..."
    rm -f vector_search_mock.db vector_search_local.db
    echo ""
    
    # Index with mock embeddings
    echo "3. Indexing demo/justfile with MOCK embeddings..."
    target/debug/just-mcp search index --directory demo --mock-embeddings --batch-size 10 --database vector_search_mock.db
    echo ""
    
    # Index with local embeddings
    echo "4. Indexing demo/justfile with LOCAL embeddings (may download model)..."
    target/debug/just-mcp search index --directory demo --local-embeddings --batch-size 10 --database vector_search_local.db
    echo ""
    
    # Compare searches
    echo "5. Comparison Results:"
    echo "====================="
    echo ""
    
    queries=(
        "How do I build my application?"
        "run tests with coverage"
        "deploy to production environment"
        "create backup of data"
        "monitor service health"
    )
    
    for query in "${queries[@]}"; do
        echo "📋 Query: '$query'"
        echo "   ▶️  Mock Embeddings Results:"
        target/debug/just-mcp search query --query "$query" --mock-embeddings --limit 2 --database vector_search_mock.db | sed 's/^/      /'
        echo ""
        echo "   ▶️  Local Embeddings Results:"
        target/debug/just-mcp search query --query "$query" --local-embeddings --limit 2 --database vector_search_local.db | sed 's/^/      /'
        echo ""
        echo "   ────────────────────────────────────────"
        echo ""
    done
    
    echo "📊 Database Statistics Comparison:"
    echo ""
    echo "   Mock Embeddings Database:"
    target/debug/just-mcp search stats --database vector_search_mock.db | sed 's/^/      /'
    echo ""
    echo "   Local Embeddings Database:"
    target/debug/just-mcp search stats --database vector_search_local.db | sed 's/^/      /'
    echo ""
    
    echo "✅ Comparison complete!"
    echo "💡 Key Differences:"
    echo "   • Mock embeddings: Fast, deterministic, poor semantic quality"
    echo "   • Local embeddings: Slower first-time, good semantic understanding, offline"
    echo "   • Local embeddings better understand natural language intent"
    echo "   • Local embeddings require ~80MB model download on first use"

# Test local embeddings with various natural language queries
[group('demo')]
demo-vector-nlp:
    #!/usr/bin/env bash
    echo "🗣️  Natural Language Processing Demo"
    echo "===================================="
    echo ""
    
    # Build and index if needed
    echo "1. Building and preparing local embeddings database..."
    cargo build --features "vector-search,local-embeddings"
    
    # Check if database exists, create if not
    if [ ! -f "vector_search_local.db" ]; then
        echo "   Creating database with local embeddings..."
        target/debug/just-mcp search index --directory demo --local-embeddings --batch-size 10 --database vector_search_local.db
    else
        echo "   Using existing database: vector_search_local.db"
    fi
    echo ""
    
    # Test various natural language queries
    echo "2. Testing natural language understanding:"
    echo ""
    
    nlp_queries=(
        "What can I do to test my code?"
        "How do I package my app for distribution?"
        "I want to start a development server"
        "Show me tasks related to quality assurance"
        "How can I monitor my application?"
        "What deployment options are available?"
        "I need to backup my important data"
        "Show me database-related operations"
        "How do I clean up temporary files?"
        "What tasks help with development workflow?"
    )
    
    for i, query in enumerate "${nlp_queries[@]}"; do
        echo "🔍 Query $((i+1)): '$query'"
        target/debug/just-mcp search query --query "$query" --local-embeddings --limit 2 --database vector_search_local.db --threshold 0.3 | sed 's/^/   /'
        echo ""
    done
    
    echo "✅ Natural language processing demo complete!"
    echo "💡 Local embeddings can understand conversational queries and intent"

# Performance benchmark - compare embedding generation speed
[group('demo')]
demo-vector-benchmark:
    #!/usr/bin/env bash
    echo "⏱️  Embedding Performance Benchmark"
    echo "=================================="
    echo ""
    
    # Build with features
    echo "1. Building for benchmark..."
    cargo build --release --features "vector-search,local-embeddings"
    echo ""
    
    # Clean databases
    echo "2. Preparing clean databases..."
    rm -f vector_search_mock_bench.db vector_search_local_bench.db
    echo ""
    
    # Benchmark mock embeddings
    echo "3. Benchmarking MOCK embeddings indexing speed..."
    time target/release/just-mcp search index --directory demo --mock-embeddings --batch-size 50 --database vector_search_mock_bench.db
    echo ""
    
    # Benchmark local embeddings  
    echo "4. Benchmarking LOCAL embeddings indexing speed..."
    time target/release/just-mcp search index --directory demo --local-embeddings --batch-size 50 --database vector_search_local_bench.db
    echo ""
    
    # Benchmark search speed
    echo "5. Benchmarking search query speed..."
    echo ""
    
    test_query="build and deploy application"
    
    echo "   Mock embeddings search speed:"
    time target/release/just-mcp search query --query "$test_query" --mock-embeddings --limit 5 --database vector_search_mock_bench.db > /dev/null
    echo ""
    
    echo "   Local embeddings search speed:"
    time target/release/just-mcp search query --query "$test_query" --local-embeddings --limit 5 --database vector_search_local_bench.db > /dev/null
    echo ""
    
    echo "✅ Benchmark complete!"
    echo "💡 Performance notes:"
    echo "   • Mock embeddings: Very fast, no model loading"
    echo "   • Local embeddings: Slower first run (model loading), then comparable search speed"
    echo "   • Local embeddings trade initial setup time for better semantic quality"

# Clean all vector search demo databases
[group('demo')]
demo-vector-clean-all:
    @echo "🧹 Cleaning all vector search demo databases..."
    @rm -f vector_search.db vector_search_local.db vector_search_mock.db vector_search_mock_bench.db vector_search_local_bench.db
    @echo "✅ All demo databases cleaned"

