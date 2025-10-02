# CodeBuddy MCP Tools Support Matrix

**Last Updated:** 2025-10-02
**Version:** 1.0.0-beta

---

## 📋 Complete MCP Function List

**Total MCP Functions**: 42

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

### Refactoring Tools (LSP-first with AST fallback)

| Function | Status | TypeScript/JS | Python | Go | Rust | Notes |
|----------|--------|---------------|--------|-----|------|-------|
| `extract_function` | ✅ Full | ✅ LSP/AST | ✅ LSP/AST | ✅ LSP/AST | ✅ LSP/AST | **LSP-first with AST fallback**: Attempts LSP code actions, falls back to AST parsing if unsupported |
| `inline_variable` | ✅ Full | ✅ LSP/AST | ✅ LSP/AST | ✅ LSP/AST | ✅ LSP/AST | **LSP-first with AST fallback**: Attempts LSP code actions, falls back to AST parsing if unsupported |
| `extract_variable` | ✅ Full | ✅ LSP/AST | ✅ LSP/AST | ✅ LSP/AST | ✅ LSP/AST | **LSP-first with AST fallback**: Attempts LSP code actions, falls back to AST parsing if unsupported |
| `fix_imports` | ✅ Full | ✅ | ✅ | ✅ | ✅ | **Convenience wrapper for organize_imports** - delegates to LSP organize_imports, removes all unused import types |

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
| `analyze_imports` | ✅ Full | ✅ AST | ✅ AST | ✅ AST | ✅ AST | **All languages use AST parsing**. Rust via syn, Go via go/parser, TS/JS via SWC, Python via native AST |
| `find_dead_code` | ✅ Full | ✅ | ✅ | ✅ | ✅ | **LSP-based via workspace/symbol + textDocument/references** |
| `update_dependencies` | ✅ Full | ✅ npm/yarn/pnpm | ✅ pip | ✅ go mod | ✅ cargo | **Executes package manager commands**, auto-detects via project files, returns stdout/stderr |
| `extract_module_to_package` | ✅ Full | ✅ | ✅ | ✅ | ✅ | **Multi-language**: Rust via syn, TS/JS via directory move, Python via package structure, Go via go/parser, Java via Maven/Gradle. Extracts module to separate package, updates imports across workspace |

### Advanced Operations

| Function | Status | TypeScript/JS | Python | Go | Rust | Notes |
|----------|--------|---------------|--------|-----|------|-------|
| `apply_edits` | ✅ Full | ✅ | ✅ | ✅ | ✅ | **Atomic multi-file edits with rollback** |
| `rename_symbol_with_imports` | ✅ Full | ✅ | ✅ | ✅ | ✅ | **LSP-based symbol rename with automatic import updates**. Implemented as workflow via `achieve_intent` |
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

## 📚 Additional Resources

- **[MCP_API.md](./MCP_API.md)** - Complete API reference with parameters, examples, and return types for all 41 tools
- **[docs/architecture/ARCHITECTURE.md](./docs/architecture/ARCHITECTURE.md)** - Implementation architecture and design decisions
- **[CLAUDE.md](./CLAUDE.md)** - Project overview and development guide

---

**Notes**:
- This matrix reflects the current codebase state as of 2025-10-02
- Language support depends on configured LSP servers in `.codebuddy/config.json`
- **LSP-first with AST fallback** means the tool attempts to use LSP code actions first, and falls back to AST parsing if the language server doesn't support the operation
