# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 📚 Essential Documentation for AI Agents

**Before working with this codebase, please read:**

1. **[API.md](API.md)** - **READ THIS FIRST** - Complete MCP tools API reference
   - Tool parameters, return types, and examples
   - [Language Support Matrix](API.md#language-support-matrix) - Which tools work with which languages
   - Internal vs public tools distinction

2. **[docs/architecture/ARCHITECTURE.md](docs/architecture/ARCHITECTURE.md)** - System architecture
   - Component overview and data flow
   - LSP integration patterns
   - Plugin system design

3. **[CONTRIBUTING.md](CONTRIBUTING.md)** - Contributor guide
   - How to add new MCP tools
   - Handler architecture and registration
   - Best practices and code standards

4. **[docs/development/LOGGING_GUIDELINES.md](docs/development/LOGGING_GUIDELINES.md)** - Structured logging
   - Required logging format (structured key-value)
   - Log levels and conventions
   - Anti-patterns to avoid

5. **[docs/deployment/OPERATIONS.md](docs/deployment/OPERATIONS.md)** - Operations guide
   - Configuration and deployment
   - CLI usage and commands

## Project Information

**Package**: `codebuddy` | **Command**: `codebuddy` | **Runtime**: Rust

Pure Rust MCP server bridging Language Server Protocol (LSP) functionality to AI coding assistants with comprehensive tools for navigation, refactoring, code intelligence, and batch operations.

## MCP Tools

Codebuddy provides comprehensive MCP tools for code intelligence and refactoring. See **[API.md](API.md)** for complete API reference with detailed parameters, return types, and examples.

**Note:** Additional internal tools exist for backend use only (lifecycle hooks, workflow plumbing). These are hidden from MCP `tools/list` to simplify the API surface. See [API.md Internal Tools](API.md#internal-tools) section.

### Quick Reference

**Navigation & Intelligence**
- `find_definition`, `find_references`, `search_workspace_symbols`
- `get_document_symbols`, `get_hover`, `get_completions`
- `get_signature_help`, `get_diagnostics`
- `prepare_call_hierarchy`, `get_call_hierarchy_incoming_calls`, `get_call_hierarchy_outgoing_calls`
- `find_implementations`, `find_type_definition`, `web_fetch`

**Editing & Refactoring**
- `rename_symbol`, `rename_symbol_strict`
- `organize_imports`, `get_code_actions`, `format_document`
- `extract_function`, `inline_variable`, `extract_variable`

**File Operations**
- `create_file`, `read_file`, `write_file`, `delete_file`
- `rename_file` (auto-updates imports)
- `list_files`

**Workspace Operations**
- `rename_directory` (auto-updates imports, supports Rust crate consolidation)
- `analyze_imports`, `find_dead_code`, `update_dependencies`
- `extract_module_to_package`, `update_dependency`

**Advanced Operations**
- `apply_edits` (atomic multi-file edits)
- `batch_execute` (batch file operations)
- See [docs/features/WORKFLOWS.md](docs/features/WORKFLOWS.md) for intent-based workflow automation

**System & Health**
- `health_check`, `web_fetch`, `system_status`

### MCP Usage Pattern

```json
{
  "method": "tools/call",
  "params": {
    "name": "find_definition",
    "arguments": {
      "file_path": "src/app.ts",
      "line": 10,
      "character": 5
    }
  }
}
```

### Dry Run Mode

Most file-modifying operations support `dry_run: true` for safe previews:

```json
{
  "method": "tools/call",
  "params": {
    "name": "rename_file",
    "arguments": {
      "old_path": "src/old.ts",
      "new_path": "src/new.ts",
      "dry_run": true
    }
  }
}
```

**Supported operations:**
- File operations: `create_file`, `write_file`, `delete_file`, `rename_file`
- Directory operations: `rename_directory` (including consolidation mode)
- Refactoring: `rename_symbol`, `rename_symbol_strict`, `extract_function`, `inline_variable`, `extract_variable`

**Benefits:**
- Preview changes before applying them
- No file system modifications occur
- Returns detailed preview of what would happen
- Safe for testing and validation

### Rust Crate Consolidation

The `rename_directory` tool supports a special **consolidation mode** for merging Rust crates:

```json
{
  "method": "tools/call",
  "params": {
    "name": "rename_directory",
    "arguments": {
      "old_path": "crates/source-crate",
      "new_path": "crates/target-crate/src/module",
      "consolidate": true,
      "dry_run": true
    }
  }
}
```

**What consolidation does:**
1. Moves `source-crate/src/*` into `target-crate/src/module/*`
2. Merges dependencies from source `Cargo.toml` into target `Cargo.toml`
3. Removes source crate from workspace members
4. Updates all imports across workspace (`use source_crate::*` → `use target_crate::module::*`)
5. Deletes the source crate directory

**Important:** After consolidation, manually add to `target-crate/src/lib.rs`:
```rust
pub mod module;  // Exposes the consolidated code
```

**Use cases:**
- Simplifying workspace structure by reducing number of crates
- Merging experimental features back into main crate
- Consolidating related functionality into a single package

For detailed parameters, return types, and examples, see **[API.md](API.md)**.

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

## Architecture & Configuration

For detailed system architecture, see **[docs/architecture/ARCHITECTURE.md](docs/architecture/ARCHITECTURE.md)**.

For operations and configuration details, see **[docs/deployment/OPERATIONS.md](docs/deployment/OPERATIONS.md)**.

### Quick Configuration Example

The server loads configuration from `.codebuddy/config.json` in the current working directory. Run `codebuddy setup` for smart auto-detection.

```json
{
  "servers": [
    {
      "extensions": ["ts", "tsx", "js", "jsx"],
      "command": ["typescript-language-server", "--stdio"]
    },
    {
      "extensions": ["py", "pyi"],
      "command": ["pylsp"]
    }
  ]
}
```

### Service Layer (`crates/cb-services/`)

- File service for file system operations
- AST service for code parsing and analysis
- Lock manager for concurrent operation safety
- Operation queue for request management
- Planner and workflow executor for complex operations

**Additional Components**

- **Plugin System** (`crates/cb-plugins/`) - Extensible plugin architecture
- **AST Processing** (`crates/cb-ast/`) - Code parsing and analysis
- **Virtual Filesystem** (`crates/cb-vfs/`) - FUSE filesystem support (Unix only)
  - ⚠️ **EXPERIMENTAL - Development Only**
  - Requires `SYS_ADMIN` capability (disables container security boundaries)
  - Not recommended for production use
  - To disable: set `"fuse": null` in `.codebuddy/config.json`
  - Docker: Use `deployment/docker-compose --profile fuse up codebuddy-fuse` to enable
- **API Interfaces** (`crates/cb-protocol/`) - Service trait definitions
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
- Rust: `rust-analyzer`
- Java: `jdtls` (Eclipse JDT Language Server)

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

# Or use Makefile targets
make check                # Run fmt + clippy + test
make check-duplicates     # Detect duplicate code & complexity
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

## For Contributors

### Adding New Language Plugins

**See [crates/languages/README.md](crates/languages/README.md)** for complete guide on implementing language plugins:
- Plugin structure and schema requirements
- `LanguagePlugin` trait implementation
- Data types (ParsedSource, Symbol, ManifestData)
- Plugin registration and testing
- Reference implementations (Rust, Go, TypeScript)

### Adding New MCP Tools

**See [CONTRIBUTING.md](CONTRIBUTING.md)** for complete step-by-step guide on adding new tools with handler architecture, registration, and best practices.

---

## Debug and Development Code Organization

**⚠️ IMPORTANT: Use `.debug/` directory for ALL debugging work**

All debug scripts, test analysis, and experimental code goes in `.debug/` (gitignored):

**What to put in `.debug/`:**
- Test failure analysis documents (`.debug/test-failures/`)
- Temporary debugging scripts
- Performance investigations
- Experimental prototypes

**Guidelines:**
- Organize with subdirectories
- Keep analysis docs for reference, delete temp scripts after use
- Never commit to repository

**Examples:**
- `.debug/test-failures/ATOMIC_FAILURE_ANALYSIS.md` - Root cause analysis
- `.debug/test_timing.rs` - Temporary test script

## 📖 Additional Documentation

### For Contributors
- **[CONTRIBUTING.md](CONTRIBUTING.md)** - Setup, PR process, adding tools, best practices
- **[docs/development/LOGGING_GUIDELINES.md](docs/development/LOGGING_GUIDELINES.md)** - Structured logging standards
- **[integration-tests/TESTING_GUIDE.md](integration-tests/TESTING_GUIDE.md)** - Testing architecture

### For Operators
- **[docs/deployment/OPERATIONS.md](docs/deployment/OPERATIONS.md)** - Production deployment and CLI usage
- **[deployment/docker/README.md](deployment/docker/README.md)** - Docker deployment

### For Understanding the System
- **[docs/architecture/ARCHITECTURE.md](docs/architecture/ARCHITECTURE.md)** - Complete system architecture
- **[docs/architecture/INTERNAL_TOOLS.md](docs/architecture/INTERNAL_TOOLS.md)** - Internal vs public tools policy
- **[docs/features/WORKFLOWS.md](docs/features/WORKFLOWS.md)** - Workflow automation engine

### For Tool Reference
- **[API.md](API.md)** - Complete MCP tools API with examples
- **[API.md#language-support-matrix](API.md#language-support-matrix)** - Language support by tool
