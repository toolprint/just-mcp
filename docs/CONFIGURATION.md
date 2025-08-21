# Configuration Settings for just-mcp

This document provides comprehensive documentation of all configuration settings available in the just-mcp system.

## Overview

just-mcp offers extensive configuration options across multiple domains:
- Command-line interface arguments
- Security policies and validation
- Resource limits and performance constraints
- Vector search capabilities
- Model caching for local embeddings
- Server protocol settings

## 1. Command-Line Interface Arguments

### Main Server Configuration

The primary configuration is provided through command-line arguments defined in `src/cli/mod.rs`:

| Argument | Short | Type | Default | Description |
|----------|-------|------|---------|-------------|
| `--watch-dir` | `-w` | Vec<String> | Current directory | Directory to watch for justfiles, optionally with name (format: `path` or `path:name`). Can be specified multiple times. |
| `--admin` | | bool | false | Enable administrative tools |
| `--json-logs` | | bool | false | Enable JSON output for logs |
| `--log-level` | | String | "info" | Log level (trace, debug, info, warn, error) |

#### Watch Directory Format

The `--watch-dir` argument supports two formats:
- `path`: Watch directory at path with auto-generated name
- `path:name`: Watch directory at path with custom name

Example:
```bash
just-mcp --watch-dir ./project1 --watch-dir ./project2:backend --watch-dir /opt/scripts:system
```

### Vector Search Commands (Feature: `vector-search`)

When the `vector-search` feature is enabled, additional subcommands are available:

#### Index Command
```bash
just-mcp search index [OPTIONS]
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `--directory` | PathBuf | Required | Directory to index for vector search |
| `--output` | PathBuf | "./search_index.db" | Output database file path |
| `--local-embeddings` | bool | false | Use local embedding models instead of remote API |
| `--batch-size` | usize | 32 | Batch size for processing |
| `--chunk-size` | usize | 512 | Text chunk size for embedding |

#### Query Command
```bash
just-mcp search query [OPTIONS]
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `--query` | String | Required | Search query text |
| `--database` | PathBuf | "./search_index.db" | Database file to search |
| `--limit` | usize | 10 | Maximum number of results |
| `--local-embeddings` | bool | false | Use local embedding models |
| `--threshold` | f32 | None | Similarity threshold (0.0-1.0) |

