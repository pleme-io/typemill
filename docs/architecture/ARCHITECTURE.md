# Architecture Documentation

## Overview

Codebuddy is a pure Rust MCP server that bridges Model Context Protocol (MCP) with Language Server Protocol (LSP) functionality. The architecture follows a service-oriented design with clear crate separation, AI-friendly boundaries, and comprehensive code intelligence tools.

## High-Level Architecture

The system is built on a multi-crate architecture with focused responsibilities and clear dependency hierarchies:

```mermaid
graph TD
    A[apps/codebuddy] --> B[cb-server]
    B --> C[cb-api]
    B --> D[cb-core]
    B --> E[cb-ast]
    B --> F[cb-transport]
    B --> G[cb-vfs]
    B --> H[cb-plugins]

    D --> C
    E --> C
    F --> C
    G --> C
    H --> C

    I[cb-client] --> C
    I --> D

    J[tests] --> K[All crates]
```

## Crate Responsibilities

### Foundation Layer

**`cb-api`** - The contract crate
- Defines shared traits: `AstService`, `LspService`
- Data structures: `EditPlan`, `ImportGraph`, error types
- No dependencies on other cb-* crates
- Ensures clear interfaces between components

**`cb-core`** - Configuration and core types
- Application configuration (`AppConfig`, `LspConfig`)
- Core data models and utilities
- Depends only on `cb-api`

### Service Layer

**`cb-ast`** - Language intelligence
- Code parsing, analysis, and transformation
- Import graph management and refactoring
- Implements `AstService` trait from `cb-api`

**`cb-transport`** - Communication protocols
- WebSocket and stdio transport layers
- MCP protocol implementation
- Session management and message routing

**`cb-vfs`** - Virtual filesystem
- FUSE filesystem implementation
- File system abstraction and caching
- Read-only workspace mounting

**`cb-plugins`** - Extensibility system
- Plugin management and registry
- Language-specific adapters
- Tool registration and dispatch

### Orchestration Layer

**`cb-server`** - Central orchestration
- Implements all service traits
- Wires services together in `AppState`
- Message dispatching and request routing
- LSP client management

### Application Layer

**`apps/codebuddy`** - Executable entry point
- CLI argument parsing
- Server bootstrap and initialization
- Process management (stdio/WebSocket modes)

## Request Lifecycle

### Modern Request Flow

The current architecture uses a plugin-based dispatch system:

1. **Request Reception**
   ```
   Transport Layer (stdio/WebSocket) → JSON parsing → McpMessage
   ```

2. **Plugin Dispatch**
   ```
   PluginDispatcher::dispatch() → MessageDispatcher → Tool lookup
   ```

3. **Service Execution**
   ```
   Plugin handler → AppState services → Service implementations
   ```

4. **Response Generation**
   ```
   Service result → MCP response → JSON serialization → Transport
   ```

### Key Components

**`PluginDispatcher`**
- Central request orchestrator
- Manages plugin lifecycle and routing
- Provides unified error handling

**`MessageDispatcher`**
- Routes messages to appropriate plugins
- Handles tool registration and discovery
- Manages concurrent request processing

**`AppState`**
- Shared service container
- Provides dependency injection for services
- Maintains application-wide state

## Core Architecture: Unified Handlers & Plugins

The "Foundations First" architecture unifies all 44 MCP tools through a consistent, high-performance handler pattern. This design eliminates technical debt, enables zero-cost abstractions, and provides a scalable foundation for future tool additions.

### The Unified `ToolHandler` Trait

All tool handlers implement a single, consistent interface defined in `crates/cb-server/src/handlers/tools/mod.rs`:

```rust
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// Returns the list of tool names this handler provides
    fn tool_names(&self) -> &[&str];

    /// Handles a tool call with full context
    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value>;
}
```

**Key Design Principles:**

1. **Single Responsibility**: Each handler focuses on a specific category of tools
2. **Compile-Time Safety**: Rust's type system ensures handler correctness
3. **Zero-Cost Abstractions**: No runtime overhead for handler dispatch
4. **Context Injection**: Handlers receive all needed services through `ToolHandlerContext`

**Handler Context Structure:**

```rust
pub struct ToolHandlerContext {
    pub app_state: Arc<AppState>,
    pub plugin_manager: Arc<PluginManager>,
    pub lsp_adapter: Arc<Mutex<Option<Arc<DirectLspAdapter>>>>,
}
```

