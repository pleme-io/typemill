# Mill Tool Parameters Analysis

## Summary

This document details the required and optional parameters for three key Mill tools: `analyze.structure`, `analyze.dependencies`, and `rename`. It also examines circular dependency detection capabilities.

---

## 1. analyze.structure

**Handler Location:** `/workspace/crates/mill-handlers/src/handlers/tools/analysis/structure.rs`

**Public Tool:** Yes (line 1184-1185)

### Handler Class
- **File:** `structure.rs`
- **Class:** `StructureHandler` (lines 1170-1251)
- **Method:** `handle_tool_call()` (lines 1188-1250)

### Required Parameters

| Parameter | Type | Description | Validation |
|-----------|------|-------------|-----------|
| `kind` | string | Analysis subtype. Must be one of: `"symbols"`, `"hierarchy"`, `"interfaces"`, `"inheritance"`, `"modules"` | Validated at lines 1202-1210. Returns error if invalid. |

### Optional Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `scope` | object | (derived from context) | Analysis scope (file, directory, workspace) - handled by shared engine |
| `options` | object | {} | Additional configuration for analysis |

### Supported Kinds (Subcommands)

The `kind` parameter dispatches to different analysis functions:

1. **`"symbols"` (lines 1216-1217)**
   - Function: `detect_symbols()`
   - Returns: Total symbols, categorization by kind, visibility breakdown
   - Severity: Low (informational)

2. **`"hierarchy"` (lines 1220-1222)**
   - Function: `detect_hierarchy()`
   - Returns: Hierarchy depth metrics, root/leaf classes
   - Severity: Medium if depth > 5
   - Suggests flattening if too deep

3. **`"interfaces"` (lines 1224-1231)**
   - Function: `detect_interfaces()`
   - Returns: Interface count, method counts, fat interfaces (>10 methods)
   - Severity: Medium if "fat interfaces" detected (ISP violation)
   - Flags interfaces with >10 methods

4. **`"inheritance"` (lines 1234-1242)**
   - Function: `detect_inheritance()`
   - Returns: Inheritance depth chains, depth metrics
   - Severity: High if depth > 4 (fragile base class problem)
   - Suggests composition over inheritance

5. **`"modules"` (lines 1244-1246)**
   - Function: `detect_modules()`
   - Returns: Module count, items per module, god modules, orphaned items
   - Severity: Medium if god modules (>50 items) or many orphaned items
   - Suggests module splitting

### Current Error Messages

**Missing `kind` parameter (lines 1196-1199):**
```text
ServerError::InvalidRequest("Missing 'kind' parameter")
```text
**Unsupported `kind` value (lines 1206-1210):**
```text
ServerError::InvalidRequest(
    "Unsupported kind '{}'. Supported: 'symbols', 'hierarchy', 'interfaces', 'inheritance', 'modules'"
)
```text
### Current Documentation

- **Location:** `/workspace/docs/tools/analysis.md` (lines 435-535)
- **Status:** Well documented
- **Includes:**
  - Purpose statement
  - Parameter table
  - All 5 supported kinds with descriptions
  - Scope types (file, directory, workspace)
  - Return structure details
  - Example JSON request/response
  - Implementation notes

---

## 2. analyze.dependencies

**Handler Location:** `/workspace/crates/mill-handlers/src/handlers/tools/analysis/dependencies.rs`

**Public Tool:** Yes (line 1181-1182)

### Handler Class
- **File:** `dependencies.rs`
- **Class:** `DependenciesHandler` (lines 1167-1356)
- **Method:** `handle_tool_call()` (lines 1185-1355)

### Required Parameters

| Parameter | Type | Description | Validation |
|-----------|------|-------------|-----------|
| `kind` | string | Analysis subtype. Must be one of: `"imports"`, `"graph"`, `"circular"`, `"coupling"`, `"cohesion"`, `"depth"` | Validated at lines 1199-1207. Returns error if invalid. |

### Optional Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `scope` | object | (derived from context) | Analysis scope - handled by shared engine |
| `options` | object | {} | Additional configuration |

