# docker.just - Docker/Dagger operations module
#
# This module provides Docker and Dagger-based CI/CD operations with platform validation
# and consistent error handling. All operations support multiple platforms and include
# proper prerequisites checking.
#
# 🚀 CI/CD PIPELINE (dagger-ci group):
#   • dagger-ci                      - Run complete CI pipeline locally
#   • dagger-format                  - Check code formatting with Dagger
#   • dagger-lint                    - Run Clippy linting with Dagger
#   • dagger-test [platform]         - Run tests on specified platform
#   • dagger-coverage                - Generate test coverage reports
#
# 🏗️  BUILD OPERATIONS (dagger-build group):
#   • dagger-build [platform]        - Build debug version for platform
#   • dagger-build-release [platform] - Build release version for platform
#
# 🔧 UTILITY OPERATIONS (docker group):
#   • docker-help                    - Show all available Docker/Dagger commands
#   • docker-check                   - Verify Docker daemon and Dagger installation
#   • docker-clean                   - Clean Docker build artifacts
#
# 🌍 SUPPORTED PLATFORMS:
#   • linux/amd64                    - Linux x86-64
#   • linux/arm64                    - Linux ARM64
#   • darwin/amd64                   - macOS Intel
#   • darwin/arm64                   - macOS Apple Silicon
#
# PREREQUISITES:
#   • Docker daemon running
#   • Dagger CLI installed
#   • Platform-specific dependencies
#
# USAGE EXAMPLES:
#   just dagger-ci                   # Run full CI pipeline
#   just dagger-test linux/arm64     # Test on Linux ARM64
#   just dagger-build-release        # Build release for default platform
#   just docker-check                # Verify prerequisites
#
# NOTE: Release operations have been moved to release.just module.
#       Use 'just release-targets' to see all release options.
#
# Import common utilities
import '../just/common.just'

# Valid platforms for validation
_valid_platforms := "linux/amd64 linux/arm64 darwin/amd64 darwin/arm64"


# Platform validation
_validate_platform platform:
    @just _validate "{{platform}}" "{{_valid_platforms}}"

# =====================================
# Dagger CI/CD Commands
# =====================================

# Run Dagger CI pipeline locally
[group('dagger-ci')]
dagger-ci:
    @just _require-command dagger
    @just _run "Running Dagger CI pipeline" "dagger call ci --source ."

# Run Dagger format check
[group('dagger-ci')]
dagger-format:
    @just _require-command dagger
    @just _run "Checking code formatting with Dagger" "dagger call format --source ."

# Run Dagger lint
[group('dagger-ci')]
dagger-lint:
    @just _require-command dagger
    @just _run "Running clippy with Dagger" "dagger call lint --source ."

# Run Dagger tests
[group('dagger-ci')]
dagger-test platform="linux/amd64":
    @just _validate_platform "{{platform}}"
    @just _require-command dagger
    @just _run "Running tests on {{platform}} with Dagger" "dagger call test --source . --platform {{platform}}"

# Run Dagger coverage
[group('dagger-ci')]
dagger-coverage:
    @just _require-command dagger
    @echo "📊 Generating coverage report with Dagger..."
    dagger call coverage --source . export --path ./tarpaulin-report.html
    @just _success "Coverage report saved to tarpaulin-report.html"

# =====================================
# Dagger Build Commands
# =====================================

# Build with Dagger
[group('dagger-build')]
dagger-build platform="linux/amd64":
    @just _validate_platform "{{platform}}"
    @just _require-command dagger
    @echo "🔨 Building for {{platform}} with Dagger..."
    @mkdir -p ./build
    dagger call build --source . --platform {{platform}} export --path ./build/just-mcp-debug-{{replace(platform, "/", "-")}}
    @just _success "Debug build for {{platform}} complete"

# Build release with Dagger
[group('dagger-build')]
dagger-build-release platform="linux/amd64":
    @just _validate_platform "{{platform}}"
    @just _require-command dagger
    @echo "📦 Building release for {{platform}} with Dagger..."
    @mkdir -p ./build
    dagger call build-release --source . --platform {{platform}} export --path ./build/just-mcp-release-{{replace(platform, "/", "-")}}
    @just _success "Release build for {{platform}} complete"


# =====================================
# Docker Utility Commands
# =====================================

# List all available Docker/Dagger commands
[group('docker')]
docker-help:
    @echo "🐳 Docker/Dagger Commands Available:"
    @echo ""
    @echo "CI/CD Pipeline:"
    @echo "  dagger-ci        - Run full CI pipeline"
    @echo "  dagger-format    - Check code formatting"
    @echo "  dagger-lint      - Run linting with clippy"
    @echo "  dagger-test      - Run tests on specified platform"
    @echo "  dagger-coverage  - Generate test coverage report"
    @echo ""
    @echo "Build Commands:"
    @echo "  dagger-build         - Build debug version for platform"
    @echo "  dagger-build-release - Build release version for platform"
    @echo ""
    @echo "Valid platforms: {{_valid_platforms}}"
    @echo ""
    @echo "Note: Release commands have been moved to 'just/release.just'"
    @echo "      Use 'just release-targets' to see all release options"

# Check Docker and Dagger requirements
[group('docker')]
docker-check:
    @just _require-command docker
    @just _require-command dagger
    @echo "🔍 Checking Docker daemon..."
    @docker info > /dev/null 2>&1 || just _error "Docker check" "Docker daemon is not running"
    @just _success "Docker and Dagger are properly configured"

# Clean Docker build artifacts
[group('docker')]
docker-clean:
    @echo "🧹 Cleaning Docker build artifacts..."
    @rm -rf ./build || true
    @rm -f ./tarpaulin-report.html || true
    @just _success "Docker build artifacts cleaned"
    @echo "Note: To clean release artifacts, use 'just release-clean'"