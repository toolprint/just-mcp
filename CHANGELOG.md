# Changelog

All notable changes to just-mcp will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0]

### Added

- Complete ultrafast-mcp framework integration for enhanced MCP protocol support
- MCP prompt support with natural language task execution
- AST parser with tree-sitter integration for accurate justfile parsing
- Modular justfile architecture with specialized modules (rust, setup, vector, docker, release)
- Embedded best practices content surfaced via MCP resources
- Enhanced admin tools for task creation and management
- Three-tier parser fallback system (AST → CLI → Regex)
- Comprehensive help system with progressive disclosure

### Changed

- Refactored justfile from monolithic 870-line file to modular architecture
- Improved error handling with standardized messages across modules
- Enhanced parameter validation for all user inputs

### Fixed

- Parser accuracy for complex justfile syntax
- Resource management and concurrent execution limits

## [0.1.1]

### Added

- Vector search capabilities with local embeddings (optional feature)
- Support for sentence-transformers models (all-MiniLM-L6-v2)
- SQLite-based vector storage for semantic search
- Multi-provider embedding support (Local, OpenAI, Mock)

### Changed

- Improved parser support for multiline strings and complex expressions
- Enhanced documentation for AST parser usage

### Fixed

- Parser handling of doc attributes and comments
- File watching debouncing issues

## [0.1.0]

### Added

- Initial release of just-mcp
- Real-time justfile monitoring and parsing
- MCP protocol implementation for AI assistant integration
- Dynamic tool generation from justfile tasks
- Security features: input validation, resource limits, path sanitization
- Multi-project support with named directories
- Hot reloading on justfile changes
- Administrative tools for manual sync and task creation

### Security

- Input validation to prevent command injection
- Path traversal protection
- Configurable resource limits and timeouts

[0.2.0]: https://github.com/toolprint/just-mcp/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/toolprint/just-mcp/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/toolprint/just-mcp/releases/tag/v0.1.0