#### Stats Command
```bash
just-mcp search stats [OPTIONS]
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `--database` | PathBuf | "./search_index.db" | Database file to analyze |

## 2. Security Configuration

Security settings are defined in the `SecurityConfig` structure (`src/security/mod.rs`):

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `allowed_paths` | Vec<PathBuf> | `["."]` | Allowed directories for justfile access |
| `max_parameter_length` | usize | 1024 | Maximum parameter length to prevent buffer overflow |
| `forbidden_patterns` | Vec<Regex> | See below | Forbidden command patterns for injection prevention |
| `max_parameters` | usize | 50 | Maximum number of parameters per task |
| `strict_mode` | bool | true | Enable strict mode for restrictive validation |

### Default Forbidden Patterns

The security system blocks these patterns by default:
- **Shell injection**: `[;&|]|\$\(|\`` - Prevents command chaining and substitution
- **Path traversal**: `\.\.[/\\]` - Prevents directory traversal attacks  
- **Command substitution**: `\$\{.*\}` - Prevents variable expansion attacks

### Security Validation

The security validator performs these checks:
- **Path validation**: Ensures all paths are within allowed directories
- **Task name validation**: Alphanumeric, underscore, and hyphen only (1-100 chars)
- **Parameter validation**: Checks length, forbidden patterns, and null bytes
- **Command validation**: Warns/blocks potentially dangerous commands

## 3. Resource Limits Configuration

Resource limits are managed through the `ResourceLimits` structure (`src/resource_limits/mod.rs`):

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `max_execution_time` | Duration | 5 minutes | Maximum execution time for tasks |
| `max_memory_bytes` | Option<usize> | None | Maximum memory usage in bytes (platform-dependent) |
| `max_cpu_percent` | Option<u8> | None | Maximum CPU percentage 0-100 (platform-dependent) |
| `max_concurrent_executions` | usize | 10 | Maximum concurrent task executions |
| `max_output_size` | usize | 10MB | Maximum output size (stdout + stderr) |
| `enforce_hard_limits` | bool | true | Kill tasks exceeding limits vs warning only |

### Platform-Specific Limits

- **Unix**: Memory limits use `ulimit`-style constraints, CPU limits use `nice` priority
- **Windows**: Memory and CPU limits log warnings but are not enforced (requires Job Objects API)

## 4. Vector Search Model Cache Configuration

When the `local-embeddings` feature is enabled, model caching is configured via `ModelCacheConfig` (`src/vector_search/model_cache.rs`):

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `cache_dir` | PathBuf | `~/.cache/just-mcp/models/` | Base directory for model cache |
| `max_cache_size` | u64 | 10GB | Maximum total cache size in bytes (0=unlimited) |
| `max_age_days` | u32 | 30 | Maximum age for unused models (0=no expiration) |
| `verify_integrity` | bool | true | Verify model integrity on load using SHA256 |
| `auto_cleanup` | bool | true | Automatically clean up old/unused models |
| `download_timeout_secs` | u64 | 300 | Timeout for model downloads (5 minutes) |

### Model Information Tracking

Each cached model stores this metadata:
- **`model_id`**: Hugging Face Hub identifier
- **`local_path`**: Local storage path
- **`config_hash`**: SHA256 hash for integrity verification
- **`dimension`**: Embedding dimension extracted from config
- **`max_length`**: Maximum sequence length from config
- **`revision`**: Model version/revision if specified

### Required Model Files

The cache downloads these files for each model:
- **Required**: `config.json`, `pytorch_model.bin`, `tokenizer.json`, `tokenizer_config.json`
- **Optional**: `model.safetensors`, `special_tokens_map.json`, `vocab.txt`, `sentence_bert_config.json`, `modules.json`, `README.md`

## 5. Server Protocol Configuration

The MCP server handler (`src/server/handler.rs`) supports these components:

| Component | Type | Description |
|-----------|------|-------------|
| `admin_tools` | Option<AdminTools> | Administrative functionality (_admin_sync, _admin_create_recipe) |
| `security_config` | Option<SecurityConfig> | Security validation and enforcement |
| `resource_limits` | Option<ResourceLimits> | Resource constraint management |
| `resource_provider` | Option<EmbeddedResourceProvider> | Embedded content and documentation |

### Execution Context

Task execution supports these context settings:
- **`working_directory`**: Custom working directory for task execution
- **`environment`**: Environment variables passed to tasks (HashMap<String, String>)
- **`timeout`**: Task-specific timeout in seconds (default: 300)

### Server Capabilities

The server advertises these MCP protocol capabilities:
- **Tools**: Support for listing and calling tools with change notifications
- **Logging**: Standard MCP logging support
- **Resources**: Static resource serving (subscribe: false, list_changed: false)
- **Resource Templates**: Template-based resource generation (list_changed: false)
- **Completion**: Argument completion support

## 6. Feature Flags

Build-time feature configuration is managed in `Cargo.toml`:

| Feature | Dependencies | Description |
|---------|--------------|-------------|
| `stdio` (default) | | Standard I/O transport for MCP protocol |
| `http` | axum, tower, hyper | HTTP transport support |
| `vector-search` | libsql, rusqlite, ndarray, sqlite-vss, reqwest | Vector search and similarity matching |
| `local-embeddings` | candle-core, candle-nn, candle-transformers, hf-hub, tokenizers, dirs, chrono | Local embedding model support |
| `ast-parser` | tree-sitter, tree-sitter-just, streaming-iterator | AST-based justfile parsing |
| `all` | | Enable all features |

### Feature Usage Examples

```bash
# Build with vector search only
cargo build --features vector-search

# Build with local embeddings and vector search
cargo build --features "vector-search,local-embeddings"

# Build with all features
cargo build --features all
```

## 7. Environment Variables

### Logging Configuration

| Variable | Purpose | Example |
|----------|---------|---------|
| `RUST_LOG` | Controls tracing/logging levels | `RUST_LOG=just_mcp=debug,just_mcp::parser=trace` |

The `--log-level` command-line argument overrides `RUST_LOG` environment variable.

### System Paths

The system uses these standard paths:
- **Cache directory**: `dirs::cache_dir()` → `~/.cache/just-mcp/` (Unix) or equivalent
- **Temporary directory**: `std::env::temp_dir()` → fallback for cache operations
- **Working directory**: `std::env::current_dir()` → default watch directory

## 8. Configuration Examples

### Basic Server Setup
```bash
# Start server watching current directory with admin tools
just-mcp --admin --log-level debug
```

### Multi-Directory Monitoring
```bash
# Watch multiple directories with custom names
just-mcp \
  --watch-dir ./frontend:ui \
  --watch-dir ./backend:api \
  --watch-dir /opt/scripts:system \
  --admin \
  --json-logs
```

### Vector Search Indexing
```bash
# Index directory with local embeddings
just-mcp search index \
  --directory ./project \
  --output ./project.db \
  --local-embeddings \
  --batch-size 64 \
  --chunk-size 256
```

### Vector Search Querying
```bash
# Search with similarity threshold
just-mcp search query \
  --query "build and test" \
  --database ./project.db \
  --local-embeddings \
  --limit 5 \
  --threshold 0.7
```

## 9. Default Values Summary

| Category | Setting | Default | Notes |
|----------|---------|---------|-------|
| **CLI** | Watch Directory | Current directory | Auto-detected |
| **CLI** | Log Level | "info" | Can use RUST_LOG env var |
| **CLI** | Admin Tools | Disabled | Security consideration |
| **CLI** | JSON Logs | Disabled | Human-readable by default |
| **Security** | Strict Mode | Enabled | Recommended for production |
| **Security** | Max Parameter Length | 1024 chars | Prevents buffer overflow |
| **Security** | Max Parameters | 50 | Reasonable limit |
| **Resources** | Max Execution Time | 5 minutes | Prevents hanging tasks |
| **Resources** | Max Concurrent Executions | 10 | Balance performance/safety |
| **Resources** | Max Output Size | 10MB | Memory protection |
| **Resources** | Enforce Hard Limits | Enabled | Kill vs warn on violations |
| **Models** | Cache Size | 10GB | Generous for multiple models |
| **Models** | Cache Age | 30 days | Automatic cleanup |
| **Models** | Verify Integrity | Enabled | SHA256 checking |
| **Models** | Auto Cleanup | Enabled | Maintenance automation |

## 10. Security Considerations

### Production Recommendations

1. **Path Restrictions**: Configure `allowed_paths` to specific directories only
2. **Strict Mode**: Keep `strict_mode = true` for parameter validation
3. **Resource Limits**: Set appropriate `max_execution_time` and `max_output_size`
4. **Admin Tools**: Only enable `--admin` when necessary
5. **Logging**: Use structured JSON logs in production with `--json-logs`

### Development Settings

1. **Verbose Logging**: Use `--log-level debug` or `RUST_LOG=just_mcp=debug`
2. **Admin Access**: Enable `--admin` for recipe management
3. **Relaxed Limits**: Increase timeouts for debugging sessions
4. **Local Features**: Use `--local-embeddings` to avoid API dependencies

This configuration system provides comprehensive control over security, performance, resource management, and feature availability in the just-mcp system.