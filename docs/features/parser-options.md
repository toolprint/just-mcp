# Parser Options and Selection

This document describes the parser selection system in just-mcp, including the new CLI options for controlling which parser is used for justfile parsing.

## Overview

just-mcp supports multiple parsing strategies for justfiles, with an intelligent two-tier fallback system that prioritizes accuracy and performance:

1. **AST Parser** (Default, most accurate) - Uses Tree-sitter for precise syntax analysis
2. **CLI Parser** (Fallback) - Uses `just --summary` command for recipe discovery
3. **Regex Parser** (Deprecated) - Simple pattern matching, scheduled for removal

## Parser Selection

### Command Line Interface

The `--parser` flag allows explicit parser selection:

```bash
# Use intelligent fallback (default: AST → CLI)
just-mcp --parser auto

# Force AST parser only (fail if unavailable or errors)
just-mcp --parser ast

# Force CLI parser only (requires 'just' command available)
just-mcp --parser cli

# Force regex parser (deprecated, not recommended)
just-mcp --parser regex
```

### Parser Modes

#### Auto Mode (Recommended Default)

```bash
just-mcp --parser auto
# or simply
just-mcp
```

**Behavior:**

- **Primary**: Attempts AST parser (if `ast-parser` feature enabled)
- **Fallback**: Falls back to CLI parser if AST parsing fails
- **No regex fallback**: Cleaner, more predictable behavior

**Use Cases:**

- Production deployments
- General usage
- When you want the best available parser with graceful fallback

#### AST Only Mode

```bash
just-mcp --parser ast
```

**Behavior:**

- Uses only the Tree-sitter AST parser
- Fails immediately if AST parser unavailable or encounters errors
- No fallback to other parsers

**Use Cases:**

- Testing AST parser specifically
- Environments where you want to ensure AST parsing accuracy
- Development and debugging of AST parser functionality

**Requirements:**

- Must be built with `--features ast-parser`
- Tree-sitter dependencies must be available

#### CLI Only Mode

```bash
just-mcp --parser cli
```

**Behavior:**

- Uses only the CLI parser (`just --summary`)
- Fails if `just` command is not available
- No fallback to other parsers

**Use Cases:**

- Environments where Tree-sitter dependencies are problematic
- Testing CLI parser behavior
- Compatibility with complex justfile features that require `just` CLI

**Requirements:**

- `just` command must be available in PATH
- Working justfile syntax (parseable by `just`)

#### Regex Mode (Deprecated)

```bash
just-mcp --parser regex
```

**⚠️ DEPRECATED**: This mode is deprecated and will be removed in a future version.

**Behavior:**

- Uses simple regex pattern matching
- Limited accuracy and feature support
- Emits deprecation warnings

**Migration Path:**

- Replace with `--parser auto` for best results
- Use `--parser cli` if AST parser is unavailable
- Report any issues that prevent migration from regex parser

## Default Behavior Changes

### Version 0.1.2+ (Current)

- **Default Features**: `ast-parser` included by default
- **Default Mode**: `--parser auto` (AST → CLI fallback)
- **Build Command**: `cargo build` (includes AST parser)

### Previous Versions

- **Default Features**: Only `stdio` (no AST parser)
- **Default Behavior**: CLI → Regex fallback
- **AST Parser**: Required explicit `cargo build --features ast-parser`

## Performance Characteristics

| Parser | Accuracy | Speed | Memory | Dependencies |
|--------|----------|-------|---------|--------------|
| AST | Highest | Fast | Moderate | Tree-sitter |
| CLI | High | Moderate | Low | `just` command |
| Regex | Low | Fastest | Lowest | None |

### AST Parser Advantages

- **Complete syntax support**: Handles all Just language features
- **Error recovery**: Continues parsing despite syntax errors  
- **Rich metadata**: Extracts detailed attribute information
- **Future-proof**: Easy to support new Just language features

### CLI Parser Advantages

- **Just compatibility**: 100% compatible with `just` command behavior
- **Import resolution**: Handles `import` statements correctly
- **Minimal dependencies**: Only requires `just` binary
- **Proven reliability**: Uses official Just parser

### Regex Parser Limitations (Why Deprecated)

