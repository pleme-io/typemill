# Service Architecture Proposal

## Goal
Enable CodeFlow Buddy to run as a centralized WebSocket service, allowing multiple Docker containers to connect and work on different projects simultaneously.

## Architecture Overview
- **WebSocket MCP Server**: Handles multiple client connections
- **LSP Server Pool**: One LSP server per project/language combination
- **File Streaming Protocol**: Bidirectional file access between client and server
- **Session Management**: Isolates clients working on different projects

## Implementation Phases

### Phase 1: "Make it Work" (Week 1)
**Goal**: Get multi-client, multi-project functionality working with correct architecture.

#### Days 1-2: WebSocket Transport & Sessions
```typescript
// src/transports/websocket.ts
interface ClientSession {
  id: string;
  projectId: string;      // e.g., "frontend-app"
  projectRoot: string;    // Client's local path
  socket: WebSocket;
}

class WebSocketServer {
  private sessions = new Map<string, ClientSession>();

  handleConnection(socket: WebSocket) {
    // First message must be initialization
    socket.once('message', (data) => {
      const init = JSON.parse(data);
      // { "method": "initialize", "project": "frontend-app", "projectRoot": "/app" }

      const session: ClientSession = {
        id: generateId(),
        projectId: init.project,
        projectRoot: init.projectRoot,
        socket
      };

      this.sessions.set(session.id, session);
      this.routeRequests(session);
    });
  }
}
```

#### Days 3-4: LSP Server Pool
```typescript
class LSPServerPool {
  private pools = new Map<string, LSPServer>();

  getServer(projectId: string, language: string): LSPServer {
    const key = `${projectId}:${language}`;

    if (!this.pools.has(key)) {
      // Spawn new LSP server for this project/language
      const server = this.spawnLSPServer(language, projectId);
      this.pools.set(key, server);
    }

    return this.pools.get(key);
  }
}
```

#### Days 5-6: File Streaming Protocol
```typescript
// Bidirectional file operations
interface FileStreamProtocol {
  // Server → Client: Request file content
  'client/readFile': {
    params: { path: string };
    result: { content: string; mtime: number };
  };

  // Client → Server: File changed notification
  'server/fileChanged': {
    params: { path: string; changeType: 'created' | 'changed' | 'deleted' };
  };
}

class StreamingFileAccess {
  async readFile(session: ClientSession, path: string): Promise<string> {
    // Send request to client via WebSocket
    const response = await session.socket.call('client/readFile', { path });
    return response.content;
  }
}
```

#### Day 7: Integration & Testing
- Wire up all components
- Test with 2-3 concurrent clients
- Verify LSP operations work across projects

**Deliverables**:
- ✅ Multiple clients can connect
- ✅ Each project gets its own LSP server
- ✅ Files stream correctly
- ✅ All 25 MCP tools work

### Phase 2: "Make it Right" (Week 2)
**Goal**: Add robustness and production readiness.

#### Days 8-9: Error Handling & Recovery
```typescript
// Connection recovery
class SessionManager {
  handleDisconnect(session: ClientSession) {
    // Keep LSP server alive for 60 seconds
    // Allow reconnection with same project ID
  }

  handleLSPCrash(projectId: string, language: string) {
    // Auto-restart LSP server
    // Replay pending requests
  }
}
```

#### Days 10-11: Docker Deployment
```yaml
# docker-compose.yml
version: '3.8'
services:
  codeflow-service:
    build:
      dockerfile: Dockerfile.service
    ports:
      - "3000:3000"
    environment:
      - MAX_CLIENTS=10
      - FILE_SIZE_LIMIT=10MB

  dev-container-1:
    image: node:20
    environment:
      - MCP_SERVER=ws://codeflow-service:3000
      - PROJECT_ID=frontend-app
```

#### Days 12-14: Basic Performance & Monitoring
- Add request/response logging
- Implement basic rate limiting
- Add health check endpoint
- Simple memory cache (5-second TTL)

