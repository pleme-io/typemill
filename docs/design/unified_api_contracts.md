# Unified API Contracts

**Status**: Specification
**Version**: 1.0
**Date**: 2025-10-10

Formal contracts for Unified Refactoring and Analysis APIs.

---

## Common Types

### LSP-Compatible Types (Reused)

We reuse LSP types where possible to avoid parallel type systems:

```typescript
Position = { line: number, character: number }  // 0-indexed
Range = { start: Position, end: Position }
Location = { uri: string, range: Range }
TextEdit = { range: Range, newText: string }
WorkspaceEdit = { changes: { [uri: string]: TextEdit[] } }  // LSP standard
```

### Extensions (Our Types)

```typescript
FileChecksum = string  // Format: "sha256:{64-hex-chars}"
Timestamp = string     // ISO-8601: "2025-10-10T12:34:56Z"
Severity = "high" | "medium" | "low"
Language = "typescript" | "rust"
```

---

## Refactoring API Contracts

### Plan Structure (All Plan Types)

All `*.plan` commands return a discriminated union:

```typescript
interface BasePlan {
  plan_type: "RenamePlan" | "ExtractPlan" | "InlinePlan" | "MovePlan" |
             "ReorderPlan" | "TransformPlan" | "DeletePlan";
  plan_version: "1.0";
  edits: WorkspaceEdit;  // LSP standard
  summary: {
    affected_files: number;
    created_files: number;
    deleted_files: number;
  };
  warnings: Warning[];
  metadata: {
    kind: string;  // e.g., "symbol", "function", "unused_imports"
    language: Language;
    estimated_impact: "low" | "medium" | "high";
    created_at: Timestamp;
  };
  file_checksums: { [file_path: string]: FileChecksum };
}

interface Warning {
  code: string;          // e.g., "AMBIGUOUS_TARGET"
  message: string;
  severity: Severity;
  candidates?: any[];    // Optional additional context
}
```

### Example: rename.plan Response

```json
{
  "plan_type": "RenamePlan",
  "plan_version": "1.0",
  "edits": {
    "changes": {
      "file:///src/lib.rs": [
        { "range": {...}, "newText": "new_name" }
      ]
    }
  },
  "summary": {
    "affected_files": 3,
    "created_files": 0,
    "deleted_files": 0
  },
  "warnings": [],
  "metadata": {
    "kind": "symbol",
    "language": "rust",
    "estimated_impact": "low",
    "created_at": "2025-10-10T12:34:56Z"
  },
  "file_checksums": {
    "/src/lib.rs": "sha256:abc123...",
    "/src/app.rs": "sha256:def456..."
  }
}
```

### workspace.apply_edit Request

```typescript
interface ApplyEditRequest {
  plan: BasePlan;  // Any plan type
  options?: {
    dry_run?: boolean;              // default: false
    validate_checksums?: boolean;   // default: true
    validate_plan_type?: boolean;   // default: true
    force?: boolean;                // default: false (skip validation)
    rollback_on_error?: boolean;    // default: true
  };
}
```

### workspace.apply_edit Response

```typescript
interface ApplyEditResult {
  success: boolean;
  applied_files: string[];
  created_files: string[];
  deleted_files: string[];
  warnings: Warning[];
  rollback_available: boolean;
  error?: ErrorPayload;  // if success=false
}
```

---

## Analysis API Contracts

### AnalysisResult Structure (All analyze.* Commands)

