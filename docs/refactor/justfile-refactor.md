# Justfile Refactoring Plan - just-mcp

## Goaly Goal ID: 14

## Overview

This document analyzes the current just-mcp justfile against the [Best Practices for Modular Justfiles](../../assets/docs/JUSTFILE_BEST_PRACTICES.md) guide and provides a comprehensive refactoring plan to improve maintainability, reduce duplication, and enhance developer experience.

## Current State Analysis

### 📊 Statistics

- **File size**: 870 lines (exceeds 200-line guideline by 435%)
- **Recipe count**: 50+ recipes across 8 functional areas
- **Code duplication**: Binary parsing logic repeated 3+ times
- **Complexity**: Monolithic structure with mixed concerns

### 🚨 Issues Identified

#### 1. Monolithic Structure

- Single 870-line file violates modularization guidelines
- Mixed concerns: Rust builds, Docker operations, vector search, git operations
- Difficult to navigate and maintain
- No separation of responsibilities

#### 2. Inconsistent Naming Conventions

```just
# Current inconsistent patterns:
git-status         # ✅ Good: action-object
test               # ❌ Poor: missing object
build              # ❌ Poor: missing object  
demo-vector-search # ✅ Good: domain-action-object
pre-commit         # ❌ Poor: wrong group assignment
```

#### 3. Code Duplication

Binary parsing logic duplicated across:

- `release-info` (lines 101-120)
- `install` (lines 164-181)
- `install-with-vector-search` (lines 247-264)

#### 4. Poor Default Experience

```just
# Current default recipe
_default:
    @just -l -u
```

- No comprehensive help
- No discovery guidance
- Underscore prefix violates conventions

#### 5. Missing Common Utilities

- No shared error handling patterns
- No parameter validation helpers
- No consistent messaging system
- No command requirement checking

#### 6. Inconsistent Parameter Handling

```just
# Mixed validation patterns:
deploy environment="staging":     # No validation
zigbuild-test target="...":      # No validation  
demo-vector-search:              # No parameters despite needing them
```

#### 7. Group Organization Issues

```just
[group('format')]
pre-commit:    # Should be in 'quality' or 'ci' group
```

## Proposed Modular Structure

### 🏗️ Directory Layout

```
just-mcp/
├── justfile              # Main orchestrator (simplified)
└── just/                 # Module directory
    ├── common.just       # Shared utilities and helpers
    ├── rust.just         # Rust-specific recipes (build, test, lint)
    ├── vector.just       # Vector search demo recipes
    ├── docker.just       # Dagger/Docker operations
    ├── git.just          # Git operations 
    ├── setup.just        # Installation and setup
    └── release.just      # Release and deployment
```

### 📋 Module Responsibilities

#### `common.just` - Shared Utilities

```just
# Error handling
_error context message:
    #!/usr/bin/env bash
    echo "❌ {{context}}: {{message}}" >&2
    exit 1

# Success messages
_success message:
    @echo "✅ {{message}}"

# Parameter validation
_validate choice valid_options:
    #!/usr/bin/env bash
    if [[ ! " {{valid_options}} " =~ " {{choice}} " ]]; then
        just _error "validation" "Invalid option: '{{choice}}'. Valid: {{valid_options}}"
    fi

# Command requirements
_require-command command:
    #!/usr/bin/env bash
    if ! command -v {{command}} &> /dev/null; then
        just _error "dependency" "{{command}} is required but not installed"
    fi

# Consistent command execution
_run description command:
    @echo "🔄 {{description}}..."
    @{{command}}
    @echo "✅ {{description}} complete"

# Binary parsing utility (eliminate duplication)
_get-binaries:
    #!/usr/bin/env bash
    if command -v tq >/dev/null 2>&1 && command -v jq >/dev/null 2>&1; then
        tq -o json -f Cargo.toml 'bin' 2>/dev/null | jq -r '.[].name' 2>/dev/null | tr '\n' ' '
    elif command -v tq >/dev/null 2>&1; then
        bin_json=$(tq -o json -f Cargo.toml 'bin' 2>/dev/null)
        echo "$bin_json" | sed 's/.*"name":"\([^"]*\)".*/\1/g' | tr '\n' ' '
    else
        awk '/^\[\[bin\]\]/ { in_bin=1; next } /^\[/ { in_bin=0 } in_bin && /^name = / { gsub(/^name = "|"$/, ""); print }' Cargo.toml | tr '\n' ' '
    fi
```

#### `rust.just` - Rust Development

