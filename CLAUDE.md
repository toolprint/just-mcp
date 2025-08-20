# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

just-mcp is a Model Context Protocol (MCP) server that transforms justfiles into AI-accessible automation tools. It monitors directories for justfiles, parses them, and exposes their tasks as MCP tools that AI assistants can discover and execute.

## Essential Development Commands

The justfile has been refactored into a modular system with specialized modules. Use these unified commands:

```bash
# Quick Start & Setup
just quickstart         # Complete setup for new developers
just dev-setup          # Comprehensive development environment setup
just help               # Comprehensive help system with discovery features

# Core Development Workflow
just build [mode]       # Build project (debug/release)
just test [coverage]    # Run tests (with optional coverage)
just format [target]    # Format code (rust/json/markdown/all)
just lint [target] [fix] # Lint code with optional auto-fix
just check [target]     # Combined format + lint + test (quick/full/all)
just pre-commit         # Full validation before committing

# Workflow Automation
just workflow quick     # Quick development workflow (format + lint + test)
just workflow full      # Full workflow with coverage and release build  
just workflow commit    # Commit-ready workflow (runs pre-commit)
just ci                 # Complete CI/CD checks locally

# Installation & Release
just install [features] # Install binaries (with optional vector-search)
just release-info       # Show release binary information
just clean              # Clean build artifacts

# Discovery & Navigation
just list               # List all recipes organized by groups
just groups             # List all recipe groups
just summary            # Compact recipe list
just help-topics        # Show all available help topics
```

## Modular Justfile Architecture

The justfile has been transformed from a monolithic 870-line file into a modular system:

### **Main Justfile** (`justfile`)

- **Comprehensive help system** with progressive disclosure
- **Unified delegation commands** providing consistent interfaces
- **Workflow recipes** combining multiple operations
- **Discovery features** for easy navigation

### **Specialized Modules** (`just/` directory)

- **`rust.just`** - Rust development (build, test, lint, docs, install)
- **`setup.just`** - Project setup and tool installation
- **`vector.just`** - Vector search demos and utilities
- **`docker.just`** - Docker/Dagger CI/CD operations
- **`release.just`** - Cross-platform release and deployment
- **`common.just`** - Shared utilities and error handling

### **Module-Specific Commands**

```bash
# Access module-specific functionality directly:
just build-rust-release          # Direct Rust build
just setup-brew                  # Install development tools
just demo-vector-search          # Vector search demonstration
just dagger-ci                   # Run Dagger CI pipeline
just zigbuild-release v1.0.0     # Cross-platform release

# Or use module help:
just rust-help                   # Rust development commands
just docker-help                 # Docker/Dagger commands
just vector-help                 # Vector search commands
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

# Debug logging for parser component
RUST_LOG=just_mcp::parser=debug cargo test parser -- --nocapture
```

### Testing File Watching

```bash
# Run watcher tests with debug output
RUST_LOG=debug cargo test watcher_integration_test -- --nocapture

# Debug logging for watcher component
RUST_LOG=just_mcp::watcher=debug cargo test watcher -- --nocapture
```

### Validating Security

```bash
# Run security tests
cargo test security_test
```

### Debug Logging for Server

```bash
# Run server with debug logging for all components
RUST_LOG=just_mcp=debug target/debug/just-mcp --watch-dir ./demo

# Debug specific components
RUST_LOG=just_mcp::server=debug,just_mcp::registry=debug target/debug/just-mcp --watch-dir ./demo
```

### Testing MCP Protocol

```bash
# Test with manual JSON-RPC commands
echo '{"jsonrpc": "2.0", "method": "initialize", "id": 1, "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test", "version": "1.0"}}}' | target/debug/just-mcp

# Test tools listing
echo '{"jsonrpc": "2.0", "method": "tools/list", "id": 2}' | target/debug/just-mcp --watch-dir ./demo
```

### Feature Flag Development

