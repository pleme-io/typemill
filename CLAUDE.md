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

**MCP Server Layer** (`apps/server/src/main.rs`, `crates/cb-server/`)

- Entry point that implements MCP protocol via stdio and WebSocket
- Exposes comprehensive MCP tools covering navigation, refactoring, intelligence, diagnostics, and batch operations
- Handles MCP client requests via plugin dispatcher system
- CLI subcommand handling in `apps/server/src/cli.rs` for `setup`, `status`, `start`, `stop`, `serve`, `link`, `unlink`

**LSP Client Layer** (`crates/cb-server/src/systems/lsp/`)

- Manages multiple LSP server processes concurrently
- Handles LSP protocol communication (JSON-RPC over stdio)
- Maps file extensions to appropriate language servers
- Maintains process lifecycle and request/response correlation

**MCP Tool Handlers** (`crates/cb-server/src/handlers/`)

- Plugin dispatcher routing MCP requests to appropriate handlers
- Native Rust tool implementations with compile-time type safety
- Comprehensive error handling and validation
- Integration with LSP client layer and service layer

**Configuration System** (`.codebuddy/config.json`, `crates/cb-core/`)

- Defines which LSP servers to use for different file extensions
- Smart setup with auto-detection via `codebuddy setup` command
- File scanning with gitignore support for project structure detection
- Native Rust configuration parsing and validation

**WebSocket Transport Layer** (`crates/cb-transport/`)

- Production-ready WebSocket server with HTTP health endpoints
- Stdio transport for MCP protocol over standard input/output
- JWT authentication support for enterprise security
- Structured logging and comprehensive monitoring

**Service Layer** (`crates/cb-server/src/services/`)

- File service for file system operations
- AST service for code parsing and analysis
- Lock manager for concurrent operation safety
- Operation queue for request management
- Planner and workflow executor for complex operations

**Additional Components**

- **Plugin System** (`crates/cb-plugins/`) - Extensible plugin architecture
- **AST Processing** (`crates/cb-ast/`) - Code parsing and analysis
- **Virtual Filesystem** (`crates/cb-vfs/`) - FUSE filesystem support (Unix only)
- **API Interfaces** (`crates/cb-api/`) - Service trait definitions
- **Client Library** (`crates/cb-client/`) - CLI client and WebSocket client

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

## Structured Logging Standards

The codebase uses **structured tracing** for all logging to enable machine-readable logs and enhanced production observability.

### Logging Requirements

**✅ DO - Use structured key-value format:**
```rust
// Correct structured logging
error!(error = %e, file_path = %path, "Failed to read file");
info!(user_id = %user.id, action = "login", "User authenticated");
debug!(request_id = %req_id, duration_ms = elapsed, "Request completed");
```

**❌ DON'T - Use string interpolation:**
```rust
// Incorrect - string interpolation
error!("Failed to read file {}: {}", path, e);
info!("User {} authenticated with action {}", user.id, "login");
debug!("Request {} completed in {}ms", req_id, elapsed);
```

### Field Naming Conventions

- **Errors**: `error = %e`
- **File paths**: `file_path = %path.display()` or `path = ?path`
- **IDs**: `user_id = %id`, `request_id = %req_id`
- **Counts**: `files_count = count`, `items_processed = num`
- **Durations**: `duration_ms = elapsed`, `timeout_seconds = timeout`

### Log Levels

- **`error!`**: Runtime errors, failed operations, system failures
- **`warn!`**: Recoverable issues, deprecation warnings, fallback actions
- **`info!`**: Important business events, service lifecycle, user actions
- **`debug!`**: Detailed execution flow, internal state changes

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
./target/release/codebuddy serve
```

### WebSocket Server Configuration
```bash
# Start WebSocket server (default port 3000)
./target/release/codebuddy serve
```

### Environment Variables
- `RUST_LOG` - Logging level (debug/info/warn/error)

## Performance Features

### Native Performance
- **Zero-cost abstractions** - Rust's compile-time optimizations
- **Memory safety** - No garbage collection overhead
- **Async runtime** - Efficient tokio-based concurrency
- **Native compilation** - Platform-optimized machine code

### Security Features
- **Memory safety** - Rust's ownership system prevents common vulnerabilities
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