This provides handlers with access to:
- **AppState**: File service, lock manager, operation queue
- **PluginManager**: Language-specific plugin dispatch
- **LSP Adapter**: Direct LSP server communication

### Macro-Based Registration

The system uses declarative macros for clean, maintainable handler registration (defined in `crates/cb-server/src/handlers/macros.rs`):

```rust
register_handlers_with_logging!(registry, {
    SystemHandler => "SystemHandler with 3 tools: health_check, web_fetch, ping",
    LifecycleHandler => "LifecycleHandler with 3 tools: notify_file_opened, notify_file_saved, notify_file_closed",
    NavigationHandler => "NavigationHandler with 10 tools: find_definition, find_references, ...",
    EditingHandler => "EditingHandler with 9 tools: rename_symbol, format_document, ...",
    RefactoringHandler => "RefactoringHandler with 4 tools: extract_function, inline_variable, ...",
    FileOpsHandler => "FileOpsHandler with 6 tools: read_file, write_file, ...",
    WorkspaceHandler => "WorkspaceHandler with 7 tools: list_files, find_dead_code, ...",
});
```

**Benefits:**

- **Declarative**: Clear intent, no boilerplate
- **Automatic Logging**: Debug output for each registered handler
- **Compile-Time Validation**: Ensures all handlers implement `ToolHandler`
- **Easy Extension**: Add new handlers by adding one line

**Implementation in `plugin_dispatcher.rs`:**

```rust
pub async fn initialize(&self) -> ServerResult<()> {
    let mut registry = self.tool_registry.lock().await;

    // All 44 tools registered in ~10 lines of declarative code
    register_handlers_with_logging!(registry, {
        SystemHandler => "SystemHandler with 3 tools...",
        // ... 7 handlers total
    });

    Ok(())
}
```

### Priority-Based Plugin Selection

The plugin system uses a sophisticated multi-tiered selection algorithm to choose the best plugin for each tool request.

**Configuration in `.codebuddy/config.json`:**

```json
{
  "plugin_selection": {
    "priorities": {
      "typescript-plugin": 100,
      "rust-analyzer-plugin": 90,
      "generic-lsp-plugin": 50
    },
    "error_on_ambiguity": false
  }
}
```

**Selection Algorithm (in `crates/cb-plugins/src/registry.rs`):**

```rust
pub fn find_best_plugin(&self, file_path: &Path, method: &str) -> PluginResult<String> {
    // Step 1: Determine tool scope from capabilities
    let tool_scope = self.get_tool_scope(method);

    // Step 2: Filter plugins based on scope
    let candidates = match tool_scope {
        Some(ToolScope::File) => {
            // File-scoped tools require BOTH file extension AND method match
            self.find_plugins_for_file(file_path)
                .into_iter()
                .filter(|p| self.supports_method(p, method))
                .collect()
        }
        Some(ToolScope::Workspace) | None => {
            // Workspace-scoped tools only need method match
            self.find_plugins_for_method(method)
        }
    };

    // Step 3: Select highest priority plugin
    // Ties broken by lexicographic order (deterministic)
    self.select_by_priority(&candidates, method)
}
```

**Priority Tiers:**

1. **Config Overrides** (highest): User-defined priorities in config file
2. **Plugin Metadata**: `priority` field in `PluginMetadata` (default: 50)
3. **Lexicographic Order**: Deterministic fallback for tied priorities

**Tool Scope System:**

Tools are classified by scope to optimize plugin selection:

```rust
pub enum ToolScope {
    /// Tool operates on a specific file (requires file_path)
    File,        // Example: find_definition, rename_symbol
    /// Tool operates at workspace level (no file_path required)
    Workspace,   // Example: search_workspace_symbols, list_files
}
```

**Scope Detection (in `crates/cb-plugins/src/capabilities.rs`):**

```rust
impl Capabilities {
    pub fn get_tool_scope(&self, method: &str) -> Option<ToolScope> {
        match method {
            // File-scoped tools
            | "find_definition"
            | "rename_symbol"
            | "format_document" => Some(ToolScope::File),

            // Workspace-scoped tools
            | "search_workspace_symbols"
            | "list_files"
            | "find_dead_code" => Some(ToolScope::Workspace),

            _ => None,
        }
    }
}
```

