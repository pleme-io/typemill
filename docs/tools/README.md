# MCP Tools Reference

**Complete API reference for all 36 TypeMill MCP tools**

This directory contains focused documentation for each tool category. Each category file follows a consistent structure with terse but complete documentation, real examples from the codebase, and common patterns.

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
| **Editing & Refactoring (15 tools)** ||||
| `rename.plan` | Refactoring | Plan rename operation (dry-run) | [refactoring.md](refactoring.md#renameplan) |
| `rename` | Refactoring | Execute rename (one-step) | [refactoring.md](refactoring.md#rename) |
| `extract.plan` | Refactoring | Plan extract function/variable (dry-run) | [refactoring.md](refactoring.md#extractplan) |
| `extract` | Refactoring | Execute extract (one-step) | [refactoring.md](refactoring.md#extract) |
| `inline.plan` | Refactoring | Plan inline operation (dry-run) | [refactoring.md](refactoring.md#inlineplan) |
| `inline` | Refactoring | Execute inline (one-step) | [refactoring.md](refactoring.md#inline) |
| `move.plan` | Refactoring | Plan move symbol (dry-run) | [refactoring.md](refactoring.md#moveplan) |
| `move` | Refactoring | Execute move (one-step) | [refactoring.md](refactoring.md#move) |
| `reorder.plan` | Refactoring | Plan reorder params/imports (dry-run) | [refactoring.md](refactoring.md#reorderplan) |
| `reorder` | Refactoring | Execute reorder (one-step) | [refactoring.md](refactoring.md#reorder) |
| `transform.plan` | Refactoring | Plan code transformation (dry-run) | [refactoring.md](refactoring.md#transformplan) |
| `transform` | Refactoring | Execute transform (one-step) | [refactoring.md](refactoring.md#transform) |
| `delete.plan` | Refactoring | Plan delete operation (dry-run) | [refactoring.md](refactoring.md#deleteplan) |
| `delete` | Refactoring | Execute delete (one-step) | [refactoring.md](refactoring.md#delete) |
| `workspace.apply_edit` | Refactoring | Apply refactoring plan | [refactoring.md](refactoring.md#workspaceapply_edit) |
| **Analysis (8 tools)** ||||
| `analyze.quality` | Analysis | Code quality analysis | [analysis.md](analysis.md#analyzequality) |
| `analyze.dead_code` | Analysis | Unused code detection | [analysis.md](analysis.md#analyzedead_code) |
| `analyze.dependencies` | Analysis | Dependency analysis | [analysis.md](analysis.md#analyzedependencies) |
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
**15 tools** following the unified plan → apply pattern for safe refactoring.

All refactoring operations support two-step workflow: generate a plan (dry-run) with `*.plan` tools, review changes, then apply with `workspace.apply_edit`. Quick one-step versions available for trusted operations.

### [Analysis](analysis.md)
**8 unified analysis tools** with consistent kind/scope API.

Comprehensive code analysis covering quality, dead code, dependencies, structure, documentation, and tests. All tools follow the same parameter pattern for easy adoption.

### [Workspace](workspace.md)
**4 workspace management tools** for package and text operations.

Create packages, extract dependencies for crate extraction, manage workspace member lists, and perform workspace-wide find/replace operations. Essential for maintaining multi-package Rust workspaces.

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
4. **Related code:** Check `crates/mill-protocol/`, `crates/cb-plugins/`

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
```

### Dry-Run Pattern

Refactoring tools support dry-run mode:

**Two-step (recommended):**
1. Generate plan with `*.plan` tool (always dry-run, never modifies files)
2. Review the plan
3. Apply with `workspace.apply_edit` (can set `dryRun: true` for final preview)

**One-step (quick):**
Use tool without `.plan` suffix to combine plan + execute in one call.

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
| TypeScript/JavaScript | ts, tsx, js, jsx | typescript-language-server | All navigation, refactoring, analysis |
| Rust | rs | rust-analyzer | All navigation, refactoring, analysis + Rust-specific features |

**Note:** Additional languages (Python, Go, Java, Swift, C#) available in git tag `pre-language-reduction`.

---

## See Also

- **[CLAUDE.md](../../CLAUDE.md)** - Main project documentation and AI agent instructions
- **[contributing.md](../../contributing.md)** - How to add new tools and contribute

---

## Links

- **[Main Documentation](../README.md)** - Complete documentation index
- **[Architecture](../architecture/overview.md)** - System architecture
- **[Contributing](../../contributing.md)** - Development guide
- **[API Contracts](../architecture/api_contracts.md)** - JSON schemas and validation rules

---

**Last Updated:** 2025-10-22
**API Version:** 1.0.0-rc4
