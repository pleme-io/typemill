# CodeBuddy MCP Tools Support Matrix

**Last Updated:** 2025-10-01
**Version:** 0.1.0

---

## 📋 Complete MCP Function List

**Total MCP Functions**: 43

### Navigation & Intelligence (LSP-based)

| Function | Status | TypeScript/JS | Python | Go | Rust | Notes |
|----------|--------|---------------|--------|-----|------|-------|
| `find_definition` | ✅ Full | ✅ | ✅ | ✅ | ✅ | LSP-based, language server dependent |
| `find_references` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Supports `include_declaration` param |
| `find_implementations` | ✅ Full | ✅ | ✅ | ✅ | ✅ | For interfaces/abstract classes |
| `find_type_definition` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Find underlying type definitions |
| `search_workspace_symbols` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Queries ALL active LSP servers, merges results (max 10k symbols) |
| `get_document_symbols` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Hierarchical symbol structure |
| `prepare_call_hierarchy` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Returns call hierarchy item |
| `get_call_hierarchy_incoming_calls` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Requires item from prepare step |
| `get_call_hierarchy_outgoing_calls` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Requires item from prepare step |
| `get_hover` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Documentation, types, signatures |
| `get_completions` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Project-aware suggestions |
| `get_signature_help` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Parameter information |
| `get_diagnostics` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Errors, warnings, hints |

### Editing & Refactoring (LSP-based)

| Function | Status | TypeScript/JS | Python | Go | Rust | Notes |
|----------|--------|---------------|--------|-----|------|-------|
| `rename_symbol` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Supports dry_run, may return multiple candidates |
| `rename_symbol_strict` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Position-specific rename |
| `organize_imports` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Language-specific conventions |
| `get_code_actions` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Quick fixes, refactors |
| `format_document` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Language server formatter |

### Refactoring Tools (AST-based - System Plugin)

| Function | Status | TypeScript/JS | Python | Go | Rust | Notes |
|----------|--------|---------------|--------|-----|------|-------|
| `extract_function` | ⚠️ Stub | ⚠️ Basic | ⚠️ Basic | ⚠️ Basic | ⚠️ Basic | **STUB**: Basic line extraction only, no AST analysis |
| `inline_variable` | ⚠️ Stub | ⚠️ Preview | ⚠️ Preview | ⚠️ Preview | ⚠️ Preview | **STUB**: Returns preview only, no actual changes |
| `extract_variable` | ⚠️ Stub | ⚠️ Preview | ⚠️ Preview | ⚠️ Preview | ⚠️ Preview | **STUB**: Returns preview only, no actual changes |
| `fix_imports` | ✅ Full | ✅ | ✅ | ✅ | ✅ | **Delegates to LSP organize_imports**, removes all unused import types |

### File Operations

| Function | Status | TypeScript/JS | Python | Go | Rust | Notes |
|----------|--------|---------------|--------|-----|------|-------|
| `create_file` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Notifies LSP servers, handles overwrite |
| `read_file` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Via FileService with locking |
| `write_file` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Cache invalidation, locking |
| `delete_file` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Checks for imports, force option |
| `rename_file` | ✅ Full | ✅ | ✅ | ✅ | ✅ | **Updates imports automatically**, supports dry_run |
| `list_files` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Respects .gitignore, recursive option |

### Workspace Operations

| Function | Status | TypeScript/JS | Python | Go | Rust | Notes |
|----------|--------|---------------|--------|-----|------|-------|
| `rename_directory` | ✅ Full | ✅ | ✅ | ✅ | ✅ | **Automatically updates imports for all files**, supports dry_run |
| `analyze_imports` | ⚠️ Partial | ✅ | ✅ | ❌ | ❌ | TS/JS via cb_ast, Python via native parser |
| `find_dead_code` | ⚠️ Partial | ✅ | ❌ | ❌ | ❌ | TS/JS only via AST analysis |
| `update_dependencies` | ✅ Full | ✅ npm/yarn/pnpm | ✅ pip | ❌ | ✅ cargo | **Executes package manager commands**, returns stdout/stderr |

