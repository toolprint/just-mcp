# release.just - Release and deployment operations module
#
# This module provides comprehensive release automation including cross-compilation,
# packaging, and deployment workflows. Supports both cargo-zigbuild and Dagger
# for maximum flexibility and platform coverage.
#
# 🚀 MAIN RELEASE COMMANDS:
#   • release [version] [method]     - Complete release workflow (zigbuild/dagger)
#   • zigbuild-release [version]     - Cross-compile all platforms with zigbuild
#   • dagger-release [version]       - Parallel build all platforms with Dagger
#
# 🎯 TARGETED BUILDS:
#   • zigbuild-target <target> [ver] - Build specific target with zigbuild
#   • zigbuild-test <target>         - Test zigbuild for single target
#   • dagger-release-platform <plat> - Build specific platform with Dagger
#
# 🔍 INFORMATION & VALIDATION:
#   • release-targets                - Show all available targets and platforms
#   • release-check                  - Validate release prerequisites
#   • release-clean                  - Clean all release artifacts
#
# 🌍 SUPPORTED TARGETS:
#   Zigbuild targets:
#   • x86_64-unknown-linux-gnu       - Linux x86-64
#   • aarch64-unknown-linux-gnu      - Linux ARM64
#   • x86_64-apple-darwin            - macOS Intel
#   • aarch64-apple-darwin           - macOS Apple Silicon
#   • universal2-apple-darwin        - macOS Universal Binary
#
#   Dagger platforms:
#   • linux/amd64, linux/arm64       - Linux platforms
#   • darwin/amd64, darwin/arm64     - macOS platforms
#
# 📦 PACKAGING:
#   • Automatic tar.gz creation
#   • SHA256 checksums generated
#   • README.md and LICENSE included
#   • Artifacts saved to ./release-artifacts/
#
# VERSION REQUIREMENTS:
#   • Semantic versioning: v1.2.3, v1.2.3-alpha, v1.2.3-beta.1, v1.2.3+build.123
#   • Must start with 'v'
#   • Pre-release and build metadata supported
#
# USAGE EXAMPLES:
#   just release v1.0.0 zigbuild     # Full release with zigbuild
#   just zigbuild-release v1.0.0     # Cross-compile all targets
#   just zigbuild-test x86_64-apple-darwin  # Test single target
#   just release-targets             # Show available options
#   just release-check               # Verify prerequisites
#
# PREREQUISITES:
#   • Docker daemon running (for zigbuild/dagger)
#   • README.md and LICENSE files present
#   • Valid Cargo.toml configuration
#
# Import common utilities
import '../just/common.just'

# =====================================
# Release Configuration and Validation
# =====================================

# Valid platforms for Dagger release builds
_release_platforms := "linux/amd64 linux/arm64 darwin/amd64 darwin/arm64"

# Valid zigbuild targets for cross-compilation
_valid_zigbuild_targets := "x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu x86_64-apple-darwin aarch64-apple-darwin universal2-apple-darwin"

# All supported release targets (combination of both)
_all_release_targets := "x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu x86_64-apple-darwin aarch64-apple-darwin universal2-apple-darwin"

