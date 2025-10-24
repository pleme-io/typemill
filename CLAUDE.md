<!-- This is the source of truth for AI agent instructions. CLAUDE.md and GEMINI.md are synchronized from this file. -->
# AGENTS.md

This file provides guidance to AI assistants when working with code in this repository.

## 📚 Essential Documentation for AI Agents

**Before working with this codebase, please read:**

1. **[docs/tools/](docs/tools/)** - **READ THIS FIRST** - Complete MCP tools API reference (36 tools across 5 categories)

---

**Architecture & Development:**
- **[docs/architecture/overview.md](docs/architecture/overview.md)** - System architecture (components, data flow, LSP integration)
- **[contributing.md](contributing.md)** - Contributor guide (add tools, handler architecture, best practices)
- **[docs/development/logging_guidelines.md](docs/development/logging_guidelines.md)** - Structured logging standards

**Deployment:**
- **[docs/operations/docker_deployment.md](docs/operations/docker_deployment.md)** - Docker deployment (dev and production)

## Project Information

**Package**: `codebuddy` | **Command**: `codebuddy` | **Runtime**: Rust

Pure Rust MCP server bridging Language Server Protocol (LSP) functionality to AI coding assistants with comprehensive tools for navigation, refactoring, code intelligence, and batch operations.

## MCP Tools

Codebuddy provides comprehensive MCP tools for code intelligence and refactoring. See **[docs/tools/](docs/tools/)** for complete API reference with detailed parameters, return types, and examples.

**Current Architecture**: 36 public tools visible to AI agents via MCP `tools/list`, plus 20 internal tools for backend workflows.

**Note:** Internal tools exist for backend use only (lifecycle hooks, workflow plumbing, legacy operations). These are hidden from MCP `tools/list` to simplify the API surface for AI agents. See [docs/architecture/internal_tools.md](docs/architecture/internal_tools.md) for details.

### Quick Reference (36 Public Tools)

**Navigation & Intelligence (8 tools)**
- `find_definition`, `find_references`, `search_symbols`
- `find_implementations`, `find_type_definition`, `get_symbol_info`
- `get_diagnostics`, `get_call_hierarchy`

**Editing & Refactoring (15 tools - Unified API)**
- **Plan Operations (7 tools)**: `rename.plan`, `extract.plan`, `inline.plan`, `move.plan`, `reorder.plan`, `transform.plan`, `delete.plan`
- **Quick Operations (7 tools)**: `rename`, `extract`, `inline`, `move`, `reorder`, `transform`, `delete` (one-step plan+execute)
- **Apply**: `workspace.apply_edit` (executes any plan)

**Workspace Operations (4 tools)**
- `workspace.create_package` (create new packages in workspace)
- `workspace.extract_dependencies` (extract module dependencies for crate extraction)
- `workspace.update_members` (update workspace member list)
- `workspace.find_replace` (find and replace text across workspace)

**Analysis (8 tools - Unified Analysis API)**
- `analyze.quality` (complexity, smells, maintainability, readability)
- `analyze.dead_code` (unused imports, symbols, parameters, variables, types, unreachable code)
- `analyze.dependencies` (imports, graph, circular dependencies, coupling, cohesion, depth)
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

### Refactoring Patterns: Two-Step vs One-Step

The Unified Refactoring API supports both safe two-step and convenient one-step patterns:

#### Two-Step Pattern (Recommended for Safety)
- **`*.plan()` commands are always dry runs.** They generate a plan of changes but never write to the filesystem. This is the primary way to preview a refactoring.
- **`workspace.apply_edit`** can be run with `dry_run: true` in its options for a final preview before execution.

#### One-Step Pattern (Quick Operations)
For convenience, each refactoring has a "quick" version that combines plan + execute in one call:
- **Quick tools**: `rename`, `extract`, `inline`, `move`, `reorder`, `transform`, `delete`
- **Usage**: Same parameters as `*.plan` versions, but automatically applies changes
- **Safety**: Less safe than two-step pattern - no preview before execution
- **When to use**: For small, low-risk refactorings when you trust the operation

**Example comparison:**
```json
// Two-step (safer): Preview first, then apply
{"name": "rename.plan", "arguments": {...}}
{"name": "workspace.apply_edit", "arguments": {"plan": ...}}

// One-step (faster): Direct execution
{"name": "rename", "arguments": {...}}
```

**Supported operations:**
- Refactoring plans: All `*.plan` commands (always dry-run)
- Workspace execution: `workspace.apply_edit` (supports `dry_run: true`)

