# just-mcp

[![CI](https://github.com/onegrep/just-mcp/actions/workflows/dagger-ci.yml/badge.svg)](https://github.com/onegrep/just-mcp/actions/workflows/dagger-ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.88+-blue.svg)](https://www.rust-lang.org)
[![MCP](https://img.shields.io/badge/MCP-1.0-green.svg)](https://modelcontextprotocol.io/)
[![Vector Search](https://img.shields.io/badge/Vector%20Search-Optional-orange.svg)](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2)

> Transform justfiles into AI-accessible automation tools through the Model Context Protocol

**just-mcp** bridges [justfiles](https://just.systems/) and AI assistants by exposing justfile tasks as dynamically discoverable MCP tools.

This enables AI assistants to understand, explore, and execute a project's common development workflows similar to how a human would.

## Quick Start

1. Install [just](https://github.com/casey/just?tab=readme-ov-file#packages) - `cargo install just`
2. Install [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) - `cargo install cargo-binstall`
3. `cargo binstall --git https://github.com/toolprint/just-mcp just-mcp`

## Features

### 🔍 **Intelligent Justfile Discovery**

- Real-time monitoring with filesystem watching
- Multi-project support with optional naming (`--watch-dir path:name`)
- Dynamic tool generation from justfile tasks
- Hot reloading on justfile modifications

### 📝 **Comprehensive Justfile Parsing**

- AST-based parser using Tree-sitter for accurate syntax understanding
- Full syntax support: parameters, defaults, dependencies, comments, attributes
- Three-tier fallback system: AST → CLI → Regex for maximum compatibility
- Parameter documentation from `# {{param}}: description` comments
- Multiple parameter formats: `task(param)` and `task param`
- Doc attributes: `[doc("description")]` for enhanced documentation

### 🛡️ **Security & Resource Management**

- Input validation prevents command injection and path traversal
- Configurable timeouts, memory limits, and output size controls
- Directory whitelisting and parameter sanitization
- Shell escaping and strict validation modes

### ⚙️ **Administrative Tools**

- `admin_sync`: Manual justfile re-scanning and registry refresh
- `admin_create_task`: AI-assisted task creation with automatic backup
- Conflict prevention and task name validation
- Multi-directory targeting for task creation

### 🔍 **Semantic Vector Search** *(Optional)*

- **Offline-First**: Local embeddings with sentence-transformers (no API keys needed)
- **Natural Language**: Search tasks using everyday language like "deploy to production"
- **Cross-Project Discovery**: Find similar automation patterns across all your projects
- **Multiple Providers**: Local (Candle + ONNX), OpenAI, or Mock for testing
- **Smart Caching**: Models cached locally for instant subsequent searches

### 🚀 **MCP Protocol Compliance**

- Full JSON-RPC 2.0 MCP specification implementation
- Real-time tool list updates via notifications
- Standard capabilities: `tools/list`, `tools/call`, change notifications
- Comprehensive error reporting and validation

## Installation

### Prerequisites

- `just` command runner ([installation guide](https://just.systems/man/en/chapter_4.html))
- Rust 1.88.0 (only needed for building from source)

### Using cargo-binstall (Recommended)

The fastest way to install just-mcp is using [cargo-binstall](https://github.com/cargo-bins/cargo-binstall), which downloads pre-built binaries from GitHub releases.

```bash
# Install cargo-binstall if you haven't already
cargo install cargo-binstall

# Install just-mcp from GitHub releases
cargo binstall --git https://github.com/toolprint/just-mcp just-mcp
```

This will automatically download the appropriate pre-built binary for your platform from the GitHub releases.

### Pre-built Binaries

Download the latest release from [GitHub Releases](https://github.com/toolprint/just-mcp/releases/latest):

```bash
# Linux x86_64
curl -L https://github.com/toolprint/just-mcp/releases/latest/download/just-mcp-v0.2.0-x86_64-unknown-linux-gnu.tar.gz | tar xz

# Linux ARM64
curl -L https://github.com/toolprint/just-mcp/releases/latest/download/just-mcp-v0.2.0-aarch64-unknown-linux-gnu.tar.gz | tar xz

# macOS (Universal Binary - works on both Intel and Apple Silicon)
curl -L https://github.com/toolprint/just-mcp/releases/latest/download/just-mcp-v0.2.0-universal2-apple-darwin.tar.gz | tar xz
```

Then move the binary to your PATH:

```bash
sudo mv just-mcp /usr/local/bin/
# or
mv just-mcp ~/.local/bin/
```

### From Source

```bash
git clone https://github.com/toolprint/just-mcp.git
cd just-mcp
cargo install --path .

# With all features including AST parser (recommended)
cargo install --path . --features all

# With specific features
cargo install --path . --features "ast-parser"  # AST parser only
cargo install --path . --features "vector-search,local-embeddings"  # Vector search
```

### Using Just

```bash
just install  # Builds and installs to ~/.cargo/bin (includes all features)
```

### Development Setup

For contributing to just-mcp, install development dependencies:

```bash
# macOS users
just brew  # Installs prettier, markdownlint-cli2, and other tools

# All platforms
cargo install cargo-tarpaulin  # Coverage testing
npm install -g prettier markdownlint-cli2  # Code formatting

# Optional: Install Dagger for CI/CD workflows
curl -L https://dl.dagger.io/dagger/install.sh | sh
```

## Quick Start

### Basic Usage

Monitor current directory for justfiles:

```bash
just-mcp
```

Monitor specific directory:

```bash
just-mcp --watch-dir /path/to/project
```

### Multi-Project Setup

Monitor multiple projects with custom names:

```bash
just-mcp \
  --watch-dir ~/projects/api:backend \
  --watch-dir ~/projects/web:frontend \
  --watch-dir ~/projects/tools
```

### MCP Configuration

Add to your MCP settings file (e.g., `~/.config/mcp/settings.json`):

```json
{
  "mcpServers": {
    "just": {
      "command": "just-mcp",
      "args": ["--watch-dir", "/path/to/project"],
      "env": {}
    }
  }
}
```

## Usage

### Tool Naming Convention

Tools are exposed with the format:

- Single directory: `just_<task>`
- Named directories: `just_<task>@<name>`
- Multiple unnamed directories: `just_<task>_<full_path>`

### Example Workflow

Given a justfile:

```just
# Deploy the application
deploy env="prod":
  echo "Deploying to {{env}}"
  ./deploy.sh {{env}}

# Run tests with coverage
test coverage="false":
  cargo test {{if coverage == "true" { "--coverage" } else { "" }}}
```

The AI assistant can:

1. **Discover available tools**:

   ```json
   {
     "method": "tools/list",
     "result": {
       "tools": [
         {
           "name": "just_deploy",
           "description": "Deploy the application",
           "inputSchema": {
             "type": "object",
             "properties": {
               "env": {
                 "type": "string",
                 "default": "prod"
               }
             }
           }
         }
       ]
     }
   }
   ```

2. **Execute tasks**:

   ```json
   {
     "method": "tools/call",
     "params": {
       "name": "just_deploy",
       "arguments": {
         "env": "staging"
       }
     }
   }
   ```

### Real-World Examples

#### Web Development Project

```just
# Node.js project justfile
[doc("Install dependencies and setup development environment")]
setup:
  npm install
  cp .env.example .env
  npm run db:migrate

[doc("Start development server with hot reload")]
dev port="3000":
  npm run dev -- --port {{port}}

[doc("Run linting and formatting")]
lint fix="false":
  npm run lint {{if fix == "true" { "--fix" } else { "" }}}
  npm run prettier {{if fix == "true" { "--write" } else { "--check" }}} .

[doc("Run tests with optional watch mode")]
test watch="false" coverage="false":
  npm test {{ if watch == "true" { "--watch" } else { "" } }} \
           {{ if coverage == "true" { "--coverage" } else { "" } }}

[doc("Build for production with optional analysis")]
build analyze="false":
  npm run build
  {{ if analyze == "true" { "npm run build:analyze" } else { "" } }}

[doc("Deploy to environment")]
deploy env target="":
  #!/usr/bin/env bash
  set -euo pipefail
  echo "Deploying to {{env}}..."
  if [[ "{{target}}" != "" ]]; then
    npm run deploy:{{env}} -- --target {{target}}
  else
    npm run deploy:{{env}}
  fi
```

#### Rust Development Project

```just
# Rust project justfile
[doc("Run all checks before committing")]
pre-commit:
  cargo fmt --all -- --check
  cargo clippy -- -D warnings
  cargo test
  cargo doc --no-deps

[doc("Run benchmarks with optional baseline")]
bench baseline="":
  {{ if baseline != "" { "cargo bench -- --baseline " + baseline } else { "cargo bench" } }}

[doc("Generate and open documentation")]
docs open="true":
  cargo doc --no-deps --all-features
  {{ if open == "true" { "open target/doc/$(cargo pkgid | cut -d# -f1 | rev | cut -d/ -f1 | rev)/index.html" } else { "" } }}

[doc("Create a new release")]
release version:
  # Ensure working directory is clean
  git diff-index --quiet HEAD --
  # Update version
  cargo set-version {{version}}
  # Run tests
  cargo test --all-features
  # Commit and tag
  git add Cargo.toml Cargo.lock
  git commit -m "Release v{{version}}"
  git tag -a v{{version}} -m "Release v{{version}}"
  echo "Ready to push: git push && git push --tags"
```

#### DevOps/Infrastructure Project

```just
# Infrastructure justfile
[doc("Initialize Terraform workspace")]
tf-init env:
  cd terraform/{{env}} && terraform init -upgrade

[doc("Plan infrastructure changes")]
tf-plan env:
  cd terraform/{{env}} && terraform plan -out=tfplan

[doc("Apply infrastructure changes")]
tf-apply env:
  cd terraform/{{env}} && terraform apply tfplan

[doc("Check Kubernetes cluster health")]
k8s-health context="":
  #!/usr/bin/env bash
  {{ if context != "" { "kubectl config use-context " + context } else { "" } }}
  kubectl cluster-info
  kubectl get nodes
  kubectl get pods --all-namespaces | grep -v Running | grep -v Completed

[doc("Deploy application to Kubernetes")]
k8s-deploy app namespace="default" image_tag="latest":
  kubectl apply -f k8s/{{app}}/namespace.yaml
  kubectl apply -f k8s/{{app}}/config.yaml -n {{namespace}}
  kubectl set image deployment/{{app}} {{app}}={{app}}:{{image_tag}} -n {{namespace}}
  kubectl rollout status deployment/{{app}} -n {{namespace}}

[doc("Stream logs from application")]
logs app namespace="default" follow="true":
  kubectl logs -l app={{app}} -n {{namespace}} {{ if follow == "true" { "-f" } else { "" } }}
```

#### Data Science Project

```just
# Data science project justfile
[doc("Setup Python virtual environment")]
venv:
  python -m venv .venv
  .venv/bin/pip install -r requirements.txt
  .venv/bin/pip install -r requirements-dev.txt

[doc("Run Jupyter lab with specific port")]
jupyter port="8888":
  .venv/bin/jupyter lab --port={{port}} --no-browser

[doc("Train model with hyperparameters")]
train model="baseline" epochs="100" batch_size="32":
  .venv/bin/python src/train.py \
    --model {{model}} \
    --epochs {{epochs}} \
    --batch-size {{batch_size}} \
    --output models/{{model}}_{{datetime()}}.pkl

[doc("Evaluate model on test set")]
evaluate model_path dataset="test":
  .venv/bin/python src/evaluate.py \
    --model {{model_path}} \
    --dataset data/{{dataset}}.csv \
    --output reports/evaluation_{{datetime()}}.json

[doc("Generate data quality report")]
data-report input="data/raw" output="reports/data_quality.html":
  .venv/bin/python -m pandas_profiling {{input}} {{output}}
```

### MCP Configuration Examples

#### Multi-Project Configuration

```json
{
  "mcpServers": {
    "just": {
      "command": "just-mcp",
      "args": [
        "--watch-dir", "~/projects/web-app:webapp",
        "--watch-dir", "~/projects/api:backend",
        "--watch-dir", "~/projects/ml-pipeline:ml",
        "--watch-dir", "~/infrastructure:infra",
        "--timeout", "300",
        "--output-limit", "2097152"
      ],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

#### Development Environment with Debugging

```json
{
  "mcpServers": {
    "just-dev": {
      "command": "just-mcp",
      "args": [
        "--watch-dir", ".",
        "--verbose"
      ],
      "env": {
        "RUST_LOG": "debug",
        "JUST_MCP_TIMEOUT": "600"
      }
    }
  }
}
```

### Administrative Commands

**Sync justfiles** (refresh tool registry):

```json
{
  "method": "tools/call",
  "params": {
    "name": "admin_sync"
  }
}
```

**Create new task** with AI assistance:

```json
{
  "method": "tools/call",
  "params": {
    "name": "admin_create_task",
    "arguments": {
      "task_name": "lint",
      "task_description": "Run code linting",
      "directory_name": "backend"
    }
  }
}
```

## Vector Search (Optional)

just-mcp includes powerful semantic search capabilities to help discover and understand justfile tasks across your projects using natural language queries.

### Quick Start

```bash
# Install with vector search support
cargo install --path . --features "vector-search,local-embeddings"

# Index your projects (offline, no API keys needed)
just-mcp search index --local-embeddings

# Search using natural language
just-mcp search query --query "deploy to production" --local-embeddings
```

### Key Features

- **🔌 Offline-First**: Uses local embeddings - no internet or API keys required
- **🚀 Smart Caching**: Models cached after first download for instant startup
- **🔍 Natural Language**: Search with queries like "build docker image" or "run tests"
- **📊 Cross-Project**: Discover similar patterns across all your repositories
- **🎯 Semantic Understanding**: Finds conceptually related tasks, not just text matches

### Embedding Providers

#### 1. Local Embeddings (Recommended)

The default choice for privacy-conscious users and offline environments.

```bash
# Uses sentence-transformers/all-MiniLM-L6-v2
just-mcp search index --local-embeddings
```

- **Model**: all-MiniLM-L6-v2 (384 dimensions, ~80MB)
- **First Run**: Downloads from Hugging Face Hub to `~/.cache/just-mcp/models/`
- **Performance**: Fast after initial setup, runs entirely on your machine
- **Privacy**: Your code never leaves your computer

#### 2. OpenAI Embeddings

For users who prefer OpenAI's embedding models.

```bash
export OPENAI_API_KEY="sk-..."
just-mcp search index --openai-api-key $OPENAI_API_KEY
```

- **Model**: text-embedding-ada-002 (1536 dimensions)
- **Requirements**: Active OpenAI API key and internet connection
- **Cost**: Standard OpenAI embedding pricing applies

#### 3. Mock Embeddings

For testing and development only.

```bash
just-mcp search index --mock-embeddings
```

### Common Operations

#### Indexing Projects

```bash
# Index current directory
just-mcp search index --local-embeddings

# Index specific directories
just-mcp search index --directory ~/projects/backend --local-embeddings
just-mcp search index --directory ~/projects/frontend --local-embeddings

# Re-index to update after changes
just-mcp search index --directory . --force --local-embeddings
```

#### Searching Tasks

```bash
# Basic search
just-mcp search query --query "start development server" --local-embeddings

# Search with similarity threshold (0.0-1.0, higher = more similar)
just-mcp search query --query "deploy production" --threshold 0.7 --local-embeddings

# Limit number of results
just-mcp search query --query "run tests" --limit 10 --local-embeddings

# Combine threshold and limit
just-mcp search query \
  --query "database migration" \
  --threshold 0.6 \
  --limit 5 \
  --local-embeddings
```

#### Advanced Features

```bash
# Find tasks similar to a description
just-mcp search similar --task "build and push docker image" --local-embeddings

# Search by text content (exact match)
just-mcp search text --text "cargo build"

# Filter by metadata
just-mcp search filter --filter has_params=true --filter category=deployment

# View index statistics
just-mcp search stats
```

### Real-World Examples

#### Example 1: DevOps Engineer

```bash
# Index all infrastructure projects
for dir in ~/infra/*; do
  just-mcp search index --directory "$dir" --local-embeddings
done

# Find deployment-related tasks
just-mcp search query --query "deploy kubernetes production" --local-embeddings
just-mcp search query --query "terraform apply" --local-embeddings
just-mcp search query --query "docker build push registry" --local-embeddings
```

#### Example 2: Full-Stack Developer

```bash
# Index frontend and backend
just-mcp search index --directory ~/projects/web-app --local-embeddings
just-mcp search index --directory ~/projects/api --local-embeddings

# Find development tasks
just-mcp search query --query "start dev server hot reload" --local-embeddings
just-mcp search query --query "run unit tests coverage" --local-embeddings
just-mcp search query --query "database seed development" --local-embeddings
```

#### Example 3: Data Scientist

```bash
# Index ML projects
just-mcp search index --directory ~/ml-projects --local-embeddings

# Find ML workflow tasks
just-mcp search query --query "train model hyperparameters" --local-embeddings
just-mcp search query --query "evaluate model metrics" --local-embeddings
just-mcp search query --query "jupyter notebook gpu" --local-embeddings
```

### Integration with MCP

Combine vector search with the MCP server for enhanced AI-powered discovery:

```json
{
  "mcpServers": {
    "just-search": {
      "command": "just-mcp",
      "args": [
        "--watch-dir", "~/projects:all-projects"
      ],
      "env": {
        "RUST_LOG": "info",
        "JUST_MCP_VECTOR_SEARCH": "true",
        "JUST_MCP_EMBEDDING_PROVIDER": "local"
      }
    }
  }
}
```

### Performance Tips

1. **Initial Setup**: First-time model download takes ~30 seconds
2. **Indexing Speed**: ~100-500 tasks/second depending on hardware
3. **Search Speed**: Sub-second for most queries
4. **Storage**: Database size is roughly 1MB per 100 indexed tasks

### Troubleshooting

#### Model Download Issues

```bash
# Clear cache and retry
rm -rf ~/.cache/just-mcp/models/
just-mcp search index --local-embeddings

# Use custom cache directory
export XDG_CACHE_HOME=/custom/cache
just-mcp search index --local-embeddings
```

#### Search Quality

- Use specific, descriptive queries
- Include action verbs: "build", "deploy", "test", "run"
- Add context: "production", "development", "docker", "database"
- Adjust threshold: Lower for broader results, higher for exact matches

## Configuration

### Command Line Options

```bash
just-mcp [OPTIONS]

Options:
  -w, --watch-dir <PATH[:NAME]>  Directory to watch (can be specified multiple times)
  -t, --timeout <SECONDS>         Default task timeout (default: 300)
  -o, --output-limit <BYTES>      Max output size per task (default: 1MB)
  -v, --verbose                   Enable verbose logging
  -h, --help                      Print help
  -V, --version                   Print version
```

### Environment Variables

- `RUST_LOG`: Set logging level (e.g., `debug`, `info`, `warn`, `error`)
- `JUST_MCP_TIMEOUT`: Default timeout for task execution
- `JUST_MCP_OUTPUT_LIMIT`: Maximum output size for tasks

## Architecture

### Core Components

```
     ┌───────────┐     ┌──────────┐
   Watcher   │────▶│  Parser   │────▶│ Registry │
     └───────────┘     └──────────┘
       │                                     │
       ▼                                     ▼
     ┌───────────┐     ┌──────────┐
   Server    │◀────│  Handler  │◀────│   MCP    │
     └───────────┘     └──────────┘
                            │
                            ▼
                    ┌───────────┐
                    │ Executor  │
                    └───────────┘
```

### Key Design Decisions

- **Async-first**: Built on Tokio for concurrent operations
- **Channel-based**: Components communicate via broadcast channels
- **Security-focused**: All inputs validated, paths restricted, resources limited
- **Hot-reload**: File changes trigger automatic tool updates

### Performance & Parallel Builds

just-mcp leverages modern concurrency patterns for optimal performance:

#### Parallel Build System

The Dagger-based release system uses Go routines to build all platforms concurrently:

```go
// Platforms build in parallel
platforms := []string{
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu", 
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "universal2-apple-darwin",
}
```

**Performance Benefits**:

- **5x Faster Releases**: All platforms build simultaneously instead of sequentially
- **Intelligent Caching**: Dagger caches dependencies across parallel builds
- **Resource Efficiency**: Single Docker image shared across all platform builds
- **Automatic Scaling**: Utilizes available CPU cores for maximum throughput

#### Real-time File Monitoring

The watcher system provides instant feedback:

- **Debounced Updates**: 500ms debounce prevents thrashing on rapid changes
- **Content Hashing**: SHA256 verification ensures only real changes trigger updates
- **Broadcast Channels**: O(1) event distribution to all subscribers

## Development

### Essential Commands

```bash
# Daily development
just check         # Format, lint, and test (use before committing)
just test          # Run all tests
just build-release # Build optimized binary

# CI/CD with Dagger
just dagger-ci     # Run complete CI pipeline locally
just dagger-release version="v1.0.0"  # Build ALL platforms in parallel
```

### Dagger CI/CD

just-mcp uses [Dagger](https://dagger.io) for containerized CI/CD pipelines that work identically locally and in GitHub Actions.

#### Key Commands

```bash
# CI Pipeline - matches GitHub Actions exactly
just dagger-ci          # Full pipeline: format, lint, test, coverage

# Individual steps (for debugging)
just dagger-test        # Run tests in container
just dagger-coverage    # Generate coverage report

# Release builds - all platforms in parallel
just dagger-release version="v1.0.0"  # Creates ./release-artifacts/*.tar.gz
```

#### Why Dagger?

- **Parallel Builds**: All platforms build concurrently (5x faster)
- **Cross-Platform**: Build macOS binaries from Linux using cargo-zigbuild
- **Reproducible**: Identical results locally and in CI
- **Cached**: Dependencies and artifacts cached automatically

#### GitHub Actions Integration

Two minimal workflows leverage Dagger:

- **CI** (`.github/workflows/dagger-ci.yml`): On all pushes/PRs
- **Release** (`.github/workflows/dagger-release.yml`): On version tags

#### Cross-Platform Builds

All platforms built from Linux via cargo-zigbuild:

- Linux x86_64/ARM64
- macOS x86_64/ARM64/Universal

For debugging specific platforms:

```bash
just zigbuild-test target="x86_64-apple-darwin"
```

### Project Structure

```
just-mcp/
 src/
   ├── main.rs           # Entry point and CLI
   ├── server/           # MCP protocol implementation
   ├── parser/           # Justfile parsing logic
   ├── registry/         # Tool registry management
   ├── watcher/          # File system monitoring
   ├── executor/         # Task execution engine
   ├── admin/            # Administrative tools
   └── security/         # Security and validation
 tests/                # Integration tests
 demo/                 # Example justfiles
 scripts/              # Development scripts
```

## Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

### Quick Steps

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes and add tests
4. Run `just check` to ensure code quality
5. Commit your changes (`git commit -m 'Add amazing feature'`)
6. Push to your branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

### Development Notes

- Follow Rust idioms and best practices
- Add tests for new functionality
- Update documentation for API changes
- Use `just format` and `just lint` before committing

## Troubleshooting

### Common Issues and Solutions

#### MCP Connection Issues

**Problem**: AI assistant can't connect to just-mcp server

```
Error: Failed to connect to MCP server
```

**Solutions**:

1. Verify just-mcp is in your PATH:

   ```bash
   which just-mcp
   ```

2. Check MCP configuration syntax in settings file
3. Ensure no other process is using the configured port
4. Try running just-mcp manually to see error messages:

   ```bash
   just-mcp --watch-dir /path/to/project
   ```

#### Justfile Not Detected

**Problem**: Tasks from justfile aren't appearing in tool list

**Solutions**:

1. Verify file is named exactly `justfile` (case-sensitive)
2. Check file permissions:

   ```bash
   ls -la justfile
   ```

3. Manually trigger sync:

   ```json
   {"method": "tools/call", "params": {"name": "admin_sync"}}
   ```

4. Enable verbose logging to see file detection:

   ```bash
   RUST_LOG=debug just-mcp --verbose
   ```

#### Task Execution Failures

**Problem**: Tasks fail with permission denied or command not found

**Solutions**:

1. Ensure `just` is installed and in PATH:

   ```bash
   just --version
   ```

2. Check task has proper shell permissions
3. Verify working directory is correct
4. For complex tasks, test directly with just first:

   ```bash
   just task-name
   ```

#### Performance Issues

**Problem**: Slow response times or high resource usage

**Solutions**:

1. Limit watched directories to only necessary paths
2. Increase debounce time for frequently changing files
3. Set appropriate resource limits:

   ```bash
   just-mcp --timeout 60 --output-limit 500000
   ```

4. Monitor system resources during operation

#### Platform-Specific Issues

**macOS**: "Operation not permitted" errors

- Grant Terminal/IDE full disk access in System Preferences
- Use explicit paths instead of `~/` shortcuts

**Linux**: File watching limits exceeded

- Increase inotify limits:

  ```bash
  echo fs.inotify.max_user_watches=524288 | sudo tee -a /etc/sysctl.conf
  sudo sysctl -p
  ```

**Windows**: Not currently supported

- Use WSL2 with Linux installation instructions
- Docker container support coming in future release

### Debug Mode

Enable comprehensive debug logging:

```bash
RUST_LOG=just_mcp=debug,tokio=debug just-mcp --verbose
```

Key debug indicators:

- `[WATCHER]`: File system monitoring events
- `[PARSER]`: Justfile parsing details
- `[REGISTRY]`: Tool registration/updates
- `[EXECUTOR]`: Task execution traces

### Getting Help

If issues persist:

1. Check existing issues: [GitHub Issues](https://github.com/onegrep/just-mcp/issues)
2. Gather debug information:

   ```bash
   just-mcp --version
   just --version
   uname -a  # System info
   ```

3. Create a minimal reproducible example
4. Open an issue with debug logs and system details

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [just](https://just.systems/) - The command runner that inspired this project
- [Model Context Protocol](https://modelcontextprotocol.io/) - The MCP specification
- [Anthropic](https://anthropic.com/) - For developing the MCP standard
- [Dagger](https://dagger.io/) - For portable CI/CD pipelines

---

**Need help?** Open an issue or check out the [demo](./demo/) directory for examples.
