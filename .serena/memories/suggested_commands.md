# Suggested Development Commands

## Essential Commands

### Build and Testing

```bash
just build              # Build debug binary
just build-release      # Build optimized release
just test               # Run all tests
just test-coverage      # Generate HTML coverage report with tarpaulin
cargo test --test <name> -- --nocapture  # Run specific test suite with output
```

### Code Quality (MUST RUN BEFORE COMMITS)

```bash
just format             # Auto-format Rust code, JSON, and Markdown
just lint               # Run clippy and format checks
just check              # Run format + lint + test
just pre-commit         # Full validation before committing
```

### Development Setup

```bash
just setup              # Install cargo-tarpaulin for coverage
just brew               # Install macOS dev dependencies (prettier, markdownlint)
```

### Installation

```bash
just install            # Build and install to ~/.cargo/bin
just install-with-vector-search  # Install with vector search features
just release-info       # Show release binary information
```

## Debug Commands

### Component-Specific Testing

```bash
cargo test parser       # Test parser module
cargo test watcher      # Test file watching
cargo test security     # Test security features
cargo test vector_search --features "vector-search,local-embeddings"  # Test vector search
```

### Debug Logging

```bash
RUST_LOG=just_mcp=debug target/debug/just-mcp --watch-dir ./demo
RUST_LOG=just_mcp::parser=debug cargo test parser -- --nocapture
RUST_LOG=just_mcp::watcher=debug cargo test watcher -- --nocapture
```

### MCP Protocol Testing

```bash
# Start server
just-mcp --watch-dir ./demo

# Test with JSON-RPC
echo '{"jsonrpc": "2.0", "method": "tools/list", "id": 1}' | target/debug/just-mcp
```

## CI/CD Commands

### Dagger Pipeline

```bash
just dagger-ci          # Run complete CI pipeline locally
just dagger-test        # Run tests in container
just dagger-coverage    # Generate coverage report
just dagger-release version="v1.0.0"  # Build all platforms in parallel
```

### Cross-Platform Builds

```bash
just zigbuild-release version="v1.0.0"  # Build all platforms using cargo-zigbuild
just zigbuild-test target="x86_64-apple-darwin"  # Test specific platform
```

## Vector Search Commands

```bash
# Build with vector search
cargo build --features "vector-search,local-embeddings"

# Demo commands
just demo-vector-search      # Basic vector search demo
just demo-vector-local       # Local embeddings demo
just demo-vector-compare     # Compare local vs mock embeddings
just demo-vector-nlp         # Natural language processing demo

# CLI usage
just-mcp search index --local-embeddings
just-mcp search query --query "build app" --local-embeddings
```

## Git Commands

```bash
git status              # Check current branch status
git checkout -b feat/new-feature  # Create new feature branch
```

## System Utilities (Linux)

- ls, cd, grep, find - Standard Unix commands
- Git for version control
- Just command runner (required)

## Important Notes

- Always run `just format` before committing
- Run `just lint` to check for issues
- Use `just pre-commit` for comprehensive validation
- For vector search features, use `--features "vector-search,local-embeddings"`
