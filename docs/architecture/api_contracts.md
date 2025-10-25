# Unified Analysis & Refactoring API Contracts

> **Note:** This document provides formal implementation contracts. For user-friendly documentation with examples, see [docs/tools/README.md](../tools/README.md).

## Scope
- Defines canonical request/response contracts for the unified analysis and refactoring APIs.
- Applies to MCP tools exposed by the `mill` server in the unified release.
- Overrides conflicting details in proposal drafts; downstream docs should link here.

## Serialization Conventions
- Transport format: UTF-8 JSON.
- Identifiers: lowercase snake case (`plan_type`, `file_path`).
- Timestamps: ISO-8601 UTC (`YYYY-MM-DDTHH:MM:SSZ`).
- Numbers: use JSON numbers (no quoted numerics); floats allowed only where noted.
- Strings enumerated in this document are case-sensitive.
- Objects are closed by default (unknown properties rejected) **unless** a section explicitly notes `// additional properties allowed`. Extension points must be documented here.

## Shared Types

### `Position`
```json
{ "line": 0, "character": 0 }
```
Zero-based indices aligned with LSP.

### `Range`
```json
{ "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 0 } }
```
End is exclusive. Required for regions.

### `Location`
```json
{ "file_path": "src/lib.rs", "range": { ... } }
```
`file_path` is workspace-relative POSIX path.

### `SymbolIdentifier`
```json
{
  "symbol": "process_order",
  "symbol_kind": "function",
  "language": "rust"
}
```
`symbol_kind` matches LSP `SymbolKind` enums (lowercase strings).

### `Severity`
Enumerated string: `"high" | "medium" | "low"`.

---

## Analysis API Contracts

### Request Envelope
```json
{
  "category": "quality",
  "kind": "complexity",
  "scope": { ... },
  "options": { ... }
}
```
- `category`: `"quality" | "dead_code" | "dependencies" | "structure" | "documentation" | "tests"`.
- `kind`: category-specific enumerations (see below).
- `scope`: filters target files.
- `options`: category-specific parameters; all optional unless marked required.

#### `Scope`
```json
{
  "type": "workspace" | "directory" | "file" | "symbol",
  "path": "src/",
  "symbol_name": "MyStruct",
  "include": ["*.rs"],
  "exclude": ["tests/"]
}
```
- `type` required.
- `path` required for `directory` and `file`.
- `symbol_name` required when `type="symbol"`.
- `include`/`exclude` arrays use glob syntax; treat empty arrays as unset.

#### Pagination
Available for all categories. Clients may omit `options` entirely or send `{}` to rely on defaults.
```json
{
  "limit": 1000,
  "offset": 0
}
```
- `limit`: integer 1–5000 (default 1000).
- `offset`: integer ≥0 (default 0).
- Pagination parameters live inside `options`.

### Response Envelope `AnalysisResult`
```json
{
  "findings": [ { ... } ],
  "summary": {
    "total_findings": 0,
    "returned_findings": 0,
    "has_more": false,
    "by_severity": { "high": 0, "medium": 0, "low": 0 },
    "files_analyzed": 0,
    "symbols_analyzed": 0,
    "analysis_time_ms": 0
  },
  "metadata": {
    "category": "quality",
    "kind": "complexity",
    "scope": { ... },
    "language": "rust",
    "timestamp": "2025-10-10T12:00:00Z",
    "thresholds": { "cyclomatic_complexity": 15 }
  }
}
```
- `findings`: array length ≤ `limit`.
- `summary.has_more` true when additional findings exist beyond `offset + return_count`.
- `summary.by_severity` must include keys for all Severity values (0 allowed); omission implies 0.
- `metadata.language` optional; omit when heterogeneous.
- `metadata.thresholds` echoes applied thresholds; empty object allowed.

#### `Finding`
```json
{
  "id": "complexity-1",
  "kind": "complexity_hotspot",
  "severity": "high",
  "location": { ... },
  "symbol": { ... },
  "metrics": { "cyclomatic_complexity": 25 },
  "message": "Function has high cyclomatic complexity (25)",
  "suggestions": [ { ... } ]
}
```
- `id`: stable identifier within result set.
- `kind`: category-specific enumeration (snake case).
- `symbol`: optional `SymbolIdentifier`.
- `metrics`: numeric or string values; explicit keys per category documented below. Keys may be omitted when not applicable. Additional keys are forbidden unless the category explicitly allows extensions.
- `suggestions`: optional array (see Refactoring integration).

