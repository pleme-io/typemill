# PROPOSAL: MCP API Cleanup (Beta Breaking Changes)

**Status:** Proposal
**Date:** 2025-10-08
**Effort:** ~30 hours
**Impact:** 44 tools → 35 tools (-20%)

---

## Summary

Aggressive cleanup of MCP API surface now that we're in beta (no backwards compatibility needed). Consolidate redundant tools, remove/internalize non-essential tools, and standardize naming.

---

## Changes Table

| Action | Tool(s) | New State | Reason | Effort |
|--------|---------|-----------|--------|--------|
| **MERGE** | `optimize_imports` + `organize_imports` | `organize_imports(remove_unused: bool)` | Single tool, remove_unused=true by default | 2h |
| **MERGE** | `system_status` + `health_check` | `health_check(level: "basic"\|"full")` | Same operation, different detail levels | 2h |
| **MERGE** | `suggest_refactoring` + `analyze_complexity` | `analyze_code(include_suggestions: bool)` | Suggestions use complexity metrics anyway | 4h |
| **MERGE** | `find_complexity_hotspots` + `analyze_project_complexity` | `analyze_project(output: "full"\|"hotspots"\|"summary", limit: int)` | Same scan, different views | 6h |
| **MERGE** | `prepare_call_hierarchy` + `get_call_hierarchy_incoming_calls` + `get_call_hierarchy_outgoing_calls` | `get_call_hierarchy(file, line, char, direction: "incoming"\|"outgoing"\|"both")` | Hide LSP 2-step protocol | 8h |
| **DELETE** | `web_fetch` | ❌ Removed | Claude has built-in WebFetch, security risk | 1h |
| **INTERNAL** | `get_completions` | 🔒 Internal only | AI doesn't need autocomplete | 2h |
| **INTERNAL** | `get_signature_help` | 🔒 Internal only | Not useful for AI agents | 2h |
| **DELETE** | `rename_symbol_strict` | ❌ Removed | `rename_symbol` handles position already | 1h |
| **RENAME** | `apply_edits` | `execute_edits` | Verb-noun consistency | 1h |
| **RENAME** | `batch_execute` | `execute_batch` | Verb-noun consistency | 1h |
| **KEEP** | `update_dependency` + `update_dependencies` | No change | Different operations (pin vs bulk) | 0h |

**Total Effort:** ~30 hours

---

## Before/After Comparison

### Before: 44 Public Tools

- **Navigation & Intelligence:** 13 tools
- **Editing & Refactoring:** 10 tools
- **Code Analysis:** 5 tools
- **File Operations:** 6 tools
- **Workspace Operations:** 5 tools
- **Advanced Operations:** 2 tools
- **System & Health:** 3 tools

### After: 35 Public Tools

- **Navigation & Intelligence:** 10 tools (-3)
  - Merged: 3 call hierarchy tools → 1
  - Internal: `get_completions`, `get_signature_help`

- **Editing & Refactoring:** 8 tools (-2)
  - Merged: `optimize_imports` into `organize_imports`
  - Removed: `rename_symbol_strict`

- **Code Analysis:** 3 tools (-2)
  - Merged: `analyze_complexity` + `suggest_refactoring` → `analyze_code`
  - Merged: `analyze_project_complexity` + `find_complexity_hotspots` → `analyze_project`

- **File Operations:** 6 tools (no change)

- **Workspace Operations:** 5 tools (no change)

- **Advanced Operations:** 2 tools (renamed)
  - `apply_edits` → `execute_edits`
  - `batch_execute` → `execute_batch`

- **System & Health:** 1 tool (-2)
  - Merged: `system_status` into `health_check`
  - Removed: `web_fetch`

---

## New Tool Signatures

### `organize_imports`
```json
{
  "file_path": "src/app.ts",
  "remove_unused": true  // NEW - default: true
}
```

### `health_check`
```json
{
  "level": "basic" | "full"  // NEW - default: "full"
}
```

### `analyze_code`
```json
{
  "file_path": "src/app.ts",
  "include_suggestions": true  // NEW - default: true
}
```

### `analyze_project`
```json
{
  "directory_path": "src/",
  "output": "full" | "hotspots" | "summary",  // NEW - default: "full"
  "limit": 10  // Optional: For hotspots mode
}
```

### `get_call_hierarchy`
```json
{
  "file_path": "src/app.ts",
  "line": 10,
  "character": 5,
  "direction": "incoming" | "outgoing" | "both"  // NEW
}
```

---

## Implementation Order

### Phase 1: Quick Wins (5 hours)
- Renames: `apply_edits` → `execute_edits`, `batch_execute` → `execute_batch`
- Delete: `web_fetch`, `rename_symbol_strict`
- Mark internal: `get_completions`, `get_signature_help`

### Phase 2: Simple Merges (6 hours)
- `organize_imports(remove_unused: bool)`
- `health_check(level: "basic"|"full")`

### Phase 3: Complex Merges (19 hours)
- `analyze_code(include_suggestions: bool)` - 4h
- `analyze_project(output, limit)` - 6h
- `get_call_hierarchy(direction)` - 8h

---

## Files to Modify

### Core Handlers
- `crates/cb-handlers/src/handlers/intelligence.rs` - Call hierarchy merge
- `crates/cb-handlers/src/handlers/editing.rs` - Import tools merge
- `crates/cb-handlers/src/handlers/analysis.rs` - Complexity tools merge
- `crates/cb-handlers/src/handlers/system.rs` - Health check merge
- `crates/cb-handlers/src/lib.rs` - Tool registration updates

### Type Definitions
- `crates/cb-protocol/src/types.rs` - Parameter type updates

### Documentation
- `API_REFERENCE.md` - Complete rewrite of affected sections
- `CLAUDE.md` - Update tool listings

### Tests
- `apps/codebuddy/tests/intelligence_tests.rs` - Call hierarchy tests
- `apps/codebuddy/tests/editing_tests.rs` - Import tests
- `apps/codebuddy/tests/analysis_tests.rs` - Complexity tests
- `apps/codebuddy/tests/system_tests.rs` - Health check tests

---

## Benefits

1. **Reduced Cognitive Load:** 20% fewer tools to learn
2. **Clearer Intent:** Tool names match user mental models
3. **Better Discoverability:** Related functionality grouped in parameters
4. **Consistent API:** Verb-noun naming throughout
5. **Simplified Maintenance:** Less code duplication

---

## Risks

- **Breaking Changes:** All existing MCP clients must update (acceptable for beta)
- **Migration Effort:** Users need to update tool calls
- **Testing Burden:** Need comprehensive integration tests for merged tools

---

## Decision Points

1. **Keep `update_dependency` + `update_dependencies`?**
   - YES - Different operations (pin specific version vs bulk upgrade)

2. **Remove `web_fetch` entirely?**
   - YES - Claude has built-in WebFetch, security risk, out of scope

3. **Internal vs Delete for `get_completions`?**
   - INTERNAL - Keep for potential future use, but hide from MCP listing

4. **Naming: `analyze_code` vs `analyze_complexity`?**
   - `analyze_code` - Broader scope, includes suggestions

---

## Next Steps

1. ✅ Review and approve proposal
2. 🔨 Implement Phase 1 (Quick Wins)
3. 🔨 Implement Phase 2 (Simple Merges)
4. 🔨 Implement Phase 3 (Complex Merges)
5. 📝 Update all documentation
6. ✅ Run full test suite
7. 🚀 Release as breaking v1.0.0

---

**Approval Required:** @maintainer
