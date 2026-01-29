# TypeMill AI Agent Guide

> **📍 You are here:** AI agent quick reference (tool catalog, MCP patterns, common workflows)
> - 👤 **Human users**: See [README.md](README.md) for project overview & getting started
> - 📚 **Full docs**: See [docs/](docs/) for complete documentation

TypeMill is a Pure Rust MCP server that bridges Language Server Protocol (LSP) functionality to AI coding assistants, providing public tools for code navigation, refactoring, and workspace operations.

**Package**: `mill` | **Command**: `mill` | **Runtime**: Rust

---

## Essential Links

**Start Here:**
- **[docs/tools/](docs/tools/)** - Complete tool catalog with parameters, examples, return types
- **[docs/architecture/core-concepts.md](docs/architecture/core-concepts.md)** - System architecture and data flow

**Development:**
- **[contributing.md](contributing.md)** - Add tools, handler architecture, best practices
- **[docs/development/logging_guidelines.md](docs/development/logging_guidelines.md)** - Structured logging

**Deployment:**
- **[docs/operations/docker_deployment.md](docs/operations/docker_deployment.md)** - Production deployment

---

## Tool Quick Reference

### Code Intelligence (2 tools)
- `inspect_code` - Aggregate code intelligence (definition, references, types, diagnostics)
- `search_code` - Search workspace symbols

### Refactoring & Editing (4 tools)
All refactoring tools support **`options.dryRun`** for safe preview (default: true).

- `rename_all` - Rename symbols, files, directories (updates all references)
- `relocate` - Move symbols, files, directories
- `prune` - Delete symbols, files, directories with cleanup
- `refactor` - Extract, inline, reorder, transform code

### Workspace Management (1 tool)
- `workspace` - Package management, find/replace, dependency extraction, project verification

**Details**: [docs/tools/README.md](docs/tools/README.md)

---

## Critical Safety Pattern: dryRun

⚠️ **All refactoring tools default to preview mode** - changes are NOT applied unless explicitly opted in.

### Default Behavior (Safe Preview)
```json
{
  "name": "rename_all",
  "arguments": {
    "target": {"kind": "file", "path": "src/old.rs"},
    "newName": "src/new.rs"
    // options.dryRun defaults to true
  }
}
```
**Returns**: EditPlan showing what WOULD change

### Execution Mode (Explicit Opt-in)
```json
{
  "name": "rename_all",
  "arguments": {
    "target": {"kind": "file", "path": "src/old.rs"},
    "newName": "src/new.rs",
    "options": {"dryRun": false}  // Explicitly execute
  }
}
```
**Returns**: ApplyResult with files modified

**Workflow**: Always preview first, then execute if plan looks correct.

---

## Common Patterns

### MCP Tool Call Format
```json
{
  "method": "tools/call",
  "params": {
    "name": "inspect_code",
    "arguments": {
      "filePath": "src/app.ts",
      "line": 10,
      "character": 5,
      "include": ["definition", "typeInfo"]
    }
  }
}
```

### File Rename (Updates Imports Automatically)
```json
{
  "name": "rename_all",
  "arguments": {
    "target": {"kind": "file", "filePath": "src/utils.rs"},
    "newName": "src/helpers.rs",
    "options": {"dryRun": false}
  }
}
```
**Auto-updates**: Module declarations, use statements, qualified paths across all files.

### Code Symbol Move
```json
{
  "name": "relocate",
  "arguments": {
    "target": {"kind": "symbol", "filePath": "src/app.rs", "line": 10, "character": 5},
    "destination": {"filePath": "src/utils.rs"},
    "options": {"dryRun": false}
  }
}
```

### Extract Function
```json
{
  "name": "refactor",
  "arguments": {
    "action": "extract",
    "params": {
      "kind": "function",
      "source": {"filePath": "src/app.rs", "line": 15, "character": 8},
      "name": "handleLogin"
    },
    "options": {"dryRun": false}
  }
}
```