**Deliverables**:
- ✅ Handles connection drops gracefully
- ✅ LSP servers auto-restart on crash
- ✅ Docker deployment works
- ✅ Basic monitoring in place

### Phase 3: "Make it Fast" (✅ COMPLETED)
**Goal**: Optimize for production scale.

- ✅ **Advanced Caching**: Intelligent file cache with invalidation
- 💡 **Predictive Loading**: Preload imports when file opens (Future)
- ✅ **Delta Updates**: Send only diffs for large files
- 💡 **Metrics Dashboard**: Prometheus + Grafana (Future)
- ✅ **Authentication**: JWT tokens with refresh
- ✅ **TLS Support**: Secure WebSocket connections
- 💡 **Horizontal Scaling**: Multiple service instances (Future)

## Key Design Decisions

### LSP Server Pool Strategy
- **One LSP server per project/language combination**
- **Shared across clients working on the same project**
- **60-second idle timeout before shutdown**
- **Automatic restart on crash (Phase 2)**

### File Access Strategy
- **Phase 1**: File streaming only (simplest to implement)
- **Phase 2**: Add shared volume support for Docker scenarios
- **Future**: SSHFS for persistent remote development

### Session Management
- **Project-based isolation**: Each client declares its project on connection
- **Stateless after Phase 1**: Can reconnect anytime with same project ID
- **Path translation**: Client paths mapped to server's project namespace

## Project Structure

```
codeflow-buddy/
├── src/
│   ├── transports/
│   │   └── websocket.ts     # New: WebSocket transport
│   ├── server/
│   │   ├── ws-server.ts     # WebSocket server
│   │   ├── session.ts       # Client session management
│   │   └── lsp-pool.ts      # LSP server pool
│   └── fs/
│       └── stream.ts        # File streaming protocol
├── Dockerfile.service        # Service container
└── docker-compose.yml        # Local development setup
```

## Testing Strategy

### Phase 1 Validation
- [ ] 3 clients connect simultaneously
- [ ] Each gets separate LSP server for different projects
- [ ] File streaming works bidirectionally
- [ ] All 25 MCP tools function correctly

### Phase 2 Validation
- [ ] Reconnection after network drop
- [ ] LSP server auto-restart after crash
- [ ] Docker deployment works
- [ ] Performance <100ms latency

## Quick Start Guide

```bash
# Phase 1: Run locally
codeflow-buddy serve --port 3000

# Client connects with:
{
  "method": "initialize",
  "project": "my-app",
  "projectRoot": "/path/to/my-app"
}

# Phase 2: Docker deployment
docker-compose up -d codeflow-service
```

## Example Docker Setup (Phase 2)

```yaml
# docker-compose.yml
version: '3.8'
services:
  codeflow-service:
    build:
      dockerfile: Dockerfile.service
    ports:
      - "3000:3000"
    environment:
      - MAX_CLIENTS=10

  frontend-dev:
    image: node:20
    environment:
      - MCP_SERVER=ws://codeflow-service:3000
      - PROJECT_ID=frontend-app

  backend-dev:
    image: python:3.11
    environment:
      - MCP_SERVER=ws://codeflow-service:3000
      - PROJECT_ID=backend-api
```

## Client Implementation Example

```typescript
// Simple client connection
import WebSocket from 'ws';

const ws = new WebSocket('ws://localhost:3000');

// Initialize session
ws.send(JSON.stringify({
  method: 'initialize',
  project: 'my-app',
  projectRoot: '/workspace/my-app'
}));

// Handle file read requests from server
ws.on('message', async (data) => {
  const msg = JSON.parse(data);
  if (msg.method === 'client/readFile') {
    const content = await fs.readFile(msg.params.path, 'utf8');
    ws.send(JSON.stringify({
      id: msg.id,
      result: { content, mtime: Date.now() }
    }));
  }
});

// Use MCP tools normally
ws.send(JSON.stringify({
  method: 'find_definition',
  params: {
    file_path: '/workspace/my-app/src/index.ts',
    line: 10,
    character: 5
  }
}));
```