# Tools Visibility Specification

**Purpose**: Definitive reference for which tools are public vs internal in final architecture.

---

## Public Tools (23 final, 17 current)

### Navigation (8) - Point Queries for IDE Workflows
- `find_definition`
- `find_references`
- `find_implementations`
- `find_type_definition`
- `search_symbols` (or `search_workspace_symbols`)
- `get_symbol_info`
- `get_diagnostics`
- `get_call_hierarchy`

### Refactoring (7) - Unified Refactoring API
- `rename.plan`
- `extract.plan`
- `inline.plan`
- `move.plan`
- `reorder.plan`
- `transform.plan`
- `delete.plan`

### Workspace (1) - Single Execution Command
- `workspace.apply_edit`

### System (1) - Health Monitoring
- `health_check`

### Analysis (6) - **NOT YET IMPLEMENTED** - Unified Analysis API
- `analyze.quality` *(future)*
- `analyze.dead_code` *(future)*
- `analyze.dependencies` *(future)*
- `analyze.structure` *(future)*
- `analyze.documentation` *(future)*
- `analyze.tests` *(future)*

---

## Internal Tools (32 final, 25 current)

### Lifecycle (3) - Event Notifications
- `notify_file_opened`
- `notify_file_saved`
- `notify_file_closed`

### Internal Editing (1) - Backend Plumbing
- `rename_symbol_with_imports`

### Internal Workspace (1) - Backend Plumbing
- `apply_workspace_edit`

### Internal Intelligence (2) - LSP Backend
- `get_completions`
- `get_signature_help`

### Workspace Tools (4) - Legacy Operations
- `move_directory`
- `find_dead_code`
- `update_dependencies`
- `update_dependency`

### File Operations (4) - Legacy CRUD
- `create_file`
- `delete_file`
- `rename_file`
- `rename_directory`

### File Utilities (3) - Basic I/O
- `read_file`
- `write_file`
- `list_files`

### Legacy Analysis (5) - **MOVE TO INTERNAL** - Replaced by Unified Analysis API
- `find_unused_imports` → `analyze.dead_code("unused_imports")`
- `analyze_code` → `analyze.quality("complexity"|"smells")`
- `analyze_project` → `analyze.quality("maintainability")`
- `analyze_imports` → `analyze.dependencies("imports")`
- `get_document_symbols` → `analyze.structure("symbols")`

### Legacy Advanced (2) - **MOVE TO INTERNAL** - Low-Level Plumbing
- `execute_edits` → replaced by `workspace.apply_edit`
- `execute_batch` → replaced by `analyze.batch` *(future)*

---

## Design Rationale

### Public API Philosophy
**AI agents and MCP clients see high-level semantic operations:**
- Navigation: Point queries for specific code locations
- Refactoring: Two-step plan → apply pattern with safety guarantees
- Analysis: Bulk workspace analysis with actionable suggestions *(when implemented)*

### Internal API Philosophy
**Backend/workflows have access to low-level primitives:**
- Direct file I/O without semantic understanding
- Legacy operations for backward compatibility
- LSP plumbing (completions, signature help)
- Event lifecycle hooks

### Migration Path
1. **Current state**: 24 public, 18 internal (file utilities recently internalized)
2. **Next step**: Make 7 legacy tools internal → 17 public, 25 internal
3. **Future state**: Implement Unified Analysis API → 23 public, 32 internal

---

**Reference**:
- Strategic architecture: `docs/architecture/PRIMITIVES.md`
- Unified Analysis API: `40_PROPOSAL_UNIFIED_ANALYSIS_API.md`
- Current state: `STRATEGIC_ARCHITECTURE_COMPLETE.md`

**Date**: 2025-10-11
