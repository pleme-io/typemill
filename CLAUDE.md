# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Information

**Package**: `@goobits/codeflow-buddy` | **Command**: `codeflow-buddy` | **Runtime**: Bun/Node.js

MCP server bridging Language Server Protocol (LSP) functionality to AI coding assistants with 25 MCP tools for navigation, refactoring, code intelligence, and batch operations.

## Development Commands

```bash
# Install dependencies
bun install

# Development with hot reload
bun run dev

# Build for production
bun run build

# Run the built server
bun run start
# or directly
node dist/index.js

# CLI commands for configuration and management
codeflow-buddy setup    # Smart setup with auto-detection
codeflow-buddy status   # Show what's working right now
codeflow-buddy start    # Start the MCP server for Claude Code
codeflow-buddy stop     # Stop the running MCP server
codeflow-buddy serve    # Start WebSocket server

# Quality assurance
bun run lint         # Check code style and issues
bun run lint:fix     # Auto-fix safe issues
bun run format       # Format code with Biome
bun run typecheck    # Run TypeScript type checking
bun run test         # Run unit tests
bun run test:all     # Run all tests
bun run test:e2e          # Run end-to-end tests
bun run test:fuse    # Run FUSE-specific tests (includes real FUSE tests)
bun run test:fuse:real    # Run real FUSE tests (requires FUSE installed)

# Test Performance Optimizations (for slow systems)
bun run test:fast     # Optimized test runner with system detection
bun run test:minimal  # Ultra-minimal runner for very slow systems
# Fast runner: 5min timeout, parallel on fast systems, LSP preload optional
# Minimal runner: 10min timeout, sequential only, no LSP preload, minimal config

# Full pre-publish check
bun run prepublishOnly  # build + test + typecheck

# IMPORTANT: After modifying codeflow-buddy's own source code, restart the MCP server to load changes!

# WebSocket Server Commands
node dist/index.js serve --port 3000                    # Basic WebSocket server
node dist/index.js serve --require-auth --jwt-secret KEY # With JWT authentication
node dist/index.js serve --tls-key key.pem --tls-cert cert.pem # With TLS/WSS
docker-compose up -d                                     # Full Docker deployment
```

## Architecture

### Core Components

**MCP Server Layer** (`index.ts`)

- Entry point that implements MCP protocol
- Exposes 31 MCP tools covering navigation, refactoring, intelligence, diagnostics, and batch operations
- Handles MCP client requests and delegates to LSP layer
- Includes CLI subcommand handling for `setup`, `status`, `start`, `stop`

**LSP Client Layer** (`src/lsp/client.ts`)

- Manages multiple LSP server processes concurrently
- Handles LSP protocol communication (JSON-RPC over stdio)
- Maps file extensions to appropriate language servers
- Maintains process lifecycle and request/response correlation

**Tool Registry** (`src/mcp/tool-registry.ts`)

- Central registry for all MCP tool handlers
- Decouples batch executor from handler implementations
- Supports dynamic tool registration at module load time
- Tracks which module registered each tool for debugging

**Configuration System** (`.codebuddy/config.json`)

- Defines which LSP servers to use for different file extensions
- Smart setup with auto-detection via `codeflow-buddy setup` command
- File scanning with gitignore support for project structure detection
- Automatic migration from old `codebuddy.json` format

**WebSocket Server Layer** (`src/server/ws-server.ts`)

- Production-ready WebSocket server with HTTP health endpoints
- Session management with connection recovery (60-second grace periods)
- JWT authentication and TLS/WSS support for enterprise security
- Structured logging and comprehensive monitoring

**Authentication System** (`src/auth/jwt-auth.ts`) - *Phase 3*

- JWT-based authentication with configurable expiry and permissions
- Project-based access control with granular permissions
- `/auth` HTTP endpoint for token generation
- Token validation during WebSocket initialization

**Delta Update System** (`src/fs/delta.ts`) - *Phase 3*

- diff-match-patch integration for efficient file synchronization
- Automatic compression ratio analysis (only uses delta if >20% savings)
- Graceful fallback to full updates when delta is inefficient
- Network bandwidth optimization for large file modifications

**Advanced Caching** (`src/core/cache.ts`) - *Phase 3*

- Event-driven cache invalidation replacing TTL-based expiration
- Persistent file cache until explicit invalidation events
- Hit rate tracking and comprehensive cache statistics
- Pattern-based bulk invalidation for directory changes

**Streaming File Access** (`src/fs/stream.ts`)

- Enhanced with delta updates and intelligent caching
- Real-time file change notification handling
- Cache invalidation on file modification events
- Performance monitoring and statistics

### Data Flow

**Traditional MCP Flow:**
1. MCP client sends tool request (e.g., `find_definition`)
2. Main server looks up tool handler in central registry
3. Tool handler is executed with appropriate service injection
4. LSP client determines appropriate language server for file extension
5. If server not running, spawns new LSP server process
6. Sends LSP request to server and correlates response
7. Transforms LSP response back to MCP format

**WebSocket Server Flow (Phase 2+):**
1. Client connects via WebSocket (with optional JWT authentication)
2. Session manager creates/recovers client session with project context
3. WebSocket transport receives MCP message and validates permissions
4. Streaming file access provides cached content or requests from client
5. Delta processor optimizes file updates using diff-match-patch
6. LSP servers process requests with intelligent crash recovery
7. Response sent back through WebSocket with structured logging

### Tool Registration Pattern

Each handler module self-registers its tools with the central registry:

```typescript
// At the end of each handler file (e.g., core-handlers.ts)
import { registerTools } from '../tool-registry.js';

registerTools(
  {
    find_definition: { handler: handleFindDefinition, requiresService: 'symbol' },
    find_references: { handler: handleFindReferences, requiresService: 'symbol' },
    // ... other tools
  },
  'core-handlers' // module name for tracking
);
```