```just
# Import common utilities
import '../common.just'

# Rust tool detection
_cargo := "cargo"
_rustfmt := "rustfmt"

[group: 'rust']
setup-rust:
    @just _require-command cargo
    @just _run "Installing cargo-tarpaulin" "cargo install cargo-tarpaulin"
    @just _success "Rust development environment ready"

[group: 'rust']
build-rust mode="debug":
    @just _validate "{{mode}}" "debug release"
    @just _run "Building Rust project ({{mode}})" "cargo build{{if mode == "release" { " --release" } else { "" }}}"

[group: 'rust']
test-rust coverage="false":
    #!/usr/bin/env bash
    if [ "{{coverage}}" = "true" ]; then
        just _run "Running tests with coverage" "cargo tarpaulin --out Html"
    else
        just _run "Running tests" "cargo test"
    fi

[group: 'rust']
format-rust:
    @just _run "Formatting Rust code" "cargo fmt"

[group: 'rust']
lint-rust fix="false":
    #!/usr/bin/env bash
    if [ "{{fix}}" = "true" ]; then
        just _run "Auto-fixing Rust issues" "cargo clippy --fix --allow-dirty"
    else
        just _run "Linting Rust code" "cargo clippy -- -D warnings"
    fi

[group: 'rust']
install-rust features="":
    #!/usr/bin/env bash
    binaries=$(just _get-binaries)
    if [ -z "$binaries" ]; then
        just _error "install" "No binaries found in Cargo.toml"
    fi
    
    feature_flag=""
    if [ -n "{{features}}" ]; then
        feature_flag='--features "{{features}}"'
    fi
    
    just _run "Installing Rust binaries" "cargo install --path . --force $feature_flag"
    just _success "Installation complete for: $binaries"
```

#### Main `justfile` - Orchestrator

```just
# Import all modules
import 'just/common.just'
import 'just/rust.just'
import 'just/vector.just'
import 'just/docker.just'
import 'just/git.just'
import 'just/setup.just'
import 'just/release.just'

# Default recipe - comprehensive help
default:
    @just help

# Comprehensive help system
[group: 'help']
help:
    #!/usr/bin/env bash
    echo "🚀 just-mcp Development Commands"
    echo "================================"
    echo ""
    echo "🎯 QUICK START:"
    echo "  just quickstart    - Complete setup for new developers"
    echo "  just dev-setup     - Setup development environment"
    echo "  just ci            - Run all CI checks"
    echo ""
    echo "🔧 DEVELOPMENT:"
    echo "  just build [mode]  - Build project (debug|release)"
    echo "  just test [opts]   - Run tests"
    echo "  just format        - Format all code"
    echo "  just lint          - Lint all code"
    echo ""
    echo "🔍 DISCOVERY & NAVIGATION:"
    echo "  just --list        - List all recipes organized by groups"
    echo "  just --groups      - List all recipe groups"
    echo "  just --show <name> - Show recipe source code"
    echo "  just --choose      - Interactive recipe picker"
    echo ""
    echo "💡 TIPS:"
    echo "  Use 'just --show <recipe>' to see how recipes work"
    echo "  Recipes are organized by logical groups"

# Quick workflows
quickstart: setup-all install-all
    @just _success "Project setup complete!"

dev-setup: setup-rust setup-tools
    @just _success "Development environment ready!"

ci: format-all lint-all test-all
    @just _success "CI pipeline complete!"

# Unified commands (delegate to modules)
build mode="debug": (build-rust mode)
test coverage="false": (test-rust coverage)
format: format-rust format-other
lint fix="false": (lint-rust fix) (lint-other fix)

# Convenience wrappers for native Just commands
[group: 'help']
groups:
    @just --groups

[group: 'help']
list:
    @just --list
```

## Implementation Plan

### Phase 1: Create Infrastructure

1. **Create `just/` directory**
2. **Create `just/common.just`** with shared utilities
3. **Test common utilities** with simple recipes

### Phase 2: Extract Core Modules

1. **Create `just/rust.just`**
   - Move: `build`, `test`, `format`, `lint`, `install*` recipes
   - Apply consistent naming: `build-rust`, `test-rust`, etc.
   - Eliminate binary parsing duplication
2. **Create `just/setup.just`**
   - Move: `setup`, `brew`, `doppler-install` recipes
3. **Create `just/git.just`**
   - Move: `git-*`, `sync-submodules` recipes

### Phase 3: Extract Feature Modules  

1. **Create `just/vector.just`**
   - Move: All `demo-vector-*` recipes
   - Apply consistent naming and grouping
2. **Create `just/docker.just`**
   - Move: All `dagger-*` recipes
3. **Create `just/release.just`**
   - Move: `zigbuild-*`, `release-*` recipes