### Supported Kinds (Subcommands)

1. **`"imports"` (lines 1315-1323)**
   - Function: `detect_imports()`
   - Returns: Import statements categorized as external, internal, or relative
   - Uses plugin-based AST parsing for accuracy
   - Severity: Low (informational)

2. **`"graph"` (lines 1325-1327)**
   - Function: `detect_graph()`
   - Returns: Dependency graph with fan-in/fan-out metrics
   - Metrics: direct_dependencies, indirect_dependencies, fan_in, fan_out
   - Severity: Low (informational)

3. **`"circular"` (lines 1212-1311)**
   - Special handling with workspace-wide analysis (if feature enabled)
   - Uses `DependencyGraphBuilder` (when `analysis-circular-deps` feature enabled)
   - Function: `find_circular_dependencies()`
   - Returns: Cycles with full import chains
   - Severity: High (architectural smell)
   - Generates actionable suggestions (extract interface, dependency injection, etc.)
   - **Note:** Can run in two modes:
     - With feature: Workspace-wide cycle detection (lines 1213-1311)
     - Without feature: File-level detection via `detect_circular()` (lines 1303-1310)

4. **`"coupling"` (lines 1329-1337)**
   - Function: `detect_coupling()`
   - Returns: Afferent/efferent coupling, instability metric
   - Severity: Medium if instability > 0.7
   - Suggests reducing coupling via interfaces or DI

5. **`"cohesion"` (lines 1339-1347)**
   - Function: `detect_cohesion()`
   - Returns: LCOM (Lack of Cohesion of Methods) score
   - Severity: Medium if LCOM > 0.5
   - Suggests module splitting if low cohesion

6. **`"depth"` (lines 1349-1351)**
   - Function: `detect_depth()`
   - Returns: Max dependency depth, dependency chain
   - Severity: Medium if depth > 5
   - Suggests flattening dependency tree

### Current Error Messages

**Missing `kind` parameter (lines 1193-1196):**
```text
ServerError::InvalidRequest("Missing 'kind' parameter")
```text
**Unsupported `kind` value (lines 1203-1207):**
```text
ServerError::InvalidRequest(
    "Unsupported kind '{}'. Supported: 'imports', 'graph', 'circular', 'coupling', 'cohesion', 'depth'"
)
```text
### Current Documentation

- **Location:** `/workspace/docs/tools/analysis.md` (lines 313-432)
- **Status:** Well documented
- **Includes:**
  - Purpose statement
  - Parameter table with all 6 kinds
  - Scope types
  - Return structure details
  - Example JSON for "imports" kind
  - Plugin-based parsing notes
  - Circular dependency feature requirements

---

## 3. rename

**Handler Location:** `/workspace/crates/mill-handlers/src/handlers/rename_handler/mod.rs`

**Public Tool:** Yes (line 146-147)

### Handler Class
- **File:** `mod.rs` (rename_handler/)
- **Class:** `RenameHandler` (lines 28-277)
- **Method:** `handle_tool_call()` (lines 150-277)

### Required Parameters

The `rename` tool supports two modes: **single mode** and **batch mode**.

#### Single Target Mode (Existing API)

| Parameter | Type | Required | Description | Validation |
|-----------|------|----------|-------------|-----------|
| `target` | object | Yes (if not using `targets`) | Target to rename | Must have `kind`, `path`. See structure below. |
| `target.kind` | string | Yes | Target type: `"symbol"`, `"file"`, or `"directory"` | Validated at line 197 |
| `target.path` | string | Yes | File/directory path or file for symbol | Must be valid path |
| `new_name` | string | Yes | New name or path | Required for single mode (line 171-175) |

#### Batch Mode (New API)

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `targets` | array | Yes (if not using `target`) | Array of RenameTarget objects |
| `targets[].kind` | string | Yes | Target type per target |
| `targets[].path` | string | Yes | Path per target |
| `targets[].new_name` | string | Yes | Required in batch mode (line 473-479) |