**Performance Characteristics:**

- **Plugin Selection**: 141ns (1 plugin) to 1.7µs (20 plugins)
- **Priority Lookup**: O(1) hash map access with config overrides
- **Scope Detection**: O(1) match expression (constant time)
- **Ambiguity Resolution**: Configurable error or deterministic fallback

**Error Handling:**

When `error_on_ambiguity: true` in config:
```rust
PluginError::AmbiguousPluginSelection {
    method: "find_definition",
    plugins: vec!["plugin-a", "plugin-b"],
    priority: 50,
}
```

When `error_on_ambiguity: false` (default):
- Automatically selects first plugin by lexicographic order
- Logs warning with candidates for debugging

### Handler Architecture

**Current Handler Organization:**

| Handler | Tools | Scope | Purpose |
|---------|-------|-------|---------|
| **SystemHandler** | 3 | System | Health checks, web fetch, ping |
| **LifecycleHandler** | 3 | Lifecycle | File open/save/close notifications |
| **NavigationHandler** | 10 | LSP | Symbol navigation, references, definitions |
| **EditingHandler** | 9 | LSP | Symbol renaming, formatting, code actions |
| **RefactoringHandler** | 4 | AST | Extract function/variable, inline operations |
| **FileOpsHandler** | 6 | File | File read/write/delete/create operations |
| **WorkspaceHandler** | 7 | Workspace | Workspace-wide analysis and refactoring |

**Total: 44 Tools across 7 Handlers**

### Dispatch Flow

```mermaid
graph TD
    A[MCP Request] --> B[PluginDispatcher]
    B --> C[ToolRegistry.handle_tool]
    C --> D{Tool Lookup}
    D -->|Found| E[Handler.handle_tool_call]
    D -->|Not Found| F[Error: Unsupported Tool]
    E --> G{Tool Type}
    G -->|LSP Tool| H[Plugin Manager]
    G -->|Native Tool| I[Direct Execution]
    H --> J[Priority-Based Selection]
    J --> K[Plugin.handle_request]
    K --> L[LSP Client]
    L --> M[Language Server]
    I --> N[Service Layer]
    M --> O[Result]
    N --> O
    O --> P[MCP Response]
```

**Key Optimizations:**

1. **Single Lookup**: Tool name → Handler (O(1) hash map)
2. **No Adapters**: Direct handler invocation (zero overhead)
3. **Lazy Plugin Selection**: Only computed when needed
4. **Concurrent Safe**: All handlers are `Send + Sync`

### Backward Compatibility

The architecture maintains full backward compatibility through the `compat` module:

```rust
// crates/cb-server/src/handlers/compat.rs
pub use crate::handlers::tools::ToolHandler as LegacyToolHandler;
pub use crate::handlers::tools::ToolHandlerContext as ToolContext;
```

Legacy handlers can be gradually migrated without breaking existing functionality.

### Testing Strategy

**Safety Net Test (crates/cb-server/tests/tool_registration_test.rs):**

```rust
#[tokio::test]
async fn test_all_42_tools_are_registered() {
    let dispatcher = create_test_dispatcher();
    dispatcher.initialize().await.unwrap();

    let registry = dispatcher.tool_registry.lock().await;
    let registered_tools = registry.list_tools();

    // Verify all 44 tools are present
    assert_eq!(registered_tools.len(), 44);
    assert!(registered_tools.contains(&"find_definition".to_string()));
    // ... validate all tools
}
```

This test ensures no tools are accidentally removed during refactoring.

**Plugin Selection Tests (crates/cb-plugins/src/registry.rs):**

- `test_scope_aware_file_tool_selection`: File-scoped tool routing
- `test_scope_aware_workspace_tool_selection`: Workspace-scoped tool routing
- `test_priority_based_selection`: Priority ordering
- `test_priority_override`: Config override behavior
- `test_ambiguous_selection_error`: Ambiguity detection
- `test_ambiguous_selection_fallback`: Deterministic fallback

**Total: 41 cb-plugins tests, 67 cb-ast tests, 1 integration test - All passing**

## Component Interactions

### Service Architecture

The architecture is built around service traits defined in `cb-api`:

