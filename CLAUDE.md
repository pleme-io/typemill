# TypeMill - AI Agent Instructions

**Pure Rust MCP server bridging LSP functionality to AI coding assistants**

---

## 📚 Essential Documentation

**Before working with this codebase, read these docs:**

1. **[docs/tools/](docs/tools/)** - Complete MCP tools API reference (28 tools)
2. **[docs/quickstart.md](docs/quickstart.md)** - 5-minute setup guide
3. **[docs/cheatsheet.md](docs/cheatsheet.md)** - Command quick reference

**For contributors:**
- **[contributing.md](contributing.md)** - Setup, workflow, PR process
- **[docs/architecture/overview.md](docs/architecture/overview.md)** - System architecture
- **[docs/development/logging_guidelines.md](docs/development/logging_guidelines.md)** - Logging standards

**For operators:**
- **[docs/operations/docker_deployment.md](docs/operations/docker_deployment.md)** - Docker deployment

---

## Project Information

**Package**: `mill` | **Command**: `mill` | **Runtime**: Rust

Pure Rust MCP server providing 28 MCP tools for code navigation, refactoring, analysis, and batch operations across TypeScript, Rust, and Python projects.

---

## MCP Tools Quick Reference

**28 public tools across 5 categories:**

### Navigation & Intelligence (8 tools)
- `find_definition`, `find_references`, `search_symbols`
- `find_implementations`, `find_type_definition`, `get_symbol_info`
- `get_diagnostics`, `get_call_hierarchy`

### Editing & Refactoring (7 tools)
All support unified `dryRun` API:
- `rename` - Rename symbols/files/directories
- `extract` - Extract functions/variables/constants
- `inline` - Inline variables/functions
- `move` - Move symbols/files
- `reorder` - Reorder parameters
- `transform` - Code transformations
- `delete` - Delete symbols/files/dead code

### Workspace Operations (4 tools)
- `workspace.create_package` - Create new packages
- `workspace.extract_dependencies` - Extract module dependencies
- `workspace.update_members` - Update workspace members
- `workspace.find_replace` - Find and replace across workspace

### Analysis (8 tools)
- `analyze.quality` - Code quality analysis
- `analyze.dead_code` - Unused code detection
- `analyze.dependencies` - Dependency analysis
- `analyze.structure` - Code structure analysis
- `analyze.documentation` - Documentation quality
- `analyze.tests` - Test analysis
- `analyze.batch` - Multi-file batch analysis
- `analyze.module_dependencies` - Rust module dependencies

### System (1 tool)
- `health_check` - Server health & statistics

**Detailed API documentation**: See **[docs/tools/](docs/tools/)** for complete parameters, return types, and examples.

---

## Common Patterns

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

### Refactoring with dryRun

**Safe default** (preview only):
```json
{
  "name": "rename",
  "arguments": {
    "target": {"kind": "file", "path": "src/old.rs"},
    "newName": "src/new.rs"
    // options.dryRun defaults to true
  }
}
```

**Execution** (explicit opt-in):
```json
{
  "name": "rename",
  "arguments": {
    "target": {"kind": "file", "path": "src/old.rs"},
    "newName": "src/new.rs",
    "options": {"dryRun": false}
  }
}
```

**Key features:**
- All 7 refactoring tools use this pattern
- `dryRun: true` (default) - Preview changes only
- `dryRun: false` - Execute with rollback support

**Advanced refactoring patterns**: See **[docs/tools/refactoring.md](docs/tools/refactoring.md)** for:
- Batch renames
- Rust crate consolidation
- Comprehensive rename coverage (code, docs, configs)
- Scope control (code-only, standard, comments, everything)

---

## Development Commands

```bash
# Build
cargo build
cargo build --release

# Run server
cargo run

# Tests
cargo nextest run                                    # Fast tests (~10s)
cargo nextest run --features lsp-tests               # With LSP tests (~60s)
cargo nextest run --all-features                     # Full suite (~80s)

# Code quality
cargo fmt && cargo clippy && cargo nextest run       # Standard checks
cargo xtask check-all                                # All checks + deny

# CLI usage
./target/release/mill --version                      # Version info
./target/release/mill setup                          # Auto-setup with detection
./target/release/mill status                         # Show status
./target/release/mill start                          # Start MCP server
./target/release/mill docs                           # View embedded docs
./target/release/mill tool <name> <args>             # Call tool directly
./target/release/mill tools                          # List all tools

# Build automation (xtask - cross-platform Rust tasks)
cargo xtask install                                  # Install mill to ~/.local/bin
cargo xtask check-all                                # Run all checks
cargo xtask new-lang python                          # Scaffold new language plugin
```

---

## Testing Workflow

Tests are organized by speed for fast iteration:

```bash
# Fast tests (mock-based, ~10s) - default
cargo nextest run --workspace

# With LSP servers (~60s, requires installed LSP servers)
cargo nextest run --workspace --features lsp-tests

# Full suite with performance tests (~80s)
cargo nextest run --workspace --all-features
```