**Benefits:**
- Preview changes before applying them
- No file system modifications occur
- Returns detailed preview of what would happen
- Safe for testing and validation

### Rust Crate Consolidation

The `rename.plan` command supports a special **consolidation mode** for merging Rust crates via the `options.consolidate` parameter:

```json
{
  "method": "tools/call",
  "params": {
    "name": "rename.plan",
    "arguments": {
      "target": {
        "kind": "directory",
        "path": "crates/source-crate"
      },
      "newName": "crates/target-crate/src/module",
      "options": {
        "consolidate": true
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
  "target": {"kind": "directory", "path": "crates/cb-types"},
  "newName": "crates/codebuddy-core/src/types"
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

When renaming Rust files using `rename.plan` + `workspace.apply_edit`, codebuddy automatically updates:

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
// 1. Generate rename plan
{
  "method": "tools/call",
  "params": {
    "name": "rename.plan",
    "arguments": {
      "target": {
        "kind": "file",
        "path": "src/utils.rs"
      },
      "newName": "src/helpers.rs"
    }
  }
}

// 2. Apply the plan (updates mod, use, and qualified paths)
{
  "method": "tools/call",
  "params": {
    "name": "workspace.apply_edit",
    "arguments": {
      "plan": "<plan from step 1>",
      "options": { "dryRun": false }
    }
  }
}
```

**Coverage:** Handles 80% of common rename scenarios. Complex cases involving non-parent file updates with nested module paths may require manual verification.

### Comprehensive Rename Coverage

CodeBuddy's rename functionality provides **100% coverage** of affected references by updating multiple file types during directory and file renames. All edits are surfaced in the dry-run plan for review before execution.

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

- `"all"` (default): Updates everything (100% coverage)
- `"code-only"`: Only code files (.rs, .ts, .js) - skips docs/configs
- `"custom"`: Fine-grained control with exclude patterns

**Example:**
```json
{
  "target": {"kind": "directory", "path": "old-dir"},
  "newName": "new-dir",
  "options": {
    "scope": "code-only"  // Skip .md, .toml, .yaml files
  }
}
```

**Coverage Example:**

Renaming `integration-tests/e2e/` → `tests/e2e/`:
- ✅ 3 Rust files (imports + string literals like `"tests/e2e/fixtures/data.json"`)
- ✅ 3 Cargo.toml files (workspace members list, package name, moved manifest)
- ✅ 3 Markdown files (links `[readme](tests/e2e/README.md)`)
- ✅ 2 Config files (.cargo/config.toml, CI YAML workflows)
- **Total: 11 files updated (100% of affected references)**

**Path Detection:**

Smart heuristic only updates strings that look like paths:
- Contains slash: `"old-dir/file.rs"` ✅
- Has file extension: `"config.toml"` ✅
- Prose text: `"We use old-dir as a pattern"` ❌ (skipped - no slash or extension)
- Relative paths match absolute: `"config/file.toml"` matches `/workspace/config`

**Implementation Details:**
- Scans all file types during planning phase (not just at execution)
- Uses language-specific plugins (Rust, Markdown, TOML, YAML)
- All edits appear in `rename.plan` dry-run output for review
- Atomic execution with rollback on any failure

### Batch Rename (Multiple Items at Once)

CodeBuddy supports renaming **multiple files and/or directories** in a single atomic operation using the `targets` parameter:

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
- ✅ Works with both `rename.plan` (preview) and `rename` (quick)

**Example - Batch rename with CLI:**
```bash
# Preview batch rename
codebuddy tool rename.plan '{
  "targets": [
    {"kind": "file", "path": "src/utils.rs", "newName": "src/helpers.rs"},
    {"kind": "file", "path": "src/config.rs", "newName": "src/settings.rs"}
  ],
  "options": {"scope": "all"}
}'

# Apply immediately (one-step)
codebuddy tool rename '{
  "targets": [
    {"kind": "directory", "path": "tests/unit", "newName": "tests/unit-tests"},
    {"kind": "directory", "path": "tests/integration", "newName": "tests/e2e"}
  ]
}'
```

**Use cases:**
- Renaming multiple files to match naming conventions (snake_case → kebab-case)
- Reorganizing project structure in one operation
- Refactoring related components together