### Optional Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `options` | object | {} | Rename options |
| `options.dryRun` | boolean | true | Preview mode (no changes) vs execution (line 82) |
| `options.scope` | string | "standard" | What to update: `"code"`, `"standard"`, `"comments"`, `"everything"`, or `"custom"` (line 94) |
| `options.custom_scope` | RenameScope | null | Fine-grained scope control when scope="custom" (line 98) |
| `options.consolidate` | boolean | auto-detect | Merge Rust crate into another (line 104) |

### Target Structure (for symbol renames)

For symbol renaming, additional selector parameter is optional:

```json
{
  "kind": "symbol",
  "path": "/path/to/file.rs",
  "selector": {
    "position": {"line": 10, "character": 5}
  }
}
```text
### Current Error Messages

**Missing arguments (lines 158-161):**
```text
ServerError::InvalidRequest("Missing arguments for rename")
```text
**Invalid parameters (lines 163-165):**
```text
ServerError::InvalidRequest("Invalid rename parameters: {error}")
```text
**Missing new_name in single mode (lines 171-175):**
```text
ServerError::InvalidRequest("new_name is required for single target mode")
```text
**Both target and targets specified (lines 215-218):**
```text
ServerError::InvalidRequest(
    "Cannot specify both 'target' and 'targets'. Use 'target' for single rename or 'targets' for batch."
)
```text
**Neither target nor targets specified (lines 220-224):**
```text
ServerError::InvalidRequest(
    "Must specify either 'target' (for single rename) or 'targets' (for batch)."
)
```text
**Unsupported kind (lines 197-201):**
```text
ServerError::InvalidRequest(
    "Unsupported rename kind: {}. Must be one of: symbol, file, directory"
)
```text
**Missing new_name in batch mode (lines 474-479):**
```text
ServerError::InvalidRequest(
    "Target {} (path: {}) missing new_name field (required for batch mode)"
)
```text
**Batch rename conflicts (lines 509-516):**
```text
ServerError::InvalidRequest(
    "Batch rename has naming conflicts: {}"
)
```text
### Return Values

#### Preview Mode (dryRun: true, default)

Returns `RenamePlan` object:
```json
{
  "plan_type": "RenamePlan",
  "edits": {...},
  "summary": {"affected_files": N, "created_files": N, "deleted_files": N},
  "warnings": [],
  "metadata": {...},
  "file_checksums": {...},
  "is_consolidation": false
}
```text
#### Execution Mode (dryRun: false)

Returns `ExecutionResult` object:
```json
{
  "success": true,
  "applied_files": [...],
  "created_files": [...],
  "deleted_files": [...],
  "warnings": [],
  "validation": null,
  "rollback_available": false
}
```text
### Current Documentation

- **Location:** `/workspace/docs/tools/refactoring.md` (lines 38-180)
- **Status:** Excellent, comprehensive documentation
- **Includes:**
  - Purpose statement
  - Parameter table with all fields
  - Both single and batch modes clearly explained
  - Safe preview pattern (dryRun default)
  - Execution pattern
  - Checksum validation details
  - Rust-specific behaviors (crate consolidation, module updates)
  - Scope control options
  - Comprehensive rename coverage explanation

---

## 4. Circular Dependency Analysis (cycle detection)

### Does it exist?

**YES** - Multiple implementations exist:

1. **analyze.dependencies with kind: "circular"** (Primary)
   - Location: `/workspace/crates/mill-handlers/src/handlers/tools/analysis/dependencies.rs` (lines 316-393)
   - Handler: `DependenciesHandler.handle_tool_call()` with kind="circular"

2. **Dedicated circular dependencies analysis**
   - Location: `/workspace/crates/mill-handlers/src/handlers/tools/analysis/circular_dependencies.rs`
   - Status: Separate handler for workspace-wide analysis

### How it works

#### File-Level Detection (MVP)
- Location: Lines 316-393 in `dependencies.rs`
- Detects self-referential imports (module importing itself)
- Limited to obvious cycles within single file

