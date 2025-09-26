#!/bin/bash

# Test the MCP server by sending a analyze_imports tool call

echo "Testing MCP server dynamic dispatch..."

# Create a test JSON request
cat << 'EOF' | node dist/index.js
{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "analyze_imports", "arguments": {"file_path": "/workspace/packages/server/index.ts"}}}
EOF