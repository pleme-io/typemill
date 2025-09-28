# API Contracts - Final Implementation

This document describes the complete API contracts for the production-ready Rust MCP server implementation, including transport protocols, CLI interface, and internal crate contracts.

## Transport Layer Contracts

### WebSocket Transport (Default)
- **Endpoint**: `ws://127.0.0.1:3040`
- **Protocol**: JSON-RPC 2.0 over WebSocket
- **Command**: `cb-server serve` or `cb-server` (default)
- **Features**: Session management, concurrent connections, health endpoints

### Stdio Transport
- **Protocol**: JSON-RPC 2.0 over stdin/stdout (newline-delimited)
- **Command**: `cb-server start`
- **Features**: MCP protocol compatibility, editor integration support
- **Usage**: Designed for MCP clients like Claude Code

## CLI Interface Contract

```bash
# WebSocket server (production mode)
cb-server serve

# Stdio server (MCP client mode)
cb-server start

# Help and version
cb-server --help
cb-server --version
```

## MCP Tools Contract

The server implements 21 MCP tools across 5 categories:

### Navigation Tools (4)
- `find_definition` - Find symbol definitions via LSP
- `find_references` - Find all references to a symbol via LSP
- `get_document_symbols` - Get document outline via LSP
- `search_workspace_symbols` - Search symbols across workspace via LSP

### Intelligence Tools (4)
- `get_hover` - Get hover information via LSP
- `get_completions` - Get code completions via LSP
- `get_signature_help` - Get function signature help via LSP
- `get_diagnostics` - Get syntax/semantic errors via LSP

### Editing Tools (4)
- `rename_symbol` - Rename symbol across workspace via LSP
- `format_document` - Format code via LSP
- `get_code_actions` - Get available code actions via LSP
- `apply_workspace_edit` - Apply multi-file edits via LSP

### Filesystem Tools (5)
- `list_files` - List files and directories
- `read_file` - Read file contents
- `write_file` - Write file contents
- `create_file` - Create new files
- `delete_file` - Delete files
- `rename_file` - Rename/move files
- `update_dependencies` - Universal dependency management across languages

### Analysis Tools (4)
- `analyze_imports` - Analyze import relationships using cb-ast
- `find_dead_code` - Find potentially unused code via LSP
- `health_check` - Check system and LSP server health
- `restart_server` - Restart LSP servers

## Request/Response Format Contract

### Standard MCP Request
```json
{
  "jsonrpc": "2.0",
  "id": "unique-request-id",
  "method": "tools/call",
  "params": {
    "name": "tool_name",
    "arguments": {
      "file_path": "/absolute/path/to/file",
      "line": 10,
      "character": 5
    }
  }
}
```

### Standard MCP Response
```json
{
  "jsonrpc": "2.0",
  "id": "unique-request-id",
  "result": {
    "content": {
      // Tool-specific response data
    }
  }
}
```

### Error Response Contract
```json
{
  "jsonrpc": "2.0",
  "id": "unique-request-id",
  "error": {
    "code": -1,
    "message": "Error description",
    "data": null
  }
}
```

## FUSE Filesystem Contract

### Mount Configuration
```toml
[fuse]
enabled = true
mount_point = "/tmp/codeflow-workspace"
permissions = "755"
```

### Filesystem Operations
- **Read-only** virtual filesystem
- **1-second TTL** for metadata caching
- **Inode-based** file identification
- **Background mounting** via dedicated thread

### File Attributes
- Proper file type detection (regular files vs directories)
- Accurate file sizes and modification times
- POSIX-compliant permission bits
- Stable inode numbers for consistent access

---

## Internal Crate Contracts

## cb-core

### Purpose
Foundation crate providing shared types, configuration, and error handling.

### Public API

