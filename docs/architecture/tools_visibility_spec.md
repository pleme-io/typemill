# Tools Visibility Specification

**Purpose**: Definitive reference for which tools are public vs internal in final architecture.

---

## Public Tools (28 total)

### Navigation (8) - Point Queries for IDE Workflows
- `find_definition`
- `find_references`
- `find_implementations`
- `find_type_definition`
- `search_symbols` (or `search_workspace_symbols`)
- `get_symbol_info`
- `get_diagnostics`
- `get_call_hierarchy`

### Refactoring (7) - Unified API with dryRun Option
- `rename` - Rename files, directories, symbols (options.dryRun: true/false)
- `extract` - Extract function, variable, constant (options.dryRun: true/false)
- `inline` - Inline variable, function, constant (options.dryRun: true/false)
- `move` - Move symbols, files, directories (options.dryRun: true/false)
- `reorder` - Reorder imports, parameters, fields (options.dryRun: true/false)
- `transform` - Code transformations (options.dryRun: true/false)
- `delete` - Delete files, directories, dead code (options.dryRun: true/false)

### Workspace (4) - Workspace Operations
- `workspace.create_package`
- `workspace.extract_dependencies`
- `workspace.update_members`
- `workspace.find_replace`

### System (1) - Health Monitoring
- `health_check`

### Analysis (8) - Unified Analysis API ✅ **IMPLEMENTED**
- `analyze.quality` - Code quality analysis (complexity, smells, maintainability, readability)
- `analyze.dead_code` - Unused code detection (imports, symbols, parameters, variables, types, unreachable)
- `analyze.dependencies` - Dependency analysis (imports, graph, circular, coupling, cohesion, depth)
- `analyze.structure` - Code structure analysis (symbols, hierarchy, interfaces, inheritance, modules)
- `analyze.documentation` - Documentation quality (coverage, quality, style, examples, todos)
- `analyze.tests` - Test analysis (coverage, quality, assertions, organization)
- `analyze.batch` - Multi-file batch analysis with optimized AST caching
- `analyze.module_dependencies` - Rust module dependency analysis for crate extraction

---

## Internal Tools (19 total)

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

### Workspace Tools (3) - Legacy Operations
- `move_directory`
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

### Legacy Advanced (2) - Low-Level Plumbing
- `execute_edits` → replaced by unified refactoring API (rename, extract, etc.)
- `execute_batch` → replaced by `analyze.batch`

### Legacy Analysis - **FULLY REMOVED** ✅
The following legacy analysis tools were retired in Proposal 45:
- `analyze_project` → replaced by `analyze.quality("maintainability")`
- `analyze_imports` → replaced by `analyze.dependencies("imports")`
- `find_dead_code` → replaced by `analyze.dead_code`

Additional dead-weight tools removed:
- `find_unused_imports` - no unique functionality, covered by `analyze.dead_code`
- `analyze_code` - no unique functionality, covered by unified analysis API

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
2. **After Unified API**: 23 public, 25 internal (6 analysis tools moved to public)
3. **Proposal 45 cleanup**: 23 public, 19 internal (3 legacy analysis tools removed, analyze.batch deferred)
4. **Final state**: Analysis tools now public (analyze.quality, analyze.dead_code, analyze.dependencies, analyze.structure, analyze.documentation, analyze.tests)

**Proposal 45 Retirement Summary:**
- Removed 3 legacy analysis handlers (`analyze_project`, `analyze_imports`, `find_dead_code`)
- Total cleanup: 5 handlers removed (includes `find_unused_imports`, `analyze_code`)
- `analyze.batch` deferred to future implementation

---

**Reference**:
- Strategic architecture: `docs/architecture/PRIMITIVES.md`
- Unified Analysis API: `40_PROPOSAL_UNIFIED_ANALYSIS_API.md`
- Current state: `STRATEGIC_ARCHITECTURE_COMPLETE.md`

**Date**: 2025-10-12 (Updated after Proposal 45 completion)
