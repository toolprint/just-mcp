# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

just-mcp is a Model Context Protocol (MCP) server that transforms justfiles into AI-accessible automation tools. It monitors directories for justfiles, parses them, and exposes their tasks as MCP tools that AI assistants can discover and execute.

## Essential Development Commands

```bash
# Build and Testing
just build              # Build debug binary
just build-release      # Build optimized release
just test               # Run all tests
just test-coverage      # Generate HTML coverage report with tarpaulin
cargo test --test <name> -- --nocapture  # Run specific test suite with output

# Code Quality
just format             # Auto-format Rust code, JSON, and Markdown
just lint               # Run clippy and format checks
just check              # Run format + lint + test
just pre-commit         # Full validation before committing

# Development Setup
just setup              # Install cargo-tarpaulin for coverage
just brew               # Install macOS dev dependencies (prettier, markdownlint)

# Installation
just install            # Build and install to ~/.cargo/bin
just release-info       # Show release binary information
```

## Architecture Overview

### Core Flow: Justfile → MCP Tool

1. **Watcher** monitors directories for justfile changes
2. **Parser** extracts tasks with parameters, dependencies, descriptions
3. **Registry** converts tasks to MCP tools with JSON schemas
4. **Server** exposes tools via MCP protocol over stdio
5. **Executor** runs just commands when tools are called

### Key Architectural Decisions

- **Async Everything**: Built on Tokio for concurrent operations
- **Channel-Based Communication**: Components communicate via broadcast channels for decoupling
- **Security by Design**: All inputs validated, paths restricted, resources limited
- **Tool Naming**: Format is `just_<task>@<name>` or `just_<task>_<full_path>`

### Module Interactions

```text
main.rs → Server → Registry ← Watcher
                ↓            ↓
             Handler      Parser
                ↓
            Executor → Security + ResourceLimits
```

## Vector Search (Optional Feature)

just-mcp includes optional semantic search capabilities for discovering and understanding justfile tasks across projects. This requires building with the `vector-search` and `local-embeddings` features.

### Key Components

- **LocalEmbeddingProvider**: Offline embedding generation using Candle and sentence transformers
- **VectorSearchManager**: High-level interface combining vector storage and embedding providers
- **LibSqlVectorStore**: SQLite-based vector database with similarity search
- **Model Cache**: Downloads and caches models from Hugging Face Hub

### Build Commands with Vector Search

```bash
# Build with all vector search features
cargo build --features "vector-search,local-embeddings"

# Test vector search functionality
cargo test --features "vector-search,local-embeddings" vector_search
cargo test --features "vector-search,local-embeddings" local_embedding

# CLI commands for vector search
just-mcp search index --local-embeddings     # Index justfiles with local embeddings
just-mcp search query --query "build app" --local-embeddings  # Semantic search
```

### Local Embeddings

The local embedding provider uses the **sentence-transformers/all-MiniLM-L6-v2** model:

- **Dimensions**: 384
- **Model Size**: ~80MB cached locally
- **Advantages**: Offline, private, no API costs
- **First Run**: Downloads model from Hugging Face Hub
- **Cache Location**: `~/.cache/just-mcp/models/`

### Vector Search Architecture

```text
CLI Commands → VectorSearchManager → LocalEmbeddingProvider → ModelCache
                        ↓                                        ↓
              LibSqlVectorStore ← Vector Database ← Hugging Face Hub
                        ↓
               Document Indexing & Similarity Search
```

### Testing Vector Search

```bash
# Unit tests for local embedding provider
cargo test --features "local-embeddings,vector-search" local_embedding

# Integration tests with demo justfile
cargo test --features "local-embeddings,vector-search" local_embedding_interface

# Vector store tests
cargo test --features "vector-search" vector_store

# End-to-end vector search integration
cargo test --features "vector-search,local-embeddings" vector_search_integration
```

### Key Test Files for Vector Search

- `local_embedding_interface_test.rs` - LocalEmbeddingProvider interface tests
- `vector_search_integration_test.rs` - Full vector search workflow tests
- `vector_store_test.rs` - LibSqlVectorStore unit tests

## Testing Strategy

- **Unit Tests**: In each module's `mod.rs` file
- **Integration Tests**: In `tests/` directory
- **Key Test Files**:
  - `mcp_protocol_test.rs` - Protocol compliance
  - `watcher_integration_test.rs` - File monitoring
  - `executor_integration_test.rs` - Task execution
  - `security_test.rs` - Security validation

Run tests for a specific component:

```bash
cargo test parser    # Test parser module
cargo test watcher   # Test file watching
cargo test security  # Test security features
```

## Working with MCP Protocol

The server implements MCP via JSON-RPC 2.0 over stdio:

- **Initialize**: Client sends capabilities, server responds with tool list support
- **Tools List**: Returns all registered justfile tasks as tools
- **Tools Call**: Executes the requested task via just command
- **Notifications**: Sends `tools/list_changed` when justfiles change

Test MCP interactions:

```bash
# Start server
just-mcp --watch-dir ./demo

# In another terminal, send JSON-RPC:
echo '{"jsonrpc": "2.0", "method": "tools/list", "id": 1}' | nc -U stdio
```

## Security Considerations

When modifying security-sensitive code:

1. **Path Validation**: `security::validate_path()` prevents directory traversal
2. **Parameter Sanitization**: `security::validate_*` functions prevent injection
3. **Resource Limits**: Platform-specific limits in `resource_limits` module
4. **Concurrent Execution**: Limited by `ResourceManager` permits

Always add tests for new security validations.

## Adding New Features

### Adding a New Admin Tool

1. Define the tool in `admin/mod.rs::create_admin_tools()`
2. Implement handler in `admin/mod.rs::handle_admin_tool()`
3. Add tests in the admin module

### Supporting New Justfile Syntax

1. Update regex patterns in `parser/mod.rs`
2. Add test cases to `tests/justfile_parser_test.rs`
3. Ensure `to_tool_definition()` handles new fields

### Extending MCP Protocol

1. Add new method to `server/handler.rs::handle_request()`
2. Update capabilities in `handle_initialize()` if needed
3. Add protocol tests to `tests/mcp_protocol_test.rs`

## Common Development Tasks

### Debugging Parser Issues

```bash
# Check parser output for a specific justfile
cargo test parser::tests::test_parse_complex -- --nocapture
```

### Testing File Watching

```bash
# Run watcher tests with debug output
RUST_LOG=debug cargo test watcher_integration_test -- --nocapture
```

### Validating Security

```bash
# Run security tests
cargo test security_test
```

## Performance Considerations

- **Debouncing**: File changes debounced by 500ms to avoid thrashing
- **Hashing**: SHA256 used to detect actual content changes
- **Channels**: Broadcast channels for O(1) event distribution
- **Resource Limits**: Configurable timeouts and output limits

## Error Handling Patterns

The codebase uses:

- `anyhow` for application errors with context
- `thiserror` for typed errors (see `error.rs`)
- `?` operator extensively for error propagation
- Detailed error messages for MCP protocol errors

When adding error handling, provide context:

```rust
.with_context(|| format!("Failed to parse justfile at {}", path.display()))?
```