See **[docs/tools/refactoring.md](docs/tools/refactoring.md#renameplan)** for complete details on validation, conflict detection, and advanced usage.

### Actionable Suggestions Configuration

Configure suggestion generation in `.codebuddy/analysis.toml`:

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
./target/release/codebuddy --version # Show version information
./target/release/codebuddy setup    # Smart setup with auto-detection
./target/release/codebuddy status   # Show what's working right now
./target/release/codebuddy start    # Start the MCP server for AI assistants
./target/release/codebuddy stop     # Stop the running MCP server
./target/release/codebuddy serve    # Start WebSocket server
./target/release/codebuddy link     # Link to AI assistants
./target/release/codebuddy unlink   # Remove AI from config

# Build automation (xtask pattern - cross-platform Rust tasks)
cargo xtask install           # Install codebuddy to ~/.local/bin
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
- `lsp-tests`: Tests requiring real LSP servers (TypeScript, Rust)
- `e2e-tests`: End-to-end workflow tests
- `heavy-tests`: Performance benchmarks and property-based testing

**Note:** Language support temporarily reduced to TypeScript + Rust during unified API refactoring. Multi-language support (Python, Go, Java, Swift, C#) preserved in git tag `pre-language-reduction`.

## Architecture & Configuration

### Access Patterns: Single Source of Truth

All tool functionality is implemented ONCE in `mill-handlers` and accessed via multiple interfaces:

```
Handler (mill-handlers)
    ↓
┌───┴────┬──────────┬──────────┐
│        │          │          │
MCP   CLI-JSON  CLI-Flags  CLI-Helpers
(WS)                      (convert-naming)
```

- **MCP Protocol**: JSON-RPC over stdio/WebSocket
- **CLI JSON**: `codebuddy tool <name> '{"target": ...}'`
- **CLI Flags**: `codebuddy tool <name> --target file:path`
- **CLI Helpers**: `codebuddy convert-naming --from kebab-case --to camelCase`

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

- **Plugin System** (`crates/cb-plugins/`) - Extensible plugin architecture
- **AST Processing** (`crates/cb-ast/`) - Code parsing and analysis
- **Virtual Filesystem** (`crates/cb-vfs/`) - FUSE filesystem support (Unix only)
  - ⚠️ **EXPERIMENTAL - Development Only**
  - Requires `SYS_ADMIN` capability (disables container security boundaries)
  - Not recommended for production use
  - To disable: set `"fuse": null` in `.codebuddy/config.json`
  - Docker: Use `deployment/docker-compose --profile fuse up codebuddy-fuse` to enable
- **API Interfaces** (`crates/cb-protocol/`) - Service trait definitions
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

**Note:** Additional language servers (Python/pylsp, Go/gopls, Java/jdtls) can be configured but require language plugins from git tag `pre-language-reduction`.

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
cargo fmt && cargo clippy && cargo nextest run

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

**Logging:**
- `RUST_LOG` - Logging level (debug/info/warn/error)

**Cache Control:**
- `CODEBUDDY_DISABLE_CACHE=1` - Disable all caches (master switch)
- `CODEBUDDY_DISABLE_AST_CACHE=1` - Disable only AST cache
- `CODEBUDDY_DISABLE_IMPORT_CACHE=1` - Disable only import cache
- `CODEBUDDY_DISABLE_LSP_METHOD_CACHE=1` - Disable only LSP method translation cache

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

**See [docs/development/plugin_development.md](docs/development/plugin_development.md)** for complete guide on implementing language plugins:
- Plugin structure and schema requirements
- `LanguagePlugin` trait implementation
- Data types (ParsedSource, Symbol, ManifestData)
- Plugin registration and testing
- Reference implementations (Rust, TypeScript)

**Note:** Additional language plugin implementations (Python, Go, Java, Swift, C#) available in git tag `pre-language-reduction`.

### Capability Trait Pattern

The codebase uses a **capability-based dispatch pattern** where plugins expose capabilities via traits instead of using downcasting or feature flags. This enables:
- **Language-agnostic code** - No compile-time coupling to specific language plugins
- **Plug-and-play** - Add new languages without touching shared crates
- **File-extension routing** - Correct plugin selected automatically

**Core capability traits:**
- `ManifestUpdater` - Update package manifest files (Cargo.toml, package.json)
- `ModuleLocator` - Find module files within packages
- `RefactoringProvider` - AST-based refactoring operations

For implementation details and examples, see **[docs/development/plugin_development.md](docs/development/plugin_development.md#plugin-dispatch-patterns)**.

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
- **[docs/development/workflows.md](docs/development/workflows.md)** - Workflow automation engine

### For Tool Reference
- **[docs/tools/](docs/tools/)** - Complete MCP tools API organized by category
- **[docs/tools/README.md](docs/tools/README.md)** - Quick catalog of all 36 tools