**Test categories:**
- `fast-tests` (default): Mock-based unit/integration tests
- `lsp-tests`: Tests requiring real LSP servers (TypeScript, Rust, Python)
- `e2e-tests`: End-to-end workflow tests
- `heavy-tests`: Performance benchmarks

**Language support**: TypeScript, Rust, Python (100% parity)

---

## Architecture Quick Facts

**Access pattern**: Single source of truth
- Handler logic in `mill-handlers` (one implementation)
- Multiple interfaces: MCP (WebSocket), CLI JSON, CLI flags

**Service layer** (`crates/mill-services/`):
- File service, AST service
- Lock manager, operation queue
- Planner, workflow executor

**Plugin system** (`crates/mill-plugins/`):
- Extensible language support
- Auto-discovery via inventory
- Capability-based traits

**Data flow**:
1. MCP client → tool request
2. Handler registry lookup
3. LSP client determines language server
4. LSP request/response
5. Transform to MCP format

**For complete architecture**: See **[docs/architecture/overview.md](docs/architecture/overview.md)**

---

## Configuration

### Smart Setup

```bash
mill setup
```

Auto-detects languages, generates `.typemill/config.json`, and installs LSP servers.

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

### Environment Variables

Override any config value with `TYPEMILL__` prefix:

```bash
# Server configuration
export TYPEMILL__SERVER__PORT=3000
export TYPEMILL__SERVER__HOST="127.0.0.1"

# JWT Authentication (recommended for secrets)
export TYPEMILL__SERVER__AUTH__JWT_SECRET="your-secret-key"

# Cache control
export TYPEMILL__CACHE__ENABLED=true
export TYPEMILL__CACHE__TTL_SECONDS=3600

# Logging
export TYPEMILL__LOGGING__LEVEL="info"
```

**Full configuration guide**: See **[docs/operations/cache_configuration.md](docs/operations/cache_configuration.md)**

---

## Language Plugin Development

### Quick scaffold with macro

```rust
use mill_lang_common::define_language_plugin;

define_language_plugin! {
    struct: MyLanguagePlugin,
    name: "mylang",
    extensions: ["ml"],
    manifest: "Package.mylang",
    lsp_command: "mylang-lsp",
    capabilities: [with_imports, with_workspace],
    // ... (eliminates ~70 lines of boilerplate)
}
```

**Complete plugin guide**: See **[docs/development/plugin-development.md](docs/development/plugin-development.md)**

---

## Performance & Security

**Performance**:
- Native Rust performance with zero-cost abstractions
- Memory safety without garbage collection overhead
- Efficient tokio-based async concurrency

**Security**:
- Memory safety via Rust ownership system
- Type safety prevents data races and null pointers
- JWT authentication for WebSocket server
- TLS enforcement for non-loopback addresses

---

## Production Deployment

### Binary Distribution
```bash
# Build optimized release
cargo build --release

# Binary ready for deployment
./target/release/mill serve
```

### WebSocket Server
```bash
./target/release/mill serve  # Default port 3000
```

**Docker deployment**: See **[docs/operations/docker_deployment.md](docs/operations/docker_deployment.md)**

---

## Code Quality Standards

- **Linting**: Clippy with strict rules
- **Formatting**: rustfmt with standard conventions
- **Type Safety**: Rust compile-time checks
- **Testing**: nextest with organized test suites
- **Logging**: Structured tracing with key-value pairs

**Run quality checks**:
```bash
cargo fmt && cargo clippy && cargo nextest run
make check  # Or use Makefile
```

**Logging standards**: See **[docs/development/logging_guidelines.md](docs/development/logging_guidelines.md)**

---

## Additional Documentation

### For Contributors
- **[contributing.md](contributing.md)** - Setup, PR process, best practices
- **[docs/development/plugin-development.md](docs/development/plugin-development.md)** - Create language plugins
- **[docs/development/testing.md](docs/development/testing.md)** - Testing architecture

### For Operators
- **[docs/operations/docker_deployment.md](docs/operations/docker_deployment.md)** - Docker deployment
- **[docs/operations/cache_configuration.md](docs/operations/cache_configuration.md)** - Cache configuration
- **[docs/operations/cicd.md](docs/operations/cicd.md)** - CI/CD integration

### For Understanding the System
- **[docs/architecture/overview.md](docs/architecture/overview.md)** - Complete system architecture
- **[docs/architecture/internal_tools.md](docs/architecture/internal_tools.md)** - Public vs internal tools
- **[docs/architecture/api_contracts.md](docs/architecture/api_contracts.md)** - Handler contracts

### For Tool Reference
- **[docs/tools/](docs/tools/)** - Complete MCP tools API (28 tools)
- **[docs/tools/README.md](docs/tools/README.md)** - Tools catalog
- **[docs/README.md](docs/README.md)** - Documentation hub

---

**📚 For detailed documentation:** Run `mill docs` or browse `docs/` directory
