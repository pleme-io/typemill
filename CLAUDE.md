<!-- This is the source of truth for AI agent instructions. CLAUDE.md and GEMINI.md are synchronized from this file. -->
# AGENTS.md

This file provides guidance to AI assistants when working with code in this repository.

## 📚 Essential Documentation for AI Agents

**Before working with this codebase, please read:**

1. **[docs/tools/](docs/tools/)** - **READ THIS FIRST** - Complete MCP tools API reference

---

**Architecture & Development:**
- **[docs/architecture/overview.md](docs/architecture/overview.md)** - System architecture (components, data flow, LSP integration)
- **[contributing.md](contributing.md)** - Contributor guide (add tools, handler architecture, best practices)
- **[docs/development/logging_guidelines.md](docs/development/logging_guidelines.md)** - Structured logging standards

**Deployment:**
- **[docs/operations/docker_deployment.md](docs/operations/docker_deployment.md)** - Docker deployment (dev and production)

## Project Information

**Package**: `mill` | **Command**: `mill` | **Runtime**: Rust

Pure Rust MCP server bridging Language Server Protocol (LSP) functionality to AI coding assistants with comprehensive tools for navigation, refactoring, code intelligence, and batch operations.

## MCP Tools

TypeMill provides comprehensive MCP tools for code intelligence and refactoring. See **[docs/tools/](docs/tools/)** for complete API reference with detailed parameters, return types, and examples.

**Current Architecture**: Public tools visible to AI agents via MCP `tools/list`, plus internal tools for backend workflows.

**Note:** Internal tools exist for backend use only (lifecycle hooks, workflow plumbing, legacy operations). These are hidden from MCP `tools/list` to simplify the API surface for AI agents. See [docs/architecture/internal_tools.md](docs/architecture/internal_tools.md) for details.

### Quick Reference

**Navigation & Intelligence (8 tools)**
- `find_definition`, `find_references`, `search_symbols`
- `find_implementations`, `find_type_definition`, `get_symbol_info`
- `get_diagnostics`, `get_call_hierarchy`

**Editing & Refactoring (7 tools - Unified API with dryRun)**
- `rename` (symbol, file, directory - with `options.dryRun`)
- `extract` (function, variable, constant, module - with `options.dryRun`)
- `inline` (variable, function, constant - with `options.dryRun`)
- `move` (symbol, file, directory, module - with `options.dryRun`)
- `reorder` (parameters - with `options.dryRun`)
- `transform` (code patterns - with `options.dryRun`)
- `delete` (symbol, file, directory, dead code - with `options.dryRun`)

**Workspace Operations (4 tools)**
- `workspace.create_package` (create new packages in workspace)
- `workspace.extract_dependencies` (extract module dependencies for crate extraction)
- `workspace.update_members` (update workspace member list)
- `workspace.find_replace` (find and replace text across workspace)

**Analysis (9 tools - Unified Analysis API)**
- `analyze.quality` (complexity, smells, maintainability, readability, markdown structure, markdown formatting)
- `analyze.dead_code` (unused imports, symbols, parameters, variables, types, unreachable code)
- `analyze.dependencies` (imports, graph, circular dependencies, coupling, cohesion, depth)
- `analyze.cycles` (dedicated circular dependency detection with cycle paths)
- `analyze.structure` (symbols, hierarchy, interfaces, inheritance, modules)
- `analyze.documentation` (coverage, quality, style, examples, todos)
- `analyze.tests` (coverage, quality, assertions, organization)
- `analyze.batch` (multi-file batch analysis with optimized AST caching)
- `analyze.module_dependencies` (Rust module dependency analysis for crate extraction)

**System & Health (1 tool)**
- `health_check`

**Note**: File operations, workspace tools, and legacy analysis tools are now internal-only. AI agents should use the public API above. See [docs/tools/](docs/tools/) for complete details.

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

### Refactoring Pattern: Unified API with dryRun

All refactoring tools support a unified `dryRun` option for preview vs execution:

**Default Behavior (Safe Preview):**
```json
{
  "name": "rename",
  "arguments": {
    "target": {"kind": "file", "path": "src/old.rs"},
    "newName": "src/new.rs"
    // options.dryRun defaults to true - preview only
  }
}
```