#### Workspace-Level Detection (with feature)
- Feature flag: `analysis-circular-deps`
- Uses: `mill_analysis_circular_deps::find_circular_dependencies()`
- Builds full dependency graph using `DependencyGraphBuilder`
- Detects multi-module cycles with full import chains
- Returns cycles with import chain details
- Lines 1213-1311 in `dependencies.rs`

### Parameters for circular analysis

When calling `analyze.dependencies` with `kind: "circular"`:

| Parameter | Type | Required | Notes |
|-----------|------|----------|-------|
| `kind` | string | Yes | Must be `"circular"` |
| `scope` | object | Yes | Scope parameter (file, directory, workspace) |

### Return Structure

For each detected cycle:
```json
{
  "id": "circular-dependency-{cycle_id}",
  "kind": "circular_dependency",
  "severity": "high",
  "location": {...},
  "metrics": {
    "cycle_length": number,
    "cycle_path": [module_names...],
    "import_chain": [
      {
        "from": string,
        "to": string,
        "symbols": [...]
      }
    ]
  },
  "message": "Circular dependency detected: X modules form a cycle",
  "suggestions": [...]
}
```text
### Actionable Suggestions Generated

The handler generates context-aware suggestions (lines 781-845):

1. **Extract Interface** (for 2-module cycles)
   - Creates shared interface between modules
   - Breaks cycle via abstraction

2. **Dependency Injection**
   - Inverts dependency direction
   - Improves testability

3. **Extract Shared Module** (for multi-module cycles)
   - Creates common module for shared code
   - Converts cycle to tree

4. **Merge Modules** (for small 2-module cycles)
   - Simplifies by removing artificial separation

### Documentation Status

- **analyze.dependencies documentation:** `/workspace/docs/tools/analysis.md` (lines 313-432)
- **Status:** Documents "circular" as supported kind
- **Coverage:** Explains circular dependency detection, mentions feature requirement
- **Gaps:** Doesn't detail the feature flag requirement or workspace vs file scope differences

---

## Analysis Summary

### Parameter Validation Patterns

All three tools follow consistent validation patterns:

1. **Check for required parameter** - Return descriptive error if missing
2. **Validate parameter value** - Check against allowed set
3. **Dispatch based on parameter** - Route to appropriate handler function

### Error Handling

All tools return `ServerError::InvalidRequest()` with clear messages describing:
- What parameter is missing/invalid
- What valid values are expected
- Which mode (single/batch) has what requirements

### Documentation Quality

| Tool | Documentation | Completeness | Status |
|------|---------------|--------------|--------|
| `analyze.structure` | `/workspace/docs/tools/analysis.md` | Excellent | Up-to-date |
| `analyze.dependencies` | `/workspace/docs/tools/analysis.md` | Good | Needs circular deps feature detail |
| `rename` | `/workspace/docs/tools/refactoring.md` | Excellent | Comprehensive |

### Missing Documentation

1. **Circular dependencies feature flag** - Not documented that workspace-level detection requires `analysis-circular-deps` feature
2. **Scope parameter details** - All tools use scope but engine documentation in code (lines 46-91 of engine.rs)
3. **Batch rename deduplication** - Not documented how same-file edits are merged (lines 285-334 in rename mod.rs)

---

## Recommendations

### 1. Add circular dependencies feature documentation
Update `/workspace/docs/tools/analysis.md` to clarify:
- The `analysis-circular-deps` feature requirement
- Difference between file-level and workspace-level detection
- When each mode is used (auto-detection logic)

### 2. Document scope parameter structure
Create separate documentation for the shared `ScopeParam` structure used by all analysis tools in `/workspace/crates/mill-handlers/src/handlers/tools/analysis/engine.rs` (lines 46-91)

### 3. Add batch rename deduplication to docs
Document that multiple targets modifying the same file (e.g., Cargo.toml) have their edits merged via `dedupe_document_changes()` method

### 4. Create parameter validation reference
Document the validation flow and error messages for tool authors adding new tools

---

**Generated:** 2025-10-28
**Scope:** Mill version indicated by git commits
**Tool Coverage:** 3 tools + circular dependency analysis