- **Limited syntax support**: Cannot handle complex expressions
- **No error recovery**: Fails on unexpected syntax
- **Maintenance burden**: Requires manual updates for new Just features
- **Accuracy issues**: Pattern matching is inherently error-prone

## Build Configuration

### Including AST Parser (Default)

```bash
# Standard build (includes AST parser)
cargo build

# Explicit feature specification
cargo build --features ast-parser

# All features
cargo build --all-features
```

### Excluding AST Parser

```bash
# Minimal build (no AST parser)
cargo build --no-default-features --features stdio

# With other features but no AST parser
cargo build --no-default-features --features "stdio,vector-search"
```

## Environment Detection

just-mcp automatically detects available parsing capabilities:

```bash
# Check configuration
just-mcp --help

# View current parser status in config.json resource
# (Available via MCP protocol at file:///config.json)
```

The configuration resource shows:

- `ast_parser_available`: Whether AST parser was compiled in
- `cli_parser_available`: Whether `just` command is available  
- `default_parser`: Currently configured default parser
- `parser_priority`: Fallback chain for auto mode

## Migration Guide

### From Regex Parser

If you're currently using configurations that rely on regex parsing:

1. **Test with auto mode**:

   ```bash
   just-mcp --parser auto
   ```

2. **Compare results** with your current setup

3. **Report any issues** where AST/CLI parsers don't match regex parser results

4. **Update scripts/configs** to remove any regex-specific workarounds

### From CLI-Only Setups

If you're currently forcing CLI parser usage:

1. **Try auto mode** to get AST parser benefits with CLI fallback:

   ```bash
   just-mcp --parser auto  # Instead of --parser cli
   ```

2. **Keep CLI mode** if you have specific requirements:

   ```bash
   just-mcp --parser cli   # Explicit CLI-only if needed
   ```

## Troubleshooting

### AST Parser Issues

**Problem**: AST parser fails with syntax errors

```bash
# Fallback to CLI parser
just-mcp --parser cli

# Or use auto mode for automatic fallback
just-mcp --parser auto
```

**Problem**: AST parser not available

```bash
# Check if built with AST parser feature
cargo build --features ast-parser

# Or use CLI parser
just-mcp --parser cli
```

### CLI Parser Issues

**Problem**: `just` command not found

```bash
# Install just command
curl --proto '=https' --tlsv1.2 -sSf https://just.systems/install.sh | bash

# Or use AST parser only
just-mcp --parser ast
```

**Problem**: Complex justfile features not parsed

```bash
# Use CLI parser for full compatibility
just-mcp --parser cli
```

### Performance Issues

**Problem**: Parser is too slow

```bash
# Try different parsers to compare
just-mcp --parser ast    # Usually fastest
just-mcp --parser cli    # Moderate speed
just-mcp --parser regex  # Fastest but deprecated
```

## Future Plans

### Short Term (v0.2.0)

- Enhanced AST parser error reporting
- Performance optimizations
- Better fallback logging

### Medium Term (v0.3.0)

- Remove regex parser completely
- Add parser performance metrics
- Enhanced configuration validation

### Long Term (v1.0.0)

- AST parser becomes the only parser
- CLI parser used only for import resolution
- Complete Tree-sitter integration

## Examples

### Development Environment

```bash
# Use AST parser for accuracy during development
just-mcp --admin --parser ast --log-level debug
```

### Production Deployment

```bash
# Use auto mode for reliability
just-mcp --parser auto --log-level info
```

### Testing Environment

```bash
# Test all parsers for compatibility
just-mcp --parser ast && echo "AST OK"
just-mcp --parser cli && echo "CLI OK"
just-mcp --parser auto && echo "Auto OK"
```

### CI/CD Pipeline

```bash
# Ensure consistent builds with AST parser
cargo build --features ast-parser
just-mcp --parser ast --watch-dir ./scripts
```

## See Also

- [AST Parser Architecture](../architecture/ast-parser-architecture.md)
- [AST Parser API Reference](../api/ast-parser-api.md)
- [Migration Guide](../migration/ast-parser-migration.md)
- [Configuration Documentation](../CONFIGURATION.md)
