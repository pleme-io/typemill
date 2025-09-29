# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Information

**Package**: `codebuddy` | **Command**: `codebuddy` | **Runtime**: Rust

Pure Rust MCP server bridging Language Server Protocol (LSP) functionality to AI coding assistants with comprehensive tools for navigation, refactoring, code intelligence, and batch operations.

## Development Commands

```bash
# Build the project
cargo build

# Development build with debug info
cargo build --release

# Run the server directly
cargo run

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run clippy for linting
cargo clippy

# Format code
cargo fmt

# Check code without building
cargo check

# CLI commands for configuration and management
./target/release/codebuddy setup    # Smart setup with auto-detection
./target/release/codebuddy status   # Show what's working right now
./target/release/codebuddy start    # Start the MCP server for Claude Code
./target/release/codebuddy stop     # Stop the running MCP server
./target/release/codebuddy serve    # Start WebSocket server
./target/release/codebuddy link     # Link to AI assistants
./target/release/codebuddy unlink   # Remove AI from config
```

## Architecture

### Core Components

**MCP Server Layer** (`src/main.rs`)

- Entry point that implements MCP protocol
- Exposes comprehensive MCP tools covering navigation, refactoring, intelligence, diagnostics, and batch operations
- Handles MCP client requests and delegates to LSP layer
- Includes CLI subcommand handling for `setup`, `status`, `start`, `stop`

**LSP Client Layer** (`src/lsp/`)

- Manages multiple LSP server processes concurrently
- Handles LSP protocol communication (JSON-RPC over stdio)
- Maps file extensions to appropriate language servers
- Maintains process lifecycle and request/response correlation

**Tool Registry** (`src/mcp/`)

- Central registry for all MCP tool handlers
- Decouples batch executor from handler implementations
- Native Rust tool definitions and handlers
- Comprehensive error handling and validation

**Configuration System** (`.codebuddy/config.json`)

- Defines which LSP servers to use for different file extensions
- Smart setup with auto-detection via `codebuddy setup` command
- File scanning with gitignore support for project structure detection
- Native Rust configuration parsing and validation

**WebSocket Server Layer** (`src/server/`)

- Production-ready WebSocket server with HTTP health endpoints
- Session management with connection recovery
- JWT authentication and TLS/WSS support for enterprise security
- Structured logging and comprehensive monitoring

**File System Operations** (`src/fs/`)

- Native file system access and manipulation
- Efficient file reading and writing operations
- Directory traversal and pattern matching
- Cross-platform compatibility

### Data Flow

**MCP Flow:**
1. MCP client sends tool request (e.g., `find_definition`)
2. Main server looks up tool handler in registry
3. Tool handler is executed with appropriate LSP client
4. LSP client determines appropriate language server for file extension
5. If server not running, spawns new LSP server process
6. Sends LSP request to server and correlates response
7. Transforms LSP response back to MCP format

**WebSocket Server Flow:**
1. Client connects via WebSocket (with optional JWT authentication)
2. Session manager creates/recovers client session with project context
3. WebSocket transport receives MCP message and validates permissions
4. File system operations provide direct file access
5. LSP servers process requests with intelligent crash recovery
6. Response sent back through WebSocket with structured logging

### Tool Registration Pattern

Tools are registered as native Rust functions with compile-time type safety:

```rust
// Tool handlers are defined as async functions
pub async fn find_definition(
    params: FindDefinitionParams,
    lsp_client: &LspClient,
) -> Result<FindDefinitionResult, McpError> {
    // Implementation
}

// Tools are registered in the main MCP server setup
let tools = vec![
    Tool::new("find_definition", find_definition),
    Tool::new("find_references", find_references),
    // ... other tools
];
```

This approach provides:
- Compile-time type safety
- Zero-cost abstractions
- Memory safety guarantees
- High performance native execution

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

The server loads configuration from `.codebuddy/config.json` in the current working directory. If no configuration exists, run `codebuddy setup` to create one.

### Smart Setup  

Use `codebuddy setup` to configure LSP servers with auto-detection:

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

The project uses Rust's built-in tooling for code quality:

- **Linting**: Clippy with strict rules and custom configurations
- **Formatting**: rustfmt with standard Rust formatting conventions
- **Type Safety**: Rust's compile-time type checking and borrow checker
- **Testing**: Built-in Rust test framework with unit and integration tests

Run quality checks before committing:

```bash
cargo fmt && cargo clippy && cargo test
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

## Production Deployment

### Binary Distribution
```bash
# Build optimized release binary
cargo build --release

# The resulting binary is self-contained and ready for deployment
./target/release/codebuddy serve --port 3000
```

### WebSocket Server Configuration
```bash
# Basic server
./target/release/codebuddy serve --port 3000 --max-clients 10

# With authentication
./target/release/codebuddy serve --require-auth --jwt-secret "your-secret"

# With TLS/WSS
./target/release/codebuddy serve --tls-key server.key --tls-cert server.crt

# Enterprise setup
./target/release/codebuddy serve \
  --port 3000 --max-clients 10 \
  --require-auth --jwt-secret "enterprise-key" \
  --tls-key /etc/ssl/server.key --tls-cert /etc/ssl/server.crt
```

### Environment Variables
- `RUST_LOG` - Logging level (debug/info/warn/error)
- `JWT_SECRET` - JWT signing secret for authentication
- `JWT_EXPIRY` - Token expiry time (default: 24h)
- `JWT_ISSUER` - Token issuer (default: codebuddy)
- `JWT_AUDIENCE` - Token audience (default: codebuddy-clients)

## Performance Features

### Native Performance
- **Zero-cost abstractions** - Rust's compile-time optimizations
- **Memory safety** - No garbage collection overhead
- **Async runtime** - Efficient tokio-based concurrency
- **Native compilation** - Platform-optimized machine code

### Security Features
- **Memory safety** - Rust's ownership system prevents common vulnerabilities
- **JWT Authentication** - Token-based project access control
- **TLS/WSS Support** - Encrypted WebSocket connections
- **Type safety** - Compile-time prevention of data races and null pointer errors

## Adding New MCP Tools (For Contributors)

To add a new MCP tool to the system:

1. **Define the tool schema** in `src/mcp/tools.rs`
2. **Implement the handler** in the appropriate handler module
3. **Register the tool** in the main server setup:
   ```rust
   tools.push(Tool::new("your_tool", handle_your_tool));
   ```
4. **Add tests** for the new functionality

The tool will be automatically available through:
- Direct MCP calls
- Batch execution system
- Tool discovery

All tools benefit from Rust's compile-time guarantees for safety and performance!