**Execution Mode (Explicit Opt-in):**
```json
{
  "name": "rename",
  "arguments": {
    "target": {"kind": "file", "path": "src/old.rs"},
    "newName": "src/new.rs",
    "options": {
      "dryRun": false  // Explicitly execute changes
    }
  }
}
```

**Key Features:**
- **Safe default**: `dryRun: true` requires explicit opt-in for execution
- **Preview mode** (`dryRun: true`): Returns plan, never modifies files
- **Execute mode** (`dryRun: false`): Applies changes with checksums, rollback, validation
- **Single tool**: No separate `.plan` suffix - one tool does both
- **Consistent**: All 7 refactoring tools use identical pattern

**Supported operations:**
- All refactoring tools: `rename`, `extract`, `inline`, `move`, `reorder`, `transform`, `delete`

**Benefits:**
- Simpler API (7 tools vs 15 previously)
- Consistent behavior across all refactorings
- Safe defaults prevent accidental modifications
- Preview before execution workflow

### Rust Crate Consolidation

The `rename` command supports a special **consolidation mode** for merging Rust crates via the `options.consolidate` parameter:

```json
{
  "method": "tools/call",
  "params": {
    "name": "rename",
    "arguments": {
      "target": {
        "kind": "directory",
        "path": "crates/source-crate"
      },
      "newName": "crates/target-crate/src/module",
      "options": {
        "consolidate": true,
        "dryRun": false  // Execute the consolidation
      }
    }
  }
}
```

**Auto-Detection:** When `consolidate` is not specified, the command automatically detects consolidation moves by checking if:
- Source path is a Cargo package (has `Cargo.toml`)
- Target path is inside another crate's `src/` directory
- Parent of target `src/` has `Cargo.toml`

Example auto-detected consolidation (no `consolidate: true` needed):
```json
{
  "target": {"kind": "directory", "path": "crates/mill-types"},
  "newName": "crates/mill-core/src/types"
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

**Override auto-detection:** Set `"consolidate": false` to force a simple directory rename even when the pattern matches consolidation.

For detailed parameters, return types, and examples, see **[docs/tools/](docs/tools/)** for complete tool documentation.

### Rust File Renames with Automatic Updates

When renaming Rust files using the `rename` command, mill automatically updates:

**1. Module Declarations** - Parent files (lib.rs/mod.rs) get updated:
```rust
// Before: src/lib.rs
pub mod utils;

// After renaming src/utils.rs → src/helpers.rs
pub mod helpers;
```

**2. Use Statements** - Import statements are updated:
```rust
// Before
use utils::helper;
use utils::another;

// After
use helpers::helper;
use helpers::another;
```

**3. Qualified Paths** - Inline qualified paths in code:
```rust
// Before
pub fn lib_fn() {
    utils::helper();
    utils::another();
}

// After
pub fn lib_fn() {
    helpers::helper();
    helpers::another();
}
```

**What gets updated:**
- ✅ `pub mod utils;` → `pub mod helpers;` (mod declarations)
- ✅ `use utils::*` → `use helpers::*` (use statements)
- ✅ `utils::helper()` → `helpers::helper()` (qualified paths)
- ✅ `parent::utils::*` → `parent::helpers::*` (nested paths)
- ✅ Cross-crate imports when moving files between crates
- ✅ Same-crate imports when moving files within a crate

**Example workflow:**
```json
// 1. Preview rename plan
{
  "method": "tools/call",
  "params": {
    "name": "rename",
    "arguments": {
      "target": {
        "kind": "file",
        "path": "src/utils.rs"
      },
      "newName": "src/helpers.rs"
      // options.dryRun defaults to true - preview only
    }
  }
}

