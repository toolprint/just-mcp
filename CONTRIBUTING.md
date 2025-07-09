# Contributing to just-mcp

Thank you for your interest in contributing to just-mcp! This document provides guidelines and instructions for contributing to the project.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/your-username/just-mcp.git`
3. Create a feature branch: `git checkout -b feature/your-feature-name`
4. Make your changes
5. Commit your changes: `git commit -m "feat: add amazing feature"`
6. Push to your fork: `git push origin feature/your-feature-name`
7. Open a Pull Request

## Development Setup

### Prerequisites

- Rust 1.70 or higher
- Just command runner
- Development tools (see below)

### Installing Dependencies

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install just
cargo install just

# Install development dependencies
just setup  # Installs cargo-tarpaulin
just brew   # macOS: installs prettier, markdownlint, etc.
```

### Building the Project

```bash
just build         # Debug build
just build-release # Release build
```

## Making Changes

### Branch Naming

Use descriptive branch names:
- `feature/add-xyz-support`
- `fix/issue-123-description`
- `docs/update-readme`
- `refactor/improve-parser`

### Development Workflow

1. **Create a new branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes** following the coding standards

3. **Run tests and checks**:
   ```bash
   just check  # Runs format, lint, and test
   ```

4. **Commit your changes**:
   ```bash
   git add .
   git commit -m "feat: add support for XYZ"
   ```

## Commit Messages

We follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` New features
- `fix:` Bug fixes
- `docs:` Documentation changes
- `style:` Code style changes (formatting, missing semicolons, etc)
- `refactor:` Code refactoring
- `test:` Adding or updating tests
- `chore:` Maintenance tasks

## Coding Standards

### Rust Guidelines

- Follow Rust idioms and best practices
- Use meaningful variable and function names
- Add comments for complex logic
- Prefer clarity over cleverness

### Code Organization

- Keep modules focused and cohesive
- Use appropriate visibility modifiers
- Group related functionality
- Follow existing project structure

### Error Handling

- Use `anyhow` for application errors
- Use `thiserror` for library errors
- Provide context with `.context()`
- Handle all error cases appropriately

## Testing

### Running Tests

```bash
just test          # Run all tests
just test-coverage # Generate coverage report
cargo test parser  # Test specific module
```

### Writing Tests

- Add unit tests in module files
- Add integration tests in `tests/` directory
- Test edge cases and error conditions
- Use descriptive test names

### Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_description() {
        // Arrange
        let input = "test data";
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert_eq!(result, expected);
    }
}
```

## Documentation

### Code Documentation

- Document public APIs with doc comments
- Include examples in doc comments
- Update README for user-facing changes
- Keep CLAUDE.md updated for AI context

### Documentation Standards

```rust
/// Brief description of the function.
///
/// More detailed explanation if needed.
///
/// # Arguments
///
/// * `param` - Description of parameter
///
/// # Returns
///
/// Description of return value
///
/// # Examples
///
/// ```
/// let result = function(param);
/// ```
pub fn function(param: Type) -> Result<ReturnType> {
    // Implementation
}
```

## Pull Request Process

1. Update the README.md if needed
2. Ensure all tests pass
3. Update documentation for API changes
4. Get at least one code review approval
5. Squash commits if requested

### PR Requirements

- All tests must pass
- Code must be formatted (`just format`)
- No clippy warnings (`just lint`)
- Documentation updated if needed
- Follows project coding standards

## Code of Conduct

Please read our [Code of Conduct](CODE_OF_CONDUCT.md) before contributing.

## Questions?

Feel free to open an issue for any questions about contributing.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