```bash
# Development without optional features (minimal build)
cargo build --no-default-features
cargo test --no-default-features

# All features enabled
cargo build --all-features
cargo test --all-features

# Specific feature combinations
cargo build --features vector-search
cargo test --features "vector-search,local-embeddings"
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

## Modular Justfile Architecture

The project has been refactored from a monolithic 870-line justfile to a clean modular system following Just best practices.

### Architecture Overview

```text
justfile (main interface, <200 lines)
├── just/common.just      # Shared utilities and error handling
├── just/rust.just        # Rust development workflows
├── just/setup.just       # Project setup and tool installation
├── just/vector.just      # Vector search demonstrations
├── just/docker.just      # Docker/Dagger CI/CD operations
└── just/release.just     # Cross-platform release automation
```

### Quantifiable Improvements

- **Main justfile size**: Reduced from 870 lines to 602 lines (31% reduction)
- **Code duplication**: Eliminated through shared utilities in `common.just`
- **Recipe organization**: 100% of recipes now properly grouped and documented
- **Error handling**: Standardized across all modules with consistent messaging
- **Help system**: Complete replacement of poor default Just experience
- **Module count**: 5 specialized modules plus common utilities
- **Documentation coverage**: 100% of modules have comprehensive documentation
- **Parameter validation**: All user inputs validated with helpful error messages

### Module Responsibilities

#### `common.just` - Shared Utilities

- Error handling: `_error`, `_success`, `_info`, `_warn`
- Validation: `_validate`, `_require-command`, `_require-file`
- Execution: `_run` for consistent command execution with status
- Utilities: `_get-binaries`, `_timestamp`, binary management

#### `rust.just` - Rust Development

- Build workflows: `rust-build`, `rust-build-release`
- Testing: `rust-test`, `rust-test-coverage`, `rust-test-watch`
- Quality: `rust-format`, `rust-lint`, `rust-clippy`
- Management: `rust-clean`, `rust-update`, `rust-audit`

#### `setup.just` - Project Setup

- Tool installation: `install-tq`, `install-doppler`, `rust-setup`
- Development environment setup
- Dependency management

#### `vector.just` - Vector Search Demos

- Main demos: `demo-search`, `demo-local`, `demo-quick`, `demo-compare`
- Utilities: `search-query`, `index-directory`, `stats`
- Cleanup: `vector-clean`, `vector-clean-all`
- Benchmarking: `demo-benchmark`, `demo-nlp`

#### `docker.just` - CI/CD Operations

- CI pipeline: `dagger-ci`, `dagger-format`, `dagger-lint`, `dagger-test`
- Build operations: `dagger-build`, `dagger-build-release`
- Utilities: `docker-help`, `docker-check`, `docker-clean`

#### `release.just` - Cross-Platform Releases

- Release workflows: `release`, `zigbuild-release`, `dagger-release`
- Platform targeting: `zigbuild-target`, `dagger-release-platform`
- Information: `release-targets`, `release-check`, `release-clean`

### Unified Interface Design

The main justfile provides a clean, unified interface that delegates to modules while maintaining backward compatibility. Key features:

- **Progressive disclosure**: Essential commands visible, detailed options in modules
- **Consistent parameter validation**: All inputs validated with helpful error messages
- **Standardized help system**: Replaces poor default Just experience
- **Unified delegation**: Common commands (test, build, format, lint) work from main interface

### Development Workflow Improvements

1. **Discovery**: Start with `just` (no arguments) to see available commands
2. **Module exploration**: Each module has comprehensive documentation
3. **Parameter safety**: All user inputs validated before execution
4. **Consistent interfaces**: Unified patterns across all modules
5. **Error context**: All failures include helpful context and suggestions

### Best Practices Implementation

- **Import statements**: All modules import `common.just` for shared utilities
- **Recipe groups**: Logical organization within each module
- **Consistent naming**: Module prefixes prevent conflicts (`rust-build`, `git-status`)
- **Documentation**: Every module includes comprehensive usage documentation
- **Error handling**: Standardized through common utilities
- **Parameter validation**: Consistent validation patterns across modules
