# Tools Visibility Specification

**Purpose**: Definitive reference for which tools are public vs internal in final architecture.

---

## Public Tools (23 total)

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

### Analysis (6) - Unified Analysis API ✅ **IMPLEMENTED**
- `analyze.quality` - Code quality analysis (complexity, smells, maintainability, readability)
- `analyze.dead_code` - Unused code detection (imports, symbols, parameters, variables, types, unreachable)
- `analyze.dependencies` - Dependency analysis (imports, graph, circular, coupling, cohesion, depth)
- `analyze.structure` - Code structure analysis (symbols, hierarchy, interfaces, inheritance, modules)
- `analyze.documentation` - Documentation quality (coverage, quality, style, examples, todos)
- `analyze.tests` - Test analysis (coverage, quality, assertions, organization)

---

## Internal Tools (25 total)

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
1. **Previous state**: 17 public, 25 internal (before Unified Analysis API)
2. **Current state**: 23 public, 25 internal (Unified Analysis API implemented)
3. **Note**: Analysis tools now public (analyze.quality, analyze.dead_code, analyze.dependencies, analyze.structure, analyze.documentation, analyze.tests)

---

**Reference**:
- Strategic architecture: `docs/architecture/PRIMITIVES.md`
- Unified Analysis API: `40_PROPOSAL_UNIFIED_ANALYSIS_API.md`
- Current state: `STRATEGIC_ARCHITECTURE_COMPLETE.md`

**Date**: 2025-10-11
