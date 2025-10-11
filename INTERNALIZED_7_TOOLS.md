# 7 Tools Internalized - Strategic Alignment Complete

**Date**: 2025-10-11
**Status**: ✅ COMPLETE
**Test Results**: Tool registration tests passing (2/2)

---

## Summary

Internalized 7 legacy tools in preparation for Unified Analysis API implementation, reducing public API from 24 to 17 tools.

---

## Changes Made

### 1. AnalysisHandler → Internal (4 tools)
**File**: `crates/cb-handlers/src/handlers/tools/analysis/mod.rs`
**Tools**:
- `find_unused_imports` → will be replaced by `analyze.dead_code("unused_imports")`
- `analyze_code` → will be replaced by `analyze.quality("complexity"|"smells")`
- `analyze_project` → will be replaced by `analyze.quality("maintainability")`
- `analyze_imports` → will be replaced by `analyze.dependencies("imports")`

### 2. NavigationHandler → Split (1 tool moved)
**Files**:
- `crates/cb-handlers/src/handlers/tools/navigation.rs` - Split into two handlers
- Created `InternalNavigationHandler` for internal structure analysis

**Tools**:
- `get_document_symbols` → will be replaced by `analyze.structure("symbols")`
- **Kept public**: find_definition, find_references, find_implementations, find_type_definition, search_symbols, get_symbol_info, get_diagnostics, get_call_hierarchy (8 navigation tools)

### 3. AdvancedToolsHandler → Internal (2 tools)
**File**: `crates/cb-handlers/src/handlers/tools/advanced.rs`
**Tools**:
- `execute_edits` → replaced by `workspace.apply_edit`
- `execute_batch` → will be replaced by `analyze.batch` (future)

### 4. Test Updates
**File**: `crates/cb-server/tests/tool_registration_test.rs`
- `test_all_24_public_tools` → `test_all_17_public_tools`
- `test_all_18_internal_tools` → `test_all_25_internal_tools`
- **Result**: ✅ 2/2 tests passing

### 5. Registration Updates
**File**: `crates/cb-handlers/src/handlers/plugin_dispatcher.rs`
- Added `InternalNavigationHandler` to imports and registration
- Marked handlers as INTERNAL in logging messages

---

## Final Architecture

### Public API (17 tools)
- **Navigation (8)**: find_definition, find_references, find_implementations, find_type_definition, search_symbols, get_symbol_info, get_diagnostics, get_call_hierarchy
- **Refactoring Plans (7)**: rename.plan, extract.plan, inline.plan, move.plan, reorder.plan, transform.plan, delete.plan
- **Workspace (1)**: workspace.apply_edit
- **System (1)**: health_check

### Internal API (25 tools)
- **Lifecycle (3)**: notify_file_opened, notify_file_saved, notify_file_closed
- **Internal Editing (1)**: rename_symbol_with_imports
- **Internal Workspace (1)**: apply_workspace_edit
- **Internal Intelligence (2)**: get_completions, get_signature_help
- **Workspace Tools (4)**: move_directory, find_dead_code, update_dependencies, update_dependency
- **File Operations (4)**: create_file, delete_file, rename_file, rename_directory
- **File Utilities (3)**: read_file, write_file, list_files
- **Legacy Analysis (4)**: find_unused_imports, analyze_code, analyze_project, analyze_imports ← **NEW**
- **Structure Analysis (1)**: get_document_symbols ← **NEW**
- **Advanced (2)**: execute_edits, execute_batch ← **NEW**

---

## Rationale

Per `40_PROPOSAL_UNIFIED_ANALYSIS_API.md` and `TOOLS_VISIBILITY_SPEC.md`:

1. **Legacy analysis tools** are being replaced by Unified Analysis API (`analyze.*` commands)
2. **Low-level advanced tools** are plumbing replaced by higher-level APIs
3. **AI agents** should work with high-level semantic operations only
4. **Backend** retains access to all 42 tools (17 public + 25 internal)

---

## Next Steps

1. ✅ Tests passing
2. ✅ Build succeeds
3. 🔄 Commit changes
4. 🔮 **Future**: Implement Unified Analysis API (6 new public tools: analyze.quality, analyze.dead_code, analyze.dependencies, analyze.structure, analyze.documentation, analyze.tests)

---

**Final State**: 17 public tools, 25 internal tools, zero technical debt
