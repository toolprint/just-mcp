#!/bin/bash
# Quick manual test for config.json resource

echo "Building just-mcp..."
cargo build --bin just-mcp --quiet || exit 1

echo "Testing config.json resource availability..."

# Start server in background and capture output
echo "Starting just-mcp server..."
timeout 10s ./target/debug/just-mcp --admin --log-level debug > server_output.log 2>&1 &
server_pid=$!
sleep 2

# Test resources/list to see if config.json is available
echo "Testing resources/list..."
echo '{"jsonrpc": "2.0", "method": "resources/list", "id": 1}' | ./target/debug/just-mcp --admin > list_output.json 2>/dev/null &
list_pid=$!
sleep 3

# Kill the list test process
kill $list_pid 2>/dev/null || true

# Test resources/read for config.json
echo "Testing resources/read for config.json..."
echo '{"jsonrpc": "2.0", "method": "resources/read", "id": 2, "params": {"uri": "file:///config.json"}}' | ./target/debug/just-mcp --admin > read_output.json 2>/dev/null &
read_pid=$!
sleep 3

# Kill the read test process
kill $read_pid 2>/dev/null || true

# Kill the server
kill $server_pid 2>/dev/null || true

echo "Test complete. Checking outputs..."

# Check if list_output.json contains config.json
if [ -f "list_output.json" ] && grep -q "config.json" list_output.json; then
    echo "✓ config.json found in resources/list response"
else
    echo "✗ config.json NOT found in resources/list response"
fi

# Check if read_output.json contains valid config data
if [ -f "read_output.json" ] && grep -q '"server"' read_output.json; then
    echo "✓ config.json resource readable with valid content"
else
    echo "✗ config.json resource NOT readable or invalid content"
fi

# Show sample of the config content
if [ -f "read_output.json" ]; then
    echo "Sample config content:"
    cat read_output.json | jq -r '.result.contents[0].text' 2>/dev/null | jq '.server, .cli' 2>/dev/null || echo "Could not parse config content"
fi

# Cleanup
rm -f server_output.log list_output.json read_output.json

echo "Manual test completed."