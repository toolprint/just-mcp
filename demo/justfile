# Just-MCP Demo Justfile
# This file demonstrates various justfile features that can be exposed as MCP tools

# Default recipe - shows available commands
default:
    @just --list

# {{name}}: person or thing to greet
# Simple greeting task
hello name="World":
    @echo "Hello, {{name}}!"

# System information task
system-info:
    @echo "=== System Information ==="
    @echo "OS: $(uname -s)"
    @echo "Architecture: $(uname -m)"
    @echo "Hostname: $(hostname)"
    @echo "Current directory: $(pwd)"
    @echo "Current user: $(whoami)"

# {{target}}: the build target mode (debug, release, optimized)
# Build simulation with different targets
build target="debug":
    @echo "Building project in {{target}} mode..."
    @sleep "1"
    @echo "✓ Compilation complete"
    @echo "✓ Binary created at: ./target/{{target}}/app"

# {{filter}}: test name pattern to filter tests (e.g., "unit", "integration")
# Run tests with optional filter
test filter="":
    @echo "Running tests..."
    @sleep "1"
    @echo "✓ All tests passed!"

# {{direction}}: migration direction (up to apply, down to rollback)
# Database operations
db-migrate direction="up":
    @echo "Running database migrations {{direction}}..."
    @echo "✓ Migration successful"

# {{count}}: number of records to seed
[doc("Seed the database with sample data")]
db-seed count="10":
    @echo "Seeding database with {{count}} records..."
    @echo "✓ Database seeded"

# {{environment}}: target deployment environment (staging, production, dev)
# Deployment simulation
deploy environment="staging":
    @echo "Deploying to {{environment}} environment..."
    @echo "📦 Building application..."
    @sleep "1"
    @echo "🚀 Uploading to {{environment}} server..."
    @sleep "1"
    @echo "✅ Deployment complete!"

# {{name}}: name of the configuration file to create
# {{template}}: template to use for the configuration
# File operations
create-config name template="default":
    @echo "Creating configuration file: {{name}}"
    @echo "Using template: {{template}}"
    @mkdir -p configs
    @echo "# Configuration: {{name}}" > configs/{{name}}.conf
    @echo "# Template: {{template}}" >> configs/{{name}}.conf
    @echo "# Created: $(date)" >> configs/{{name}}.conf
    @echo "✓ Configuration created at: configs/{{name}}.conf"

# Cleanup task
clean:
    @echo "Cleaning project..."
    @rm -rf target/
    @rm -rf configs/
    @echo "✓ Cleanup complete"

# Complex task with dependencies
full-deploy: test build deploy
    @echo "Full deployment pipeline complete!"

# {{input}}: input file path to process
# {{output}}: output file path for results
# Task that processes input data
process-data input output="output.json":
    @echo "Processing data from {{input}} to {{output}}..."
    @echo "{\"processed\": true, \"input\": \"{{input}}\", \"timestamp\": \"$(date)\"}" > {{output}}
    @echo "✓ Data processed and saved to {{output}}"

# Interactive task simulation
interactive-setup:
    @echo "Starting interactive setup..."
    @echo "1. Checking prerequisites..."
    @sleep "1"
    @echo "   ✓ Git installed"
    @echo "   ✓ Node.js installed"
    @echo "   ✓ Docker running"
    @echo "2. Creating project structure..."
    @mkdir -p src tests docs
    @echo "   ✓ Directories created"
    @echo "3. Initializing configuration..."
    @echo "   ✓ Config initialized"
    @echo "Setup complete! 🎉"

# {{service}}: name of the service to monitor
# {{interval}}: monitoring interval in seconds
# Monitoring task
monitor service="web" interval="5":
    @echo "Monitoring {{service}} service (interval: {{interval}}s)..."
    @echo "Press Ctrl+C to stop"
    @echo "[$(date)] {{service}} is running ✓"

# {{action}}: version action to perform (show, bump)
# Version management
version action="show":
    #!/bin/bash
    if [[ "{{action}}" == "show" ]]; then
        echo "Current version: 1.0.0"
    elif [[ "{{action}}" == "bump" ]]; then
        echo "Version bumped to: 1.0.1"
    else
        echo "Unknown action: {{action}}"
    fi

# {{tag}}: Docker image tag to use
# Docker operations
docker-build tag="latest":
    @echo "Building Docker image with tag: {{tag}}..."
    @echo "FROM ubuntu:latest" > Dockerfile.tmp
    @echo "✓ Docker image built: myapp:{{tag}}"
    @rm -f Dockerfile.tmp

# {{registry}}: Docker registry URL to push to
[doc("Push Docker image to registry")]
docker-push registry="docker.io":
    @echo "Pushing image to {{registry}}..."
    @echo "✓ Image pushed successfully"

# {{endpoint}}: API endpoint URL to test
# {{method}}: HTTP method to use (GET, POST, PUT, DELETE)
# {{data}}: request body data (JSON format)
# API testing
api-test endpoint method="GET" data="":
    @echo "Testing API endpoint: {{endpoint}}"
    @echo "Method: {{method}}"
    @if [[ -n "{{data}}" ]]; then echo "Data: {{data}}"; fi
    @echo "Response: {\"status\": 200, \"message\": \"OK\"}"

# {{destination}}: directory path where backups will be stored
# Backup operations
backup destination="./backups":
    @echo "Creating backup..."
    @mkdir -p {{destination}}
    @echo "Backup created at $(date)" > {{destination}}/backup-$(date +%Y%m%d-%H%M%S).txt
    @echo "✓ Backup saved to {{destination}}"

# {{iterations}}: number of benchmark iterations to run
# Performance testing
benchmark iterations="1000":
    @echo "Running performance benchmark ({{iterations}} iterations)..."
    @echo "🏃 Running tests..."
    @sleep "2"
    @echo "📊 Results:"
    @echo "   Average response time: 23ms"
    @echo "   Throughput: 435 req/s"
    @echo "   Success rate: 99.9%"

# {{action}}: action to perform on secrets (list, rotate)
# Secret management (simulation)
secrets action="list":
    @if [[ "{{action}}" == "list" ]]; then \
        echo "Available secrets:"; \
        echo "  - DATABASE_URL"; \
        echo "  - API_KEY"; \
        echo "  - JWT_SECRET"; \
    elif [[ "{{action}}" == "rotate" ]]; then \
        echo "Rotating secrets..."; \
        echo "✓ Secrets rotated successfully"; \
    fi

# Health check
health-check:
    @echo "Performing health checks..."
    @echo "✓ Database: Connected"
    @echo "✓ Cache: Available"
    @echo "✓ External APIs: Reachable"
    @echo "✓ Disk space: 45% used"
    @echo "Overall status: HEALTHY 💚"

# {{level}}: log level to filter (error, warn, info, debug)
# {{limit}}: maximum number of log entries to display
# Log analysis
analyze-logs level="error" limit="10":
    @echo "Analyzing logs (level: {{level}}, limit: {{limit}})..."
    @echo "Found 3 {{level}} entries:"
    @echo "[2024-01-15 10:23:45] {{level}}: Connection timeout"
    @echo "[2024-01-15 11:45:12] {{level}}: Invalid request format"
    @echo "[2024-01-15 14:32:08] {{level}}: Database query failed"

# {{format}}: documentation format (markdown, html, pdf)
# Generate documentation
docs format="markdown":
    @echo "Generating documentation in {{format}} format..."
    @mkdir -p docs
    @echo "# Project Documentation" > docs/README.{{format}}
    @echo "Generated on: $(date)" >> docs/README.{{format}}
    @echo "✓ Documentation generated in docs/"