### Phase 4: Redesign Main Justfile

1. **Replace `_default`** with comprehensive `help`
2. **Add unified commands** that delegate to modules
3. **Add workflow recipes**: `quickstart`, `dev-setup`, `ci`
4. **Add convenience wrappers**: `groups`, `list`

### Phase 5: Quality & Consistency

1. **Standardize naming** throughout all modules
2. **Add parameter validation** using common utilities
3. **Update recipe groups** for logical organization
4. **Add comprehensive documentation**

## Migration Strategy

### Backward Compatibility

- **Preserve all existing recipes** initially
- **Add aliases** for renamed recipes during transition
- **Gradual deprecation** with helpful messages

### Testing Approach

- **Validate all imports** work correctly
- **Test unified commands** delegate properly
- **Verify all existing functionality** still works
- **Test new help and discovery features**

### Rollout Plan

1. **Branch creation**: `feature/modular-justfile`
2. **Incremental implementation** following phases
3. **Testing at each phase** before proceeding
4. **Documentation updates** alongside code changes
5. **Review and merge** after complete testing

## Expected Improvements

### Developer Experience

- **90% reduction** in time to find relevant recipes
- **Clear help system** with progressive disclosure
- **Better organization** with logical grouping
- **Interactive discovery** with `--choose`

### Maintainability

- **Eliminate duplication**: Single binary parsing utility
- **Clear responsibilities**: Each module owns its domain
- **Easier testing**: Isolated, focused modules
- **Simpler changes**: Modify only relevant modules

### Code Quality

- **Consistent naming**: `action-object` pattern throughout
- **Parameter validation**: Early validation with helpful errors
- **Error handling**: Consistent messaging and exit codes
- **Documentation**: Self-documenting with clear help

## Success Metrics

### Quantitative

- **File size reduction**: Main justfile < 200 lines
- **Duplication elimination**: 0 repeated code blocks
- **Recipe organization**: 100% properly grouped
- **Test coverage**: All recipes tested and validated

### Qualitative  

- **New developer onboarding**: Faster understanding
- **Recipe discovery**: Easier to find relevant commands
- **Maintenance burden**: Reduced complexity for changes
- **Code consistency**: Uniform patterns throughout

## Task Tracking

### Goaly Task Breakdown

This refactor has been broken down into 24 discrete tasks in Goaly (Goal ID: 14):

### Phase 1: Infrastructure (Task IDs: 79-81)

- Task 79: Create just/ directory structure
- Task 80: Create common.just utilities module
- Task 81: Test common utilities functionality

### Phase 2: Core Modules (Task IDs: 82-84)

- Task 82: Create rust.just module
- Task 83: Create setup.just module  
- Task 84: Create git.just module

### Phase 3: Feature Modules (Task IDs: 85-87)

- Task 85: Create vector.just module
- Task 86: Create docker.just module
- Task 87: Create release.just module

### Phase 4: Main Justfile (Task IDs: 88-91)

- Task 88: Design comprehensive help system
- Task 89: Create unified delegation commands
- Task 90: Create workflow recipes
- Task 91: Add convenience wrapper recipes

### Phase 5: Quality & Consistency (Task IDs: 92-95)

- Task 92: Standardize naming conventions
- Task 93: Add parameter validation throughout
- Task 94: Update recipe groups organization
- Task 95: Add comprehensive module documentation

### Migration & Testing (Task IDs: 96-102)

- Task 96: Create backward compatibility aliases
- Task 97: Validate all imports work correctly
- Task 98: Test unified command delegation
- Task 99: Verify all existing functionality
- Task 100: Test new help and discovery features
- Task 101: Update project documentation
- Task 102: Measure and validate improvements

### Task Dependencies

Tasks are organized with proper dependencies to ensure logical workflow:

- Infrastructure tasks (79-81) must complete first
- Core modules (82-84) depend on common utilities
- Feature modules (85-87) depend on core modules
- Main justfile work (88-91) depends on all modules being available
- Quality work (92-95) refines the complete structure
- Testing and validation (96-102) ensures everything works

Each task is atomic and can be assigned to individual contributors while maintaining proper sequencing through the dependency system.

## Conclusion

This refactoring transforms the just-mcp justfile from a monolithic 870-line script into a well-organized, modular system that exemplifies Just best practices. The result will be more maintainable, discoverable, and extensible while preserving all existing functionality.

The modular structure provides clear separation of concerns, eliminates code duplication, and creates a foundation for future growth. New contributors will find the system approachable, and existing workflows will benefit from improved consistency and reliability.
