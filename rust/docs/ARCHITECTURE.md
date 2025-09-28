# Architecture Documentation

## Overview

The Rust MCP Server is a production-ready implementation that bridges the Model Context Protocol (MCP) with Language Server Protocol (LSP) functionality. The architecture follows a layered design with clear separation of concerns, enabling both WebSocket and stdio transports while providing comprehensive code intelligence tools.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Transport Layer                          │
├─────────────────────┬───────────────────────────────────────────┤
│   WebSocket Server  │              Stdio Server                 │
│   (Production)      │              (MCP Clients)                │
│   Port 3040         │              stdin/stdout                 │
└─────────────────────┴───────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Plugin Dispatcher                            │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │   TypeScript    │  │     Python      │  │       Go        │  │
│  │     Plugin      │  │     Plugin      │  │     Plugin      │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
│  ┌─────────────────┐  ┌─────────────────┐                       │
│  │     Rust        │  │     Other       │                       │
│  │     Plugin      │  │   Languages     │                       │
│  └─────────────────┘  └─────────────────┘                       │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│                      App State                                  │
│                                                                 │
│  ┌─────────────────┐              ┌─────────────────┐           │
│  │   LSP Manager   │◄────────────►│   AST Service   │           │
│  │                 │              │                 │           │
│  │ ┌─────────────┐ │              │ ┌─────────────┐ │           │
│  │ │TypeScript LS│ │              │ │   Parser    │ │           │
│  │ │Python LSP   │ │              │ │  Analyzer   │ │           │
│  │ │Rust Analyzer│ │              │ │Transformer  │ │           │
│  │ │   Clangd    │ │              │ └─────────────┘ │           │
│  │ │    Gopls    │ │              └─────────────────┘           │
│  │ └─────────────┘ │                                            │
│  └─────────────────┘                                            │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Subsystems                                    │
│                                                                 │
│  ┌─────────────────┐              ┌─────────────────┐           │
│  │ FUSE Filesystem │              │  Configuration  │           │
│  │                 │              │    Management   │           │
│  │ • Virtual FS    │              │                 │           │
│  │ • 1s TTL Cache  │              │ • JSON Config   │           │
│  │ • Inode Mgmt    │              │ • LSP Servers   │           │
│  │ • Background    │              │ • FUSE Options  │           │
│  │   Mounting      │              │ • Logging       │           │
│  └─────────────────┘              └─────────────────┘           │
└─────────────────────────────────────────────────────────────────┘
```

## Request Lifecycle

### MCP Request Flow (Stdio Transport)

1. **Request Reception**
   ```
   stdin → BufReader → JSON parsing → McpMessage::Request
   ```

2. **Dispatch Processing**
   ```
   PluginDispatcher::dispatch() → Plugin lookup → Handler execution
   ```

3. **Tool Execution**
   ```
   Tool handler → AppState services → LSP/AST operations
   ```

4. **Response Generation**
   ```
   Tool result → MCP response → JSON serialization → stdout
   ```

### WebSocket Request Flow

1. **Connection Management**
   ```
   WebSocket connection → Session creation → Initialize handshake
   ```

2. **Message Processing**
   ```
   WebSocket frame → JSON parsing → MCP dispatch → Response frame
   ```

3. **Session State**
   ```
   Connection pooling → Concurrent request handling → Cleanup
   ```

## Component Interactions

### Plugin Dispatcher

The central orchestrator that:
- Manages language-specific plugins dynamically
- Routes requests based on file extensions
- Provides direct LSP access bypassing legacy mappings
- Handles protocol translation via plugins

```rust
pub struct PluginDispatcher {
    plugins: HashMap<String, Arc<dyn LanguagePlugin>>,
    app_state: Arc<AppState>
}

