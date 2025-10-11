# MCP Tools Quick Reference

**Purpose:** Fast lookup table for all Codebuddy MCP tools
**Format:** Tool name → Parameters → Returns (no examples or details)
**For detailed documentation:** See [API_REFERENCE.md](../API_REFERENCE.md)

**Version:** 1.0.0-rc4
**Last Updated:** 2025-10-09

---

## What's the Difference?

| This File | API_REFERENCE.md |
|-----------|--------|
| **Quick cheat sheet** (113 lines) | **Complete reference** (2,760 lines) |
| Tool names + parameters only | Examples, errors, patterns |
| 30-second scan | Implementation guide |

**Use this when:** You need to remember parameter names or check if a tool exists
**Use API_REFERENCE.md when:** You need to understand how to use a tool or handle errors

---

**Tools:** 48 public MCP tools
**Internal tools:** 5 backend-only tools (see [API_REFERENCE.md Internal Tools](../API_REFERENCE.md#internal-tools))

---

## Navigation & Intelligence (13 tools)

| Tool | Description | Required Parameters | Returns |
|------|-------------|---------------------|---------|
| `find_definition` | Find the definition of a symbol at a position | `file_path`, `line`, `character` | Definition locations with ranges |
| `find_references` | Find all references to a symbol | `file_path`, `line`, `character`, `symbol_name` | Array of reference locations |
| `find_implementations` | Find implementations of an interface/abstract class | `file_path`, `line`, `character` | Implementation locations |
| `find_type_definition` | Find the underlying type definition | `file_path`, `line`, `character` | Type definition locations |
| `get_document_symbols` | Get hierarchical symbol structure for a file | `file_path` | Nested symbol tree with ranges |
| `search_workspace_symbols` | Search for symbols across the workspace | `query` | Array of matching symbols with locations |
| `get_hover` | Get hover info (docs, types, signatures) | `file_path`, `line`, `character` | Markdown documentation content |
| `get_completions` | Get intelligent code completions | `file_path`, `line`, `character` | Completion items with details |
| `get_signature_help` | Get function signature help | `file_path`, `line`, `character` | Signature info with parameters |
| `get_diagnostics` | Get diagnostics (errors, warnings, hints) | `file_path` | Array of diagnostics with severity |
| `prepare_call_hierarchy` | Prepare call hierarchy for a symbol | `file_path`, `line`, `character` | Call hierarchy item |
| `get_call_hierarchy_incoming_calls` | Get incoming calls for a hierarchy item | `item` (from prepare) | Array of callers |
| `get_call_hierarchy_outgoing_calls` | Get outgoing calls from a hierarchy item | `item` (from prepare) | Array of callees |

---

## Editing & Refactoring (13 tools)

| Tool | Description | Required Parameters | Returns |
|------|-------------|---------------------|---------|
| `rename_symbol` | Rename a symbol across the project by name | `file_path`, `symbol_name`, `new_name` | Workspace edits with file changes |
| `rename_symbol_strict` | Rename a symbol at a specific position | `file_path`, `line`, `character`, `new_name` | Workspace edits with file changes |
| `rename.plan` | **Plan** rename refactoring (dry-run, part of unified API) | `file_path`, `symbol_name`, `new_name` | Refactoring plan (not applied) |
| `extract.plan` | **Plan** extract function/variable (dry-run, part of unified API) | `file_path`, `start_line`, `end_line`, `name`, `kind` | Refactoring plan (not applied) |
| `inline.plan` | **Plan** inline variable refactoring (dry-run, part of unified API) | `file_path`, `symbol_name`, `line` | Refactoring plan (not applied) |
| `move.plan` | **Plan** move symbol refactoring (dry-run, part of unified API) | `file_path`, `symbol_name`, `target_path` | Refactoring plan (not applied) |
| `workspace.apply_edit` | **Apply** a refactoring plan from *.plan tools | `edit` (from *.plan result) | Applied changes |
| `organize_imports` | Organize and sort imports, remove unused | `file_path` | Import changes applied |
| `optimize_imports` | Organize imports AND remove unused imports | `file_path` | Optimized import summary |
| `get_code_actions` | Get available quick fixes and refactorings | `file_path` | Array of code actions |
| `format_document` | Format document using language server | `file_path` | Formatting changes |
| `extract_variable` | Extract expression into a new variable | `file_path`, `start_line`, `start_character`, `end_line`, `end_character`, `variable_name` | Workspace edits |

---

## File Operations (6 tools)

| Tool | Description | Required Parameters | Returns |
|------|-------------|---------------------|---------|
| `create_file` | Create a new file with optional content | `file_path` | Success status, file path |
| `read_file` | Read file contents | `file_path` | File content as string |
| `write_file` | Write content to a file | `file_path`, `content` | Success status |
| `delete_file` | Delete a file with safety checks | `file_path` | Success status, warnings if imported |
| `rename_file` | Rename file and auto-update imports | `old_path`, `new_path` | Files updated, imports changed |
| `list_files` | List files in a directory with optional glob pattern filtering | `directory` (optional), `recursive` (optional), `pattern` (optional glob) | Array of file entries with metadata |

---

## Code Analysis (5 tools)

| Tool | Description | Required Parameters | Returns |
|------|-------------|---------------------|---------|
| `find_unused_imports` | Detect unused imports in a file | `file_path` | Array of unused import details |
| `analyze_complexity` | Calculate complexity metrics for functions | `file_path` | Complexity report with metrics |
| `suggest_refactoring` | Suggest refactoring opportunities | `file_path` | Array of refactoring suggestions |
| `analyze_project_complexity` | Scan directory for complexity metrics | `directory_path` | Project-wide complexity statistics |
| `find_complexity_hotspots` | Find most complex functions/classes | `directory_path` | Top N complexity hotspots |

---

## Workspace Operations (6 tools)

| Tool | Description | Required Parameters | Returns |
|------|-------------|---------------------|---------|
| `rename_directory` | Rename directory and auto-update all imports | `old_path`, `new_path` | Files moved, imports updated |
| `analyze_imports` | Analyze import statements in a file | `file_path` | Import breakdown by type |
| `find_dead_code` | Find potentially unused code in workspace | None | Array of unused symbols |
| `update_dependencies` | Update project dependencies via package manager | None (auto-detects) | Updated packages list |
| `update_dependency` | Update a single dependency to specific version | `dependency_name`, `version` | Old/new version, success status |
| `extract_module_to_package` | Extract code to new package (Rust-specific) | `source_module`, `target_package`, `symbols` | Package created, symbols moved |

---

## Advanced Operations (2 tools)

| Tool | Description | Required Parameters | Returns |
|------|-------------|---------------------|---------|
| `apply_edits` | Apply atomic multi-file edits with rollback | `edit_plan` | Files modified, edits applied |
| `batch_execute` | Execute multiple file operations atomically | `operations` (array) | Results per operation |

---

## System & Health (3 tools)

| Tool | Description | Required Parameters | Returns |
|------|-------------|---------------------|---------|
| `health_check` | Get comprehensive server health and statistics | None | Status, uptime, LSP servers, memory |
| `web_fetch` | Fetch content from a URL | `url` | Content, status code, headers |
| `system_status` | Get basic system operational status | None | Status, uptime, message |

---

## Quick Notes

### Unified Refactoring API (Two-Step Pattern)
The new unified API uses a **plan → apply** pattern for safe refactoring:
1. **`*.plan` tools** (e.g., `rename.plan`, `extract.plan`, `inline.plan`, `move.plan`) - Generate refactoring plan (dry-run, never writes to filesystem)
2. **`workspace.apply_edit`** - Apply the plan to make actual changes

**Benefits:**
- Preview all changes before applying
- Safe by design (*.plan commands are always dry-run)
- Consistent pattern across all refactorings
- Can use `dry_run: true` in `workspace.apply_edit` for final preview

### Common Optional Parameters
- **`dry_run`**: Preview changes without applying (many editing/file tools, workspace.apply_edit)
- **`workspace_id`**: Execute in remote workspace (read_file, write_file)
- **`include_declaration`**: Include definition in results (find_references)

### Indexing Conventions
- **Lines**: 1-indexed in user-facing APIs, 0-indexed in LSP protocol
- **Characters**: Always 0-indexed

### Language Support
LSP-based tools depend on configured language servers. Native tools (file ops, AST-based) support:
- TypeScript/JavaScript (SWC parser)
- Python (native AST)
- Go (tree-sitter-go)
- Rust (syn crate)
- Java (tree-sitter-java)
- Swift (tree-sitter-swift)
- C# (tree-sitter-c-sharp)

**AST Refactoring Support:**
- ✅ Full: TypeScript/JavaScript, Python, Rust, Go, Java, Swift
- ⚠️ Partial: C# (extract_function works, extract/inline_variable have bugs)

---

**For detailed parameters, return types, examples, and error handling, see [API_REFERENCE.md](../API_REFERENCE.md)**
