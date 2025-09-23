#!/bin/bash

# Quick start script for multi-tenant FUSE service

echo "🚀 Starting Multi-Tenant FUSE Service..."

# Create necessary directories
mkdir -p /tmp/workspaces /tmp/fuse-mounts

# Start the WebSocket server with FUSE enabled
node dist/index.js serve \
  --port 3000 \
  --enable-fuse \
  --max-clients 10 &

echo "✅ Service running on ws://localhost:3000"
echo "📁 Workspaces: /tmp/workspaces"
echo "🗂️ FUSE Mounts: /tmp/fuse-mounts"
echo ""
echo "Each WebSocket connection gets:"
echo "  - Isolated workspace directory"
echo "  - FUSE mount point for filesystem access"
echo "  - Automatic cleanup on disconnect"