```rust
// Configuration
pub struct AppConfig {
    pub server: ServerConfig,
    pub lsp: LspConfig,
    pub fuse: Option<FuseConfig>,
    pub logging: LoggingConfig,
    pub cache: CacheConfig,
}

impl AppConfig {
    pub fn load() -> Result<Self, CoreError>;
    // Note: validate() is currently private
}

// Error Handling
pub enum CoreError {
    Config { message: String },
    Io(std::io::Error),
    Json(serde_json::Error),
    ConfigParsing(config::ConfigError),
    InvalidData { message: String },
    NotSupported { operation: String },
    NotFound { resource: String },
    PermissionDenied { operation: String },
    Timeout { operation: String },
    Internal { message: String },
}

impl CoreError {
    pub fn config(message: impl Into<String>) -> Self;
    pub fn invalid_data(message: impl Into<String>) -> Self;
    pub fn not_supported(operation: impl Into<String>) -> Self;
    pub fn not_found(resource: impl Into<String>) -> Self;
    pub fn permission_denied(operation: impl Into<String>) -> Self;
    pub fn timeout(operation: impl Into<String>) -> Self;
    pub fn internal(message: impl Into<String>) -> Self;
}

// MCP Models
pub enum McpMessage {
    Request(McpRequest),
    Response(McpResponse),
    Notification(McpNotification),
}

pub struct McpRequest {
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

pub struct ToolCall {
    pub name: String,
    pub arguments: Option<Value>,
}

// LSP Models
pub struct LspRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    pub params: Option<Value>,
}

pub struct LspResponse {
    pub jsonrpc: String,
    pub id: Value,
    pub result: Option<Value>,
    pub error: Option<LspError>,
}

// Intent Models
pub struct IntentSpec {
    pub name: String,
    pub arguments: serde_json::Value,
    pub metadata: Option<IntentMetadata>,
}

pub struct IntentMetadata {
    pub source: String,
    pub correlation_id: Option<String>,
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
    pub priority: Option<u8>,
    pub context: HashMap<String, serde_json::Value>,
}
```

### Dependencies
- serde: Serialization
- serde_json: JSON handling
- thiserror: Error derivation
- chrono: Timestamps

## cb-ast

### Purpose
AST parsing, analysis, and transformation for code intelligence.

### Public API

```rust
// Parser
pub fn build_import_graph(source: &str, path: &Path) -> Result<ImportGraph, AstError>;

// Analyzer
pub struct ImportGraph {
    pub source_file: String,
    pub imports: Vec<ImportInfo>,
    pub importers: Vec<String>,
    pub metadata: ImportGraphMetadata,
}

// Note: Higher-level analysis methods are provided via the DependencyGraph struct
pub struct DependencyGraph {
    pub fn get_importers(&self, file_path: &str) -> Vec<String>;
    pub fn get_imports(&self, file_path: &str) -> Vec<String>;
    pub fn has_dependency_path(&self, from: &str, to: &str) -> bool;
}

pub fn build_dependency_graph(import_graphs: &[ImportGraph]) -> DependencyGraph;

// Transformer
pub fn plan_refactor(intent: &IntentSpec, source: &str) -> Result<EditPlan, AstError>;

pub struct EditPlan {
    pub source_file: String,
    pub edits: Vec<TextEdit>,
    pub dependency_updates: Vec<DependencyUpdate>,
    pub validations: Vec<ValidationRule>,
    pub metadata: EditPlanMetadata,
}

pub struct TextEdit {
    pub edit_type: EditType,
    pub location: EditLocation,
    pub original_text: String,
    pub new_text: String,
    pub priority: u32,
    pub description: String,
}

// Note: EditPlan methods (apply/preview) not currently implemented

// Error Handling
pub enum AstError {
    ParseError(String),
    AnalysisError(String),
    TransformError(String),
    Core(CoreError),
}
```

### Dependencies
- cb-core: Core types
- petgraph: Graph algorithms
- regex: Pattern matching (temporary, see README.md)
- chrono: Timestamps

Note: SWC integration planned but not yet implemented due to network restrictions

## cb-server

### Purpose
MCP server implementation with tool handlers and transport layers.

### Public API

```rust
// Bootstrap
pub async fn bootstrap(options: ServerOptions) -> Result<ServerHandle, ServerError>;

pub struct ServerOptions {
    pub config: AppConfig,
    pub debug: bool,
}

pub struct ServerHandle {
    pub async fn start(&self) -> Result<(), ServerError>;
    pub async fn shutdown(self) -> Result<(), ServerError>;
}

// Plugin Dispatcher (replaces McpDispatcher)
pub struct PluginDispatcher {
    pub fn new() -> Self;
    pub fn register_plugin(&mut self, extension: String, plugin: Arc<dyn LanguagePlugin>);
    pub async fn dispatch(&self, message: McpMessage) -> Result<McpMessage, ServerError>;
}

// Plugin registration handled internally via plugin system

// Service Traits
#[async_trait]
pub trait AstService: Send + Sync {
    async fn build_import_graph(&self, file: &Path) -> Result<ImportGraph, CoreError>;
    async fn plan_refactor(&self, intent: &IntentSpec, file: &Path) -> Result<EditPlan, CoreError>;
}

#[async_trait]
pub trait LspService: Send + Sync {
    async fn request(&self, message: McpMessage) -> Result<McpMessage, CoreError>;
    async fn is_available(&self, extension: &str) -> bool;
    async fn restart_servers(&self, extensions: Option<Vec<String>>) -> Result<(), CoreError>;
}

// Error Handling
pub enum ServerError {
    Config { message: String },
    Bootstrap { message: String },
    Runtime { message: String },
    InvalidRequest(String),
    Unsupported(String),
    Core(CoreError),
}
```

