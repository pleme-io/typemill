# Service Architecture Proposal

## Goal
Enable CodeFlow Buddy to run as a centralized WebSocket service, allowing multiple clients to share LSP servers.

## Overview
Run CodeFlow Buddy as a centralized service in Docker, allowing multiple development environments to connect via MCP protocol. This enables one LSP setup to serve all projects and clients.

## Implementation Strategy

### Phase 1: WebSocket Transport (Week 1)

**Success Criteria**: Existing MCP tools work over WebSocket with single client.

```bash
# New command
codeflow-buddy serve --port 3000

# Client connects via WebSocket instead of stdio
```

**Implementation**:
```typescript
// src/transports/websocket.ts
class WebSocketTransport implements Transport {
  // Adapts MCP protocol from stdio to WebSocket
}
```

**Validation**: Run existing test suite through WebSocket transport.

### Phase 2: Multi-Client Support (Week 2)

**Success Criteria**: 3 concurrent clients can use different projects simultaneously.

**Authentication**: Simple token-based auth for MVP
```typescript
// Connection handshake
{
  "method": "initialize",
  "token": "shared-secret-from-env",
  "projectRoot": "/local/project/path"
}
```

**Session Management**:
```typescript
interface ClientSession {
  id: string;
  token: string;
  projectRoot: string;    // Client's local path
  serverRoot?: string;    // Mapped server path (if using shared volume)
}
```

### Phase 3: File Access (Week 2-3)

**Unified Strategy**: Auto-detect based on client configuration.

```json
// Client config in .mcp/config.json
{
  "transport": "websocket",
  "url": "ws://localhost:3000",
  "fileAccess": "stream",  // Options: "shared", "stream", "sshfs"
  "projectPath": "/Users/me/project",
  "serverPath": "/code/project"  // Only for "shared" mode
}
```

**Priority Order**:
1. **Shared Volume** (Docker): Direct file access via mapped paths
2. **Stream** (Remote): Client streams file content on-demand
3. **SSHFS** (Future): For persistent remote development

**Path Translation**:
```typescript
function translatePath(clientPath: string, session: ClientSession): string {
  if (session.serverRoot) {
    // Simple prefix replacement for shared volumes
    return clientPath.replace(session.projectRoot, session.serverRoot);
  }
  // For streaming mode, return as-is (will be fetched from client)
  return clientPath;
}
```

## LSP Server Pool Management

### Hybrid Approach: One LSP server per project root, shared across clients working on the same project.

```typescript
interface LSPPoolEntry {
  projectRoot: string;      // e.g., "/code/project1"
  language: string;         // e.g., "typescript"
  server: LSPServer;
  clients: Set<string>;     // Client IDs using this server
  lastActivity: Date;
}

class LSPServerPool {
  private pools = new Map<string, LSPPoolEntry>();

  getServer(session: ClientSession, language: string): LSPServer {
    const key = `${session.serverRoot}:${language}`;

    if (!this.pools.has(key)) {
      // Spawn new LSP server for this project/language combo
      const server = this.spawnLSPServer(language, session.serverRoot);
      this.pools.set(key, {
        projectRoot: session.serverRoot,
        language,
        server,
        clients: new Set([session.id]),
        lastActivity: new Date()
      });
    } else {
      // Reuse existing server, add client to set
      const entry = this.pools.get(key);
      entry.clients.add(session.id);
      entry.lastActivity = new Date();
    }

    return this.pools.get(key).server;
  }

  releaseServer(session: ClientSession, language: string) {
    const key = `${session.serverRoot}:${language}`;
    const entry = this.pools.get(key);

    if (entry) {
      entry.clients.delete(session.id);
      if (entry.clients.size === 0) {
        // No more clients, schedule shutdown after idle period
        setTimeout(() => {
          if (entry.clients.size === 0) {
            entry.server.shutdown();
            this.pools.delete(key);
          }
        }, 60000); // 1 minute idle timeout
      }
    }
  }
}
```

### Rationale
- **Efficiency**: Multiple developers on same project share one LSP server
- **Isolation**: Different projects get separate LSP servers (avoiding cross-project state issues)
- **Scalability**: Servers are created on-demand and cleaned up when idle
- **Simplicity**: LSP servers maintain their natural project-scoped state

