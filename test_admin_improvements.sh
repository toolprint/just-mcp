#!/bin/bash

# Test admin_create_task improvements

echo "=== Testing admin_create_task with single directory ==="
cd demo

# Test with single directory (no path needed)
{
  echo '{"jsonrpc": "2.0", "method": "initialize", "params": {"clientInfo": {"name": "test", "version": "1.0"}}, "id": 1}'
  sleep 0.5
  echo '{"jsonrpc": "2.0", "method": "initialized", "params": {}, "id": 2}'
  sleep 0.5
  echo '{"jsonrpc": "2.0", "method": "tools/call", "params": {"name": "admin_create_task", "arguments": {"task_name": "test-single", "recipe": "echo test"}}, "id": 3}'
  sleep 2
} | ../target/release/just-mcp 2>/dev/null | grep -A 5 '"id":3' | head -10

echo -e "\n=== Testing with multiple named directories ==="
cd ..

# Create test directories
mkdir -p test-frontend test-backend
echo "# Frontend tasks" > test-frontend/justfile
echo "# Backend tasks" > test-backend/justfile

# Test with multiple directories using name
{
  echo '{"jsonrpc": "2.0", "method": "initialize", "params": {"clientInfo": {"name": "test", "version": "1.0"}}, "id": 1}'
  sleep 0.5
  echo '{"jsonrpc": "2.0", "method": "initialized", "params": {}, "id": 2}'
  sleep 0.5
  echo '{"jsonrpc": "2.0", "method": "tools/call", "params": {"name": "admin_create_task", "arguments": {"justfile_path": "frontend", "task_name": "build", "recipe": "npm run build"}}, "id": 3}'
  sleep 2
} | ./target/release/just-mcp --watch-dir test-frontend:frontend --watch-dir test-backend:backend --admin 2>/dev/null | grep -A 10 '"id":3' | head -15

# Check if task was created
echo -e "\n=== Verifying task creation ==="
if grep -q "build:" test-frontend/justfile; then
    echo "✓ Task created successfully in frontend justfile"
    cat test-frontend/justfile
else
    echo "✗ Task not found in frontend justfile"
fi

# Cleanup
rm -rf test-frontend test-backend