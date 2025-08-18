# Best Practices for Modular Justfiles in Polyglot Repositories

This guide provides best practices for creating maintainable, modular justfile structures for complex repositories with multiple programming languages and build systems.

## Table of Contents

1. [Modular Architecture](#modular-architecture)
2. [Naming Conventions](#naming-conventions)
3. [Reusable Templates](#reusable-templates)
4. [Cross-Language Consistency](#cross-language-consistency)
5. [Parameter Handling](#parameter-handling)
6. [Workflow Recipes](#workflow-recipes)
7. [Documentation](#documentation)
8. [Error Handling](#error-handling)
9. [Examples](#examples)

## Modular Architecture

### Core Principles

1. **Separation of Concerns**: Each module should handle a specific domain
2. **Single Source of Truth**: Avoid duplicating logic across modules
3. **Clear Hierarchy**: Main justfile orchestrates, modules implement
4. **Progressive Disclosure**: Common tasks in main file, specialized in modules

### Recommended Module Structure

```
project/
├── justfile              # Main orchestrator
└── just/                 # Module directory
    ├── common.just       # Shared utilities and helpers
    ├── dev.just          # Development workflows
    ├── quality.just      # Code quality (lint, format, type-check)
    ├── test.just         # Testing recipes
    ├── build.just        # Build and compilation
    ├── deploy.just       # Deployment and release
    ├── docker.just       # Container operations
    ├── database.just     # Database management
    ├── docs.just         # Documentation generation
    ├── utils.just        # Miscellaneous utilities
    └── legacy.just       # Backward compatibility (optional)
```

### Main Justfile Pattern

```just
# Project Name - Main Development Tasks
# Run 'just' to see available commands

# Import modular justfiles
import 'just/common.just'
import 'just/dev.just'
import 'just/quality.just'
import 'just/test.just'
import 'just/build.just'
import? 'just/legacy.just'  # Optional imports with '?'

# Default recipe - show detailed help
default:
    @just help

# Quick start workflows
quickstart: setup install
    @echo "✅ Project setup complete!"

# Pre-commit checks
pre-commit: format lint type-check test-fast
    @echo "✅ All checks passed!"

# Delegate to modules with consistent interface
build target="all":
    @just build-{{target}}

test type="all":
    @just test-{{type}}
```

## Naming Conventions

### Recipe Naming

1. **Action-Object Pattern**: `verb-noun` (e.g., `build-frontend`, `test-backend`)
2. **Hierarchical Naming**: Use hyphens for hierarchy (e.g., `db-migrate`, `docker-build`)
3. **Consistent Prefixes**: Group related commands with common prefixes

```just
# Good naming examples
format-python      # Action: format, Object: python code
lint-frontend      # Action: lint, Object: frontend
db-migrate         # Domain: db, Action: migrate
docker-build-prod  # Domain: docker, Action: build, Target: prod

# Avoid
fmt          # Too abbreviated
pythonFormat # camelCase inconsistent
format_py    # Underscores instead of hyphens
```

### Module Naming

- Use descriptive, lowercase names
- Single word when possible (e.g., `test.just`, `build.just`)
- Compound words with underscores (e.g., `code_quality.just`)

### Variable Naming

```just
# Parameters: lowercase with underscores
test type="all" coverage="false":
    @just _run-tests {{type}} {{coverage}}

# Internal variables: UPPERCASE
_build-app:
    #!/usr/bin/env bash
    BUILD_DIR="dist"
    VERSION=$(git describe --tags --always)
```

## Reusable Templates

### Common Utilities Module

Create a `common.just` with reusable helpers:

```just
# common.just - Shared utilities

# Validate choices
_validate choice valid_options:
    #!/usr/bin/env bash
    if [[ ! " {{valid_options}} " =~ " {{choice}} " ]]; then
        echo "❌ Invalid option: '{{choice}}'"
        echo "   Valid options: {{valid_options}}"
        exit 1
    fi

# Consistent error handling
_error context message:
    #!/usr/bin/env bash
    echo "❌ {{context}}: {{message}}" >&2
    exit 1

# Success messages
_success message:
    @echo "✅ {{message}}"

# Info messages
_info message:
    @echo "ℹ️  {{message}}"

# Warning messages
_warn message:
    @echo "⚠️  {{message}}"

# Check command exists
_require-command command:
    #!/usr/bin/env bash
    if ! command -v {{command}} &> /dev/null; then
        echo "❌ {{command}} is required but not installed"
        exit 1
    fi

# Run with pretty output
_run description command:
    @echo "🔄 {{description}}..."
    @{{command}}
    @echo "✅ {{description}} complete"
```

### Language-Specific Templates

#### Python Module Template

```just
# python.just - Python-specific recipes

# Python tool detection
_python := if path_exists(".venv/bin/python") == "true" { ".venv/bin/python" } else { "python" }
_pip := if path_exists(".venv/bin/pip") == "true" { ".venv/bin/pip" } else { "pip" }

# Setup Python environment
[group: 'python']
setup-python version="3.11":
    @just _require-command python{{version}}
    @just _run "Creating virtual environment" "python{{version}} -m venv .venv"
    @just _run "Upgrading pip" "{{_pip}} install --upgrade pip"

# Install dependencies
[group: 'python']
install-python requirements="requirements.txt":
    @just _run "Installing Python dependencies" "{{_pip}} install -r {{requirements}}"

# Format Python code
[group: 'python']
format-python:
    @just _run "Formatting Python code" "{{_python}} -m black src tests"
    @just _run "Sorting imports" "{{_python}} -m isort src tests"

# Lint Python code
[group: 'python']
lint-python fix="false":
    #!/usr/bin/env bash
    if [ "{{fix}}" = "true" ]; then
        just _run "Auto-fixing Python issues" "{{_python}} -m ruff check --fix src tests"
    else
        just _run "Linting Python code" "{{_python}} -m ruff check src tests"
    fi

# Type check Python
[group: 'python']
type-check-python:
    @just _run "Type checking Python" "{{_python}} -m mypy src"

# Test Python code
[group: 'python']
test-python coverage="false":
    #!/usr/bin/env bash
    cmd="{{_python}} -m pytest"
    if [ "{{coverage}}" = "true" ]; then
        cmd="$cmd --cov=src --cov-report=html --cov-report=term"
    fi
    just _run "Running Python tests" "$cmd"
```

#### Node.js Module Template

```just
# node.just - Node.js-specific recipes

# Package manager detection
_npm := if path_exists("pnpm-lock.yaml") == "true" { "pnpm" } else if path_exists("yarn.lock") == "true" { "yarn" } else { "npm" }

# Install Node dependencies
[group: 'node']
install-node:
    @just _run "Installing Node dependencies" "{{_npm}} install"

# Format JavaScript/TypeScript
[group: 'node']
format-node:
    @just _run "Formatting JS/TS code" "{{_npm}} run format"

# Lint JavaScript/TypeScript
[group: 'node']
lint-node fix="false":
    #!/usr/bin/env bash
    if [ "{{fix}}" = "true" ]; then
        just _run "Auto-fixing JS/TS issues" "{{_npm}} run lint:fix"
    else
        just _run "Linting JS/TS code" "{{_npm}} run lint"
    fi

# Type check TypeScript
[group: 'node']
type-check-node:
    @just _run "Type checking TypeScript" "{{_npm}} run type-check"

# Test Node.js code
[group: 'node']
test-node watch="false":
    #!/usr/bin/env bash
    if [ "{{watch}}" = "true" ]; then
        just _run "Running tests in watch mode" "{{_npm}} run test:watch"
    else
        just _run "Running Node tests" "{{_npm}} run test"
    fi

# Build Node.js project
[group: 'node']
build-node mode="production":
    @just _run "Building Node.js project" "{{_npm}} run build:{{mode}}"
```

## Cross-Language Consistency

### Unified Interface Pattern

Create consistent interfaces across languages:

```just
# Main justfile - Unified interface

# Format any code
format target="all":
    @just format-{{target}}

format-all: format-python format-node format-go
    @just _success "All code formatted"

# Lint any code
lint target="all" fix="false":
    @just lint-{{target}} {{fix}}

lint-all fix="false": (lint-python fix) (lint-node fix) (lint-go fix)
    @just _success "All code linted"

# Test any code
test target="all" args="":
    @just test-{{target}} {{args}}

test-all: test-python test-node test-go
    @just _success "All tests passed"

# Build any target
build target="all":
    @just build-{{target}}

build-all: build-python build-node build-go
    @just _success "All builds complete"
```

### Standard Actions Across Languages

Ensure these actions work consistently:

| Action | Python | Node.js | Go | Rust |
|--------|--------|---------|-----|------|
| setup | `python -m venv` | `npm install` | `go mod download` | `cargo fetch` |
| format | `black` | `prettier` | `gofmt` | `rustfmt` |
| lint | `ruff`/`flake8` | `eslint` | `golangci-lint` | `clippy` |
| type-check | `mypy` | `tsc` | Built-in | Built-in |
| test | `pytest` | `jest`/`mocha` | `go test` | `cargo test` |
| build | `pip install` | `npm run build` | `go build` | `cargo build` |

## Parameter Handling

### Parameter Validation

```just
# Validate parameters using common utilities
deploy env="staging" version="latest":
    @just _validate "{{env}}" "dev staging production"
    @just _deploy-{{env}} {{version}}

# Provide defaults with clear options
test type="unit" coverage="false" verbose="false":
    #!/usr/bin/env bash
    # Show available options in error messages
    case "{{type}}" in
        unit|integration|e2e|all) ;;
        *) just _error "test" "Invalid type '{{type}}'. Options: unit, integration, e2e, all" ;;
    esac
```

### Optional Parameters

```just
# Optional with default
build target="all" profile="release":
    @just _build-{{target}} --profile={{profile}}

# Optional without default (empty string)
release version="" message="":
    #!/usr/bin/env bash
    if [ -z "{{version}}" ]; then
        VERSION=$(git describe --tags --abbrev=0)
    else
        VERSION="{{version}}"
    fi
```

### Boolean Parameters

```just
# Use string "true"/"false" for consistency
test coverage="false" watch="false":
    #!/usr/bin/env bash
    if [ "{{coverage}}" = "true" ]; then
        COVERAGE_FLAGS="--cov=src --cov-report=html"
    fi
    if [ "{{watch}}" = "true" ]; then
        WATCH_FLAG="--watch"
    fi
```

## Workflow Recipes

### High-Level Workflows

Create recipes that combine multiple tasks:

```just
# Complete CI simulation
ci: lint type-check test build
    @just _success "CI pipeline complete"

# Release workflow
release version:
    @just test-all
    @just build-all
    @just _run "Creating git tag" "git tag -a v{{version}} -m 'Release v{{version}}'"
    @just _run "Building containers" "just docker-build production"
    @just _success "Release v{{version}} prepared"

# Daily development workflow
daily: 
    @just _run "Updating dependencies" "git pull"
    @just install-all
    @just db-migrate
    @just test-fast
    @just _info "Ready for development!"
```

### Progressive Workflows

```just
# Quick checks (fast)
check-fast: format lint
    @just _success "Fast checks complete"

# Standard checks (normal)
check: check-fast type-check test-fast
    @just _success "Standard checks complete"

# Full checks (slow)
check-all: check test-all
    @just _success "All checks complete"
```

### Environment-Specific Workflows

```just
# Development workflow
dev-setup: install-all db-init
    @just _run "Installing dev tools" "npm install -g @commitlint/cli"
    @just _info "Development environment ready"

# Production deployment
deploy-prod: test-all build-all
    @just _validate-prod-env
    @just docker-build production
    @just docker-push production
    @just _notify-deployment "production"
```

## Documentation

### Recipe Documentation

```just
# Clear, action-oriented descriptions
# Format: verb + object + context

# Build the application for production deployment
build-prod:
    @just _build --release --target=production

# Run integration tests with database fixtures
test-integration:
    @just _setup-test-db
    @just _run-tests integration
```

### Group Organization

```just
# Use groups for logical organization
[group: 'development']
dev:
    @just start-servers

[group: 'development']
dev-debug:
    @just start-servers --debug

[group: 'testing']
test:
    @just run-tests

[group: 'testing']
test-watch:
    @just run-tests --watch
```

### Help System

Use Just's native commands for discovery and navigation instead of custom parsing:

```just
# Comprehensive help as default command
[group: 'help']
help:
    #!/usr/bin/env bash
    PORT=$(just _get-port)
    HOST=$(just _get-host)
    echo "🚀 Project Development Commands"
    echo "==============================="
    echo ""
    echo "🎯 QUICK START:"
    echo "  just quickstart    - Complete setup for new developers"
    echo "  just start         - Start full dev environment"
    echo "  just pre-commit    - Run all checks before committing"
    echo ""
    echo "🔧 DEVELOPMENT:"
    echo "  just dev [mode]    - Start dev server"
    echo "  just test [type]   - Run tests"
    echo "  just build         - Build project"
    echo ""
    echo "🔍 DISCOVERY & NAVIGATION:"
    echo "  just --list              - List all recipes organized by groups"
    echo "  just --groups            - List all recipe groups"
    echo "  just --summary           - Compact list of recipe names only"
    echo "  just --show <recipe>     - Show recipe source code"
    echo "  just --choose            - Interactive recipe picker (requires fzf)"
    echo ""
    echo "💡 TIPS:"
    echo "  Use 'just --show <recipe>' to see how recipes work"
    echo "  Recipes are organized by logical groups (dev, test, build, etc.)"

# Native Just command wrappers for convenience
[group: 'help']
groups:
    @just --groups

[group: 'help']
list:
    @just --list

# Context-specific help
[group: 'help']
help-docker:
    @echo "🐳 Docker Commands:"
    @echo "  just docker-build  - Build image"
    @echo "  just docker-run    - Run container"
    @echo "  just docker-push   - Push to registry"
    @echo ""
    @echo "💡 Use 'just --list' to see all Docker recipes"
```

### Inline Documentation

```just
# Database operations require confirmation for destructive actions
[confirm("This will DELETE all data. Continue?")]
db-reset:
    @just db-drop
    @just db-create
    @just db-migrate
```

## Error Handling

### Consistent Error Patterns

```just
# Early validation
deploy env:
    #!/usr/bin/env bash
    # Validate environment first
    just _validate "{{env}}" "staging production"
    
    # Check prerequisites
    just _require-command kubectl
    just _require-command helm
    
    # Verify configuration exists
    if [ ! -f "deploy/{{env}}/values.yaml" ]; then
        just _error "deploy" "Configuration not found for {{env}}"
    fi
    
    # Proceed with deployment
    just _run "Deploying to {{env}}" "helm upgrade --install app ./chart -f deploy/{{env}}/values.yaml"
```

### Error Recovery

```just
# Graceful fallbacks
build:
    #!/usr/bin/env bash
    # Try fast build first
    if ! just _try-build-incremental; then
        just _warn "Incremental build failed, trying full build"
        just build-clean
    fi

# Cleanup on failure
test-integration:
    #!/usr/bin/env bash
    # Setup
    just _setup-test-env
    
    # Run tests with cleanup
    if ! just _run-integration-tests; then
        just _cleanup-test-env
        exit 1
    fi
    
    # Cleanup on success too
    just _cleanup-test-env
```

### Status Checks

```just
# Pre-flight checks
start:
    @just _check-dependencies
    @just _check-ports
    @just _check-env-vars
    @just _start-services

# Health checks
_check-service name url:
    #!/usr/bin/env bash
    echo "Checking {{name}}..."
    for i in {1..30}; do
        if curl -sf "{{url}}/health" > /dev/null; then
            just _success "{{name}} is healthy"
            return 0
        fi
        sleep 1
    done
    just _error "{{name}}" "Failed to start"
```

## Examples

### Example: Full Python/Node Polyglot Structure

```
project/
├── justfile
├── just/
│   ├── common.just      # Shared utilities
│   ├── python.just      # Python-specific
│   ├── node.just        # Node-specific
│   ├── quality.just     # Unified quality checks
│   ├── test.just        # Unified testing
│   ├── docker.just      # Container ops
│   └── deploy.just      # Deployment
├── backend/             # Python service
│   ├── src/
│   ├── tests/
│   └── requirements.txt
└── frontend/            # Node.js app
    ├── src/
    ├── tests/
    └── package.json
```

Main justfile:
```just
# Import all modules
import 'just/common.just'
import 'just/python.just'
import 'just/node.just'
import 'just/quality.just'
import 'just/test.just'
import 'just/docker.just'
import 'just/deploy.just'

# Default - show comprehensive help
default:
    @just help

# Unified commands
install: install-python install-node
format: format-python format-node
lint: lint-python lint-node
test: test-python test-node
build: build-python build-node

# Quick workflows
dev:
    @just _run "Starting backend" "cd backend && python -m src.app" &
    @just _run "Starting frontend" "cd frontend && npm run dev" &
    @wait

ci: lint test build
    @just _success "CI complete"
```

### Example: Go/Rust Microservices

```just
# go.just - Go service recipes

[group: 'go']
setup-go:
    @just _run "Downloading Go dependencies" "go mod download"
    @just _run "Installing tools" "go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest"

[group: 'go']
build-go service="all":
    #!/usr/bin/env bash
    if [ "{{service}}" = "all" ]; then
        for service in services/*/; do
            just _run "Building $(basename $service)" "cd $service && go build -o ../../bin/$(basename $service)"
        done
    else
        just _run "Building {{service}}" "cd services/{{service}} && go build -o ../../bin/{{service}}"
    fi

[group: 'go']
test-go service="all" coverage="false":
    #!/usr/bin/env bash
    FLAGS=""
    if [ "{{coverage}}" = "true" ]; then
        FLAGS="-coverprofile=coverage.out"
    fi
    
    if [ "{{service}}" = "all" ]; then
        just _run "Testing all services" "go test ./... $FLAGS"
    else
        just _run "Testing {{service}}" "cd services/{{service}} && go test ./... $FLAGS"
    fi
```

### Example: Database Migration Module

```just
# database.just - Database management

# Database URL handling
_db_url := env_var_or_default("DATABASE_URL", "postgresql://localhost/myapp_dev")

[group: 'database']
db-create:
    @just _run "Creating database" "createdb {{_db_url}}"

[group: 'database']
db-migrate direction="up" steps="all":
    #!/usr/bin/env bash
    case "{{direction}}" in
        up)
            if [ "{{steps}}" = "all" ]; then
                just _run "Running all migrations" "migrate -database {{_db_url}} -path db/migrations up"
            else
                just _run "Running {{steps}} migrations" "migrate -database {{_db_url}} -path db/migrations up {{steps}}"
            fi
            ;;
        down)
            if [ "{{steps}}" = "all" ]; then
                just _error "db-migrate" "Cannot rollback all migrations. Specify number of steps."
            else
                just _run "Rolling back {{steps}} migrations" "migrate -database {{_db_url}} -path db/migrations down {{steps}}"
            fi
            ;;
        *)
            just _error "db-migrate" "Invalid direction: {{direction}}. Use 'up' or 'down'"
            ;;
    esac

[group: 'database']
db-seed env="development":
    @just _validate "{{env}}" "development staging"
    @just _run "Seeding {{env}} data" "go run cmd/seed/main.go --env={{env}}"

[group: 'database']
db-backup name="":
    #!/usr/bin/env bash
    TIMESTAMP=$(date +%Y%m%d_%H%M%S)
    if [ -z "{{name}}" ]; then
        FILENAME="backup_$TIMESTAMP.sql"
    else
        FILENAME="{{name}}_$TIMESTAMP.sql"
    fi
    just _run "Backing up database" "pg_dump {{_db_url}} > backups/$FILENAME"
    just _success "Backup saved to backups/$FILENAME"
```

## Best Practices Summary

1. **Start Simple**: Begin with a single justfile, modularize when it exceeds 200 lines
2. **Consistent Naming**: Use verb-noun pattern, lowercase with hyphens
3. **Validate Early**: Check parameters and prerequisites before executing
4. **Provide Feedback**: Use consistent messaging (✅ ❌ ℹ️ ⚠️ 🔄)
5. **Document Intent**: Focus on "why" and "what", not just "how"
6. **Design for Reuse**: Create templates for common patterns
7. **Fail Gracefully**: Provide helpful error messages and recovery options
8. **Version Control**: Track just modules like any other code
9. **Test Recipes**: Create test recipes for your just recipes
10. **Progressive Disclosure**: Simple tasks easy, complex tasks possible
11. **Use Native Commands**: Leverage Just's built-in `--list`, `--groups`, `--show`, `--summary`, `--choose` instead of custom parsing
12. **Help as Default**: Make `just help` the default command for better discoverability
13. **Wrapper Recipes**: Provide convenience recipes like `just list` and `just groups` for common operations

## Meta-Commands and Discovery

Modern justfiles should leverage Just's native discovery features rather than implementing custom parsing:

### Native Just Commands

Just provides excellent built-in commands for exploring recipes:

```bash
# Core discovery commands
just --list              # List all recipes organized by groups
just --groups            # List all recipe groups
just --summary           # Compact list of recipe names only
just --show <recipe>     # Show recipe source code
just --choose            # Interactive recipe picker (requires fzf)
```

### Convenience Wrappers

Provide short wrapper recipes for frequently used commands:

```just
[group: 'help']
groups:
    @just --groups

[group: 'help']
list:
    @just --list
```

### Avoid Custom Parsing

**Don't do this:**
```just
# BAD: Custom AWK parsing
group name="":
    #!/usr/bin/env bash
    just --list --list-heading '' | awk '/\[{{name}}\]/ { in_group=1; print; next }'
```

**Do this instead:**
```just
# GOOD: Use native commands and teach users proper Just patterns
help:
    @echo "🔍 DISCOVERY & NAVIGATION:"
    @echo "  just --list              - List all recipes organized by groups"
    @echo "  just --groups            - List all recipe groups"
    @echo "  just --show <recipe>     - Show recipe source code"
```

## Conclusion

Modular justfiles enable maintainable build automation for complex polyglot projects. By following these patterns, you can create a consistent, discoverable interface that scales with your project's growth while remaining approachable for new contributors.

**Key Principles:**
- Use Just's native features instead of reimplementing them
- Make `just help` comprehensive and the default command
- Organize recipes into logical groups
- Provide convenience wrappers for common operations
- Focus on discoverability and ease of use

Remember: The goal is to make common tasks trivial and complex tasks manageable. A well-structured justfile becomes the single entry point for all project operations, reducing cognitive load and improving developer experience.