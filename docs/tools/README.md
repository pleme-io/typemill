# MCP Tools Reference

**Complete API reference for all 29 TypeMill MCP tools**

This directory contains focused documentation for each tool category. Each category file follows a consistent structure with terse but complete documentation, real examples from the codebase, and common patterns.

> **📋 For the authoritative tool catalog** including internal tools, see **[architecture/tools_visibility_spec.md](../architecture/tools_visibility_spec.md)**

---

## Quick Catalog

| Tool | Category | Description | Documentation |
|------|----------|-------------|---------------|
| **Navigation & Intelligence (8 tools)** ||||
| `find_definition` | Navigation | Find symbol definition location | [navigation.md](navigation.md#find_definition) |
| `find_references` | Navigation | Find all symbol references | [navigation.md](navigation.md#find_references) |
| `search_symbols` | Navigation | Search workspace symbols | [navigation.md](navigation.md#search_symbols) |
| `find_implementations` | Navigation | Find interface implementations | [navigation.md](navigation.md#find_implementations) |
| `find_type_definition` | Navigation | Find underlying type definition | [navigation.md](navigation.md#find_type_definition) |
| `get_symbol_info` | Navigation | Get detailed symbol information | [navigation.md](navigation.md#get_symbol_info) |
| `get_diagnostics` | Navigation | Get errors/warnings/hints | [navigation.md](navigation.md#get_diagnostics) |
| `get_call_hierarchy` | Navigation | Get call hierarchy (callers/callees) | [navigation.md](navigation.md#get_call_hierarchy) |
| **Editing & Refactoring (7 tools)** ||||
| `rename` | Refactoring | Rename symbols/files/directories (dryRun option) | [refactoring.md](refactoring.md#rename) |
| `extract` | Refactoring | Extract functions/variables (dryRun option) | [refactoring.md](refactoring.md#extract) |
| `inline` | Refactoring | Inline variables/functions (dryRun option) | [refactoring.md](refactoring.md#inline) |
| `move` | Refactoring | Move symbols/files (dryRun option) | [refactoring.md](refactoring.md#move) |
| `reorder` | Refactoring | Reorder params/imports (dryRun option) | [refactoring.md](refactoring.md#reorder) |
| `transform` | Refactoring | Code transformations (dryRun option) | [refactoring.md](refactoring.md#transform) |
| `delete` | Refactoring | Delete symbols/files/directories (dryRun option) | [refactoring.md](refactoring.md#delete) |
| **Analysis (9 tools)** ||||
| `analyze.quality` | Analysis | Code quality analysis | [analysis.md](analysis.md#analyzequality) |
| `analyze.dead_code` | Analysis | Unused code detection | [analysis.md](analysis.md#analyzedead_code) |
| `analyze.dependencies` | Analysis | Dependency analysis | [analysis.md](analysis.md#analyzedependencies) |
| `analyze.cycles` | Analysis | Circular dependency detection | [analysis.md](analysis.md#analyzecycles) |
| `analyze.structure` | Analysis | Code structure analysis | [analysis.md](analysis.md#analyzestructure) |
| `analyze.documentation` | Analysis | Documentation quality | [analysis.md](analysis.md#analyzedocumentation) |
| `analyze.tests` | Analysis | Test analysis | [analysis.md](analysis.md#analyzetests) |
| `analyze.batch` | Analysis | Multi-file batch analysis | [analysis.md](analysis.md#analyzebatch) |
| `analyze.module_dependencies` | Analysis | Rust module dependencies | [analysis.md](analysis.md#analyzemodule_dependencies) |
| **Workspace (4 tools)** ||||
| `workspace.create_package` | Workspace | Create new package | [workspace.md](workspace.md#workspacecreate_package) |
| `workspace.extract_dependencies` | Workspace | Extract module dependencies | [workspace.md](workspace.md#workspaceextract_dependencies) |
| `workspace.update_members` | Workspace | Update workspace members | [workspace.md](workspace.md#workspaceupdate_members) |
| `workspace.find_replace` | Workspace | Find and replace text workspace-wide | [workspace.md](workspace.md#workspacefind_replace) |
| **System (1 tool)** ||||
| `health_check` | System | Server health & statistics | [system.md](system.md#health_check) |

---

## Categories

### [Navigation & Intelligence](navigation.md)
**8 LSP-based tools** for code navigation and symbol information.

Navigate codebases with precision using language server protocol integration. Find definitions, references, implementations, and get rich symbol information with full IDE-quality intelligence.

### [Editing & Refactoring](refactoring.md)
**7 tools** with unified dryRun API for safe, reviewable refactoring.

All refactoring operations use a single tool with `options.dryRun` parameter: default `true` generates a preview plan without modifying files, explicit `false` applies changes immediately with validation and rollback support.

### [Analysis](analysis.md)
**9 unified analysis tools** with consistent kind/scope API.

Comprehensive code analysis covering quality, dead code, dependencies, structure, documentation, and tests. All tools follow the same parameter pattern for easy adoption.

### [Workspace](workspace.md)
**4 workspace management tools** for package and text operations.
**Language-specific guides:** [Rust](workspace-rust.md) | [TypeScript](workspace-typescript.md) | [Python](workspace-python.md)

Create packages, extract dependencies, manage workspace member lists, and perform workspace-wide find/replace operations. Supports Rust (Cargo), TypeScript (npm/yarn/pnpm), and Python (PDM/Poetry/Hatch) workspaces.

### [System](system.md)
**1 health monitoring tool** for server diagnostics.

Check server status, LSP server health, memory usage, and active connections. Essential for production monitoring and debugging.

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
```text
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
```text
**Notes:**
- Edge case handling
- Language-specific behavior
- Related tools and workflows
- Common errors and solutions

---
```text
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
- **Anchors** - Use lowercase with underscores for tool names (`#find_definition`)
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
```text
### Dry-Run Pattern

All refactoring tools use a unified `options.dryRun` parameter:

**Preview mode (default, safe):**
```json
{
  "name": "rename",
  "arguments": {
    "target": {...},
    "newName": "...",
    // options.dryRun defaults to true - preview only
  }
}
```text
**Execution mode (explicit opt-in):**
```json
{
  "name": "rename",
  "arguments": {
    "target": {...},
    "newName": "...",
    "options": {
      "dryRun": false  // Execute changes
    }
  }
}
```text
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
```text
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
| TypeScript/JavaScript | ts, tsx, js, jsx | typescript-language-server | All navigation, refactoring, analysis |
| Rust | rs | rust-analyzer | All navigation, refactoring, analysis + Rust-specific features |

**Note:** Additional languages (Python, Go, Java, Swift, C#) available in git tag `pre-language-reduction`.

---

## See Also

- **[CLAUDE.md](../../CLAUDE.md)** - Main project documentation and AI agent instructions
- **[contributing.md](../../contributing.md)** - How to add new tools and contribute

### Tool Categories

- **[Navigation Tools](navigation.md)** - Code navigation and intelligence
- **[Refactoring Tools](refactoring.md)** - Editing and refactoring operations
- **[Analysis Tools](analysis.md)** - Code analysis and quality checks
- **[Workspace Tools](workspace.md)** - Workspace operations
- **[System Tools](system.md)** - Health checks and server status

### Language-Specific Workspace Tools

- **[TypeScript Workspace Tools](workspace-typescript.md)** - TypeScript project operations
- **[Rust Workspace Tools](workspace-rust.md)** - Rust/Cargo workspace operations
- **[Python Workspace Tools](workspace-python.md)** - Python project operations

---

## Links

- **[Main Documentation](../README.md)** - Complete documentation index
- **[Architecture](../architecture/overview.md)** - System architecture
- **[Contributing](../../contributing.md)** - Development guide
- **[API Contracts](../architecture/api_contracts.md)** - JSON schemas and validation rules

---

**Last Updated:** 2025-10-25
**API Version:** 1.0.0-rc5