### Workspace Find & Replace
```json
{
  "name": "workspace",
  "arguments": {
    "action": "find_replace",
    "params": {
      "pattern": "oldName",
      "replacement": "newName",
      "mode": "literal"
    },
    "options": {"dryRun": false}
  }
}
```

---

## Important Notes

### Scope Control (Rename Operations)
Control what gets updated during renames:
- `"code"` - Code only (imports, module declarations)
- `"standard"` (default) - Code + docs + configs
- `"comments"` - Standard + code comments
- `"everything"` - Comments + markdown prose

```json
{
  "name": "rename_all",
  "arguments": {
    "target": {"kind": "directory", "filePath": "old-dir"},
    "newName": "new-dir",
    "options": {"scope": "standard"}
  }
}
```

### Rust Crate Consolidation
TypeMill detects when moving a crate into another crate's src/ directory and automatically:
1. Merges dependencies from source Cargo.toml
2. Removes source from workspace members
3. Updates all imports across workspace

```json
{
  "name": "rename_all",
  "arguments": {
    "target": {"kind": "directory", "filePath": "crates/source-crate"},
    "newName": "crates/target-crate/src/module",
    "options": {"dryRun": false}
  }
}
```
**Note**: Manually add `pub mod module;` to target-crate/src/lib.rs

### Comprehensive Link Updates
Renames automatically update:
- ✅ Code files (.rs, .ts, .js) - imports, module declarations, string literal paths
- ✅ Documentation (.md) - links, inline code, path references
- ✅ Configuration (.toml, .yaml) - path values, dependencies
- ✅ Cargo.toml - workspace members, package names

---

## Development Commands

```bash
# Build (debug - required for tests)
cargo build --workspace

# Build release
cargo build --release --workspace

# Run tests
cargo nextest run --workspace

# Format and lint
cargo fmt && cargo clippy

# CLI commands
./target/release/mill setup      # Auto-detect languages, install LSPs
./target/release/mill start      # Start MCP server
./target/release/mill status     # Check server status
./target/release/mill tools      # List all public tools
```

---

## Where to Learn More

**Tool Documentation:**
- [tools/README.md](docs/tools/README.md) - Complete catalog
- [tools/inspect_code.md](docs/tools/inspect_code.md) - Code intelligence tool
- [tools/search_code.md](docs/tools/search_code.md) - Symbol search tool
- [tools/rename_all.md](docs/tools/rename_all.md) - Rename tool
- [tools/relocate.md](docs/tools/relocate.md) - Move tool
- [tools/prune.md](docs/tools/prune.md) - Delete tool
- [tools/refactor.md](docs/tools/refactor.md) - Extract/inline/transform tool
- [tools/workspace.md](docs/tools/workspace.md) - Workspace management tool

**Architecture:**
- [architecture/core-concepts.md](docs/architecture/core-concepts.md) - System design, data flow
- [architecture/specifications.md](docs/architecture/specifications.md) - API contracts and tool visibility
- [architecture/internal_tools.md](docs/architecture/internal_tools.md) - Internal vs public tools

**Configuration:**
- [user-guide/configuration.md](docs/user-guide/configuration.md) - LSP servers, environment variables
- [operations/cache_configuration.md](docs/operations/cache_configuration.md) - Performance tuning

**Language Support:**
TypeScript, Rust, Python (full parity), Markdown, YAML, TOML (config files)

---

## Quick Troubleshooting

**Tool not working?**
1. Check file extension matches LSP server config
2. Verify LSP server is installed (`mill status`)
3. Review [user-guide/troubleshooting.md](docs/user-guide/troubleshooting.md)

**Import updates not applied?**
- Use `options.scope: "standard"` (default) for comprehensive updates
- Check [tools/rename_all.md](docs/tools/rename_all.md) for scope options

**Unexpected changes?**
- Always use `dryRun: true` first to preview
- Review EditPlan before setting `dryRun: false`

---

**For detailed examples, parameters, and return types**: See [docs/tools/](docs/tools/)