This pattern eliminates circular dependencies and enables:
- Clean separation of concerns
- Easy addition/removal of tools
- Better testability (registry can be mocked)
- Plugin-style extensibility

### LSP Server Management

The system spawns separate LSP server processes per configuration. Each server:

- Runs as child process with stdio communication
- Maintains its own initialization state
- Handles multiple concurrent requests
- Gets terminated on process exit

Supported language servers (configurable):

- TypeScript: `typescript-language-server`
- Python: `pylsp`
- Go: `gopls`

## Configuration

The server loads configuration from `.codebuddy/config.json` in the current working directory. If no configuration exists, run `codeflow-buddy setup` to create one.

### Smart Setup  

Use `codeflow-buddy setup` to configure LSP servers with auto-detection:

- Scans project for file extensions (respects .gitignore)
- Presents pre-configured language server options for detected languages
- Generates `.codebuddy/config.json` configuration file  
- Tests server availability during setup

Each server config requires:

- `extensions`: File extensions to handle (array)
- `command`: Command array to spawn LSP server
- `rootDir`: Working directory for LSP server (optional)
- `restartInterval`: Auto-restart interval in minutes (optional, helps with long-running server stability, minimum 1 minute)

### Example Configuration

```json
{
  "servers": [
    {
      "extensions": ["py"],
      "command": ["pylsp"],
      "restartInterval": 5
    },
    {
      "extensions": ["ts", "tsx", "js", "jsx"],
      "command": ["typescript-language-server", "--stdio"],
      "restartInterval": 10
    }
  ]
}
```

## Code Quality & Testing

The project uses Biome for linting and formatting:

- **Linting**: Enabled with recommended rules + custom strictness
- **Formatting**: 2-space indents, single quotes, semicolons always, LF endings
- **TypeScript**: Strict type checking with `--noEmit`
- **Testing**: Bun test framework with unit tests in `src/*.test.ts`

Run quality checks before committing:

```bash
bun run lint:fix && bun run format && bun run typecheck && bun run test
```

## LSP Protocol Details

The implementation handles LSP protocol specifics:

- Content-Length headers for message framing
- JSON-RPC 2.0 message format
- Request/response correlation via ID tracking
- Server initialization handshake
- Proper process cleanup on shutdown
- Preloading of servers for detected file types
- Automatic server restart based on configured intervals
- Manual server restart via MCP tool

## Production Deployment (Phase 2+)

### Docker Deployment
```bash
# Build and start full stack
docker-compose up -d

# Production service only
docker build -f Dockerfile.service -t codeflow-buddy:latest .
docker run -d -p 3000:3000 codeflow-buddy:latest
```

### Health Monitoring
```bash
# Health check endpoint
curl http://localhost:3000/healthz

# Prometheus metrics
curl http://localhost:3000/metrics
```

### WebSocket Server Configuration
```bash
# Basic server
node dist/index.js serve --port 3000 --max-clients 10

# With authentication
node dist/index.js serve --require-auth --jwt-secret "your-secret"

# With TLS/WSS
node dist/index.js serve --tls-key server.key --tls-cert server.crt

# Enterprise setup
node dist/index.js serve \
  --port 3000 --max-clients 10 \
  --require-auth --jwt-secret "enterprise-key" \
  --tls-key /etc/ssl/server.key --tls-cert /etc/ssl/server.crt
```

### Environment Variables
- `NODE_ENV` - Environment mode (development/production)
- `LOG_LEVEL` - Logging level (debug/info/warn/error)
- `JWT_SECRET` - JWT signing secret for authentication
- `JWT_EXPIRY` - Token expiry time (default: 24h)
- `JWT_ISSUER` - Token issuer (default: codeflow-buddy)
- `JWT_AUDIENCE` - Token audience (default: codeflow-clients)

## Performance Features (Phase 3)

### Advanced Caching
- **Event-driven invalidation** - Files cached until explicit change notifications
- **Hit rate tracking** - Comprehensive cache statistics and monitoring
- **Pattern invalidation** - Bulk invalidation for directory changes
- **Cache statistics** - Available via `/healthz` endpoint

### Delta Updates
- **diff-match-patch integration** - Efficient file synchronization
- **Automatic optimization** - Only uses delta if >20% bandwidth savings
- **Compression ratio analysis** - Real-time efficiency monitoring
- **Graceful fallback** - Full updates when delta is inefficient

### Security Features
- **JWT Authentication** - Token-based project access control
- **TLS/WSS Support** - Encrypted WebSocket connections
- **Permission System** - Granular access controls per project
- **Client Certificate Validation** - Enhanced security for enterprise

## Dead Code Detection

Run dead code detection with:
- `bun run dead-code` - Check for dead code
- `bun run dead-code:fix` - Auto-fix where possible
- `bun run dead-code:ci` - CI-friendly output

Tool: Knip (detects unused files, dependencies, exports)
Config: knip.json

## Adding New MCP Tools (For Contributors)

To add a new MCP tool to the system:

1. **Define the tool schema** in the appropriate `src/mcp/definitions/*.ts` file
2. **Implement the handler** in the corresponding `src/mcp/handlers/*.ts` file
3. **Register the tool** at the end of your handler file:
   ```typescript
   registerTools(
     { your_tool: { handler: handleYourTool, requiresService: 'symbol' } },
     'your-handler-module'
   );
   ```
4. **Update tool count** in CLAUDE.md if adding to existing categories

The tool will be automatically available through:
- Direct MCP calls
- Batch execution system (`batch_execute` tool)
- Tool discovery (`/tools` command)

No need to modify the batch executor or main server - the registry handles everything!
