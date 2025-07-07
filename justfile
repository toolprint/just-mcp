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
    @echo "TODO: Add your setup command here"

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

# Build the project
[group('build')]
build:
    @echo "Building project..."
    cargo build --release

# Build for development
[group('build')]
build-dev:
    @echo "Building for development..."
    cargo build

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
    @markdownlint-cli2 --fix "**/*.md" "#node_modules" "#.git" || true
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
    @markdownlint-cli2 "**/*.md" "#node_modules" "#.git" || exit 1
    @echo "Linting complete!"

# Check code (format + lint + test)
[group('lint')]
check: format lint test