### Example Scenarios

```
Scenario 1: Two clients, same project
Client A: /Users/alice/webapp → Server: /code/webapp
Client B: /Users/bob/webapp   → Server: /code/webapp
Result: Share one TypeScript LSP server

Scenario 2: Two clients, different projects
Client A: /Users/alice/webapp → Server: /code/webapp
Client C: /Users/alice/api    → Server: /code/api
Result: Two separate TypeScript LSP servers
```

## File Streaming Protocol

### Bidirectional MCP Extension

The service can request files from clients using reverse RPC calls over the WebSocket connection.

```typescript
// Protocol Definition
interface FileStreamProtocol {
  // Server → Client: Request file content
  'client/readFile': {
    params: {
      path: string;
      encoding?: 'utf8' | 'base64';
    };
    result: {
      content: string;
      mtime: number;  // Modified time for caching
      size: number;
    };
  };

  // Server → Client: List directory
  'client/listFiles': {
    params: {
      path: string;
      pattern?: string;  // glob pattern
    };
    result: {
      files: Array<{
        path: string;
        type: 'file' | 'directory';
        size: number;
        mtime: number;
      }>;
    };
  };

  // Client → Server: File changed notification
  'server/fileChanged': {
    params: {
      path: string;
      changeType: 'created' | 'changed' | 'deleted';
    };
  };
}
```

### Implementation

```typescript
class StreamingFileAccess implements FileAccess {
  private cache = new Map<string, CachedFile>();

  async readFile(session: ClientSession, path: string): Promise<string> {
    const cacheKey = `${session.id}:${path}`;
    const cached = this.cache.get(cacheKey);

    // Check cache validity
    if (cached && Date.now() - cached.timestamp < 5000) {
      return cached.content;
    }

    // Request from client via WebSocket
    const response = await session.call('client/readFile', { path });

    // Update cache
    this.cache.set(cacheKey, {
      content: response.content,
      timestamp: Date.now(),
      mtime: response.mtime
    });

    return response.content;
  }

  handleFileChange(session: ClientSession, params: FileChangedParams) {
    // Invalidate cache for changed file
    const cacheKey = `${session.id}:${params.path}`;
    this.cache.delete(cacheKey);

    // Notify LSP server if applicable
    const lspServer = this.getRelevantLSPServer(session, params.path);
    if (lspServer) {
      lspServer.didChangeWatchedFiles([{
        uri: `file://${params.path}`,
        type: params.changeType
      }]);
    }
  }
}
```

### Client Implementation

```typescript
// Client-side handler
client.onRequest('client/readFile', async (params) => {
  const content = await fs.readFile(params.path, params.encoding || 'utf8');
  const stats = await fs.stat(params.path);
  return {
    content,
    mtime: stats.mtime.getTime(),
    size: stats.size
  };
});

// File watcher
chokidar.watch(projectRoot).on('all', (event, path) => {
  client.notify('server/fileChanged', {
    path,
    changeType: mapChokidarEvent(event)
  });
});
```

## Project Structure

```
codeflow-buddy/
├── src/
│   ├── transports/
│   │   ├── stdio.ts         # Existing
│   │   └── websocket.ts     # New
│   ├── server/
│   │   ├── ws-server.ts     # WebSocket server
│   │   ├── session.ts       # Client session management
│   │   └── auth.ts          # Token validation
│   └── fs/
│       ├── shared.ts        # Shared volume access
│       └── stream.ts        # Stream protocol
├── Dockerfile.service        # Service-specific Dockerfile
└── docker-compose.service.yml
```

## Architecture

```
┌─────────────────────────────────────┐
│     Docker: CodeFlow Service         │
│  ┌─────────────────────────────┐    │
│  │   WebSocket MCP Server       │    │
│  │   - Auth & session mgmt      │    │
│  │   - Request routing          │    │
│  └──────────┬──────────────────┘    │
│             │                        │
│  ┌──────────▼──────────────────┐    │
│  │   LSP Server Pool            │    │
│  │   - TypeScript LSP           │    │
│  │   - Python LSP               │    │
│  │   - Go LSP                   │    │
│  └──────────┬──────────────────┘    │
│             │                        │
│  ┌──────────▼──────────────────┐    │
│  │   Virtual FS Layer           │    │
│  │   - Shared volumes           │    │
│  │   - Stream protocol          │    │
│  │   - Cache layer              │    │
│  └─────────────────────────────┘    │
└─────────────────────────────────────┘
              │
              ▼
     Client Connections (MCP)
       │           │
       ▼           ▼
   Dev Container  Local Machine
