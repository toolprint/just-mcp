# AST Parser Migration Guide

This guide helps users and developers migrate from regex-based parsing to the new AST-based parser in just-mcp.

## Overview

just-mcp has evolved from a regex-based parser to a sophisticated AST-based parser using Tree-sitter. This migration brings significant improvements in accuracy, reliability, and feature support.

## Migration Timeline

- **v0.1.0 - v0.1.2**: Regex-based parser (legacy)
- **v0.2.0**: AST parser introduced as optional feature
- **v0.3.0**: AST parser becomes default when available
- **Future**: Regex parser maintained for fallback only

## Key Benefits of AST Parser

### 1. **Accuracy Improvements**
- Handles complex Just syntax correctly (multiline strings, nested expressions)
- Proper parsing of attributes like `[private]`, `[confirm]`, `[group('name')]`
- Accurate dependency extraction including parameterized dependencies
- Correct handling of doc comments and parameter descriptions

### 2. **Performance Benefits**
- Parser reuse across multiple files
- Optimized query execution with caching
- Parallel parsing capabilities
- Consistent performance regardless of file complexity

### 3. **Feature Support**
- Full support for all Just language features
- Proper error recovery and partial parsing
- Structured error messages with line/column information
- Future-proof for new Just syntax additions

## Migration Steps

### For Users

#### 1. **Default Behavior (No Action Required)**

If you're using just-mcp with default settings, the AST parser is automatically used when available:

```bash
# AST parser is used by default
just-mcp --watch-dir ./my-project
```

#### 2. **Verifying AST Parser Usage**

Check logs to confirm AST parser initialization:

```bash
RUST_LOG=just_mcp::parser=info just-mcp --watch-dir ./my-project
# Look for: "AST parser initialized successfully - using as primary parser"
```

#### 3. **Building with AST Support**

Ensure you're building with the AST parser feature:

```bash
# Recommended: Build with all features
cargo install just-mcp --features all

# Or specifically with AST parser
cargo install just-mcp --features ast-parser
```

### For Developers

#### 1. **Feature Flag Configuration**

The AST parser is behind the `ast-parser` feature flag:

```toml
[dependencies]
just-mcp = { version = "0.3", features = ["ast-parser"] }
```

#### 2. **Direct Parser Usage**

```rust
use just_mcp::parser::EnhancedJustfileParser;

// AST parser is used by default when available
let parser = EnhancedJustfileParser::new()?;
let tasks = parser.parse_file(Path::new("justfile"))?;

// Check if AST parsing is available
if parser.is_ast_parsing_available() {
    println!("Using AST parser");
}
```

#### 3. **Handling Fallback Scenarios**

The parser automatically falls back when Tree-sitter is unavailable:

```rust
// Parser selection happens automatically:
// 1. Try AST parser (if available and enabled)
// 2. Fall back to CLI parser (just --summary)
// 3. Fall back to regex parser
// 4. Create minimal error task if all fail
```

## Configuration Options

### Environment Variables

```bash
# Enable detailed parser logging
export RUST_LOG=just_mcp::parser=debug

# Run with specific parser preferences
just-mcp --watch-dir . --prefer-ast  # Force AST parser (default)
```

### Programmatic Configuration

```rust
let mut parser = EnhancedJustfileParser::new()?;

// Force specific parser usage (for testing/debugging)
parser.set_ast_parser_only();      // Use only AST parser
parser.set_command_parser_only();  // Use only CLI parser
parser.set_legacy_parser_only();   // Use only regex parser

// Enable/disable parsers
parser.set_ast_parsing_enabled(true);   // Enable AST (default)
parser.set_command_parsing_enabled(true); // Enable CLI fallback
```

## Compatibility Notes

### Behavioral Differences

1. **Private Task Detection**
   - Regex: Only detected tasks starting with `_`
   - AST: Properly detects `[private]` attribute

2. **Parameter Parsing**
   - Regex: Basic parameter extraction
   - AST: Full parameter metadata including types and validation

3. **Dependency Resolution**
   - Regex: Simple space-separated dependencies
   - AST: Handles parameterized dependencies like `build(target)`

### Breaking Changes

None! The AST parser maintains full backward compatibility:
- Same `JustTask` structure returned
- Same MCP tool interface exposed
- Graceful fallback ensures continuous operation

## Troubleshooting

### AST Parser Not Available

If you see "Failed to initialize AST parser" in logs:

1. **Check Build Features**
   ```bash
   # Verify just-mcp was built with ast-parser feature
   just-mcp --version
   ```

2. **Tree-sitter Dependencies**
   - Ensure system has required libraries
   - On some systems, you may need to install build tools

3. **Force Fallback Mode**
   ```bash
   # Temporarily disable AST parser
   just-mcp --watch-dir . --no-ast
   ```

### Performance Issues

If parsing seems slow:

1. **Check Parser Metrics**
   ```rust
   let diagnostics = parser.get_diagnostics();
   println!("{}", diagnostics);
   ```

2. **Review Fallback Patterns**
   - Frequent fallbacks indicate parsing issues
   - Check justfile syntax for errors

### Parsing Differences

If tasks appear different with AST parser:

1. **More Accurate Parsing**
   - AST parser may reveal previously hidden syntax errors
   - Some edge cases are now handled correctly

2. **Enhanced Metadata**
   - Additional fields may be populated (groups, attributes)
   - Parameter descriptions now properly extracted

## Best Practices

1. **Always Build with AST Support**
   ```bash
   cargo build --features ast-parser
   ```

2. **Monitor Parser Performance**
   ```rust
   let metrics = parser.get_metrics();
   if metrics.ast_success_rate() < 0.8 {
       // Investigate parsing issues
   }
   ```

3. **Test with Both Parsers**
   During migration, test your justfiles with both parsers to ensure compatibility

4. **Report Issues**
   If you find parsing differences or issues, please report them with:
   - The justfile content
   - Expected vs actual parsing results
   - Parser diagnostics output

## Future Roadmap

- **v0.4.0**: Enhanced Tree-sitter grammar with custom extensions
- **v0.5.0**: Performance optimizations and query caching improvements
- **v1.0.0**: Regex parser deprecated, AST parser mandatory

## Support

For migration assistance:
- Check the [AST Parser Documentation](./ast-parser-architecture.md)
- Review [Parser API Documentation](./ast-parser-api.md)
- Submit issues on GitHub with the `ast-parser` label