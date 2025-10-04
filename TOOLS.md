# CodeBuddy MCP Tools Quick Reference

**Version:** 1.0.0-rc1
**Last Updated:** 2025-10-04

Quick reference for all 44 MCP tools. For detailed API documentation, see [API.md](API.md).

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

## Editing & Refactoring (10 tools)

| Tool | Description | Required Parameters | Returns |
|------|-------------|---------------------|---------|
| `rename_symbol` | Rename a symbol across the project by name | `file_path`, `symbol_name`, `new_name` | Workspace edits with file changes |
| `rename_symbol_strict` | Rename a symbol at a specific position | `file_path`, `line`, `character`, `new_name` | Workspace edits with file changes |
| `rename_symbol_with_imports` | Rename symbol and update all imports | `file_path`, `old_name`, `new_name` | Files modified count |
| `organize_imports` | Organize and sort imports, remove unused | `file_path` | Import changes applied |
| `fix_imports` | Alias for organize_imports | `file_path` | Import changes applied |
| `get_code_actions` | Get available quick fixes and refactorings | `file_path` | Array of code actions |
| `format_document` | Format document using language server | `file_path` | Formatting changes |
| `extract_function` | Extract code into a new function | `file_path`, `start_line`, `end_line`, `function_name` | Workspace edits |
| `extract_variable` | Extract expression into a new variable | `file_path`, `start_line`, `start_character`, `end_line`, `end_character`, `variable_name` | Workspace edits |
| `inline_variable` | Inline a variable's value at usage sites | `file_path`, `variable_name`, `line` | Workspace edits |

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

## Workspace Operations (7 tools)

| Tool | Description | Required Parameters | Returns |
|------|-------------|---------------------|---------|
| `rename_directory` | Rename directory and auto-update all imports | `old_path`, `new_path` | Files moved, imports updated |
| `analyze_imports` | Analyze import statements in a file | `file_path` | Import breakdown by type |
| `find_dead_code` | Find potentially unused code in workspace | None | Array of unused symbols |
| `update_dependencies` | Update project dependencies via package manager | None (auto-detects) | Updated packages list |
| `update_dependency` | Update a single dependency to specific version | `dependency_name`, `version` | Old/new version, success status |
| `batch_update_dependencies` | Update multiple dependencies in one operation | `dependencies` (array) | Updated/failed package lists |
| `extract_module_to_package` | Extract code to new package (Rust-specific) | `source_module`, `target_package`, `symbols` | Package created, symbols moved |

---

## Advanced Operations (2 tools)

| Tool | Description | Required Parameters | Returns |
|------|-------------|---------------------|---------|
| `apply_edits` | Apply atomic multi-file edits with rollback | `edit_plan` | Files modified, edits applied |
| `batch_execute` | Execute multiple file operations atomically | `operations` (array) | Results per operation |

---

## LSP Lifecycle (3 tools)

| Tool | Description | Required Parameters | Returns |
|------|-------------|---------------------|---------|
| `notify_file_opened` | Notify LSP servers that file was opened | `file_path` | Notified servers list |
| `notify_file_saved` | Notify LSP servers that file was saved | `file_path` | Notified servers list |
| `notify_file_closed` | Notify LSP servers that file was closed | `file_path` | Notified servers list |

---

## System & Health (3 tools)

| Tool | Description | Required Parameters | Returns |
|------|-------------|---------------------|---------|
| `health_check` | Get comprehensive server health and statistics | None | Status, uptime, LSP servers, memory |
| `web_fetch` | Fetch content from a URL | `url` | Content, status code, headers |
| `system_status` | Get basic system operational status | None | Status, uptime, message |

---

## Quick Notes

### Common Optional Parameters
- **`dry_run`**: Preview changes without applying (many editing/file tools)
- **`workspace_id`**: Execute in remote workspace (read_file, write_file)
- **`include_declaration`**: Include definition in results (find_references)

### Indexing Conventions
- **Lines**: 1-indexed in user-facing APIs, 0-indexed in LSP protocol
- **Characters**: Always 0-indexed

### Language Support
LSP-based tools depend on configured language servers. Native tools (file ops, AST-based) support:
- TypeScript/JavaScript (SWC parser)
- Python (native AST)
- Go (go/parser)
- Rust (syn crate)
- Java (tree-sitter)

---

**For detailed parameters, return types, examples, and error handling, see [API.md](API.md)**
