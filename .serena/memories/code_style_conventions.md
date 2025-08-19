# Code Style and Conventions

## Rust Version

- Rust 1.88.0 (specified in rust-toolchain.toml)
- Components: rustfmt, clippy

## Code Style

- **Formatting**: Use `cargo fmt` (rustfmt) - enforced by `just format`
- **Linting**: Use `cargo clippy -- -D warnings` - enforced by `just lint`
- **Imports**: Standard Rust import ordering
- **Naming**:
  - Snake_case for functions and variables
  - CamelCase for types and structs
  - UPPER_CASE for constants
  - Prefix internal/helper functions with underscore

## Error Handling

- Use `anyhow` for application errors with context
- Use `thiserror` for typed errors (see error.rs)
- Use `?` operator extensively for error propagation
- Always provide context: `.with_context(|| format!("Failed to parse justfile at {}", path.display()))?`

## Documentation

- Module-level documentation at the top of each file
- Doc comments for public APIs
- Use `///` for item documentation
- Use `//!` for module documentation

## Testing

- Unit tests in each module's `mod.rs` file
- Integration tests in `tests/` directory
- Use `pretty_assertions` for better test output
- Run specific tests: `cargo test parser`

## Async Patterns

- Use `async-trait` for async traits
- Tokio for all async operations
- Prefer channels for inter-component communication

## Security Patterns

- All path inputs validated with `security::validate_path()`
- All parameters sanitized with `security::validate_*` functions
- Resource limits enforced via `ResourceManager`
- Shell commands escaped with `shell-escape`

## Module Organization

- Each major component in its own module directory
- Public API exposed through `mod.rs`
- Internal implementation details kept private
- Tests as submodule within each module

## Feature Flags

- `default = ["stdio"]` - Default features
- Optional features behind feature flags
- Use `#[cfg(feature = "...")]` for conditional compilation

## Logging

- Use `tracing` macros: `trace!`, `debug!`, `info!`, `warn!`, `error!`
- Structured logging with tracing-subscriber
- Environment-based log levels via RUST_LOG

## Performance Considerations

- Debounce file changes by 500ms
- Use SHA256 for change detection
- Broadcast channels for O(1) event distribution
- Configurable resource limits