```typescript
interface AnalysisResult {
  findings: Finding[];
  summary: {
    total_findings: number;       // Total available
    returned_findings: number;    // In this response (≤ limit)
    has_more: boolean;            // More results available via pagination
    by_severity: {
      high: number;
      medium: number;
      low: number;
    };
    files_analyzed: number;
    symbols_analyzed?: number;    // Optional
    analysis_time_ms: number;
  };
  metadata: {
    category: "quality" | "dead_code" | "dependencies" | "structure" |
              "documentation" | "tests";
    kind: string;  // e.g., "complexity", "unused_symbols", "circular"
    scope: {
      type: "workspace" | "directory" | "file" | "symbol";
      path?: string;
    };
    language: Language;
    timestamp: Timestamp;
    thresholds?: { [key: string]: number };  // e.g., {"complexity": 15}
  };
}

interface Finding {
  id: string;           // Unique within result
  kind: string;         // e.g., "complexity_hotspot", "unused_parameter"
  severity: Severity;
  location: Location;   // LSP type
  symbol?: string;      // Optional symbol name
  symbol_kind?: string; // Optional: "function", "class", etc.
  metrics?: { [key: string]: number };  // e.g., {"cyclomatic_complexity": 25}
  message: string;
  suggestions: Suggestion[];
}

interface Suggestion {
  action: string;  // e.g., "extract_function", "inline_variable"
  description: string;
  priority?: number;  // Higher = more important
  estimated_impact?: string;  // e.g., "reduces complexity by 8 points"
  refactor_call: {
    command: string;  // e.g., "extract.plan"
    arguments: any;   // Arguments for the refactor command
  };
}
```

### Pagination

```typescript
interface PaginationOptions {
  limit?: number;   // default: 1000, max results to return
  offset?: number;  // default: 0, starting index
}

// Response includes in summary:
{
  total_findings: 5432,      // Total available
  returned_findings: 1000,   // This response (min(limit, total - offset))
  has_more: true            // offset + returned < total
}

// Next page request:
{ limit: 1000, offset: 1000 }
```

### Example: analyze.quality Response

```json
{
  "findings": [
    {
      "id": "complexity-1",
      "kind": "complexity_hotspot",
      "severity": "high",
      "location": {
        "uri": "file:///src/app.rs",
        "range": {
          "start": { "line": 10, "character": 0 },
          "end": { "line": 45, "character": 1 }
        }
      },
      "symbol": "process_order",
      "symbol_kind": "function",
      "metrics": {
        "cyclomatic_complexity": 25,
        "cognitive_complexity": 18
      },
      "message": "Function has high cyclomatic complexity (25)",
      "suggestions": [
        {
          "action": "extract_function",
          "description": "Extract nested block to reduce complexity",
          "estimated_impact": "reduces complexity from 25 to 17",
          "refactor_call": {
            "command": "extract.plan",
            "arguments": {
              "kind": "function",
              "source": {
                "file_path": "/src/app.rs",
                "range": { "start": { "line": 15, "character": 4 }, "end": { "line": 23, "character": 5 } },
                "name": "validate_order"
              }
            }
          }
        }
      ]
    }
  ],
  "summary": {
    "total_findings": 12,
    "returned_findings": 12,
    "has_more": false,
    "by_severity": { "high": 3, "medium": 5, "low": 4 },
    "files_analyzed": 45,
    "symbols_analyzed": 234,
    "analysis_time_ms": 234
  },
  "metadata": {
    "category": "quality",
    "kind": "complexity",
    "scope": { "type": "workspace", "path": "/src" },
    "language": "rust",
    "timestamp": "2025-10-10T12:34:56Z",
    "thresholds": { "complexity": 15 }
  }
}
```

### analyze.batch Response

```typescript
interface BatchAnalysisResult {
  results: {
    command: string;  // e.g., "analyze.quality"
    result: AnalysisResult;  // Standard AnalysisResult
  }[];
  summary: {
    total_findings: number;        // Sum across all analyses
    total_files_analyzed: number;
    analysis_time_ms: number;
  };
  optimization: {
    shared_parsing: boolean;
    cache_hits: number;
    sequential_execution: boolean;
  };
}
```

---

## Error Handling

### Error Payload Structure

```typescript
interface ErrorPayload {
  code: string;          // Machine-readable error code
  message: string;       // Human-readable message
  severity: "error" | "warning";
  details?: any;         // Optional context (file path, position, etc.)
  retryable: boolean;    // Whether operation can be retried
}
```

### Common Error Codes

