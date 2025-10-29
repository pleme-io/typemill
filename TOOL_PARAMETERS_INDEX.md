# Mill Tool Parameters Documentation

This directory contains comprehensive documentation of required and optional parameters for Mill tools.

## Documents

### 1. TOOL_PARAMETERS_SUMMARY.txt
**Quick reference guide** for developers needing quick parameter lookups.

Contents:
- One-page quick reference for each tool
- Parameter lists with types
- All error messages
- Key patterns used across tools
- Absolute file paths to source code

Best for: Quick lookups, parameter validation checklists

### 2. TOOL_PARAMETERS_ANALYSIS.md
**Comprehensive analysis document** with detailed implementation information.

Contents:
- Complete handler location and line numbers
- Required vs optional parameters with detailed descriptions
- Supported kinds/subcommands with function names
- All error messages with context
- Current documentation status and locations
- Circular dependency detection analysis
- Recommendations for improvements

Best for: Deep dives, documentation updates, understanding implementation

## Tools Covered

### 1. analyze.structure
**Purpose:** Code structure analysis (symbols, hierarchy, interfaces, inheritance, modules)

**Handler:** `/workspace/crates/mill-handlers/src/handlers/tools/analysis/structure.rs` (lines 1170-1251)

**Required:** `kind` parameter (one of: symbols, hierarchy, interfaces, inheritance, modules)

**Documentation Status:** Excellent - fully documented at `/workspace/docs/tools/analysis.md`

### 2. analyze.dependencies
**Purpose:** Dependency analysis (imports, graph, circular, coupling, cohesion, depth)

**Handler:** `/workspace/crates/mill-handlers/src/handlers/tools/analysis/dependencies.rs` (lines 1167-1356)

**Required:** `kind` parameter (one of: imports, graph, circular, coupling, cohesion, depth)

**Special Note:** Circular dependency detection has two modes:
- File-level (MVP): Detects self-imports
- Workspace-level (with `analysis-circular-deps` feature): Full cycle detection

**Documentation Status:** Good - but needs feature flag clarification

### 3. rename
**Purpose:** Rename symbols, files, or directories with automatic updates

**Handler:** `/workspace/crates/mill-handlers/src/handlers/rename_handler/mod.rs` (lines 28-277)

**Modes:** Two supported APIs:
- Single target mode (existing): `target` + `new_name`
- Batch mode (new): `targets` array with per-target `new_name`

**Key Features:**
- Safe by default: `dryRun: true` (preview mode)
- Explicit execution: `dryRun: false` required for actual changes
- Scope control: Update code, docs, comments, or everything
- Rust crate consolidation support

**Documentation Status:** Excellent - comprehensive at `/workspace/docs/tools/refactoring.md`

### 4. Circular Dependency Detection
**Primary Access:** `analyze.dependencies` with `kind="circular"`

**Location:** `/workspace/crates/mill-handlers/src/handlers/tools/analysis/dependencies.rs` (lines 316-393 for MVP, 1213-1311 for full)

**Functionality:**
- File-level detection (MVP): Self-referential imports
- Workspace-level (with feature): Full cycle detection using DependencyGraphBuilder
- Generates 4 types of actionable suggestions

**Documentation Status:** Documented but feature flag not detailed

## Key Validation Patterns

### All Tools Follow Consistent Pattern:
1. Check for required parameter
2. Validate against allowed values
3. Return clear error message if invalid
4. Dispatch to appropriate handler

### Error Format:
```text
ServerError::InvalidRequest("Clear message describing what's wrong and what's expected")
```text
### Analysis Tools:
- Use shared `ScopeParam` structure (file, directory, workspace scope)
- Support optional `options` for configuration
- Documented in engine.rs (lines 46-91)

### Refactoring Tools:
- Support `dryRun` pattern (safe default: true)
- Return plan objects for preview, execution results for apply
- Include checksum validation for staleness detection

## Documentation Improvements Needed

### 1. Circular Dependencies (Priority: High)
Update `/workspace/docs/tools/analysis.md` to document:
- Feature flag requirement (`analysis-circular-deps`)
- Difference between file-level and workspace-level detection
- When each mode is automatically used

### 2. Scope Parameter (Priority: Medium)
Create standalone documentation for shared `ScopeParam`:
- Location: `/workspace/crates/mill-handlers/src/handlers/tools/analysis/engine.rs` (lines 46-91)
- Document all scope types: file, directory, workspace, symbol
- Include examples for each

### 3. Batch Rename Deduplication (Priority: Medium)
Document the `dedupe_document_changes()` method:
- Location: `/workspace/crates/mill-handlers/src/handlers/rename_handler/mod.rs` (lines 285-334)
- Explain how multiple targets modifying same file have edits merged
- Show example of merged edits

### 4. Tool Author Reference (Priority: Low)
Create guide for adding new tools:
- Document validation patterns
- Include error message conventions
- Show handler structure template

## Source Code References

### Handler Implementations
```text
/workspace/crates/mill-handlers/src/handlers/tools/analysis/
  ├── structure.rs (analyze.structure)
  ├── dependencies.rs (analyze.dependencies)
  ├── circular_dependencies.rs (circular deps support)
  └── engine.rs (shared analysis engine)

/workspace/crates/mill-handlers/src/handlers/rename_handler/
  └── mod.rs (rename handler)
```text
### Documentation Files
```text
/workspace/docs/tools/
  ├── analysis.md (analyze.structure, analyze.dependencies)
  └── refactoring.md (rename)
```text
## Quick Parameter Lookup

### analyze.structure
```json
{
  "kind": "symbols|hierarchy|interfaces|inheritance|modules",
  "scope": {"type": "file|directory|workspace", "path": "..."},
  "options": {}
}
```text
### analyze.dependencies
```json
{
  "kind": "imports|graph|circular|coupling|cohesion|depth",
  "scope": {"type": "file|directory|workspace", "path": "..."},
  "options": {}
}
```text
### rename (single target)
```json
{
  "target": {
    "kind": "symbol|file|directory",
    "path": "...",
    "selector": {"position": {"line": 0, "character": 0}} // optional, for symbols
  },
  "newName": "...",
  "options": {
    "dryRun": true,
    "scope": "standard"
  }
}
```text
### rename (batch)
```json
{
  "targets": [
    {
      "kind": "symbol|file|directory",
      "path": "...",
      "newName": "..."
    }
  ],
  "options": {
    "dryRun": true,
    "scope": "standard"
  }
}
```text
## Testing Parameters

The most common validation test cases:

1. **Missing required parameter** - Should return "Missing 'kind' parameter"
2. **Invalid kind value** - Should return "Unsupported kind '...'
3. **Both single and batch targets** (rename only) - Should return "Cannot specify both"
4. **Neither single nor batch** (rename only) - Should return "Must specify either"
5. **Batch without new_name per target** (rename only) - Should return "missing new_name field"

## File Statistics

- Total documentation lines: 702
- Detailed analysis document: 494 lines
- Quick reference document: 208 lines
- Coverage: 3 tools + circular dependency analysis
- Handler line references: 50+
- Error messages documented: 15+

## Navigation

- For quick lookups: Start with `TOOL_PARAMETERS_SUMMARY.txt`
- For implementation details: See `TOOL_PARAMETERS_ANALYSIS.md`
- For actual code: Follow file paths and line numbers provided
- For user-facing docs: Check `/workspace/docs/tools/`

---

**Generated:** 2025-10-28
**Codebase:** Mill (TypeMill project)
**Coverage:** All public analysis and refactoring tools
**Status:** Complete analysis with recommendations for improvements