impl PluginDispatcher {
    pub async fn dispatch(&self, message: McpMessage) -> Result<McpMessage, ServerError> {
        match message {
            McpMessage::Request(req) => {
                let tool_name = extract_tool_name(&req)?;
                let handler = self.tools.get(&tool_name)?;
                let result = handler.execute(&req, &self.app_state).await?;
                Ok(McpMessage::Response(result))
            }
        }
    }
}
```

### App State

Provides shared services to all tool handlers:

```rust
pub struct AppState {
    pub lsp: Arc<LspManager>,
    // Future: AST service, cache, metrics
}
```

### LSP Manager

Manages multiple Language Server Protocol clients:

```rust
pub struct LspManager {
    clients: HashMap<String, Arc<LspClient>>,
    config: LspConfig,
}

impl LspManager {
    pub async fn get_client(&self, extension: &str) -> Result<Arc<LspClient>, CoreError> {
        // Find appropriate LSP server for file extension
        // Start server if not running
        // Return client handle
    }
}
```

### LSP Client

Individual LSP server process manager:

```rust
pub struct LspClient {
    process: Child,
    stdin: ChildStdin,
    stdout_receiver: Receiver<LspResponse>,
    request_id: AtomicU64,
}

impl LspClient {
    pub async fn request(&self, method: &str, params: Value) -> Result<Value, CoreError> {
        // Generate unique request ID
        // Send JSON-RPC request to LSP server
        // Wait for correlated response
        // Handle timeouts and errors
    }
}
```

## Tool Handler Architecture

### MCP Tool Handler Pattern

To reduce boilerplate and standardize interactions with the LSP service, a helper function `forward_lsp_request` is used. This centralizes the logic for creating requests, handling responses, and managing errors.

- **Unique ID Generation**: Each call generates a unique, auto-incrementing request ID.
- **Request Forwarding**: The handler constructs the parameters and calls the helper.
- **Centralized Error Handling**: The helper is responsible for interpreting LSP responses and returning a consistent `Result`.

```rust
// Example from navigation.rs
dispatcher.register_tool("find_definition".to_string(), |app_state, args| async move {
    let params: FindDefinitionArgs = serde_json::from_value(args)?;
    
    // 1. Construct parameters for the LSP request
    let lsp_params = json!({
        "file_path": params.file_path,
        "symbol_name": params.symbol_name,
    });

    // 2. Delegate to the helper function
    util::forward_lsp_request(&app_state, "find_definition", lsp_params).await
});
```

This pattern keeps the tool handlers clean, concise, and focused on their specific logic.

### Tool Categories

1. **Navigation Tools**
   - Direct LSP integration
   - Symbol resolution
   - Cross-reference analysis

2. **Intelligence Tools**
   - Hover information
   - Code completions
   - Signature help
   - Diagnostics

3. **Editing Tools**
   - Symbol renaming
   - Code formatting
   - Code actions
   - Workspace edits

4. **Filesystem Tools**
   - Direct file system operations
   - No LSP dependency
   - Cross-platform path handling

5. **Analysis Tools**
   - cb-ast integration for import analysis
   - LSP-based dead code detection
   - System health monitoring

## FUSE Subsystem

### Filesystem Implementation

```rust
pub struct CodeflowFS {
    workspace_path: PathBuf,
    attr_cache: HashMap<u64, FileAttr>,
    next_inode: u64,
    inode_to_path: HashMap<u64, PathBuf>,
    path_to_inode: HashMap<PathBuf, u64>
}

