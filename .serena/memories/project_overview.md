# just-mcp Project Overview

## Purpose

just-mcp is a Model Context Protocol (MCP) server that transforms justfiles into AI-accessible automation tools. It monitors directories for justfiles, parses them, and exposes their tasks as MCP tools that AI assistants can discover and execute.

## Tech Stack

- **Language**: Rust 1.88.0
- **Async Runtime**: Tokio (full features)
- **Serialization**: serde, serde_json
- **CLI**: clap with derive and env features
- **File Monitoring**: notify 6.1
- **Parsing**: regex 1.11
- **Error Handling**: anyhow, thiserror
- **Logging**: tracing with tracing-subscriber
- **Security**: sha2 for hashing, shell-escape for command safety
- **Optional Features**:
  - Vector Search: libsql, rusqlite, ndarray, sqlite-vss
  - Local Embeddings: candle-core, candle-nn, candle-transformers, hf-hub, tokenizers
  - HTTP Transport: axum, tower, hyper

## Project Structure

```
src/
├── main.rs           # Entry point and CLI
├── server/           # MCP protocol implementation
├── parser/           # Justfile parsing logic
├── registry/         # Tool registry management
├── watcher/          # File system monitoring
├── executor/         # Task execution engine
├── admin/            # Administrative tools
├── security/         # Security and validation
├── vector_search/    # Optional semantic search
├── cli/              # CLI command handling
├── notification/     # Change notifications
├── resource_limits/  # Platform-specific limits
├── types/            # Shared types
└── error.rs          # Error definitions
```

## Key Features

1. **Intelligent Justfile Discovery**: Real-time monitoring with filesystem watching
2. **Comprehensive Justfile Parsing**: Full syntax support including parameters, defaults, dependencies
3. **Security & Resource Management**: Input validation, configurable timeouts, memory limits
4. **Administrative Tools**: Manual sync, AI-assisted task creation
5. **Semantic Vector Search** (Optional): Offline-first local embeddings for natural language search
6. **MCP Protocol Compliance**: Full JSON-RPC 2.0 implementation

## Architecture

- **Async Everything**: Built on Tokio for concurrent operations
- **Channel-Based Communication**: Components communicate via broadcast channels for decoupling
- **Security by Design**: All inputs validated, paths restricted, resources limited
- **Tool Naming**: Format is `just_<task>@<name>` or `just_<task>_<full_path>`

## Core Flow

1. **Watcher** monitors directories for justfile changes
2. **Parser** extracts tasks with parameters, dependencies, descriptions
3. **Registry** converts tasks to MCP tools with JSON schemas
4. **Server** exposes tools via MCP protocol over stdio
5. **Executor** runs just commands when tools are called