// 2. Execute the rename (updates mod, use, and qualified paths)
{
  "method": "tools/call",
  "params": {
    "name": "rename",
    "arguments": {
      "target": {
        "kind": "file",
        "path": "src/utils.rs"
      },
      "newName": "src/helpers.rs",
      "options": { "dryRun": false }
    }
  }
}
```

**Coverage:** Handles 80% of common rename scenarios. Complex cases involving non-parent file updates with nested module paths may require manual verification.

### Comprehensive Rename Coverage

TypeMill's rename functionality provides **100% coverage** of affected references by updating multiple file types during directory and file renames. All edits are surfaced in the dry-run plan for review before execution.

**What gets updated automatically:**

1. **Code files** (.rs, .ts, .js):
   - Import statements and module declarations
   - Qualified paths in code
   - **String literal paths** (e.g., `"config/settings.toml"`)
   - Raw string literals (r"...", r#"..."#)
   - Both absolute and relative path forms

2. **Documentation** (.md, .markdown):
   - Markdown links `[text](path)`
   - Inline code references
   - Path mentions (skips prose text without slashes/extensions)

3. **Configuration** (.toml, .yaml, .yml):
   - Path values in any field
   - Build script paths
   - CI/CD workflow paths
   - Preserves formatting and comments

4. **Cargo.toml**:
   - Workspace member paths
   - Package names (crate renames)
   - Path dependencies
   - Dependent crate references

**Scope Control:**

Use the `options.scope` parameter to control what gets updated:

- `"code"`: Code only (imports, module declarations, string literal paths)
- `"standard"` (default): Code + docs + configs (recommended for most renames)
- `"comments"`: Standard scope + code comments
- `"everything"`: Comments scope + markdown prose text
- `"custom"`: Fine-grained control with exclude patterns

**Deprecated (still works with warnings):**
- `"code-only"` → use `"code"` instead
- `"all"` → use `"standard"` instead

**Examples:**
```json
// Minimal scope (code only)
{
  "target": {"kind": "directory", "path": "old-dir"},
  "newName": "new-dir",
  "options": {
    "scope": "code"  // Skip .md, .toml, .yaml files
  }
}

// Default scope (recommended)
{
  "target": {"kind": "directory", "path": "old-dir"},
  "newName": "new-dir",
  "options": {
    "scope": "standard"
  }
}

// Include comments
{
  "target": {"kind": "directory", "path": "old-dir"},
  "newName": "new-dir",
  "options": {
    "scope": "comments"
  }
}

// Update everything including prose
{
  "target": {"kind": "directory", "path": "old-dir"},
  "newName": "new-dir",
  "options": {
    "scope": "everything"
  }
}
```

**Coverage Example:**

See `tests/e2e/src/test_comprehensive_rename_coverage.rs` for validated test cases covering:
- ✅ Rust files (imports + string literal in code)
- ✅ Cargo.toml files (workspace members, package names)
- ✅ Markdown files (inline and reference-style links)
- ✅ Config files (TOML, YAML path values)

**Path Detection:**

Smart heuristic only updates strings that look like paths:
- Contains slash: `"old-dir/file.rs"` ✅
- Has file extension: `"config.toml"` ✅
- Prose text: `"We use old-dir as a pattern"` ❌ (skipped - no slash or extension)
- Relative paths match absolute: `"config/file.toml"` matches `/workspace/config`

**Implementation Details:**
- Scans all file types during planning phase (not just at execution)
- Uses language-specific plugins (Rust, Markdown, TOML, YAML)
- All edits appear in `rename` (with `dryRun: true`) output for review
- Atomic execution with rollback on any failure

### Batch Rename (Multiple Items at Once)

TypeMill supports renaming **multiple files and/or directories** in a single atomic operation using the `targets` parameter:

**Single rename (one item):**
```json
{
  "target": {"kind": "directory", "path": "old-dir"},
  "newName": "new-dir"
}
```

**Batch rename (multiple items):**
```json
{
  "targets": [
    {"kind": "directory", "path": "old-dir1", "newName": "new-dir1"},
    {"kind": "directory", "path": "old-dir2", "newName": "new-dir2"},
    {"kind": "file", "path": "src/old.rs", "newName": "src/new.rs"}
  ]
}
```

**Key differences:**
- **Single mode**: `target` + `new_name` (separate parameters)
- **Batch mode**: `targets` array where each target includes its own `new_name`

**Features:**
- ✅ Mix files and directories in same batch
- ✅ Conflict detection (prevents multiple renames to same destination)
- ✅ Atomic operation (all succeed or all rollback)
- ✅ All references updated across all renamed items
- ✅ Shares same `options` for all targets
- ✅ Works with `dryRun` option for preview or execution

**Example - Batch rename with CLI:**
```bash
# Preview batch rename (default dryRun: true)
mill tool rename '{
  "targets": [
    {"kind": "file", "path": "src/utils.rs", "newName": "src/helpers.rs"},
    {"kind": "file", "path": "src/config.rs", "newName": "src/settings.rs"}
  ],
  "options": {"scope": "standard"}
}'

