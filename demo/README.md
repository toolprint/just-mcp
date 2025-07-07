# Just-MCP Demo Project

This demo project showcases how Just-MCP exposes justfile tasks as MCP tools that can be called by AI assistants.

## Getting Started

### Prerequisites

1. Install `just` command runner:
   ```bash
   # macOS
   brew install just

   # Or using cargo
   cargo install just
   ```

2. Build and run the Just-MCP server:
   ```bash
   # From the project root
   cargo build --release
   ```

### Running the Demo

1. Start the Just-MCP server with this demo directory:
   ```bash
   # From the just-mcp root directory
   ./target/release/just-mcp --watch-dir ./demo
   ```

2. The server will automatically discover and expose all tasks from the `justfile` as MCP tools.

3. In another terminal, you can test the MCP protocol:
   ```bash
   # Example: List available tools
   echo '{"jsonrpc": "2.0", "method": "tools/list", "id": 1}' | nc localhost 3000
   ```

### Available Demo Tasks

The demo justfile includes various task types to demonstrate Just-MCP capabilities:

#### Basic Tasks
- `hello [name]` - Simple greeting with optional parameter
- `system-info` - Display system information
- `clean` - Cleanup temporary files

#### Build & Deploy
- `build [target]` - Simulate building with different targets (debug/release)
- `test [filter]` - Run tests with optional filter
- `deploy [environment]` - Deploy to different environments
- `full-deploy` - Complex task with dependencies

#### Database Operations
- `db-migrate [direction]` - Run database migrations up/down
- `db-seed [count]` - Seed database with sample data

#### File Operations
- `create-config <name> [template]` - Create configuration files
- `process-data <input> [output]` - Process data files
- `backup [destination]` - Create backups

#### Docker & DevOps
- `docker-build [tag]` - Build Docker images
- `docker-push [registry]` - Push to registries
- `monitor [service] [interval]` - Monitor services
- `health-check` - Perform health checks

#### Development Tools
- `api-test <endpoint> [method] [data]` - Test API endpoints
- `benchmark [iterations]` - Run performance tests
- `analyze-logs [level] [limit]` - Analyze log files
- `docs [format]` - Generate documentation

#### Advanced Features
- `version [action]` - Version management (show/bump)
- `secrets [action]` - Secret management (list/rotate)
- `interactive-setup` - Multi-step setup process

### Testing with MCP

Each task becomes an MCP tool with the naming pattern: `just_<taskname>_<justfile_path>`

For example, if the justfile is at `/home/user/demo/justfile`:
- `hello` becomes `just_hello_/home/user/demo/justfile`
- `build` becomes `just_build_/home/user/demo/justfile`

### Example MCP Requests

1. **Initialize connection:**
   ```json
   {
     "jsonrpc": "2.0",
     "method": "initialize",
     "params": {
       "protocolVersion": "2024-11-05",
       "capabilities": {}
     },
     "id": 1
   }
   ```

2. **List available tools:**
   ```json
   {
     "jsonrpc": "2.0",
     "method": "tools/list",
     "params": {},
     "id": 2
   }
   ```

3. **Call a tool (example: hello):**
   ```json
   {
     "jsonrpc": "2.0",
     "method": "tools/call",
     "params": {
       "name": "just_hello_/home/user/demo/justfile",
       "arguments": {
         "name": "MCP User"
       }
     },
     "id": 3
   }
   ```

4. **Call a complex tool (example: deploy):**
   ```json
   {
     "jsonrpc": "2.0",
     "method": "tools/call",
     "params": {
       "name": "just_deploy_/home/user/demo/justfile",
       "arguments": {
         "environment": "production"
       }
     },
     "id": 4
   }
   ```

### Security Features

Just-MCP includes several security features demonstrated in this demo:

1. **Path Validation**: Only justfiles in allowed directories can be accessed
2. **Parameter Sanitization**: All parameters are escaped to prevent injection
3. **Command Validation**: Dangerous patterns in task names are rejected
4. **Resource Limits**: Execution timeouts and output size limits

### Administrative Tools

Just-MCP also provides admin tools for managing justfiles:

- `just_admin_sync` - Manually re-scan justfiles
- `just_admin_create_task` - Create new tasks with AI assistance

### Real-time Updates

If you modify the `justfile` while the server is running, Just-MCP will:
1. Detect the changes automatically
2. Re-parse the justfile
3. Update the available tools
4. Send a notification to connected clients

Try editing the justfile and watch the server logs to see this in action!

### Troubleshooting

1. **Port already in use**: Change the port with `--port` flag
2. **Permission denied**: Ensure justfiles are in allowed directories
3. **Task not found**: Check that task names don't contain special characters
4. **Timeout errors**: Adjust timeout with `--timeout` flag

### Next Steps

1. Try creating your own justfile with custom tasks
2. Integrate Just-MCP with your AI assistant using the MCP protocol
3. Explore the admin tools for dynamic task management
4. Check the main project README for advanced configuration options