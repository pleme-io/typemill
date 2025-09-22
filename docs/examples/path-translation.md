# Path Translation and File Change Notifications

This document demonstrates the corrected path translation handling and bidirectional file change notifications.

## Path Translation Example

### Client Setup
```javascript
// Client running in Docker container
const projectRoot = '/app';  // Client's project root
const ws = new WebSocket('ws://localhost:3000');

// Initialize session
ws.send(JSON.stringify({
  method: 'initialize',
  project: 'frontend-app',
  projectRoot: '/app'  // Client's absolute project root
}));
```

### Path Translation in Action

When the client sends an MCP tool request:

```javascript
// Client sends with absolute path
ws.send(JSON.stringify({
  id: 'req-123',
  method: 'find_definition',
  params: {
    file_path: '/app/src/components/Button.tsx',  // Client absolute path
    symbol_name: 'Button',
    line: 10,
    character: 5
  }
}));
```

**Server Processing:**
1. **Original client path:** `/app/src/components/Button.tsx`
2. **Converted to project-relative:** `src/components/Button.tsx` (via `toProjectPath()`)
3. **LSP server receives:** `src/components/Button.tsx`
4. **File extension extracted from:** `src/components/Button.tsx` → `tsx`

### File Change Notifications (Bidirectional)

#### Server → Client: File Read Request
```javascript
// Server requests file content from client
{
  "id": "file-read-456",
  "method": "client/readFile",
  "params": {
    "path": "/app/src/components/Button.tsx"  // Client absolute path
  }
}

// Client responds with file content
{
  "id": "file-read-456",
  "result": {
    "content": "export const Button = ...",
    "mtime": 1640995200000
  }
}
```

#### Client → Server: File Change Notification
```javascript
// Client notifies server of file changes
ws.send(JSON.stringify({
  method: 'server/fileChanged',
  params: {
    path: '/app/src/components/Button.tsx',  // Client absolute path
    changeType: 'changed'
  }
}));
```

**Server Processing:**
1. **Original client path:** `/app/src/components/Button.tsx`
2. **Converted to project-relative:** `src/components/Button.tsx`
3. **Logged as:** `File changed: src/components/Button.tsx in project frontend-app`

## Multi-Project Path Isolation

### Project A (Frontend)
- **Client Root:** `/app`
- **File:** `/app/src/index.tsx`
- **Project Path:** `src/index.tsx`
- **LSP Server Key:** `frontend-app:typescript`

### Project B (Backend)
- **Client Root:** `/workspace/api`
- **File:** `/workspace/api/src/main.py`
- **Project Path:** `src/main.py`
- **LSP Server Key:** `backend-api:python`

## Complete Client Implementation Example

```javascript
import WebSocket from 'ws';
import fs from 'fs/promises';
import { watch } from 'fs';

class CodeFlowClient {
  constructor(serverUrl, projectId, projectRoot) {
    this.ws = new WebSocket(serverUrl);
    this.projectId = projectId;
    this.projectRoot = projectRoot;
    this.pendingRequests = new Map();

    this.setupEventHandlers();
    this.setupFileWatcher();
  }

  setupEventHandlers() {
    this.ws.on('open', () => {
      // Initialize session
      this.ws.send(JSON.stringify({
        method: 'initialize',
        project: this.projectId,
        projectRoot: this.projectRoot
      }));
    });

    this.ws.on('message', async (data) => {
      const message = JSON.parse(data.toString());

      // Handle file read requests from server
      if (message.method === 'client/readFile') {
        await this.handleFileRead(message);
      }
      // Handle other file operations...
      else if (message.method === 'client/writeFile') {
        await this.handleFileWrite(message);
      }
      // Handle responses to our requests
      else if (message.id && this.pendingRequests.has(message.id)) {
        const { resolve, reject } = this.pendingRequests.get(message.id);
        this.pendingRequests.delete(message.id);

        if (message.error) {
          reject(new Error(message.error.message));
        } else {
          resolve(message.result);
        }
      }
    });
  }

  async handleFileRead(message) {
    try {
      const content = await fs.readFile(message.params.path, 'utf8');
      const stats = await fs.stat(message.params.path);

      this.ws.send(JSON.stringify({
        id: message.id,
        result: {
          content,
          mtime: stats.mtime.getTime()
        }
      }));
    } catch (error) {
      this.ws.send(JSON.stringify({
        id: message.id,
        error: { message: error.message }
      }));
    }
  }

  async handleFileWrite(message) {
    try {
      await fs.writeFile(message.params.path, message.params.content);
      this.ws.send(JSON.stringify({
        id: message.id,
        result: {}
      }));
    } catch (error) {
      this.ws.send(JSON.stringify({
        id: message.id,
        error: { message: error.message }
      }));
    }
  }

  setupFileWatcher() {
    // Watch for file changes and notify server
    watch(this.projectRoot, { recursive: true }, (eventType, filename) => {
      if (filename) {
        const fullPath = `${this.projectRoot}/${filename}`;

        // Notify server of file change
        this.ws.send(JSON.stringify({
          method: 'server/fileChanged',
          params: {
            path: fullPath,
            changeType: eventType === 'rename' ? 'created' : 'changed'
          }
        }));
      }
    });
  }

  // Use MCP tools
  async findDefinition(filePath, symbolName, line, character) {
    return new Promise((resolve, reject) => {
      const id = Math.random().toString(36);
      this.pendingRequests.set(id, { resolve, reject });

      this.ws.send(JSON.stringify({
        id,
        method: 'find_definition',
        params: {
          file_path: filePath,  // Send client absolute path
          symbol_name: symbolName,
          line,
          character
        }
      }));
    });
  }
}

// Usage
const client = new CodeFlowClient(
  'ws://localhost:3000',
  'my-frontend-app',
  '/app'
);

// Use MCP tools with client absolute paths
client.findDefinition('/app/src/index.tsx', 'MyComponent', 10, 5)
  .then(result => console.log('Definition:', result))
  .catch(error => console.error('Error:', error));
```

## Key Benefits of Path Translation

1. **Project Isolation:** Multiple clients can work on different projects without path conflicts
2. **LSP Compatibility:** LSP servers receive project-relative paths as expected
3. **File System Abstraction:** Server doesn't need to know about client file system layout
4. **Docker Support:** Works seamlessly with containers having different mount points
5. **Bidirectional Communication:** Both file reads and change notifications work correctly

The server now properly handles path translation in both directions:
- **Incoming requests:** Client absolute → Project relative
- **Outgoing requests:** Project relative → Client absolute (via toClientPath)
- **File notifications:** Client absolute → Project relative for logging/processing