# Execute batch rename (dryRun: false)
mill tool rename '{
  "targets": [
    {"kind": "directory", "path": "tests/unit", "newName": "tests/unit-tests"},
    {"kind": "directory", "path": "tests/integration", "newName": "tests/e2e"}
  ],
  "options": {"dryRun": false}
}'
```

**Use cases:**
- Renaming multiple files to match naming conventions (snake_case → kebab-case)
- Reorganizing project structure in one operation
- Refactoring related components together

See **[docs/tools/refactoring.md](docs/tools/refactoring.md#renameplan)** for complete details on validation, conflict detection, and advanced usage.

### Actionable Suggestions Configuration

Configure suggestion generation in `.typemill/analysis.toml`:

```toml
[suggestions]
min_confidence = 0.7  # Minimum confidence threshold
include_safety_levels = ["safe", "requires_review"]
max_per_finding = 3
generate_refactor_calls = true
```

**Presets**:
- `strict` - Only safe suggestions, high confidence
- `default` - Safe + requires_review, medium confidence
- `relaxed` - All levels, low confidence

## Development Commands

```bash
# Build the project
cargo build

# Development build with debug info
cargo build --release

# Run the server directly
cargo run

# Run tests/e2e
cargo nextest run

# Run tests/e2e with output
cargo nextest run --no-capture

# Run clippy for linting
cargo clippy

# Format code
cargo fmt

# Check code without building
cargo check

# CLI commands for configuration and management
./target/release/mill --version # Show version information
./target/release/mill setup    # Smart setup with auto-detection and LSP installation
./target/release/mill status   # Show what's working right now
./target/release/mill start    # Start the MCP server for AI assistants
./target/release/mill stop     # Stop the running MCP server
./target/release/mill serve    # Start WebSocket server
./target/release/mill link     # Link to AI assistants
./target/release/mill unlink   # Remove AI from config

# LSP server management
./target/release/mill install-lsp rust       # Install rust-analyzer
./target/release/mill install-lsp typescript # Install typescript-language-server
./target/release/mill install-lsp python     # Install python-lsp-server

# Build automation (xtask pattern - cross-platform Rust tasks)
cargo xtask install           # Install mill to ~/.local/bin
cargo xtask check-all         # Run all checks (fmt, clippy, test, deny)
cargo xtask check-duplicates  # Check for duplicate code
cargo xtask check-features    # Validate cargo features
cargo xtask new-lang python   # Scaffold new language plugin
cargo xtask --help            # Show all available tasks
```

## Testing Workflow

The test suite is organized into categories for fast iteration:

```bash
# Fast tests only (mock-based, ~10s)
# Note: make test auto-installs cargo-nextest if needed
cargo nextest run --workspace

# With LSP server tests (~60s, requires LSP servers installed)
cargo nextest run --workspace --features lsp-tests --status-level skip

# Full suite with heavy tests (~80s)
cargo nextest run --workspace --all-features --status-level skip

# Performance benchmarks
cargo nextest run --workspace --features heavy-tests
```

**Test Categories:**
- `fast-tests` (default): Mock-based unit and integration tests
- `lsp-tests`: Tests requiring real LSP servers (TypeScript, Rust, Python)
- `e2e-tests`: End-to-end workflow tests
- `heavy-tests`: Performance benchmarks and property-based testing

**Note:** Language support: TypeScript, Rust, Python, C#, and Swift (100% parity). Additional languages (Go, Java) preserved in git tag `pre-language-reduction` and can be restored using documented migration process (see `.debug/language-plugin-migration/`).

## Architecture & Configuration

### Access Patterns: Single Source of Truth

All tool functionality is implemented ONCE in `mill-handlers` and accessed via multiple interfaces:

```text
Handler (mill-handlers)
    ↓
