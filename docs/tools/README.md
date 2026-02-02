# MCP Tools Reference

**Complete API reference for all TypeMill MCP tools**

This directory contains focused documentation for each tool category. Each category file follows a consistent structure with terse but complete documentation, real examples from the codebase, and common patterns.

> **📋 For the authoritative tool catalog** including internal tools, see **[architecture/specifications.md](../architecture/specifications.md#tools-visibility-specification)**

---

## Quick Reference

| Tool | Description |
|------|-------------|
| [`inspect_code`](#inspect_code) | Aggregate code intelligence (definition, references, types, diagnostics) |
| [`search_code`](#search_code) | Search workspace symbols |
| [`rename_all`](#rename_all) | Rename symbols/files/directories with reference updates |
| [`relocate`](#relocate) | Move symbols/files/directories with import updates |
| [`prune`](#prune) | Delete symbols/files/directories with cleanup |
| [`refactor`](#refactor) | Extract, inline, reorder, transform code |
| [`workspace`](#workspace) | Package management, find/replace, project verification |

> All refactoring tools default to `dryRun: true` (preview mode). Set `options.dryRun: false` to apply changes.

---

## Code Intelligence Tools

### inspect_code

Aggregate code intelligence for a symbol or position in a single request.

```json
{
  "name": "inspect_code",
  "arguments": {
    "filePath": "src/app.ts",
    "line": 9,
    "character": 5,
    "include": ["definition", "typeInfo", "references"]
  }
}
```

| Parameter | Required | Description |
|-----------|----------|-------------|
| `filePath` | Yes | File path |
| `line` | Yes* | 0-based line number |
| `character` | Yes* | 0-based column |
| `symbolName` | Yes* | Alternative to line/character |
| `include` | No | Array: `definition`, `typeInfo`, `references`, `implementations`, `callHierarchy`, `diagnostics` |
| `detailLevel` | No | `basic`, `standard`, `detailed` |
| `limit` | No | Max results (default 50) |

*Either `line`+`character` or `symbolName` required.

### search_code

Search for symbols across the workspace.

```json
{
  "name": "search_code",
  "arguments": {
    "query": "Config",
    "kind": "class",
    "limit": 20
  }
}
```

| Parameter | Required | Description |
|-----------|----------|-------------|
| `query` | Yes | Search query (fuzzy matched) |
| `kind` | No | Filter: `function`, `class`, `variable`, `interface`, etc. |
| `limit` | No | Max results (default 50) |

---

## Refactoring Tools

All refactoring tools use `options.dryRun` (default `true` = preview only).

### rename_all

Rename symbols, files, or directories with reference updates.

```json
{
  "name": "rename_all",
  "arguments": {
    "target": { "kind": "symbol", "filePath": "src/app.ts", "line": 9, "character": 5 },
    "newName": "NewName",
    "options": { "dryRun": false, "scope": "standard" }
  }
}
```

| Parameter | Required | Description |
|-----------|----------|-------------|
| `target.kind` | Yes | `symbol`, `file`, or `directory` |
| `target.filePath` | Yes | Path to target |
| `target.line` | For symbols | 0-based line |
| `target.character` | For symbols | 0-based column |
| `newName` | Yes | New name/path |
| `options.dryRun` | No | Default `true` (preview) |
| `options.scope` | No | `code`, `standard`, `comments`, `everything` |

### relocate

Move symbols, files, or directories with import updates.

```json
{
  "name": "relocate",
  "arguments": {
    "target": { "kind": "symbol", "filePath": "src/app.ts", "line": 9, "character": 5 },
    "destination": { "filePath": "src/utils.ts" },
    "options": { "dryRun": false }
  }
}
```

| Parameter | Required | Description |
|-----------|----------|-------------|
| `target` | Yes | Same as rename_all |
| `destination.filePath` | Yes | Destination path |
| `options.dryRun` | No | Default `true` |

### prune

Delete symbols, files, or directories with cleanup.

```json
{
  "name": "prune",
  "arguments": {
    "target": { "kind": "file", "filePath": "src/unused.ts" },
    "options": { "dryRun": false, "cleanupImports": true }
  }
}
```

| Parameter | Required | Description |
|-----------|----------|-------------|
| `target` | Yes | Same as rename_all |
| `options.dryRun` | No | Default `true` |
| `options.cleanupImports` | No | Remove orphaned imports |
| `options.force` | No | Delete even with references |

### refactor

Extract, inline, reorder, and transform code. See [refactor.md](refactor.md) for full documentation.

```json
{
  "name": "refactor",
  "arguments": {
    "action": "extract",
    "params": {
      "kind": "function",
      "source": { "filePath": "src/app.ts", "startLine": 10, "endLine": 20 },
      "name": "extractedFunction"
    },
    "options": { "dryRun": false }
  }
}
```

Actions: `extract`, `inline`, `reorder`, `transform`

---

## Workspace Tool

### workspace

Package management, find/replace, and project operations. See [workspace.md](workspace.md) for full documentation.

**Actions:**
- `create_package` - Create new package/crate
- `extract_dependencies` - Extract dependencies from code
- `find_replace` - Workspace-wide find/replace
- `update_members` - Update workspace members
- `verify_project` - Validate project structure

**Language guides:** [Rust](workspace-rust.md) | [TypeScript](workspace-typescript.md) | [Python](workspace-python.md)

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

- **[CLAUDE.md](../../CLAUDE.md)** - AI agent instructions
- **[contributing.md](../../contributing.md)** - Development guide
- **[Architecture](../architecture/core-concepts.md)** - System design
- **[API Specs](../architecture/specifications.md)** - JSON schemas