**Refactoring Errors**:
- `STALE_PLAN`: File checksums don't match (file modified since plan created)
- `INVALID_PLAN_TYPE`: Plan type doesn't match expected schema
- `INVALID_POSITION`: Position out of file range
- `SYMBOL_NOT_FOUND`: No symbol at specified position
- `AMBIGUOUS_TARGET`: Multiple symbols match (use strict mode)
- `FILE_NOT_FOUND`: Source file doesn't exist
- `INVALID_KIND`: Unknown kind value for operation

**Analysis Errors**:
- `INVALID_KIND`: Unknown kind value for category
- `INVALID_SCOPE`: Scope type invalid for operation
- `FILE_NOT_FOUND`: Target file/directory doesn't exist
- `LSP_ERROR`: Language server error (details in error.details)
- `PARSE_ERROR`: File couldn't be parsed (syntax error)

**Retryability**:
- `STALE_PLAN`: Retryable (re-run *.plan to get fresh checksums)
- `LSP_ERROR`: Retryable (transient language server issue)
- `INVALID_KIND`, `INVALID_POSITION`: Not retryable (fix request)
- `FILE_NOT_FOUND`: Not retryable (create file first)

---

## Serialization Rules

### Data Types

**Timestamps**: ISO-8601 strings in UTC
```typescript
"2025-10-10T12:34:56Z"  // Always UTC, always 'Z' suffix
```

**Numbers**: Always numeric, never strings
```typescript
{ "complexity": 25 }      // ✅ Correct
{ "complexity": "25" }    // ❌ Wrong
```

**File Checksums**: SHA-256 with prefix
```typescript
"sha256:abc123def456..."  // Lowercase hex, 64 chars
```

**File Paths**: Absolute paths by default
```typescript
"/src/lib.rs"                    // Absolute (preferred)
"file:///src/lib.rs"            // URI format (LSP compatibility)
```

### Large Payloads

**WorkspaceEdit**: No chunking needed (LSP handles efficiently)

**Batch Results**: No pagination (client controls batch size)

**Analysis Results**: Use `limit`/`offset` for pagination

---

## Validation

### Client-Side Validation

1. **Before calling *.plan**: Validate file exists, position in range
2. **Before calling workspace.apply_edit**: Check `plan.file_checksums` still valid
3. **After receiving results**: Validate against expected structure

### Server-Side Validation

1. **All requests**: Validate required fields present
2. **Plans**: Validate `plan_type` matches schema
3. **Apply**: Validate checksums, plan version
4. **Analysis**: Validate `kind` is supported, scope is valid

### Contract Tests (CI)

Located in `tests/contracts/`:
- Validate response structure matches contract
- Golden file tests for common scenarios
- Error code coverage (all codes can be triggered)

---

## Kind Values Reference

### Refactoring (by operation)

- **rename.plan**: symbol, parameter, type, file, directory
- **extract.plan**: function, variable, module, interface, class, constant, type_alias
- **inline.plan**: variable, function, constant, type_alias
- **move.plan**: symbol, to_module, to_namespace, consolidate
- **reorder.plan**: parameters, imports, members, statements
- **transform.plan**: to_arrow_function, to_async, loop_to_iterator, callback_to_promise, add_null_check, remove_dead_branch
- **delete.plan**: unused_imports, dead_code, redundant_code, file

### Analysis (by category)

- **analyze.quality**: complexity, smells, maintainability, readability
- **analyze.dead_code**: unused_symbols, unused_imports, unreachable_code, unused_parameters, unused_types, unused_variables
- **analyze.dependencies**: imports, graph, circular, coupling, cohesion, depth
- **analyze.structure**: symbols, hierarchy, interfaces, inheritance, modules
- **analyze.documentation**: coverage, quality, missing, outdated, todos
- **analyze.tests**: coverage, untested, quality, smells

---

## Breaking Changes Policy

**Version**: All APIs versioned via `plan_version` and response metadata

**Backward compatibility**:
- New `kind` values: Non-breaking (ignored by old clients)
- New optional fields: Non-breaking
- Required field changes: Breaking (bump `plan_version`)
- Error code changes: Non-breaking (clients should handle unknown codes)

**Deprecation**: Beta product, no formal deprecation period. Breaking changes announced in changelog.