┌───┴────┬──────────┬──────────┐
│        │          │          │
MCP   CLI-JSON  CLI-Flags  CLI-Helpers
(WS)                      (convert-naming)
```

- **MCP Protocol**: JSON-RPC over stdio/WebSocket
- **CLI JSON**: `mill tool <name> '{"target": ...}'`
- **CLI Flags**: `mill tool <name> --target file:path`
- **CLI Helpers**: `mill convert-naming --from kebab-case --to camelCase`

**Zero duplication**: Business logic lives only in handlers, CLI/MCP are thin adapters.

For detailed system architecture, see **[docs/architecture/overview.md](docs/architecture/overview.md)**.

For Docker deployment details, see **[docs/operations/docker_deployment.md](docs/operations/docker_deployment.md)**. For CLI usage, see the CLI Commands section above.

### Service Layer (`crates/mill-services/`)

- File service for file system operations
- AST service for code parsing and analysis
- Lock manager for concurrent operation safety
- Operation queue for request management
- Planner and workflow executor for complex operations

**Additional Components**

- **Plugin System** (`crates/mill-plugins/`) - Extensible plugin architecture
- **AST Processing** (`crates/mill-ast/`) - Code parsing and analysis
- **Client Library** (`crates/mill-client/`) - CLI client and WebSocket client

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

### LSP Server Management

The system spawns separate LSP server processes per configuration. Each server:

- Runs as child process with stdio communication
- Maintains its own initialization state
- Handles multiple concurrent requests
- Gets terminated on process exit

Supported language servers (configurable):

- TypeScript: `typescript-language-server`
- Rust: `rust-analyzer`
- Python: `pylsp`
- Swift: `sourcekit-lsp`
- C#: `omnisharp`

**Note:** Additional language servers (Go/gopls, Java/jdtls) can be configured but require language plugins from git tag `pre-language-reduction` and documented migration process (see `.debug/language-plugin-migration/`).


### Language Plugin Parity Status

TypeMill now has **100% feature parity** across TypeScript, Rust, Python, and Swift for all common capabilities:

| Capability | TypeScript | Rust | Python | Swift |
|-----------|-----------|------|--------|-------|
| Core LanguagePlugin | ✅ | ✅ | ✅ | ✅ |
| Import Support (5 traits) | ✅ | ✅ | ✅ | ✅ |
| Workspace Operations | ✅ | ✅ | ✅ | ✅ |
| Refactoring (3 operations) | ✅ | ✅ | ✅ | ✅ |
| Analysis (2 traits) | ✅ | ✅ | ✅ | ✅ |
| Manifest Management | ✅ | ✅ | ✅ | ✅ |
| **Project Creation** | ✅ | ✅ | ✅ | ✅ |

**Rust-specific features** (not applicable to other languages):
- ReferenceDetector
- ModuleDeclarationSupport (Rust `mod` declarations)
- ModuleLocator (Rust module file structure)

**Python restored**: 2025-10-25 with full parity implementation.
**Swift restored**: 2025-10-28 with full parity implementation.

See `.debug/language-plugin-migration/COMPLETE_PARITY_ANALYSIS.md` for detailed comparison.

## Configuration

The server loads configuration from `.typemill/config.json` in the current working directory. If no configuration exists, run `mill setup` to create one.

### Smart Setup

Use `mill setup` to configure LSP servers with auto-detection and auto-installation:

- Scans project for file extensions (respects .gitignore)
- Detects required languages (TypeScript, Rust, Python)
- Generates `.typemill/config.json` configuration file
- **Auto-downloads missing LSP servers** (prompts for user permission)
- Caches LSP binaries in `~/.mill/lsp/` for reuse across projects
- Verifies LSP servers are working after installation

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
cargo fmt && cargo clippy && cargo nextest run

# Or use Makefile targets
make check                # Run fmt + clippy + test
make check-duplicates     # Detect duplicate code & complexity
```