# Version pattern validation (semantic versioning)
_validate_version version:
    #!/usr/bin/env bash
    if [[ ! "{{version}}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9]+(\.[a-zA-Z0-9]+)*)?(\+[a-zA-Z0-9]+(\.[a-zA-Z0-9]+)*)?$ ]]; then
        just _error "Version validation" "Invalid version format: {{version}}. Expected semantic versioning format: v1.2.3, v1.2.3-alpha, v1.2.3-beta.1, or v1.2.3+build.123"
    fi

# Platform validation for Dagger builds
_validate_release_platform platform:
    @just _validate "{{platform}}" "{{_release_platforms}}"

# Zigbuild target validation  
_validate_zigbuild_target target:
    @just _validate "{{target}}" "{{_valid_zigbuild_targets}}"

# Release target validation
_validate_release_target target:
    @just _validate "{{target}}" "{{_all_release_targets}}"

# Ensure release directory exists
_ensure_release_dir:
    @mkdir -p ./release-artifacts

# =====================================
# Zigbuild Cross-Compilation Commands
# =====================================

# Build all platforms using cargo-zigbuild Docker image
[group('release')]
zigbuild-release version="v0.1.0":
    #!/usr/bin/env bash
    set -euo pipefail
    
    just _validate_version "{{version}}"
    just _require-command docker
    just _ensure_release_dir
    
    echo "🚀 Building releases for all platforms using cargo-zigbuild..."
    
    # Build all platforms in a single container to maintain state
    echo "🐳 Starting cargo-zigbuild container..."
    docker run --rm -v $(pwd):/io -w /io ghcr.io/rust-cross/cargo-zigbuild:latest \
        bash -c '
            set -euo pipefail
            # Enable assertions for C code (required by tree-sitter-just)
            export CFLAGS="-UNDEBUG"
            export CXXFLAGS="-UNDEBUG"
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
    
    # Package all builds with consistent naming
    echo "📦 Packaging release artifacts..."
    just _package_release_artifacts "{{version}}"
    
    just _success "All platform releases built successfully!"
    just _show_release_summary

# Test zigbuild setup for a single platform
[group('release')]
zigbuild-test target="x86_64-apple-darwin":
    #!/usr/bin/env bash
    set -euo pipefail
    
    just _validate_zigbuild_target "{{target}}"
    just _require-command docker
    
    echo "🧪 Testing cargo-zigbuild for {{target}}..."
    docker run --rm -v $(pwd):/io -w /io ghcr.io/rust-cross/cargo-zigbuild:latest \
        bash -c "set -euo pipefail && export CFLAGS='-UNDEBUG' && export CXXFLAGS='-UNDEBUG' && rustup target add {{target}} && cargo zigbuild --release --target {{target}}"
    
    just _success "Build successful! Binary at: target/{{target}}/release/just-mcp"

# Build release for a specific target
[group('release')]
zigbuild-target target version="v0.1.0":
    #!/usr/bin/env bash
    set -euo pipefail
    
    just _validate_zigbuild_target "{{target}}"
    just _validate_version "{{version}}"
    just _require-command docker
    just _ensure_release_dir
    
    echo "🔨 Building {{target}} release using cargo-zigbuild..."
    docker run --rm -v $(pwd):/io -w /io ghcr.io/rust-cross/cargo-zigbuild:latest \
        bash -c "set -euo pipefail && export CFLAGS='-UNDEBUG' && export CXXFLAGS='-UNDEBUG' && rustup target add {{target}} && cargo zigbuild --release --target {{target}}"
    
    echo "📦 Packaging {{target}} release artifact..."
    just _package_single_target "{{target}}" "{{version}}"
    
    just _success "Release for {{target}} built successfully!"

# =====================================
# Dagger Release Commands
# =====================================

# Build releases for all platforms using Dagger with zigbuild (parallel execution)
[group('release')]
dagger-release version="v0.1.0":
    #!/usr/bin/env bash
    set -euo pipefail
    
    just _validate_version "{{version}}"
    just _require-command dagger
    just _ensure_release_dir
    
    echo "🚀 Building all platform releases in parallel with Dagger + zigbuild..."
    dagger call release-zigbuild --source . --version {{version}} export --path ./release-artifacts/
    
    just _success "All platform releases built successfully!"
    just _show_release_summary

# Build release with Dagger for specific platform (with packaging)
[group('release')]
dagger-release-platform platform="linux/amd64" version="v0.1.0":
    #!/usr/bin/env bash
    set -euo pipefail
    
    just _validate_release_platform "{{platform}}"
    just _validate_version "{{version}}"
    just _require-command dagger
    just _ensure_release_dir
    
    echo "📦 Building release for {{platform}} with Dagger..."
    platform_name=$(echo "{{platform}}" | tr '/' '-')
    dagger call build-release --source . --platform {{platform}} export --path ./release-artifacts/just-mcp-{{version}}-${platform_name}
    
    just _success "Release build for {{platform}} complete"

# =====================================
# Release Packaging and Utilities
# =====================================

# Package release artifacts for all built targets
_package_release_artifacts version:
    #!/usr/bin/env bash
    set -euo pipefail
    
    echo "📦 Packaging release artifacts for version {{version}}..."
    
    # Required files to include in releases
    INCLUDE_FILES="README.md LICENSE"
    
    # Check if required files exist
    for file in $INCLUDE_FILES; do
        if [ ! -f "$file" ]; then
            just _warn "Missing file: $file (will be skipped in release)"
        fi
    done
    
    # Package each target
    for target in {{_all_release_targets}}; do
        if [ -f "target/$target/release/just-mcp" ]; then
            echo "📦 Packaging $target..."
            
            tar czf "./release-artifacts/just-mcp-{{version}}-$target.tar.gz" \
                -C "target/$target/release" just-mcp \
                $(for file in $INCLUDE_FILES; do [ -f "$file" ] && echo "-C $(pwd) $file"; done)
            
            # Create checksum
            (cd ./release-artifacts && sha256sum "just-mcp-{{version}}-$target.tar.gz" > "just-mcp-{{version}}-$target.tar.gz.sha256")
        else
            just _warn "Binary not found for target: $target (skipping)"
        fi
    done

# Package single target release
_package_single_target target version:
    #!/usr/bin/env bash
    set -euo pipefail
    
    if [ ! -f "target/{{target}}/release/just-mcp" ]; then
        just _error "Packaging" "Binary not found for target: {{target}}"
    fi
    
    echo "📦 Packaging {{target}} release..."
    
    tar czf "./release-artifacts/just-mcp-{{version}}-{{target}}.tar.gz" \
        -C "target/{{target}}/release" just-mcp \
        $(for file in README.md LICENSE; do [ -f "$file" ] && echo "-C $(pwd) $file"; done)
    
    # Create checksum
    (cd ./release-artifacts && sha256sum "just-mcp-{{version}}-{{target}}.tar.gz" > "just-mcp-{{version}}-{{target}}.tar.gz.sha256")

# Show release summary and artifacts
_show_release_summary:
    #!/usr/bin/env bash
    echo ""
    echo "📦 Release artifacts:"
    if [ -d "./release-artifacts" ] && [ "$(ls -A ./release-artifacts 2>/dev/null)" ]; then
        ls -la ./release-artifacts/
        echo ""
        echo "📊 Release summary:"
        find ./release-artifacts -name "*.tar.gz" | wc -l | xargs echo "  Archives created:"
        find ./release-artifacts -name "*.sha256" | wc -l | xargs echo "  Checksums created:"
        
        # Calculate total size
        total_size=$(find ./release-artifacts -name "*.tar.gz" -exec ls -l {} \; | awk '{sum += $5} END {printf "%.2f MB", sum/1024/1024}')
        echo "  Total size: $total_size"
    else
        echo "  No artifacts found in ./release-artifacts/"
    fi

# =====================================
# Release Management Commands
# =====================================

# Clean all release artifacts
[group('release')]
release-clean:
    @echo "🧹 Cleaning release artifacts..."
    @rm -rf ./release-artifacts || true
    @rm -f ./tarpaulin-report.html || true
    @just _success "Release artifacts cleaned"

# Show release information and available targets
[group('release')]
release-targets:
    @echo "🚀 Release Build Information"
    @echo ""
    @echo "Available Zigbuild Targets:"
    @echo "{{_valid_zigbuild_targets}}" | tr ' ' '\n' | sed 's/^/  - /'
    @echo ""
    @echo "Available Dagger Platforms:"
    @echo "{{_release_platforms}}" | tr ' ' '\n' | sed 's/^/  - /'
    @echo ""
    @echo "Commands:"
    @echo "  zigbuild-release VERSION  - Build all platforms with zigbuild"
    @echo "  zigbuild-target TARGET    - Build specific target with zigbuild"
    @echo "  zigbuild-test TARGET      - Test zigbuild for single target"
    @echo "  dagger-release VERSION      - Build all platforms with Dagger"
    @echo "  dagger-release-platform     - Build specific platform with Dagger"
    @echo "  release-clean             - Clean release artifacts"
    @echo "  release-targets           - Show available targets and platforms"
    @echo ""
    @echo "Version format: v1.2.3, v1.2.3-alpha, v1.2.3-beta.1, v1.2.3+build.123"

# Validate release prerequisites
[group('release')]
release-check:
    @echo "🔍 Checking release prerequisites..."
    @just _require-command docker
    @echo "🔍 Checking Docker daemon..."
    @docker info > /dev/null 2>&1 || just _error "Docker check" "Docker daemon is not running"
    @just _require-file "Cargo.toml" "Release check"
    @just _require-file "README.md" "Release check"
    @just _require-file "LICENSE" "Release check"
    @just _success "Release prerequisites validated"

# Create a full release (build + package + validate)
[group('release')]
release version="v0.1.0" method="zigbuild":
    #!/usr/bin/env bash
    set -euo pipefail
    
    just _validate_version "{{version}}"
    just _validate "{{method}}" "zigbuild dagger"
    
    echo "🚀 Creating full release {{version}} using {{method}}..."
    
    # Pre-flight checks
    just release-check
    
    # Clean previous artifacts
    just release-clean
    
    # Build release
    case "{{method}}" in
        "zigbuild")
            just zigbuild-release "{{version}}"
            ;;
        "dagger")
            just dagger-release "{{version}}"
            ;;
    esac
    
    echo ""
    just _success "Release {{version}} created successfully using {{method}}!"
    echo "📝 Next steps:"
    echo "  1. Test the release artifacts"
    echo "  2. Create git tag: git tag {{version}}"
    echo "  3. Push tag: git push origin {{version}}"
    echo "  4. Create GitHub release with artifacts in ./release-artifacts/"