```rust
// Core service traits
pub trait AstService: Send + Sync {
    async fn analyze_imports(&self, file_path: &Path) -> ApiResult<ImportGraph>;
    async fn generate_edit_plan(&self, request: RefactorRequest) -> ApiResult<EditPlan>;
}

pub trait LspService: Send + Sync {
    async fn request(&self, message: Message) -> ApiResult<Message>;
    async fn notify_file_opened(&self, file_path: &Path) -> ApiResult<()>;
}
```

### AppState Service Container

The `AppState` acts as a dependency injection container:

```rust
pub struct AppState {
    pub ast_service: Arc<dyn AstService>,
    pub file_service: Arc<FileService>,
    pub project_root: PathBuf,
    pub lock_manager: Arc<LockManager>,
    pub operation_queue: Arc<OperationQueue>,
}
```

## Language Plugin System (Updated - Phase 2 Complete)

**Architecture**: Capability-based trait system with optional trait objects

**Core Design**:
- **Metadata Consolidation**: 7 methods → 1 struct (LanguageMetadata::RUST, etc.)
- **Trait Reduction**: 22 methods → 9 methods (59% reduction)
- **Sync Capabilities**: All capability methods are synchronous (no async overhead)
- **O(1) Feature Detection**: Check capabilities() before attempting operations

**Trait Structure**:
```rust
trait LanguagePlugin {
    // Core (always available)
    fn metadata() -> &LanguageMetadata;
    fn parse(...) -> ParsedSource;
    fn analyze_manifest(...) -> ManifestData;
    fn capabilities() -> LanguageCapabilities;

    // Optional capabilities (trait objects)
    fn import_support() -> Option<&dyn ImportSupport>;
    fn workspace_support() -> Option<&dyn WorkspaceSupport>;
}
```

**Benefits**:
- No more NotSupported errors - check capabilities first
- Reduced boilerplate (29-42% LOC reduction per plugin)
- Sync operations where appropriate
- Easy to add new languages with only required features

### Capability-Based Design

The plugin system uses a capability-based architecture for optional features:

**Core Trait** (`LanguagePlugin`):
- 6 required methods + 3 default methods
- Reduced from 22 methods in previous architecture
- 59% reduction in trait definition size

**Capability Flags** (`LanguageCapabilities`):
```rust
pub struct LanguageCapabilities {
    pub imports: bool,    // Import parsing and rewriting support
    pub workspace: bool,  // Workspace manifest operations support
}
```

**Benefits**:
- O(1) feature detection (no try/catch overhead)
- Opt-in functionality (implement only what you need)
- Clear API contracts (no NotSupported errors)
- Reduced boilerplate (29-42% LOC reduction per plugin)

#### Trait Structure

```rust
#[async_trait]
pub trait LanguagePlugin: Send + Sync {
    // Core functionality
    fn metadata(&self) -> &LanguageMetadata;
    async fn parse(&self, source: &str) -> PluginResult<ParsedSource>;
    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData>;

    // Capability system
    fn capabilities(&self) -> LanguageCapabilities;
    fn import_support(&self) -> Option<&dyn ImportSupport>;
    fn workspace_support(&self) -> Option<&dyn WorkspaceSupport>;

    // Downcasting support
    fn as_any(&self) -> &dyn std::any::Any;

    // Default implementations
    async fn list_functions(&self, source: &str) -> PluginResult<Vec<String>>;
    fn handles_extension(&self, ext: &str) -> bool;
    fn handles_manifest(&self, filename: &str) -> bool;
}
```

#### Metadata Consolidation

Replaced 7 separate methods with a single struct accessor:

**Old Pattern** (removed):
```rust
fn name(&self) -> &str;
fn file_extensions(&self) -> Vec<&str>;
fn manifest_filename(&self) -> &str;
fn source_dir(&self) -> &str;
fn entry_point(&self) -> &str;
fn module_separator(&self) -> &str;
fn language(&self) -> ProjectLanguage;
```

**New Pattern** (current):
```rust
fn metadata(&self) -> &LanguageMetadata;

// Usage:
let name = plugin.metadata().name;
let exts = plugin.metadata().extensions;
let manifest = plugin.metadata().manifest_filename;
```

Pre-defined constants eliminate boilerplate:
```rust
impl LanguageMetadata {
    pub const RUST: Self = Self { ... };
    pub const TYPESCRIPT: Self = Self { ... };
    pub const GO: Self = Self { ... };
    pub const PYTHON: Self = Self { ... };
}
```

#### Downcasting Pattern

For implementation-specific methods not in the core trait:

