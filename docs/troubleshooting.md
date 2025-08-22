# Troubleshooting

## Common Issues and Solutions

### MCP Connection Issues

**Problem**: AI assistant can't connect to just-mcp server

```
Error: Failed to connect to MCP server
```

**Solutions**:

1. Verify just-mcp is in your PATH:

   ```bash
   which just-mcp
   ```

2. Check MCP configuration syntax in settings file
3. Ensure no other process is using the configured port
4. Try running just-mcp manually to see error messages:

   ```bash
   just-mcp --watch-dir /path/to/project
   ```

### Justfile Not Detected

**Problem**: Tasks from justfile aren't appearing in tool list

**Solutions**:

1. Verify file is named exactly `justfile` (case-sensitive)
2. Check file permissions:

   ```bash
   ls -la justfile
   ```

3. Manually trigger sync:

   ```json
   {"method": "tools/call", "params": {"name": "admin_sync"}}
   ```

4. Enable verbose logging to see file detection:

   ```bash
   RUST_LOG=debug just-mcp --verbose
   ```

### Task Execution Failures

**Problem**: Tasks fail with permission denied or command not found

**Solutions**:

1. Ensure `just` is installed and in PATH:

   ```bash
   just --version
   ```

2. Check task has proper shell permissions
3. Verify working directory is correct
4. For complex tasks, test directly with just first:

   ```bash
   just task-name
   ```

### Performance Issues

**Problem**: Slow response times or high resource usage

**Solutions**:

1. Limit watched directories to only necessary paths
2. Increase debounce time for frequently changing files
3. Set appropriate resource limits:

   ```bash
   just-mcp --timeout 60 --output-limit 500000
   ```

4. Monitor system resources during operation

## Platform-Specific Issues

### macOS: "Operation not permitted" errors

- Grant Terminal/IDE full disk access in System Preferences
- Use explicit paths instead of `~/` shortcuts

### Linux: File watching limits exceeded

- Increase inotify limits:

  ```bash
  echo fs.inotify.max_user_watches=524288 | sudo tee -a /etc/sysctl.conf
  sudo sysctl -p
  ```

### Windows: Not currently supported

- Use WSL2 with Linux installation instructions
- Docker container support coming in future release

## Debug Mode

Enable comprehensive debug logging:

```bash
RUST_LOG=just_mcp=debug,tokio=debug just-mcp --verbose
```

Key debug indicators:

- `[WATCHER]`: File system monitoring events
- `[PARSER]`: Justfile parsing details
- `[REGISTRY]`: Tool registration/updates
- `[EXECUTOR]`: Task execution traces

## Getting Help

If issues persist:

1. Check existing issues: [GitHub Issues](https://github.com/onegrep/just-mcp/issues)
2. Gather debug information:

   ```bash
   just-mcp --version
   just --version
   uname -a  # System info
   ```

3. Create a minimal reproducible example
4. Open an issue with debug logs and system details