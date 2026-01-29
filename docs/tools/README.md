# MCP Tools Reference

**Complete API reference for all TypeMill MCP tools**

This directory contains focused documentation for each tool category. Each category file follows a consistent structure with terse but complete documentation, real examples from the codebase, and common patterns.

> **📋 For the authoritative tool catalog** including internal tools, see **[architecture/specifications.md](../architecture/specifications.md#tools-visibility-specification)**

---

## Quick Catalog

| Tool | Category | Description | Documentation |
|------|----------|-------------|---------------|
| **Code Intelligence (2 tools)** ||||
| `inspect_code` | Intelligence | Aggregate code intelligence (definition, references, types, diagnostics) | [inspect_code.md](inspect_code.md) |
| `search_code` | Intelligence | Search workspace symbols | [search_code.md](search_code.md) |
| **Refactoring & Editing (4 tools)** ||||
| `rename_all` | Refactoring | Rename symbols/files/directories (dryRun option) | [rename_all.md](rename_all.md) |
| `relocate` | Refactoring | Move symbols/files/directories (dryRun option) | [relocate.md](relocate.md) |
| `prune` | Refactoring | Delete symbols/files/directories with cleanup (dryRun option) | [prune.md](prune.md) |
| `refactor` | Refactoring | Extract, inline, reorder, transform code (dryRun option) | [refactor.md](refactor.md) |
| **Workspace Management (1 tool)** ||||
| `workspace` | Workspace | Package management, find/replace, dependency extraction, project verification | [workspace.md](workspace.md) |

---

## Categories

### Code Intelligence
**2 tools** for code navigation and symbol information.

- **[inspect_code](inspect_code.md)** - Aggregate code intelligence: definition, references, type info, implementations, call hierarchy, and diagnostics in a single request
- **[search_code](search_code.md)** - Search workspace symbols with fuzzy matching

Navigate codebases with precision using language server protocol integration. Get rich symbol information with full IDE-quality intelligence.

### Refactoring & Editing
**4 tools** with unified dryRun API for safe, reviewable refactoring.

- **[rename_all](rename_all.md)** - Rename symbols, files, directories (updates all references)
- **[relocate](relocate.md)** - Move symbols, files, directories
- **[prune](prune.md)** - Delete symbols, files, directories with cleanup
- **[refactor](refactor.md)** - Extract, inline, reorder, transform code

All refactoring operations support `options.dryRun` parameter: default `true` generates a preview plan without modifying files, explicit `false` applies changes immediately with validation and rollback support.

### Workspace Management
**1 comprehensive tool** for package and text operations.

- **[workspace](workspace.md)** - Package creation, dependency extraction, find/replace, project verification

**Language-specific guides:** [Rust](workspace-rust.md) | [TypeScript](workspace-typescript.md) | [Python](workspace-python.md)

Supports Rust (Cargo), TypeScript (npm/yarn/pnpm), and Python (PDM/Poetry/Hatch) workspaces.

---

## Documentation Template

**Each category file in this directory follows this structure:**

### File Header
```markdown
# {Category} Tools

{Category overview - what these tools do, common use cases, patterns}

**Tool count:** {N} tools
**Related categories:** Links to related tool categories

---

## Tools
```
### Tool Entry Structure
```markdown
### tool_name

**Purpose:** One-sentence description of what this tool does

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| param_name | param_type | Yes/No | What this parameter does |

**Returns:**

{Description of return value structure}

**Example:**

```json
// MCP request
{
  "method": "tools/call",
  "params": {
    "name": "tool_name",
    "arguments": {
      "param": "value"
    }
  }
}

// Response
{
  "result": {
    "field": "value"
  }
}
```
**Notes:**
- Edge case handling
- Language-specific behavior
- Related tools and workflows
- Common errors and solutions

---
```
---

## Writing Guidelines

**For agents documenting tools:**

### Research Paths
1. **Handler code:** `crates/mill-handlers/src/{category}_handler.rs`
2. **Tests:** `tests/e2e/src/test_{category}.rs` or `tests/e2e/{category}_tests.rs`
3. **Existing docs:** Extract relevant content from existing tool documentation in this directory
4. **Related code:** Check `crates/mill-protocol/`, `crates/mill-plugins/`

### Documentation Requirements
- **Terse but complete** - No fluff, but cover all parameters and edge cases
- **Real examples** - Use actual parameter values from tests
- **Error cases** - Document common errors and solutions
- **Cross-references** - Link to related tools and patterns
- **Language-specific** - Call out Rust-specific behaviors (module updates, etc.)

### Style Conventions
- **Parameters table** - Always include for clarity
- **JSON examples** - Show full MCP request/response
- **Code blocks** - Use proper syntax highlighting
- **Anchors** - Use lowercase with underscores for tool names (`#inspect_code`)
- **Consistency** - Follow the template structure exactly

---

## Common Patterns

### MCP Protocol Usage

All tools are called via MCP JSON-RPC protocol:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "tool_name",
    "arguments": {
      "param1": "value1",
      "param2": "value2"
    }
  }
}
```
### Dry-Run Pattern

All refactoring tools use a unified `options.dryRun` parameter:

**Preview mode (default, safe):**
```json
{
  "name": "rename_all",
  "arguments": {
    "target": {...},
    "newName": "...",
    // options.dryRun defaults to true - preview only
  }
}
```
**Execution mode (explicit opt-in):**
```json
{
  "name": "rename_all",
  "arguments": {
    "target": {...},
    "newName": "...",
    "options": {
      "dryRun": false  // Execute changes
    }
  }
}
```
**Safe default:** `dryRun: true` requires explicit `dryRun: false` for execution.

### Error Handling

All tools return standard error format:

```json
{
  "error": {
    "code": -32000,
    "message": "Error description",
    "data": {
      "details": "Additional context"
    }
  }
}
```
Common error codes:
- `-32602` - Invalid parameters
- `-32603` - Internal error
- `-32000` - Server error
- `-32001` - LSP server not available

### File Path Conventions

- **Absolute paths:** Always use workspace-relative absolute paths
- **Path separators:** Use forward slashes (`/`) on all platforms
- **Extensions:** Include file extension (`.rs`, `.ts`, etc.)

---

## Language Support

| Language | Extensions | LSP Server | Tools Supported |
|----------|-----------|------------|-----------------|
| TypeScript/JavaScript | ts, tsx, js, jsx | typescript-language-server | All navigation and refactoring |
| Rust | rs | rust-analyzer | All navigation and refactoring + Rust-specific features |
| Python | py | python-lsp-server (pylsp) | All navigation and refactoring |
| Markdown | md | - | Config file support |
| YAML | yaml, yml | - | Config file support |
| TOML | toml | - | Config file support |

---

## See Also

- **[CLAUDE.md](../../CLAUDE.md)** - Main project documentation and AI agent instructions
- **[contributing.md](../../contributing.md)** - How to add new tools and contribute

### Tool Categories

- **[inspect_code](inspect_code.md)** - Aggregate code intelligence
- **[search_code](search_code.md)** - Symbol search
- **[rename_all](rename_all.md)** - Rename operations
- **[relocate](relocate.md)** - Move operations
- **[prune](prune.md)** - Delete operations
- **[refactor](refactor.md)** - Extract/inline/transform operations
- **[workspace](workspace.md)** - Workspace management

### Language-Specific Workspace Guides

- **[TypeScript Workspace](workspace-typescript.md)** - TypeScript project operations
- **[Rust Workspace](workspace-rust.md)** - Rust/Cargo workspace operations
- **[Python Workspace](workspace-python.md)** - Python project operations

---

## Links

- **[Main Documentation](../README.md)** - Complete documentation index
- **[Architecture](../architecture/core-concepts.md)** - System architecture
- **[Contributing](../../contributing.md)** - Development guide
- **[API Contracts](../architecture/specifications.md)** - JSON schemas and validation rules

---

**Last Updated:** 2025-10-25
**API Version:** 1.0.0-rc5