#### `Suggestion`
```json
{
  "action": "extract_function",
  "description": "Extract nested conditional block",
  "target": { "range": { ... } },
  "estimated_impact": "reduces complexity by ~8 points",
  "refactor_call": {
    "command": "extract.plan",
    "arguments": { ... }
  }
}
```
- `refactor_call` is required when actionable.
- `command` references unified refactor API commands.
- `arguments` must conform to target plan schema.

### Category Enumerations & Metrics

#### Quality (`analyze.quality`)
`kind`: `"complexity" | "smells" | "maintainability" | "readability"`.
- `metrics` keys:
  - `cyclomatic_complexity` (int ≥0)
  - `cognitive_complexity` (int ≥0)
  - `maintainability_index` (float 0–100)
  - `nesting_depth` (int ≥0)
  - `parameter_count` (int ≥0)
  - `function_length` (int ≥0)
- `options.thresholds` may include matching keys; omit others.

#### Dead Code (`analyze.dead_code`)
`kind`: `"unused_symbols" | "unused_imports" | "unreachable_code" | "unused_parameters" | "unused_types" | "unused_variables"`.
- `metrics` optional; use:
  - `occurrences` (int ≥1)
  - `last_reference_ts` (ISO timestamp) when available.
- `options.aggressive`: boolean (default false).
- `options.include_tests`: boolean (default false).
- `options.include_private`: boolean (default true).

#### Dependencies (`analyze.dependencies`)
`kind`: `"imports" | "graph" | "circular" | "coupling" | "cohesion" | "depth"`.
- `metrics` keys:
  - `in_degree`, `out_degree` (ints)
  - `circular_path` (string array) for circular
  - `coupling_score` / `cohesion_score` (floats 0–1)
  - `dependency_depth` (int ≥0)
- `options.format`: `"detailed" | "summary" | "graph"`.
  - When `"graph"`, response includes `graph` under `metadata`:
    ```json
    {
      "graph": {
        "format": "mermaid",
        "payload": "graph TD...",
        "encoding": "utf8",
        "size_bytes": 1280
      }
    }
    ```
  - `payload` must be UTF-8 text when `encoding="utf8"`. Binary exports must set `encoding="base64"` and provide the original byte size in `size_bytes`.
  - Servers must cap `size_bytes` at 5_242_880 (5 MiB); exceedances should return `PAYLOAD_TOO_LARGE`.
- `options.export_format`: `"json" | "graphviz" | "mermaid"` (default `"json"`).

#### Structure (`analyze.structure`)
`kind`: `"symbols" | "hierarchy" | "interfaces" | "inheritance" | "modules"`.
- `metrics` keys vary:
  - `inheritance_depth`, `implementation_count`, `child_count`.
- `options.symbol_kinds`: array of lowercase LSP symbol kinds.
- `options.include_private`: boolean (default false).

#### Documentation (`analyze.documentation`)
`kind`: `"coverage" | "quality" | "missing" | "outdated" | "todos"`.
- `metrics` keys:
  - `coverage_ratio` (float 0–1)
  - `todo_count` (int ≥0)
  - `staleness_days` (int ≥0)
- `options.visibility`: `"public" | "all"` (default `"public"`).
- `options.require_examples`: boolean (default false).

#### Tests (`analyze.tests`)
`kind`: `"coverage" | "untested" | "quality" | "smells"`.
- `metrics` keys:
  - `coverage_ratio` (float 0–1)
  - `missing_tests` (int ≥0)
  - `assertion_count` (int ≥0)
  - `smell_labels` (string array).
- Coverage ingestion requires `options.coverage_format` (`"lcov" | "cobertura" | "jacoco"`) and `options.coverage_file`.