impl Filesystem for CodeflowFS {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        // Resolve path from parent inode + name
        // Generate or retrieve inode
        // Return file attributes with TTL
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        // Look up path from inode
        // Get file metadata
        // Cache and return attributes
    }
}
```

### FUSE Integration

- **Background mounting**: Runs in dedicated thread to avoid blocking main server
- **TTL-based caching**: 1-second cache for metadata to balance performance and consistency
- **Read-only access**: Prevents accidental modifications through FUSE mount
- **Graceful failure**: Server continues operation even if FUSE mount fails

## Error Handling Strategy

### Error Types Hierarchy

```rust
CoreError → ServerError → Transport-specific errors
```

### Error Propagation

1. **LSP Errors**: Wrapped and propagated as JSON-RPC errors
2. **File System Errors**: Converted to appropriate HTTP status codes
3. **Protocol Errors**: Return proper MCP error responses
4. **Configuration Errors**: Fail fast during startup

### Error Recovery

- **LSP server crashes**: Graceful error responses, no automatic restart
- **File system errors**: Per-operation error handling
- **Network errors**: Connection-level retry logic
- **Parse errors**: Detailed error messages with context

## Configuration Management

### Client Configuration

The `cb-client` crate uses a `ConfigBuilder` pattern to provide a robust and flexible way to load configuration.

#### Configuration Sources

Configuration is loaded from three sources with the following order of precedence:

1.  **Command-line arguments:** (e.g., `--url <URL>`) - Highest precedence.
2.  **Environment variables:** (`CODEFLOW_BUDDY_URL`, `CODEFLOW_BUDDY_TOKEN`).
3.  **Configuration file:** (`~/.codeflow-buddy/config.json`) - Lowest precedence.

#### `ConfigBuilder` Pattern

The `ConfigBuilder` provides a fluent API to construct a `ClientConfig` object.

```rust
// Example of building a configuration
let config = ConfigBuilder::new()
    .from_file_if_exists("~/.codeflow-buddy/config.json").await?
    .with_env_overrides()
    .with_url("ws://override.com:8080".to_string()) // This would be a CLI arg
    .build()?;
```

This pattern centralizes all configuration logic, making it predictable and easy to test.

### Server Configuration Schema

```json
{
  "server": {
    "host": "127.0.0.1",
    "port": 3040
  },
  "lsp": {
    "servers": [
      {
        "name": "typescript",
        "command": ["typescript-language-server", "--stdio"],
        "extensions": ["ts", "tsx", "js", "jsx"],
        "timeout": 30
      }
    ]
  },
  "fuse": {
    "enabled": true,
    "mount_point": "/tmp/codeflow-workspace"
  }
}
```

## Threading Model

### Async Runtime

- **Tokio runtime**: Single-threaded async executor for main server
- **Background threads**: FUSE mounting, LSP process management
- **Concurrent operations**: Multiple MCP requests handled concurrently

### Synchronization

- **Arc<T>**: Shared immutable data across threads
- **Mutex<T>**: Mutable shared state (minimized)
- **Channel communication**: LSP process communication

## Performance Characteristics

### Memory Usage

- **Baseline**: ~50MB for server core
- **LSP overhead**: Variable per language server
- **FUSE caching**: Bounded by TTL and working set size

### Response Times

- **File operations**: < 100ms typical
- **LSP operations**: < 5 seconds (LSP-dependent)
- **FUSE operations**: < 50ms (cached)

### Scalability

- **Concurrent connections**: WebSocket server supports multiple clients
- **LSP multiplexing**: Single LSP server handles multiple requests
- **Resource limits**: Configurable timeouts and request limits

## Security Considerations

### Input Validation

- **JSON schema validation**: All MCP requests validated
- **Path traversal prevention**: File operations restricted to workspace
- **Command injection prevention**: LSP commands from configuration only

### Process Isolation

- **LSP servers**: Run as separate processes
- **FUSE filesystem**: Read-only access
- **Network binding**: Localhost only by default

### Error Information

- **Error messages**: No sensitive information leaked
- **Stack traces**: Debug mode only
- **File paths**: Normalized and validated

## Testing Architecture

### Test Levels

1. **Unit Tests**: Individual component testing
2. **Contract Tests**: MCP protocol validation
3. **Integration Tests**: Cross-component interaction
4. **E2E Tests**: Full client-server scenarios

### Test Infrastructure

- **Test Harness**: A lightweight `TestLspService` allows for predictable testing of MCP handlers without heavy mocking.
- **Real I/O**: Tests use real file I/O in temporary directories and real environment variables to validate behavior accurately.
- **Property-Based Testing**: `proptest` is used to test invariants and edge cases, especially for concurrent operations like request ID generation.
- **Contract validation**: JSON schema compliance
- **Performance testing**: Response time measurement

This architecture provides a robust, scalable foundation for bridging MCP and LSP protocols while maintaining excellent performance and reliability characteristics.