```rust
use cb_lang_rust::RustPlugin;

// Get plugin from registry
let plugin = registry.find_by_extension("rs")?;

// Downcast to access Rust-specific methods
if let Some(rust_plugin) = plugin.as_any().downcast_ref::<RustPlugin>() {
    let imports = rust_plugin.parse_imports(path).await?;
    let workspace = rust_plugin.generate_workspace_manifest(&members, root).await?;
}
```

**Why Downcasting?**
- Keeps core trait small and focused
- Allows language-specific functionality
- Type-safe access to concrete implementations
- Service layers can access specialized methods as needed

#### Current Implementation Status

| Language | Imports | Workspace | Parse | Manifest |
|----------|---------|-----------|-------|----------|
| Rust | ✅ | ✅ | ✅ | ✅ |
| TypeScript | ✅ | ❌ | ✅ | ✅ |
| Go | ✅ | ❌ | ✅ | ✅ |
| Python | ✅ | ❌ | ✅ | ✅ |

### LSP Integration

The LSP system provides direct language server communication:

- **`LspManager`**: Orchestrates multiple LSP clients by file extension
- **`LspClient`**: Manages individual LSP server processes
- **Direct Communication**: Bypasses legacy request mapping through plugin adapters

### Modern Tool Registration

Tools are registered through the plugin system rather than hardcoded mappings:

```rust
// Plugin-based tool registration
impl LanguagePlugin for SystemToolsPlugin {
    async fn handle_request(&self, request: &ToolRequest, app_state: &AppState) -> PluginResult<ToolResponse> {
        match request.tool.as_str() {
            "find_definition" => self.find_definition(request, app_state).await,
            "get_diagnostics" => self.get_diagnostics(request, app_state).await,
            _ => Err(PluginError::UnsupportedTool(request.tool.clone()))
        }
    }
}
```

## Tool Categories

The system provides comprehensive code intelligence through various tool categories:

### 1. Navigation Tools
- **Symbol Definition**: Find where symbols are defined
- **Symbol References**: Find all references to symbols
- **Workspace Symbols**: Search for symbols across the project
- **Document Symbols**: Get all symbols in a file

### 2. Intelligence Tools
- **Hover Information**: Rich documentation and type information
- **Code Completions**: Context-aware code suggestions
- **Signature Help**: Function parameter assistance
- **Diagnostics**: Real-time error and warning detection

### 3. Editing Tools
- **Symbol Renaming**: Project-wide symbol renaming
- **Code Formatting**: Language-specific formatting
- **Code Actions**: Quick fixes and refactoring suggestions
- **Workspace Edits**: Multi-file atomic editing operations

### 4. Analysis Tools
- **Import Analysis**: Dependency graph analysis via `cb-ast`
- **Dead Code Detection**: Unused code identification
- **Call Hierarchy**: Function call relationships
- **Type Hierarchy**: Type inheritance relationships

### 5. Filesystem Tools
- **File Operations**: Cross-platform file manipulation
- **Directory Operations**: Workspace management
- **Path Resolution**: Canonical path handling
- **File Watching**: Real-time file system monitoring

### 6. Refactoring Tools
- **Extract Function**: Code extraction into new functions
- **Extract Variable**: Expression extraction into variables
- **Inline Operations**: Variable and function inlining
- **Import Organization**: Automatic import cleanup

## Configuration Management

### Hierarchical Configuration

Configuration is managed through the `cb-core` crate with support for multiple sources:

1. **Default Configuration**: Built-in sensible defaults
2. **Configuration Files**: JSON/TOML support (`.codebuddy/config.json`)
3. **Environment Variables**: Runtime overrides
4. **Command Line**: Highest precedence overrides

### Configuration Structure

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,    // WebSocket/stdio server settings
    pub lsp: LspConfig,         // Language server configurations
    pub fuse: Option<FuseConfig>, // Optional FUSE filesystem
    pub logging: LoggingConfig,  // Logging configuration
    pub cache: CacheConfig,     // Caching settings
}
```

### LSP Server Configuration

Each language server is configured with:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspServerConfig {
    pub extensions: Vec<String>,        // File extensions handled
    pub command: Vec<String>,           // Command to start LSP server
    pub root_dir: Option<PathBuf>,      // Working directory
    pub restart_interval: Option<u64>,  // Auto-restart interval
}
```