## Structured Logging Standards

All logging uses structured tracing with key-value pairs for machine-readable logs. See **[docs/development/logging_guidelines.md](docs/development/logging_guidelines.md)** for complete standards including field naming conventions, log levels, and production configuration.

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
./target/release/mill serve
```

### WebSocket Server Configuration
```bash
# Start WebSocket server (default port 3000)
./target/release/mill serve
```

### Environment Variables

TypeMill supports environment variable overrides for **all configuration values** using the `TYPEMILL__` prefix (note: double underscores as separators).

**Configuration Overrides (TYPEMILL__ Prefix):**

Any value in your configuration can be overridden via environment variables using the pattern `TYPEMILL__SECTION__SUBSECTION__KEY`:

```bash
# Server configuration
export TYPEMILL__SERVER__PORT=3000
export TYPEMILL__SERVER__HOST="127.0.0.1"
export TYPEMILL__SERVER__TIMEOUT_MS=5000

# JWT Authentication (RECOMMENDED for secrets)
export TYPEMILL__SERVER__AUTH__JWT_SECRET="your-secret-key-here"
export TYPEMILL__SERVER__AUTH__JWT_EXPIRY_SECONDS=3600
export TYPEMILL__SERVER__AUTH__JWT_ISSUER="typemill"
export TYPEMILL__SERVER__AUTH__JWT_AUDIENCE="typemill-clients"

# Cache configuration
export TYPEMILL__CACHE__ENABLED=true
export TYPEMILL__CACHE__TTL_SECONDS=3600
export TYPEMILL__CACHE__MAX_SIZE_BYTES=104857600

# Logging
export TYPEMILL__LOGGING__LEVEL="info"
export TYPEMILL__LOGGING__FORMAT="json"
```

**Secrets Management Best Practices:**

- ✅ **Use environment variables for secrets** (JWT_SECRET, API keys, database credentials)
- ✅ **Never commit secrets** to configuration files (`.typemill/config.json`, `mill.toml`)
- ✅ **Use `.env` files locally** (automatically gitignored)
- ✅ **Use secret management services in production** (HashiCorp Vault, AWS Secrets Manager, Azure Key Vault)
- ⚠️ **Keep server bound to `127.0.0.1`** for local development (not `0.0.0.0`)
- ⚠️ **Enable TLS when binding to non-loopback addresses** - Server enforces TLS for production deployments
- ℹ️ **Loopback addresses**: Only `127.0.0.1`, `::1`, and `localhost` are considered safe without TLS

**Example .env file:**
```bash
# .env (git ignored)
TYPEMILL__SERVER__AUTH__JWT_SECRET=dev-secret-change-in-production
TYPEMILL__SERVER__PORT=3000
TYPEMILL__CACHE__ENABLED=true
```

**Configuration Priority (highest to lowest):**

1. Environment variables (`TYPEMILL__*`)
2. Environment-specific profile in `mill.toml` (based on `TYPEMILL_ENV`)
3. Base configuration from `mill.toml` or `.typemill/config.toml`
4. Default values

**Logging:**
- `RUST_LOG` - Logging level (debug/info/warn/error)

**Cache Control (Legacy Toggles):**
- `TYPEMILL_DISABLE_CACHE=1` - Disable all caches (master switch)
- `TYPEMILL_DISABLE_AST_CACHE=1` - Disable only AST cache
- `TYPEMILL_DISABLE_IMPORT_CACHE=1` - Disable only import cache
- `TYPEMILL_DISABLE_LSP_METHOD_CACHE=1` - Disable only LSP method translation cache

See **[docs/operations/cache_configuration.md](docs/operations/cache_configuration.md)** for complete cache configuration guide.

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

**See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)** for complete guide on implementing language plugins:
- **NEW: `define_language_plugin!` macro** - Eliminates ~70 lines of boilerplate per plugin
- Plugin structure and schema requirements
- `LanguagePlugin` trait implementation
- Data types (ParsedSource, Symbol, ManifestData)
- Plugin registration and testing
- Reference implementations (Rust, TypeScript, Python)

**Plugin Refactoring (2025-10):**
The plugin system was refactored to eliminate duplication and boilerplate:
- **Phase 1:** Consolidated refactoring data structures (`CodeRange`, `ExtractableFunction`, etc.) into `mill-lang-common` - eliminated 186 lines of duplication
- **Phase 2:** Created `define_language_plugin!` macro to generate plugin scaffolding - eliminates ~70 lines per plugin
- **Phase 3:** Validated with 1086 passing tests, zero clippy warnings
- **Total impact:** ~272 lines eliminated, future plugins save ~70 lines each
- See `proposals/01_plugin_refactoring.proposal.md` for complete details

**Language Registry System (2025-10):**

TypeMill uses a centralized language registry (`languages.toml`) to manage feature flags across the workspace. This eliminates manual editing of 7 Cargo.toml files per language.

**Adding a new language (3 steps):**

1. **Create the plugin crate** in `crates/mill-lang-{name}/`
2. **Register in `languages.toml`:**
   ```toml
   [languages.newlang]
   path = "crates/mill-lang-newlang"
   plugin_struct = "NewLangPlugin"
   category = "full"     # or "config" for config-only languages
   default = false       # true = included in default build
   ```
3. **Run code generation:**
   ```bash
   cargo xtask sync-languages
   ```

This automatically generates:
- Feature flags in 7 crates (apps/mill, mill-server, mill-services, mill-ast, mill-plugin-system, mill-transport, mill-plugin-bundle)
- Dependency entries with correct optional/workspace flags
- Plugin linkage code in mill-plugin-bundle/src/lib.rs

**Language categories:**
- **Full languages** (rust/typescript/python/markdown): Propagate through 5 crates (services, ast, bundle, plugin-system, transport)
- **Config languages** (toml/yaml/gitignore): Only plugin-bundle (no AST or services)

**Testing:**
```bash
# Default build (6 languages)
cargo build -p mill

