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