```

## Validation Plan

### Unit Tests
- [ ] WebSocket transport mirrors stdio behavior
- [ ] Path translation works correctly
- [ ] Session isolation prevents cross-client access

### Integration Tests
- [ ] All 29 MCP tools work over WebSocket
- [ ] Multi-client file operations don't conflict
- [ ] Performance: <100ms latency for local operations

### Manual Testing Checklist
- [ ] Docker container with shared volume
- [ ] Local machine with streaming
- [ ] 3 concurrent clients on different projects

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Latency overhead | <50ms vs stdio | Time MCP tool calls |
| Concurrent clients | 5+ | Load test with multiple connections |
| Memory per client | <50MB | Monitor process memory |
| File cache hit rate | >80% | Log cache hits/misses |

## Execution Modes

```bash
# Traditional stdio mode (default, unchanged)
codeflow-buddy start

# New WebSocket server mode
codeflow-buddy serve [options]
  --port 3000
  --token SECRET
  --shared-root /code
  --max-clients 10
```

## Docker Deployment

```yaml
# docker-compose.service.yml
version: '3.8'
services:
  codeflow-service:
    build:
      context: .
      dockerfile: Dockerfile.service
    ports:
      - "3000:3000"
    volumes:
      - /shared/projects:/code:ro
    environment:
      - AUTH_TOKEN=${AUTH_TOKEN}
      - MAX_CLIENTS=10
      - FILE_ACCESS=shared

  dev-env-1:
    image: node:18
    environment:
      - MCP_SERVER=ws://codeflow:3000
    volumes:
      - /shared/projects/project1:/app

  dev-env-2:
    image: python:3.11
    environment:
      - MCP_SERVER=ws://codeflow:3000
    volumes:
      - /shared/projects/project2:/app

volumes:
  codeflow-cache:
```

## Performance Optimizations

### Caching Strategy
- **Memory cache**: 5-second TTL for frequently accessed files
- **Batch requests**: Group multiple file reads in single RPC
- **Predictive loading**: Preload imports when a file is opened
- **Delta updates**: Send only diffs for large file changes

### Expected Performance
| Operation | Shared Volume | Streaming (LAN) | Streaming (Internet) |
|-----------|--------------|-----------------|---------------------|
| Read file | <1ms | 5-10ms | 50-100ms |
| With cache | <1ms | <1ms | <1ms |
| First project open | 100ms | 500ms | 2-3s |
| Subsequent opens | 100ms | 150ms | 200ms |

## Benefits

1. **Single Installation**: LSP servers installed once in service container
2. **Resource Sharing**: One TypeScript LSP serves all clients
3. **Central Management**: Update/configure in one place
4. **Works Everywhere**: Any MCP client can connect via WebSocket
5. **Cached Intelligence**: Share parsed ASTs across projects

## MVP Scope (2 weeks)

**Include**:
- WebSocket transport with token auth
- Multi-client with session isolation
- Shared volume file access for Docker
- Basic streaming for non-Docker clients

**Defer**:
- SSHFS mounts
- Advanced caching
- TLS/certificates
- Client-specific LSP configs

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| WebSocket adds too much latency | Benchmark early, optimize protocol |
| File streaming too slow | Implement aggressive caching |
| Session conflicts | Strong path isolation, comprehensive tests |
| Breaking existing functionality | Keep stdio as default, WebSocket opt-in |

## Next Steps

1. **Prototype WebSocket transport** (2 days)
   - Adapt existing MCP server to WebSocket
   - Verify with single client

2. **Add multi-client support** (3 days)
   - Session management
   - Path isolation

3. **Implement file strategies** (3 days)
   - Shared volume (simplest)
   - Streaming protocol

4. **Testing & documentation** (2 days)
   - Integration tests
   - Deployment guide

Total: ~2 weeks to MVP

## Security Considerations

- WebSocket authentication via tokens
- Path sandboxing (clients can only access their project paths)
- Read-only mode option for shared code
- TLS for production deployments