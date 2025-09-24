# Proposal: Monorepo Restructure for Client/Server Architecture ✅ **IMPLEMENTED**

**✅ Status: Monorepo structure implemented in commit `48bfa5e`**

## ✅ Problem - SOLVED

Users want two deployment modes:
1. **Local MCP** - Install codeflow-buddy locally, runs LSP servers on their machine
2. **Remote MCP** - Lightweight client that connects to centralized codeflow-buddy server

✅ **Solution implemented: Both modes now supported through monorepo structure**

## ✅ Solution - IMPLEMENTED

✅ **Restructured into monorepo with two packages:**

### Package Structure
```
packages/
├── server/                    # @goobits/codeflow-buddy
│   ├── src/ (current code)    # LSP servers, FUSE, WebSocket server, etc.
│   └── package.json           # Full dependencies (~50MB)
│
└── client/                    # @goobits/codeflow-buddy-client
    ├── src/
    │   ├── types.ts           # Copy essential types from server
    │   ├── mcp-proxy.ts       # Forward MCP requests to remote server
    │   ├── websocket.ts       # WebSocket connection handling
    │   └── cli.ts             # Setup commands
    └── package.json           # Minimal deps: ws, jwt, mcp (~5MB)
```

### User Experience

**Team lead (server):**
```bash
npm install -g @goobits/codeflow-buddy
codeflow-buddy serve --port 3000 --require-auth --jwt-secret "team-key"
```

**Developers (client):**
```bash
npm install -g @goobits/codeflow-buddy-client
codeflow-buddy-client setup --server ws://team-server:3000 --tenant frontend-team
```

### Benefits

- **90% smaller client** - No LSP servers or FUSE dependencies
- **Centralized LSP servers** - Team shares powerful language servers
- **Consistent environment** - All developers use same setup
- **Clear separation** - Local vs remote modes in different packages

### Implementation

1. Create `packages/` structure
2. Move current code to `packages/server/`
3. Build lightweight client in `packages/client/`
4. Copy essential types (no shared package - would be overkill)
5. Update build/publish scripts for monorepo

### Timeline

- **Week 1**: Monorepo structure + server migration
- **Week 2**: Client package implementation
- **Week 3**: Testing + documentation
- **Week 4**: Release both packages

## No Shared Package

Analysis shows only ~2200 lines would be shared (~10% of codebase). Creating a third package would add complexity without benefit. Copy essential types instead.