# just-mcp

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org)
[![MCP](https://img.shields.io/badge/MCP-1.0-green.svg)](https://modelcontextprotocol.io/)

> Transform justfiles into AI-accessible automation tools through the Model Context Protocol

**just-mcp** bridges [justfiles](https://just.systems/) and AI assistants by exposing justfile tasks as dynamically discoverable MCP tools.

This enables AI assistants to understand, explore, and execute a project's common development workflows similar to how a human would.

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Usage](#usage)
- [Configuration](#configuration)
- [Architecture](#architecture)
- [Development](#development)
- [Contributing](#contributing)
- [License](#license)

## Features

### 🔍 **Intelligent Justfile Discovery**

- Real-time monitoring with filesystem watching
- Multi-project support with optional naming (`--watch-dir path:name`)
- Dynamic tool generation from justfile tasks
- Hot reloading on justfile modifications

### 📝 **Comprehensive Justfile Parsing**

- Full syntax support: parameters, defaults, dependencies, comments, attributes
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

### 🚀 **MCP Protocol Compliance**

- Full JSON-RPC 2.0 MCP specification implementation
- Real-time tool list updates via notifications
- Standard capabilities: `tools/list`, `tools/call`, change notifications
- Comprehensive error reporting and validation

## Installation

### Prerequisites

- Rust 1.88.0 (enforced via `rust-toolchain.toml`)
- `just` command runner ([installation guide](https://just.systems/man/en/chapter_4.html))
- [Dagger](https://dagger.io) (optional, for CI/CD workflows)

### From Source

```bash
git clone https://github.com/onegrep/just-mcp.git
cd just-mcp
cargo install --path .
```

### Using Just

```bash
just install  # Builds and installs to ~/.cargo/bin
```

### Development Dependencies

Install development tools using the provided Brewfile (macOS):

```bash
just brew  # Installs prettier, markdownlint-cli2, and other tools
```

Or install manually:

```bash
cargo install cargo-tarpaulin  # Coverage testing
npm install -g prettier markdownlint-cli2  # Formatting tools
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

## Development

### Building

```bash
just build         # Debug build
just build-release # Optimized release build
```

### Testing

```bash
just test          # Run all tests
just test-coverage # Generate coverage report
just check         # Format, lint, and test
```

### Code Quality

```bash
just format        # Auto-format code
just lint          # Run clippy lints
just pre-commit    # Full validation before committing
```

### CI/CD with Dagger

just-mcp uses [Dagger](https://dagger.io) for containerized CI/CD pipelines that work identically locally and in GitHub Actions.

#### Running CI Locally

```bash
# Run the complete CI pipeline (format, lint, test, coverage)
just dagger-ci

# Run individual CI steps
just dagger-format      # Check code formatting
just dagger-lint        # Run clippy linter
just dagger-test        # Run tests (specify platform with platform="linux/amd64")
just dagger-coverage    # Generate coverage report

# Build for specific platforms
just dagger-build platform="linux/amd64"         # Debug build
just dagger-build-release platform="linux/arm64" # Release build

# Create release packages
just dagger-package platform="linux/amd64" version="v1.0.0"

# Build releases for Linux platforms
just dagger-release version="v1.0.0"

# Build macOS universal binary (cross-compiled)
just dagger-release-darwin version="v1.0.0"
```

#### Available Platforms

- `linux/amd64` (x86_64-unknown-linux-gnu)
- `linux/arm64` (aarch64-unknown-linux-gnu)
- `darwin/amd64` (x86_64-apple-darwin) - requires macOS host
- `darwin/arm64` (aarch64-apple-darwin) - requires macOS host

#### Benefits of Dagger

- **Reproducible Builds**: Same results locally and in CI
- **Better Caching**: Automatic caching of dependencies and build artifacts
- **Parallel Execution**: Platform builds run concurrently
- **No CI Lock-in**: Works with any CI system, not just GitHub Actions
- **Artifacts**: Release packages are saved to `./release-artifacts/`

#### CI/CD Workflows

The project includes multiple GitHub Actions workflows:

1. **Standard Release** (`.github/workflows/dagger-release.yml`):
   - Linux builds via Dagger on Ubuntu runners
   - macOS builds on native macOS runners

2. **Zigbuild Release** (`.github/workflows/zigbuild-release.yml`):
   - All platforms built from a single Linux runner
   - Uses cargo-zigbuild Docker image with macOS SDK
   - Creates universal binaries for macOS

#### GitHub Actions Integration

The project includes minimal GitHub Actions workflows that leverage Dagger:

- **CI Workflow** (`.github/workflows/dagger-ci.yml`): Runs on all pushes and PRs
- **Release Workflow** (`.github/workflows/dagger-release.yml`): Builds and publishes releases on version tags

#### Platform Build Support

**Linux Builds**: Full support for cross-compilation

- x86_64 (native compilation)
- ARM64 (cross-compilation with gcc-aarch64-linux-gnu)

**macOS Builds**: Two approaches available

1. **Native builds** (requires macOS):
   - Uses native macOS toolchain
   - Required for testing on actual hardware
   - Run: `just release-darwin`

2. **Cross-compilation with cargo-zigbuild** (works on Linux):
   - Uses Docker image with macOS SDK
   - Builds all platforms from Linux
   - Run: `just zigbuild-release`

#### Cross-Compilation with cargo-zigbuild

just-mcp supports cross-compilation to all platforms (including macOS) from Linux using cargo-zigbuild:

```bash
# Build all platforms using cargo-zigbuild (Linux, macOS x86_64/ARM64, Universal)
just zigbuild-release

# Test zigbuild for a specific platform
just zigbuild-test target="x86_64-apple-darwin"

# Use Dagger with zigbuild for reproducible builds
just dagger-zigbuild-release
```

The zigbuild approach uses Docker images that include the macOS SDK, enabling cross-compilation from Linux to macOS despite the `notify` crate's framework dependencies.

#### Local Development

For local builds:

```bash
# Traditional approach
just dagger-release          # Linux builds via Dagger
just release-darwin          # macOS builds (requires macOS)

# Cross-compilation approach (works on Linux)
just zigbuild-release        # All platforms via cargo-zigbuild
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

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [just](https://just.systems/) - The command runner that inspired this project
- [Model Context Protocol](https://modelcontextprotocol.io/) - The MCP specification
- [Anthropic](https://anthropic.com/) - For developing the MCP standard
- [Dagger](https://dagger.io/) - For portable CI/CD pipelines

---

**Need help?** Open an issue or check out the [demo](./demo/) directory for examples.
