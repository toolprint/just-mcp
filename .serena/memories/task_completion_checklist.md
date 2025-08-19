# Task Completion Checklist

When completing any development task in just-mcp, follow these steps:

## 1. Code Quality Checks

- [ ] Run `just format` to auto-format code
- [ ] Run `just lint` to check for clippy warnings
- [ ] Fix any formatting or linting issues

## 2. Testing

- [ ] Run `just test` to ensure all tests pass
- [ ] If you modified a specific module, run targeted tests:
  - `cargo test parser` for parser changes
  - `cargo test watcher` for watcher changes
  - `cargo test security` for security changes
  - `cargo test vector_search --features "vector-search,local-embeddings"` for vector search

## 3. Feature-Specific Testing

- [ ] If working with vector search: `cargo test --features "vector-search,local-embeddings"`
- [ ] If modifying MCP protocol: Test with JSON-RPC commands
- [ ] If changing security: Run `cargo test security_test`

## 4. Pre-Commit Validation

- [ ] Run `just pre-commit` for full validation
- [ ] This runs: format check, clippy, and all tests
- [ ] Fix any issues before proceeding

## 5. Documentation

- [ ] Update relevant documentation if APIs changed
- [ ] Update CLAUDE.md if adding new development workflows
- [ ] Add/update doc comments for new public APIs

## 6. Final Steps

- [ ] Verify changes work as expected
- [ ] Check that no debug/temporary code remains
- [ ] Ensure error messages are helpful and include context

## Critical Commands

The most important command to remember:

```bash
just pre-commit  # Run this before any commit
```

This ensures your code meets all quality standards.

## Notes

- The codebase uses Rust 1.88.0
- All async code uses Tokio
- Security is paramount - validate all inputs
- Use anyhow for errors with `.with_context()`
