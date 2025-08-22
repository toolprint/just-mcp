# Configuration Guide

## Command Line Options

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

## Environment Variables

- `RUST_LOG`: Set logging level (e.g., `debug`, `info`, `warn`, `error`)
- `JUST_MCP_TIMEOUT`: Default timeout for task execution
- `JUST_MCP_OUTPUT_LIMIT`: Maximum output size for tasks

## MCP Client Configurations

### Claude Code (claude.ai/code)

```json
// ~/.claude.json
{
  "mcpServers": {
    "just": {
      "type": "stdio",
      "command": "just-mcp",
      "args": ["--watch-dir", "/path/to/project"],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

### Claude Desktop

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "just": {
      "command": "just-mcp",
      "args": ["--watch-dir", "/Users/username/projects"],
      "env": {}
    }
  }
}
```

### Cline (VS Code Extension)

Access via: Cline icon → MCP Servers → Configure MCP Servers

```json
{
  "mcpServers": {
    "just": {
      "command": "just-mcp",
      "args": ["--watch-dir", ".", "--timeout", "300"],
      "env": {},
      "alwaysAllow": ["just_build", "just_test"],
      "disabled": false
    }
  }
}
```

**Windows users**:
```json
{
  "mcpServers": {
    "just": {
      "command": "cmd",
      "args": ["/c", "just-mcp", "--watch-dir", "."],
      "env": {}
    }
  }
}
```

### VS Code Native MCP

Project-specific configuration in `.vscode/mcp.json`:

```json
{
  "mcpServers": {
    "just": {
      "command": "just-mcp",
      "args": ["--watch-dir", "${workspaceFolder}"],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

## Multi-Project Configuration

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

## Development Environment with Debugging

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