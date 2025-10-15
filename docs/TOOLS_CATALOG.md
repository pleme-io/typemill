# MCP Tools Catalog

Fast lookup table for all Codebuddy MCP tools.

**Format:** Tool name → Parameters → Returns (no examples)
**Detailed docs:** [API_REFERENCE.md](API_REFERENCE.md)

---

**Public Tools:** 23 MCP tools (visible to AI agents)
**Internal Tools:** 20 backend-only tools (see [Internal Tools](#internal-tools-backend-only) below)

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

## Editing & Refactoring (11 tools)

| Tool | Description | Required Parameters | Returns |
|------|-------------|---------------------|---------|
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

## Unified Analysis API (7 tools)

| Tool | Description | Required Parameters | Returns |
|------|-------------|---------------------|---------|
| `analyze.quality` | Code quality analysis (complexity, smells, maintainability, readability) | `kind`, `scope`, `options` | AnalysisResult with findings |
| `analyze.dead_code` | Unused code detection (imports, symbols, parameters, variables, types, unreachable) | `kind`, `scope`, `options` | AnalysisResult with findings |
| `analyze.dependencies` | Dependency analysis (imports, graph, circular, coupling, cohesion, depth) | `kind`, `scope`, `options` | AnalysisResult with findings |
| `analyze.structure` | Code structure (symbols, hierarchy, interfaces, inheritance, modules) | `kind`, `scope`, `options` | AnalysisResult with findings |
| `analyze.documentation` | Documentation quality (coverage, quality, style, examples, todos) | `kind`, `scope`, `options` | AnalysisResult with findings |
| `analyze.tests` | Test analysis (coverage, quality, assertions, organization) | `kind`, `scope`, `options` | AnalysisResult with findings |
| `analyze.batch` | Multi-file batch analysis with optimized AST caching | `files`, `category`, `kinds`, `options` | BatchAnalysisResult |

---

## Workspace Operations (0 public tools)

**Note:** All workspace operation tools are now internal-only, used by backend workflows.

For workspace-level analysis, use the **Unified Analysis API** tools above with `scope: { type: "workspace" }`.

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
- ⚠️ Partial: C# (extract.plan works, inline.plan has known issues)

---

## Internal Tools (Backend Only)

Not visible in MCP `tools/list`. Used by backend workflows. AI agents should use public API instead.

| Category | Tools | Count |
|----------|-------|-------|
| **Lifecycle** | notify_file_opened, notify_file_saved, notify_file_closed | 3 |
| **File Operations** | create_file, delete_file, rename_file, rename_directory | 4 |
| **File Utilities** | read_file, write_file, list_files | 3 |
| **Workspace Tools** | move_directory, update_dependencies, update_dependency | 3 |
| **Structure Analysis** | get_document_symbols (replaced by `analyze.structure`) | 1 |
| **Advanced Plumbing** | execute_edits (replaced by `workspace.apply_edit`), execute_batch | 2 |
| **Legacy Editing** | rename_symbol_with_imports | 1 |
| **Legacy Workspace** | apply_workspace_edit | 1 |
| **Intelligence** | get_completions, get_signature_help | 2 |

**Total:** 20 internal tools

**Note:** Legacy analysis tools removed (Proposal 45). All analysis via unified `analyze.*` API.

---

**Detailed docs:** [API_REFERENCE.md](API_REFERENCE.md)