### Advanced Operations

| Function | Status | TypeScript/JS | Python | Go | Rust | Notes |
|----------|--------|---------------|--------|-----|------|-------|
| `apply_edits` | ✅ Full | ✅ | ✅ | ✅ | ✅ | **Atomic multi-file edits with rollback** |
| `rename_symbol_with_imports` | ✅ Full | ✅ | ⚠️ | ⚠️ | ⚠️ | Symbol rename + import updates (TS/JS best support) |
| `achieve_intent` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Workflow planning/execution, supports resume |

### LSP Lifecycle Notifications

| Function | Status | TypeScript/JS | Python | Go | Rust | Notes |
|----------|--------|---------------|--------|-----|------|-------|
| `notify_file_opened` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Triggers plugin hooks |
| `notify_file_saved` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Triggers plugin save hooks |
| `notify_file_closed` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Triggers plugin close hooks |

### System & Health

| Function | Status | TypeScript/JS | Python | Go | Rust | Notes |
|----------|--------|---------------|--------|-----|------|-------|
| `health_check` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Server status, uptime, plugin count |

### Web/Network (System Plugin)

| Function | Status | TypeScript/JS | Python | Go | Rust | Notes |
|----------|--------|---------------|--------|-----|------|-------|
| `web_fetch` | ✅ Full | ✅ | ✅ | ✅ | ✅ | Fetches URL content (plain text) |

---

## 🔑 Legend

- ✅ **Full**: Fully implemented and tested
- ⚠️ **Partial**: Partially implemented, limited language support
- ⚠️ **Stub**: Placeholder/preview only, not functional
- ⚠️ **Basic**: Basic functionality without advanced features
- ❌ **Not Supported**: Not available for this language

---

## 🚨 Implementation Status Notes

### **Fully Implemented Functions** (40 total)
All LSP-based navigation, intelligence, and editing functions are production-ready and work across all configured language servers. File operations and workspace operations are also fully functional.

### **Stub/Incomplete Functions** (3 total)

1. **`extract_function`**
   - **Status**: STUB
   - **Issue**: Only performs basic line extraction without AST analysis
   - **Code**: `crates/cb-plugins/src/system_tools_plugin.rs:447-502`
   - **TODO**: Needs proper AST parsing for parameter detection, scope analysis

2. **`inline_variable`**
   - **Status**: STUB
   - **Issue**: Returns preview only, doesn't perform actual inlining
   - **Code**: `crates/cb-plugins/src/system_tools_plugin.rs:505-547`
   - **TODO**: Implement AST-based variable usage scanning and replacement

3. **`extract_variable`**
   - **Status**: STUB
   - **Issue**: Returns preview only, doesn't perform actual extraction
   - **Code**: `crates/cb-plugins/src/system_tools_plugin.rs:550-601`
   - **TODO**: Implement AST-based expression extraction

### **Potentially Superfluous Functions**

1. **`notify_file_saved`** / **`notify_file_closed`** - May be redundant if LSP servers handle this automatically via file watchers.

---

## 🌐 Language-Specific Support Details

### TypeScript/JavaScript (Best Support)
- ✅ All LSP features via `typescript-language-server`
- ✅ Advanced AST analysis via native Rust `swc` parser (Phase B)
- ✅ Import graph analysis and updates
- ✅ Dead code detection
- ✅ File/directory rename with automatic import updates (all languages)

### Python (Good Support)
- ✅ All LSP features via `pylsp`
- ✅ Native AST parsing via subprocess (Phase A)
- ✅ Import analysis
- ⚠️ Limited refactoring (no dead code detection yet)

### Go (LSP Only)
- ✅ All LSP features via `gopls`
- ❌ No AST-based refactoring
- ❌ No import analysis beyond LSP