## Error Handling Strategy

### Error Types Hierarchy

The system uses a layered error handling approach:

```rust
ApiError ← CoreError ← ServerError ← Transport-specific errors
```

### Error Propagation

- **Service Errors**: Propagated through `ApiResult<T>` from service traits
- **LSP Errors**: Wrapped as `ApiError` with contextual information
- **File System Errors**: Converted to appropriate error responses
- **Configuration Errors**: Fail fast during application startup

### Error Recovery

- **LSP Server Failures**: Graceful degradation with optional restart
- **Network Errors**: Connection-level retry logic
- **Parse Errors**: Detailed error messages with context
- **Resource Exhaustion**: Configurable limits and throttling

## Performance Architecture

### Async Runtime

- **Tokio-based**: Efficient async I/O with minimal thread overhead
- **Concurrent Processing**: Multiple MCP requests handled simultaneously
- **Resource Pooling**: Shared LSP clients and service instances

### Memory Management

- **Arc-based Sharing**: Efficient shared ownership of services
- **Lazy Initialization**: Services created on-demand
- **Bounded Caching**: TTL-based caching with size limits

### Optimization Features

- **Native Performance**: Zero-cost Rust abstractions
- **Memory Safety**: Compile-time guarantees prevent common vulnerabilities
- **Minimal Allocations**: Efficient data structures and borrowing

## Security Model

### Process Isolation

- **LSP Servers**: Run as separate child processes
- **Workspace Boundaries**: File operations restricted to project scope
- **Command Validation**: LSP commands validated against configuration

### Input Validation

- **JSON Schema**: All MCP requests validated against schemas
- **Path Sanitization**: Prevents directory traversal attacks
- **Type Safety**: Rust's type system prevents many common vulnerabilities

## Development Workflow

### Adding New Tools

1. **Define Tool Schema**: Add to appropriate plugin
2. **Implement Handler**: Create async handler function
3. **Register Tool**: Add to plugin's tool registry
4. **Add Tests**: Unit and integration tests
5. **Update Documentation**: Tool-specific documentation

### Language Plugin Development

For adding support for new programming languages, see the **[Language Plugins Guide](../../crates/languages/README.md)** which provides:

1. **Plugin Structure**: Directory layout and file organization
2. **Trait Implementation**: `LanguagePlugin` trait requirements
3. **Registration**: Plugin registration in `language_plugin_registry.rs`
4. **Testing**: Unit and integration test requirements
5. **Reference Examples**: Rust, Go, TypeScript plugin implementations

This architecture provides a robust, scalable foundation for bridging MCP and LSP protocols while maintaining excellent performance and reliability characteristics through Rust's safety guarantees and zero-cost abstractions.

---

## API Contracts

This section defines the external and internal contracts for the MCP server implementation.

### Transport Layer Contracts

#### WebSocket Transport (Default)
- **Endpoint**: `ws://127.0.0.1:3040`
- **Protocol**: JSON-RPC 2.0 over WebSocket
- **Command**: `codebuddy serve`
- **Features**: Session management, concurrent connections, health endpoints

#### Stdio Transport
- **Protocol**: JSON-RPC 2.0 over stdin/stdout (newline-delimited)
- **Command**: `codebuddy start`
- **Features**: MCP protocol compatibility, editor integration support
- **Usage**: Designed for MCP clients like Claude Code

### Request/Response Format Contract

#### Standard MCP Request
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

#### Standard MCP Response
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

#### Error Response Contract
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

### Performance Contracts

- **Bootstrap Time**: < 500ms (server initialization)
- **Request Dispatch**: < 10ms average (routing and validation)
- **Memory Baseline**: < 50MB (without LSP servers)
- **Health Check**: < 5ms response time

### Thread Safety Guarantees

- All public types are `Send + Sync` where appropriate
- Async functions are cancellation-safe
- No global mutable state
- All shared state protected by appropriate synchronization primitives (Arc, Mutex, RwLock)

### Error Handling Contract

- All errors implement `std::error::Error`
- Errors are convertible to `CoreError` for cross-crate consistency
- Error messages are descriptive and actionable
- No panics in production code paths (all use `.expect()` with context)

### Backward Compatibility

- Public APIs are stable within major versions
- Deprecated items have at least one minor version warning period
- Migration guides provided for breaking changes
- Semantic versioning strictly followed