### Error Contract
```json
{
  "error": {
    "code": "INVALID_SCOPE",
    "message": "Scope path must be provided for type 'directory'",
    "details": { "field": "scope.path" }
  }
}
```
- `code`: uppercase snake case stable identifiers.
- `message`: human-readable English sentence.
- `details`: optional object; may include `field`, `expected`, `actual`.
- Use HTTP-style semantics over MCP transport (e.g., 400 for validation, 500 for internal), documented in tool metadata.

---

## Refactoring API Contracts

### Plan Request Envelope
```json
{
  "operation": "rename",
  "kind": "symbol",
  "arguments": { ... },
  "options": { ... }
}
```
- `operation`: `"rename" | "extract" | "inline" | "move" | "reorder" | "transform" | "delete"`.
- `kind`: operation-specific enumeration.
- Command name mapping: `rename.plan`, `extract.plan`, etc.

### Plan Response (`PlanBase`)
```json
{
  "plan_type": "RenamePlan",
  "plan_version": "1.0",
  "edits": [ /* WorkspaceEdit */ ],
  "summary": {
    "affected_files": 0,
    "created_files": 0,
    "deleted_files": 0
  },
  "warnings": [ { "code": "AMBIGUOUS_TARGET", "message": "..." } ],
  "metadata": {
    "kind": "rename.symbol",
    "language": "rust",
    "estimated_impact": "low",
    "created_at": "2025-10-10T12:00:00Z"
  },
  "file_checksums": {
    "src/lib.rs": "sha256:abc123"
  }
}
```
- `plan_type`: `"RenamePlan" | "ExtractPlan" | "InlinePlan" | "MovePlan" | "ReorderPlan" | "TransformPlan" | "DeletePlan"`.
- `plan_version`: string, default `"1.0"`. Increment when breaking plan schema.
- `edits`: conforms to LSP `WorkspaceEdit` (no filesystem side effects beyond edits).
- `summary`: counts must match unique file paths in `edits`.
- `warnings`: optional array. `code` enumerations documented below.
- `metadata.estimated_impact`: `"low" | "medium" | "high"` (development impact).
- `metadata` object required; omit optional fields instead of setting `null`.
- `file_checksums`: map of `file_path` → `sha256:<hex>`. Required when `validate_checksums` default is true.
  - Omit entries for files created by the plan; absence signals new files.
  - For deletions, include the last known checksum when available; omission indicates unavailable.

#### Warning Codes
- `AMBIGUOUS_TARGET`: multiple matches for target selector.
- `POTENTIAL_BEHAVIOR_CHANGE`: transformation may alter semantics.
- `PARTIAL_APPLY`: plan omits unsupported files.
- `VALIDATION_SKIPPED`: generated under `force: true`.
Additional codes must be documented before release.

### Operation-Specific Arguments

#### `rename.plan`
- `kind`: `"symbol" | "parameter" | "type" | "file" | "directory"`.
- `arguments.target`:
  ```json
  {
    "path": "src/lib.rs",
    "selector": {
      "position": { "line": 0, "character": 0 },
      "name": "old_name"
    }
  }
  ```
- `arguments.new_name`: non-empty string; validated against language rules when possible.
- `options`:
  - `strict` (bool, default false)
  - `update_imports` (bool, default true)
  - `validate_scope` (bool, default true)
  - `workspace_limits` (string array of allowed path prefixes)

#### `extract.plan`
- `kind`: `"function" | "variable" | "module" | "interface" | "class" | "constant" | "type_alias"`.
- `arguments.source` requires `file_path` and `range`.
- Optional `destination` path or module.
- `options.visibility`: `"public" | "private"` (default `"private"`).
- `options.destination_path`: file path for extracted artifact.

#### `inline.plan`
- `kind`: `"variable" | "function" | "constant" | "type_alias"`.
- `arguments.target`: `{ "file_path": "...", "position": { ... } }`.
- `options.inline_all`: boolean (default false).

#### `move.plan`
- `kind`: `"symbol" | "to_module" | "to_namespace" | "consolidate"`.
- For symbol moves:
  ```json
  {
    "source": { "file_path": "...", "position": { ... } },
    "destination": { "file_path": "...", "module_path": "crate::foo" }
  }
  ```
- For consolidation:
  ```json
  {
    "source": { "directory": "crates/old" },
    "destination": { "directory": "crates/new/src/module" }
  }
  ```