### Rust (LSP Only)
- ✅ All LSP features via `rust-analyzer`
- ❌ No AST-based refactoring
- ❌ No import analysis beyond LSP

### Adding New Languages
New languages can be added by:
1. Configuring LSP server in `.codebuddy/config.json`
2. All LSP-based functions work immediately
3. AST-based functions require parser implementation in `cb-ast` crate

---

## 🔬 Critical Features

### **Atomic Multi-File Editing** (`apply_edits`)
- ✅ **Fully implemented** with rollback on failure
- Creates file snapshots before any modifications
- Rolls back ALL files if ANY edit fails
- Invalidates AST cache for modified files
- File-level locking via LockManager
- **Test coverage**: `file_service.rs:728-1026`

### **Import-Aware File Operations**
- `rename_file`: ✅ Automatically updates imports in affected files
- `delete_file`: ✅ Checks for imports before deletion (unless forced)
- `rename_directory`: ✅ **Automatically updates imports for ALL files in directory** (all languages)

### **Workflow System** (`achieve_intent`)
- ✅ Intent → Workflow planning via `DefaultPlanner`
- ✅ Multi-step workflow execution
- ✅ Workflow pause/resume functionality
- ✅ Dry-run mode support
- Configuration: `.codebuddy/workflows.json`

---

## 📊 Plugin Architecture

### LSP Adapter Plugin (`LspAdapterPlugin`)
- **File**: `crates/cb-plugins/src/adapters/lsp_adapter.rs`
- **Purpose**: Bridges MCP tool calls to LSP protocol
- **Instances**: One per language (typescript, python, go, rust)
- **Tools**: 28 LSP-based functions
- **Dynamic registration**: Auto-created from `.codebuddy/config.json`

### System Tools Plugin (`SystemToolsPlugin`)
- **File**: `crates/cb-plugins/src/system_tools_plugin.rs`
- **Purpose**: Workspace-level operations and AST analysis
- **Tools**: 13 functions (file ops, refactoring, web, etc.)
- **Language support**: Varies by function

---

## 🎯 Recommendations

### **For Users**

**Production-Ready Functions:**
- ✅ Use all LSP-based navigation/intelligence functions confidently
- ✅ Use file operations (`create_file`, `rename_file`, `delete_file`)
- ✅ Use `apply_edits` for safe multi-file refactoring
- ✅ Use `rename_file` to automatically update imports

**Avoid or Use with Caution:**
- ⚠️ `extract_function`, `inline_variable`, `extract_variable` - stubs only
- ⚠️ `find_dead_code` - TS/JS only
- ⚠️ AST-based refactoring - TS/JS has best support

### **For Contributors**

**High Priority - Complete These Stubs:**
1. `extract_function` - Implement proper AST-based extraction
2. `inline_variable` - Implement AST-based inlining
3. `extract_variable` - Implement AST-based extraction

**Medium Priority - Expand Language Support:**
1. Add Python dead code detection
2. Add Go/Rust import analysis
3. Expand AST refactoring to more languages

**Low Priority - Improve Testing:**
1. Add concurrent operation tests for LockManager
2. Add integration tests for atomic rollback
3. Individual test functions instead of loops (better isolation)

---

## 📝 Configuration

### LSP Server Setup
```json
{
  "servers": [
    {
      "extensions": ["ts", "tsx", "js", "jsx"],
      "command": ["typescript-language-server", "--stdio"],
      "restartInterval": 10
    },
    {
      "extensions": ["py"],
      "command": ["pylsp"],
      "restartInterval": 5
    }
  ]
}
```

### Smart Setup
```bash
codebuddy setup  # Auto-detects project languages and configures servers
codebuddy status # Show working LSP servers
```

---

**Note**: This matrix reflects the current codebase state as of 2025-10-01. Language support depends on configured LSP servers in `.codebuddy/config.json`.
