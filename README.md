# just-mcp

[![CI](https://github.com/onegrep/just-mcp/actions/workflows/dagger-ci.yml/badge.svg)](https://github.com/onegrep/just-mcp/actions/workflows/dagger-ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.88+-blue.svg)](https://www.rust-lang.org)
[![MCP](https://img.shields.io/badge/MCP-1.0-green.svg)](https://modelcontextprotocol.io/)
[![Vector Search](https://img.shields.io/badge/Vector%20Search-Optional-orange.svg)](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2)

> Transform justfiles into AI-accessible automation tools through the Model Context Protocol

**just-mcp** bridges [justfiles](https://just.systems/) and Coding Agents by exposing justfile recipes as dynamically discoverable MCP tools.

This enables AI assistants to understand, explore, and execute a project's common development workflows similar to how a human would.

<img width="676" height="331" alt="just-mcp" src="https://github.com/user-attachments/assets/c1222e9b-e440-4e7e-ab8c-629e727d1849" />

## Quick Start

**1. Install just-mcp**
```bash
# Using cargo-binstall (fastest - downloads pre-built binary)
cargo binstall --git https://github.com/toolprint/just-mcp just-mcp

# Or download latest release
curl -L https://github.com/toolprint/just-mcp/releases/latest/download/just-mcp-$(uname -m)-$(uname -s).tar.gz | tar xz
sudo mv just-mcp /usr/local/bin/
```

**2. Configure your Agent** (see [MCP Client Setup](#mcp-client-setup) below)

**3. Use the Slash Command**:

```bash
/just:do-it build the project
```

It will find the appropriate justfile task to run as an MCP Tool.

That's it. Your AI can now use your justfile tasks.

## MCP Client Setup

<details>
<summary><strong>Claude Code</strong></summary>

```bash
claude mcp add -s user -t stdio just -- just-mcp
```

Or manually add to `~/.claude.json`:

```json
{
  "mcpServers": {
    "just": {
      "type": "stdio", 
      "command": "just-mcp"
    }
  }
}
```

**Verification**: Start a new session and check /mcp for 'just' and view details to see how many tools loaded.

</details>

<details>
<summary><strong>Claude Desktop</strong></summary>

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "just": {
      "command": "just-mcp", 
      "args": ["--watch-dir", "/Users/username/workspace/your-project-dir"],
    }
  }
}
```

**Verification**: Restart Claude Desktop, check the settings button for custom connection tools.

</details>

<details>
<summary><strong>Cline (VS Code Extension)</strong></summary>

1. Click Cline icon → **MCP Servers** → **Configure MCP Servers**
2. Add configuration:

```json
{
  "mcpServers": {
    "just": {
      "disabled": false,
      "timeout": 60,
      "type": "stdio",
      "command": "just-mcp",
      "args": [
        "-w",
        "/Users/username/your-project-dir"
      ]
    }
}
```

**Verification**: Look for MCP server status in Cline's interface.

**NOTE:** Since cline runs outside of your current project working directory you MUST specify the watch-dir or use the admin tools at runtime to ask Cline to switch the watch-dir for you.

</details>

## Key Features

### 🔍 **Smart Justfile Discovery**
- Real-time monitoring with hot reloading
- Defaults to use the current project root directory to look for your justfile
- Dynamic tool generation from your tasks

### 📝 **Advanced Parsing**
- AST-based parser using Tree-sitter for complete syntax support
- Parameter documentation from comments: `# {{param}}: description`
- Three-tier fallback: AST → CLI → Regex for maximum compatibility

### 🛡️ **Security First**
- Input validation prevents command injection
- Configurable timeouts and resource limits
- Directory whitelisting and parameter sanitization

### ⚙️ **Admin Tools**
- `admin_sync`: Refresh tool registry
- `admin_create_task`: AI-assisted task creation with backup

### 🔍 **Vector Search** *(Optional)*
- **Offline semantic search** with local embeddings (no API keys)
- Natural language queries: *"deploy to production"*
- Cross-project discovery of similar automation patterns
- See [Vector Search docs](docs/features/vector-search.md)

## Example Workflow

Given this justfile:
```just
# Deploy the application
deploy env="prod":
  echo "Deploying to {{env}}"
  ./deploy.sh {{env}}

# Run tests with coverage
test coverage="false":
  cargo test {{if coverage == "true" { "--coverage" } else { "" }}}
```

Your AI assistant can:
- **Discover**: "What tasks are available?"
- **Execute**: "Deploy to staging" → runs `deploy env="staging"`
- **Understand**: Sees parameters, descriptions, and dependencies

## Installation Options

### Pre-built Binaries (Recommended)

Download from [GitHub Releases](https://github.com/toolprint/just-mcp/releases/latest):

```bash
# Linux x86_64
curl -L https://github.com/toolprint/just-mcp/releases/latest/download/just-mcp-x86_64-unknown-linux-gnu.tar.gz | tar xz

# Linux ARM64  
curl -L https://github.com/toolprint/just-mcp/releases/latest/download/just-mcp-aarch64-unknown-linux-gnu.tar.gz | tar xz

# macOS (Universal - works on Intel and Apple Silicon)
curl -L https://github.com/toolprint/just-mcp/releases/latest/download/just-mcp-universal2-apple-darwin.tar.gz | tar xz
```

### From Source

```bash
git clone https://github.com/toolprint/just-mcp.git
cd just-mcp
just quickstart  # Complete setup + install
```

Or manually:
```bash
cargo install --path . --features all  # All features including vector search
```

## Development

### Setup
```bash
just quickstart     # Complete setup for new developers
just dev-setup      # Comprehensive development environment
```

### Workspace Structure
- `./` - Main MCP server
- `dev-tools/` - Performance analysis utilities

### Common Commands
```bash
just build              # Build main server  
just build-dev-tools    # Build development utilities
just test               # Run tests
just check              # Format, lint, test (pre-commit)
```

## Multi-Project Example

Monitor multiple projects with custom names:

```bash
just-mcp \
  --watch-dir ~/projects/api:backend \
  --watch-dir ~/projects/web:frontend \
  --watch-dir ~/infrastructure:infra
```

Tools will be available as:
- `just_deploy@backend`
- `just_build@frontend`  
- `just_apply@infra`

## Documentation

- **[Configuration Guide](docs/CONFIGURATION.md)** - Detailed setup for all MCP clients
- **[Vector Search](docs/features/vector-search.md)** - Semantic search with natural language
- **[Troubleshooting](docs/troubleshooting.md)** - Common issues and solutions
- **[Contributing](CONTRIBUTING.md)** - Development workflow and guidelines

## Architecture

```
Justfile Watcher → AST Parser → Tool Registry → MCP Server → AI Assistant
     ↓                ↓             ↓            ↓
File Changes → Dynamic Updates → Real-time → Task Execution
```

**Key Design Principles:**
- **Async-first**: Tokio-based for concurrent operations
- **Security-focused**: All inputs validated, resources limited
- **Hot-reload**: File changes trigger automatic tool updates
- **Zero-config**: Works with existing justfiles

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Quick steps:
1. `just quickstart` - Set up development environment
2. Make your changes and add tests  
3. `just check` - Ensure code quality
4. Submit a Pull Request

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- [just](https://just.systems/) - The command runner that inspired this project
- [Model Context Protocol](https://modelcontextprotocol.io/) - Enabling AI-tool communication
- [Anthropic](https://anthropic.com/) - For developing the MCP standard

---

**Need help?** Check [troubleshooting](docs/troubleshooting.md) or open an issue on GitHub.