# With optional language
cargo build -p mill --features lang-python
```

**Note:** Additional language plugin implementations (Go, Java, Swift, C#) available in git tag `pre-language-reduction`. Python was successfully restored (2025-10-25) using the migration guide in `.debug/language-plugin-migration/PYTHON_MIGRATION_GUIDE.md`.

### Capability Trait Pattern

The codebase uses a **capability-based dispatch pattern** where plugins expose capabilities via traits instead of using downcasting or feature flags. This enables:
- **Language-agnostic code** - No compile-time coupling to specific language plugins
- **Plug-and-play** - Add new languages without touching shared crates
- **File-extension routing** - Correct plugin selected automatically

**Core capability traits:**
- `ManifestUpdater` - Update package manifest files (Cargo.toml, package.json)
- `ModuleLocator` - Find module files within packages
- `RefactoringProvider` - AST-based refactoring operations

For implementation details and examples, see **[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)**.

### Adding New MCP Tools

**See [contributing.md](contributing.md)** for complete step-by-step guide on adding new tools with handler architecture, registration, and best practices.

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
- **[contributing.md](contributing.md)** - Setup, PR process, adding tools, best practices
- **[docs/development/logging_guidelines.md](docs/development/logging_guidelines.md)** - Structured logging standards
- **[tests/e2e/TESTING_GUIDE.md](tests/e2e/TESTING_GUIDE.md)** - Testing architecture

### For Operators
- **[docs/operations/docker_deployment.md](docs/operations/docker_deployment.md)** - Docker deployment (development and production)
- **[docs/operations/cache_configuration.md](docs/operations/cache_configuration.md)** - Cache configuration and environment variables

### For Understanding the System
- **[docs/architecture/overview.md](docs/architecture/overview.md)** - Complete system architecture
- **[docs/architecture/internal_tools.md](docs/architecture/internal_tools.md)** - Internal vs public tools policy

### For Tool Reference
- **[docs/tools/](docs/tools/)** - Complete MCP tools API organized by category
- **[docs/tools/README.md](docs/tools/README.md)** - Complete tools catalog
- **[docs/README.md](docs/README.md)** - Documentation index and navigation hub