# just-mcp

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org)

> A Model Context Protocol (MCP) server that transforms justfiles into AI-accessible automation tools

**just-mcp** bridges the gap between [justfiles](https://just.systems/) and AI assistants by exposing justfile tasks as dynamically discoverable MCP tools. This enables AI assistants to understand, explore, and execute project-specific automation tasks across single or multiple repositories.

## Features

### 🔍 **Intelligent Justfile Discovery**

- **Real-time monitoring**: Automatically detects justfile changes with filesystem watching
- **Multi-project support**: Watch multiple directories with optional naming (`--watch-dir path:name`)
- **Dynamic tool generation**: Each justfile task becomes an MCP tool with proper JSON schema
- **Hot reloading**: Tools update automatically when justfiles are modified

### 📝 **Comprehensive Justfile Parsing**

- **Full syntax support**: Handles parameters, defaults, dependencies, comments, and attributes
- **Parameter documentation**: Extracts parameter descriptions from `# {{param}}: description` comments
- **Multiple formats**: Supports both `task(param)` and `task param` parameter syntax
- **Doc attributes**: Recognizes `[doc("description")]` attributes for enhanced documentation

### 🛡️ **Security & Resource Management**

- **Input validation**: Prevents command injection and path traversal attacks
- **Resource limits**: Configurable timeouts, memory limits, and output size controls
- **Access control**: Directory whitelisting and parameter sanitization
- **Safe execution**: Shell escaping and strict validation modes

### ⚙️ **Administrative Tools**

- **`admin_sync`**: Manual justfile re-scanning and registry refresh
- **`admin_create_task`**: AI-assisted task creation with automatic backup
- **Conflict prevention**: Validates task names and prevents overwrites
- **Multi-directory targeting**: Create tasks in specific named directories

### 🚀 **MCP Protocol Compliance**

- **JSON-RPC 2.0**: Full MCP specification implementation
- **Dynamic notifications**: Real-time tool list updates
- **Standard capabilities**: `tools/list`, `tools/call`, and change notifications
- **Error handling**: Comprehensive error reporting and validation

## Quick Start

### Prerequisites

- [Rust](https://rustlang.org/) 1.70+
- [just](https://just.systems/) command runner (optional, for development)

#### Development Dependencies (macOS)

For full development functionality, install additional tools using:

```bash
just brew  # Installs from Brewfile
```

This includes:

- `tarpaulin` - Code coverage reporting
- `prettier` - JSON formatting
- `markdownlint-cli2` - Markdown linting
- Other development utilities

Or install manually:

```bash
cargo install cargo-tarpaulin  # Coverage testing
npm install -g prettier markdownlint-cli2  # Formatting tools
```

### Installation

#### From Source

```bash
git clone https://github.com/onegrep/just-mcp.git
cd just-mcp
cargo install --path .
```

#### Using Just

```bash
just install  # Builds and installs to ~/.cargo/bin
```

### Basic Usage

#### Single Project

```bash
# Monitor current directory for justfiles
just-mcp

# Monitor specific directory
just-mcp --watch-dir /path/to/project
```

#### Multiple Projects

```bash
# Monitor multiple directories with names
just-mcp \
  --watch-dir /path/to/frontend:frontend \
  --watch-dir /path/to/backend:backend \
  --watch-dir /path/to/shared
```

#### With Administrative Tools

```bash
# Enable admin tools for task creation
just-mcp --admin --watch-dir /path/to/project
```

### Example Integration

Given a justfile:

```just
# Build the application
build target="debug":
    cargo build --{{target}}

# {{filter}}: test name pattern to run
# Run tests with optional filter
test filter="":
    cargo test {{filter}}

# Deploy to environment
deploy env="staging": build test
    ./scripts/deploy.sh {{env}}
```

just-mcp exposes these as MCP tools:

- `just_build` - Build the application (parameter: target="debug")
- `just_test` - Run tests (parameter: filter="")  
- `just_deploy` - Deploy to environment (parameter: env="staging", depends on build+test)

## Configuration

### Command Line Options

```bash
just-mcp [OPTIONS]

Options:
  -w, --watch-dir <WATCH_DIR>  Directory to watch for justfiles, optionally with name 
                               (path or path:name). Defaults to current directory if not specified
      --admin                  Enable administrative tools
      --json-logs              Enable JSON output for logs  
      --log-level <LOG_LEVEL>  Log level (trace, debug, info, warn, error) [default: info]
  -h, --help                   Print help
  -V, --version                Print version
```

### Multi-Directory Naming

When watching multiple directories, use the `path:name` syntax for better tool organization:

```bash
just-mcp --watch-dir ./frontend:ui --watch-dir ./backend:api
```

This creates tools like:

- `just_build@ui` (from frontend/justfile)
- `just_test@api` (from backend/justfile)

Without names, tools include the full path for disambiguation.

## Project Structure

```text
just-mcp/
├── src/
│   ├── admin/          # Administrative tools (sync, create_task)
│   ├── executor/       # Secure task execution with resource limits
│   ├── parser/         # Justfile parsing and task extraction
│   ├── registry/       # Dynamic tool registration and management
│   ├── resource_limits/ # Resource management and constraints
│   ├── security/       # Input validation and injection prevention
│   ├── server/         # MCP protocol implementation
│   │   ├── handler.rs  # Request handling logic
│   │   ├── protocol.rs # MCP protocol definitions
│   │   └── transport.rs # Transport layer implementation
│   ├── types/          # Common type definitions
│   ├── watcher/        # Filesystem monitoring and change detection
│   ├── notification/   # Change notification system
│   ├── error.rs        # Error handling and definitions
│   ├── lib.rs          # Library interface
│   └── main.rs         # CLI entry point
├── demo/               # Example justfile and usage demonstrations
├── tests/              # Integration and unit tests
├── scripts/            # Development and testing scripts
├── notes/              # Development notes and documentation
├── justfile            # Development automation tasks
├── Brewfile            # macOS development dependencies
└── Cargo.toml          # Rust project configuration
```

## Development

### Available Commands

```bash
# Development
just build              # Build for development
just build-release      # Build optimized release binary
just test               # Run all tests
just test-coverage      # Run tests with coverage report using tarpaulin

# Code Quality  
just lint               # Run clippy and formatting checks
just format             # Format code with rustfmt, prettier, and markdownlint
just check              # Run format + lint + test

# Release Management
just release-info       # Show release binary information
just install            # Install locally built binary

# Setup
just brew               # Install macOS development dependencies
just setup              # Initial project setup

# Utilities
just clean              # Clean build artifacts
```

### Running Tests

```bash
# All tests
just test  # or cargo test

# Test coverage with HTML report
just test-coverage  # uses cargo tarpaulin

# Specific test suites
cargo test --test mcp_protocol_test
cargo test --test watcher_integration_test
cargo test --test executor_integration_test
cargo test --test security_test
cargo test --test notification_test
cargo test --test resource_limits_test

# With output
cargo test -- --nocapture
```

## Demo

The `demo/` directory contains a comprehensive example showcasing just-mcp capabilities:

- **Example justfile**: Demonstrates various task patterns, parameters, and dependencies
- **Usage examples**: Step-by-step MCP protocol interactions
- **Testing scenarios**: Real-world automation task examples

See [demo/README.md](./demo/README.md) for detailed examples and usage instructions.

## Examples

### Basic MCP Interaction

```bash
# Start the server
just-mcp --watch-dir ./demo &

# List available tools
echo '{"jsonrpc": "2.0", "method": "tools/list", "id": 1}'

# Call a tool  
echo '{
  "jsonrpc": "2.0", 
  "method": "tools/call",
  "params": {
    "name": "just_hello",
    "arguments": {"name": "World"}
  },
  "id": 2
}'
```

### AI Assistant Integration

just-mcp is designed to work with MCP-compatible AI assistants like:

- **Claude Desktop**: Configure as an MCP server for project automation
- **Custom AI tools**: Integrate via the standard MCP protocol
- **Development workflows**: Enable AI-assisted project management

## Security Considerations

just-mcp includes several security features:

- **Input validation**: All parameters are validated and sanitized
- **Directory restrictions**: Only configured watch directories are accessible  
- **Command injection prevention**: Shell escaping and parameter validation
- **Resource limits**: Timeouts and memory limits prevent resource exhaustion
- **Audit logging**: All executed commands are logged for security review

For production use, consider:

- Running in a sandboxed environment
- Configuring strict resource limits
- Regular security audits of justfiles
- Using dedicated service accounts

## Additional Tools

The project includes several utility directories:

- **`scripts/`**: Development and testing scripts for various scenarios
- **`notes/`**: Development documentation and implementation notes  
- **`test-temp/`**: Temporary test files (created during testing)
- **`Brewfile`**: macOS development dependencies for easy setup

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes and add tests
4. Run the test suite (`just check`)
5. Commit your changes (`git commit -m 'Add amazing feature'`)
6. Push to your branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

### Code Standards

- Follow Rust best practices and idioms
- Add tests for new functionality (see `tests/` directory for examples)
- Update documentation for API changes
- Use `just format` and `just lint` before committing
- Test scripts can be added to `scripts/` directory for development testing
- Development notes should be placed in `notes/` directory

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [just](https://just.systems/) - The excellent command runner that inspired this project
- [MCP](https://modelcontextprotocol.io/) - The Model Context Protocol specification
- [Anthropic](https://anthropic.com/) - For developing the MCP standard

---

**Questions or need help?** Open an issue or check out the [demo](./demo/) directory for examples.