### Dependencies
- cb-core: Core types
- cb-ast: AST operations
- tokio: Async runtime
- async-trait: Async traits
- serde_json: JSON handling

## cb-client

### Purpose
CLI client for interacting with the MCP server.

### Public API

```rust
// CLI Arguments
pub struct CliArgs {
    pub command: Commands,
    pub debug: bool,
    pub config: Option<String>,
}

// Commands
pub enum Commands {
    Connect {
        url: String,
        token: Option<String>,
    },
    Request {
        url: String,
        method: String,
        params: Option<String>,
        token: Option<String>,
    },
    Status,
}

// Entry Point
pub async fn run_cli() -> Result<(), ClientError>;

// Session Report
pub struct SessionReport {
    pub success_count: usize,
    pub failure_count: usize,
    pub duration_ms: u64,
    pub operations: Vec<String>,
    pub errors: Vec<String>,
}

// Error Handling
pub enum ClientError {
    Config { message: String },
    Connection { url: String, error: String },
    Request { method: String, error: String },
    Response { error: String },
    Core(CoreError),
}
```

### Dependencies
- cb-core: Core types
- clap: CLI parsing
- tokio: Async runtime
- serde_json: JSON handling

## tests

### Purpose
Integration testing and mocks for all crates.

### Public API

```rust
// Test Helpers
pub fn create_test_config() -> AppConfig;
pub fn create_test_intent(name: &str) -> IntentSpec;
pub fn create_test_mcp_request(method: &str) -> McpMessage;
pub fn create_test_mcp_response() -> McpMessage;
pub fn create_test_import_graph(source_file: &str) -> ImportGraph;
pub fn create_test_edit_plan() -> EditPlan;
pub fn assert_json_eq(actual: &Value, expected: &Value);
pub fn create_temp_file(content: &str) -> tempfile::NamedTempFile;
pub fn get_file_extension(path: &Path) -> Option<&str>;
pub fn generate_test_id() -> String;

// Mocks (in src/mocks.rs)
pub struct MockAstService;
pub struct MockLspService;
```

### Dependencies
- All crates as dev-dependencies
- tokio-test: Async testing
- tempfile: Temporary files
- serde_json: Test fixtures

## Contract Guarantees

### Semantic Versioning
All crates follow semantic versioning:
- Breaking changes increment major version
- New features increment minor version
- Bug fixes increment patch version

### Backward Compatibility
- Public APIs are stable within major versions
- Deprecated items have at least one minor version warning
- Migration guides provided for breaking changes

### Error Handling
- All errors implement std::error::Error
- Errors are convertible to CoreError
- Error messages are descriptive and actionable

### Thread Safety
- All public types are Send + Sync where appropriate
- Async functions are cancellation-safe
- No global mutable state

### Performance Contracts
- bootstrap(): < 500ms
- dispatch(): < 10ms average
- Memory usage: < 50MB baseline

## Integration Points

### TypeScript Compatibility
- MCP protocol matches TypeScript implementation
- JSON serialization is compatible
- Tool names and parameters match exactly

### LSP Protocol
- Follows LSP 3.17 specification
- Content-Length header handling
- JSON-RPC 2.0 message format

### File System
- UTF-8 encoding for all text files
- Cross-platform path handling
- Atomic file operations where possible

## Testing Requirements

### Unit Tests
- Each public function has tests
- Error conditions are tested
- Edge cases are covered

### Integration Tests
- Cross-crate interactions tested
- E2E scenarios validated
- Performance benchmarks included

### Compatibility Tests
- TypeScript test suite can run against Rust
- Protocol compatibility verified
- Tool output matches TypeScript