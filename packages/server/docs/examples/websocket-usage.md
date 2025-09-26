# CodeFlow WebSocket Service - Phase 1 Implementation

This document demonstrates the Phase 1 implementation of the multi-client WebSocket service architecture.

## Quick Start

### Start the WebSocket Server

```bash
# Start server on default port (3000)
codeflow-buddy serve

# Start server on custom port with max clients
codeflow-buddy serve --port 3001 --max-clients 10
```

### Client Connection Example

```javascript
import WebSocket from 'ws';
import fs from 'fs';

const ws = new WebSocket('ws://localhost:3000');

// Initialize session
ws.on('open', () => {
  ws.send(JSON.stringify({
    method: 'initialize',
    project: 'my-frontend-app',
    projectRoot: '/workspace/my-app'
  }));
});

// Handle server responses
ws.on('message', async (data) => {
  const message = JSON.parse(data.toString());

  // Handle file read requests from server
  if (message.method === 'client/readFile') {
    try {
      const content = await fs.promises.readFile(message.params.path, 'utf8');
      const stats = await fs.promises.stat(message.params.path);

      ws.send(JSON.stringify({
        id: message.id,
        result: {
          content,
          mtime: stats.mtime.getTime()
        }
      }));
    } catch (error) {
      ws.send(JSON.stringify({
        id: message.id,
        error: { message: error.message }
      }));
    }
  }

  // Handle other server requests (writeFile, fileExists, etc.)
  else if (message.method === 'client/writeFile') {
    try {
      await fs.promises.writeFile(message.params.path, message.params.content);
      ws.send(JSON.stringify({
        id: message.id,
        result: {}
      }));
    } catch (error) {
      ws.send(JSON.stringify({
        id: message.id,
        error: { message: error.message }
      }));
    }
  }
});

// Use MCP tools
function findDefinition(filePath, line, character) {
  return new Promise((resolve, reject) => {
    const id = Math.random().toString(36);

    const handler = (data) => {
      const message = JSON.parse(data.toString());
      if (message.id === id) {
        ws.off('message', handler);
        if (message.error) {
          reject(new Error(message.error.message));
        } else {
          resolve(message.result);
        }
      }
    };

    ws.on('message', handler);

    ws.send(JSON.stringify({
      id,
      method: 'find_definition',
      params: {
        file_path: filePath,
        symbol_name: 'myFunction',
        line,
        character
      }
    }));
  });
}

// Example usage
findDefinition('/workspace/my-app/src/index.ts', 10, 5)
  .then(result => console.log('Definition found:', result))
  .catch(error => console.error('Error:', error));
```

## Architecture Overview

### Components Created

1. **WebSocket Transport** (`src/transports/websocket.ts`)
   - Handles WebSocket connections and message routing
   - Manages request/response correlation
   - Supports client initialization protocol

2. **Session Manager** (`src/server/session.ts`)
   - Tracks active client sessions
   - Associates sessions with projects
   - Manages session lifecycle

3. **LSP Server Pool** (`src/server/lsp-pool.ts`)
   - Manages LSP servers per project/language combination
   - Handles server lifecycle and resource management
   - Implements idle timeout and cleanup

4. **Streaming File Access** (`src/fs/stream.ts`)
   - Bidirectional file operations between server and client
   - Path translation between client and server namespaces
   - File change notification handling

5. **WebSocket Server** (`src/server/ws-server.ts`)
   - Main WebSocket server implementation
   - Integrates all components
   - Routes MCP tool requests through existing tool registry

## Protocol

### Initialization
```json
{
  "method": "initialize",
  "project": "project-id",
  "projectRoot": "/client/project/path"
}
```

### File Operations
```json
// Server -> Client: Read file
{
  "id": "req-123",
  "method": "client/readFile",
  "params": { "path": "/client/path/to/file.ts" }
}

// Client -> Server: Response
{
  "id": "req-123",
  "result": {
    "content": "file content...",
    "mtime": 1640995200000
  }
}
```

### MCP Tool Requests
```json
// Client -> Server: Use MCP tool
{
  "id": "tool-456",
  "method": "find_definition",
  "params": {
    "file_path": "/client/path/to/file.ts",
    "symbol_name": "myFunction",
    "line": 10,
    "character": 5
  }
}
```

## Testing Multi-Client Setup

1. Start the server:
   ```bash
   codeflow-buddy serve --port 3000
   ```

2. Connect multiple clients with different projects:
   - Client 1: project "frontend-app"
   - Client 2: project "backend-api"
   - Client 3: project "mobile-app"

3. Each project gets its own dedicated LSP servers for different languages

4. File operations are isolated per client session

## Current Capabilities

✅ **Multi-client connections** - Multiple clients can connect simultaneously
✅ **Project isolation** - Each project gets dedicated LSP servers
✅ **File streaming** - Bidirectional file access between client and server
✅ **All 31 MCP tools** - Complete tool compatibility through WebSocket
✅ **Session management** - Proper client session tracking
✅ **LSP server pooling** - Efficient resource management

## Next Steps (Phase 2)

- Connection recovery and reconnection handling
- LSP server auto-restart on crash
- Docker deployment configuration
- Enhanced error handling and logging
- Performance monitoring and rate limiting

## Server Stats

The server provides real-time statistics:

```bash
# View server stats (logged every 30 seconds)
Server Stats - Clients: 3, Projects: 2, LSP Servers: 5
```

This indicates:
- 3 active client connections
- 2 different projects being worked on
- 5 LSP server instances running (different language servers across projects)