- `options.merge_dependencies`: boolean (default true) for consolidation.

#### `reorder.plan`
- `kind`: `"parameters" | "imports" | "members" | "statements"`.
- `arguments.target`: `file_path` plus either `position` or `range`.
- `arguments.new_order`: array of zero-based indices; required when no `strategy`.
- `options.strategy`: `"alphabetical" | "visibility" | "dependency"`; mutually exclusive with `new_order`.

#### `transform.plan`
- `kind`: `"to_arrow_function" | "to_async" | "loop_to_iterator" | "callback_to_promise" | "add_null_check" | "remove_dead_branch"`.
- `arguments.target`: `file_path` and `position` or `range`.
- `options.language_specific`: object (`// additional properties allowed`); document accepted keys per language module.

#### `delete.plan`
- `kind`: `"unused_imports" | "dead_code" | "redundant_code" | "file"`.
- `arguments.target`:
  - For file-based deletions: `file_path`.
  - For scoped deletions: `scope` (`"workspace" | "file" | "directory"`) and optional `path`.
  - Optional `range` for precise deletions.
- `options.aggressive`: boolean (default false).

### Unified Refactoring API (with dryRun option)

All refactoring tools (`rename`, `extract`, `inline`, `move`, `reorder`, `transform`, `delete`) support execution via the `options` parameter:

```json
{
  "target": { ... },
  "newName": "...",  // or other operation-specific params
  "options": {
    "dryRun": false,
    "validateChecksums": true,
    "force": false
  }
}
```

**Options:**
- `dryRun`: when `true` (default), preview changes without modifying files; when `false`, execute changes
- `validateChecksums`: compares file hashes before applying; failure returns error code `STALE_PLAN`
- `force`: bypasses validations (sets `warnings` entry `VALIDATION_SKIPPED`)

**Execution:**
- Default behavior: `dryRun: true` returns preview plan
- Explicit execution: `dryRun: false` applies changes atomically with rollback on error

#### Apply Response
```json
{
  "success": true,
  "applied_files": ["src/app.rs"],
  "created_files": [],
  "deleted_files": [],
  "warnings": [],
  "rollback_available": true,
  "snapshot_id": "rollback-123"
}
```
- `success`: boolean.
- `applied_files`: list of file paths touched (empty on dry run).
- `created_files`/`deleted_files`: unique sets.
- `warnings`: any warnings emitted during apply.
- `rollback_available`: indicates snapshot stored for undo.
- `snapshot_id`: optional identifier clients must persist to request rollback once that command is available. Omitted when no snapshot created. Rollback tooling is deferred but must accept this identifier when delivered.

#### Apply Error Contract
```json
{
  "error": {
    "code": "STALE_PLAN",
    "message": "Plan checksums no longer match workspace state",
    "details": { "file_path": "src/app.rs" }
  }
}
```
- Additional apply errors:
  - `INVALID_PLAN_TYPE`: plan_type unrecognized.
  - `CHECKSUM_MISMATCH`: specific file mismatch.
  - `APPLY_FAILED`: underlying edit apply failed (include `details.reason`).
  - `ROLLBACK_FAILED`: rollback attempt unsuccessful (log actionable info).

---

## Contract Validation & Tooling
- Maintain JSON Schema files (`schemas/unified_analysis.schema.json`, `schemas/unified_refactor.schema.json`) mirroring this doc for automated validation (future work).
- CI must:
  - Validate sample payloads against schemas.
  - Ensure `suggestions[].refactor_call` references valid `*.plan` operations.
  - Confirm all plan implementations set `plan_type`, `plan_version`, and `file_checksums`.
- Documentation updates must reference this file; proposals should defer to it for authoritative contracts.
- Follow-up actions:
  - Create tracking issues for JSON Schema publication and rollback command implementation referencing this contract.

---

## Change Management
- Increment `plan_version` when making breaking changes to plan payloads.
- Add new enum values by appending to lists above and updating schemas; never recycle identifiers.
- Record contract updates in `CHANGELOG.md` under “Contracts” section.
- Consumers must treat unknown fields as errors unless explicitly allowed.
