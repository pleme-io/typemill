# Proposal: Unified Analysis API

**Status**: Draft
**Author**: Project Team
**Date**: 2025-10-10

---

## Executive Summary

Consolidate 35+ analysis commands into **6 unified commands** using a consistent **analyze → results** pattern. This reduces API surface by 80%+ while providing actionable insights that bridge directly into refactoring workflows.

**Context**: This is a beta product with no external users. We can make breaking changes immediately without migration paths or legacy support.

---

## Problem

Current API has fragmentation:
- **35+ separate analysis commands** with overlapping functionality
- **Inconsistent result formats** (some return arrays, some objects, some metrics)
- **No unified filtering or thresholds**
- **Difficult to compose** multi-faceted analysis
- **No actionable suggestions** linking analysis to refactoring
- **High cognitive load** for users and AI agents

---

## Solution

### Core Pattern: Analyze → Results

Every analysis operation follows a single-step pattern:

```javascript
analyze.<category>(kind, scope, options) → AnalysisResult
```

**Key principle**: Analysis is read-only, so no "apply" step is needed. All commands return a consistent result structure with findings, metrics, and actionable suggestions.

### Unified Result Structure

All analysis commands return the same shape:

```json
{
  "findings": [
    {
      "id": "complexity-1",
      "kind": "complexity_hotspot",
      "severity": "high" | "medium" | "low",
      "location": {
        "file_path": "src/app.rs",
        "range": { "start": { "line": 10, "character": 0 }, "end": { "line": 45, "character": 1 } },
        "symbol": "process_order",
        "symbol_kind": "function"
      },
      "metrics": {
        "cyclomatic_complexity": 25,
        "cognitive_complexity": 18,
        "nesting_depth": 5,
        "parameter_count": 8,
        "line_count": 35
      },
      "message": "Function 'process_order' has high cyclomatic complexity (25)",
      "suggestions": [
        {
          "action": "extract_function",
          "description": "Extract nested conditional block to separate function",
          "target": { "range": { "start": {...}, "end": {...} } },
          "estimated_impact": "reduces complexity by ~8 points"
        }
      ]
    }
  ],
  "summary": {
    "total_findings": 5432,      // total available
    "returned_findings": 1000,   // in this response (respects limit)
    "has_more": true,            // more results available via pagination
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
    "timestamp": "2025-10-10T12:00:00Z",
    "thresholds": { "complexity": 15, "nesting_depth": 4 }
  }
}
```

---

## New API Surface

### 1. Quality Analysis

**Command**: `analyze.quality(kind, scope, options)` → `QualityReport`

**Supported `kind` Values** (LOCKED):
- `"complexity"` - Cyclomatic and cognitive complexity analysis
- `"smells"` - Code smell detection (long methods, god classes, magic numbers, etc.)
- `"maintainability"` - Overall maintainability metrics
- `"readability"` - Readability issues (nesting depth, parameter count, function length)

**Arguments**:
```json
{
  "kind": "complexity" | "smells" | "maintainability" | "readability",
  "scope": {
    "type": "workspace" | "directory" | "file" | "symbol",
    "path": "src/",
    "include": ["*.rs"],
    "exclude": ["tests/", "examples/"]
  },
  "options": {
    "thresholds": {
      "cyclomatic_complexity": 15,
      "cognitive_complexity": 10,
      "nesting_depth": 4,
      "parameter_count": 5,
      "function_length": 50
    },
    "severity_filter": null,  // null = all, or "high" | "medium" | "low"
    "limit": 1000,            // default: 1000 findings max
    "offset": 0,              // for pagination
    "format": "detailed",     // "detailed" | "summary"
    "include_suggestions": true
  }
}
```

**Examples**:
```javascript
// Find complexity hotspots across workspace
analyze.quality("complexity", { type: "workspace" }, {
  thresholds: { cyclomatic_complexity: 20 },
  severity_filter: "high"
})

// Detect code smells in specific directory
analyze.quality("smells", { type: "directory", path: "src/handlers" }, {
  include_suggestions: true
})

// Check readability of single file
analyze.quality("readability", { type: "file", path: "src/app.rs" })
```

**Replaces**:
- `analyze_complexity` ✅ (already implemented)
- `analyze_project_complexity` ✅ (already implemented)
- `find_complexity_hotspots` ✅ (already implemented)
- `suggest_refactoring` ✅ (already implemented)
- `find_magic_numbers` ⚠️ (via kind="smells" or kind="readability", not yet implemented)
- `find_long_methods` ⚠️ (via kind="smells" or kind="readability", not yet implemented)
- `find_god_classes` ⚠️ (via kind="smells", not yet implemented)
- `analyze_nesting_depth` ⚠️ (via kind="readability", not yet implemented)
- `analyze_parameter_count` ⚠️ (via kind="readability", not yet implemented)
- `analyze_function_length` ⚠️ (via kind="readability", not yet implemented)

---

### 2. Dead Code Analysis

**Command**: `analyze.dead_code(kind, scope, options)` → `DeadCodeReport`

**Supported `kind` Values** (LOCKED):
- `"unused_symbols"` - Functions, classes, variables never referenced
- `"unused_imports"` - Import statements not used
- `"unreachable_code"` - Code after return/throw/break
- `"unused_parameters"` - Function parameters never accessed
- `"unused_types"` - Type definitions never referenced
- `"unused_variables"` - Local variables never read

**Arguments**:
```json
{
  "kind": "unused_symbols" | "unused_imports" | "unreachable_code" | "unused_parameters" | "unused_types" | "unused_variables",
  "scope": {
    "type": "workspace" | "directory" | "file",
    "path": "src/",
    "include": ["*.rs"],
    "exclude": ["tests/"]
  },
  "options": {
    "aggressive": false,
    "include_tests": false,
    "include_private": true,
    "severity_filter": "high" | "medium" | "low",
    "limit": 100,
    "format": "detailed" | "summary",
    "include_suggestions": true
  }
}
```

**Examples**:
```javascript
// Find all unused symbols workspace-wide
analyze.dead_code("unused_symbols", { type: "workspace" }, {
  include_private: true,
  include_suggestions: true  // suggests delete.plan calls
})

// Find unused imports in specific file
analyze.dead_code("unused_imports", { type: "file", path: "src/lib.rs" })

// Detect unreachable code with aggressive mode
analyze.dead_code("unreachable_code", { type: "workspace" }, { aggressive: true })
```

**Replaces**:
- `find_dead_code` ✅ (already implemented)
- `find_unused_imports` ✅ (already implemented)
- `find_unused_parameters` ⚠️ (via kind="unused_parameters", not yet implemented)
- `find_unreachable_code` ⚠️ (via kind="unreachable_code", not yet implemented)
- `find_unused_variables` ⚠️ (via kind="unused_variables", not yet implemented)
- `find_unused_types` ⚠️ (via kind="unused_types", not yet implemented)

---

### 3. Dependency Analysis

**Command**: `analyze.dependencies(kind, scope, options)` → `DependencyReport`

**Supported `kind` Values** (LOCKED):
- `"imports"` - Import structure and categorization
- `"graph"` - Full dependency graph (file/module level)
- `"circular"` - Circular dependency detection
- `"coupling"` - Module coupling strength analysis
- `"cohesion"` - Module cohesion metrics
- `"depth"` - Transitive dependency depth

**Arguments**:
```json
{
  "kind": "imports" | "graph" | "circular" | "coupling" | "cohesion" | "depth",
  "scope": {
    "type": "workspace" | "directory" | "file",
    "path": "src/",
    "include": ["*.rs"],
    "exclude": ["tests/"]
  },
  "options": {
    "max_depth": 5,
    "show_external": false,
    "group_by": "module" | "file" | "package",
    "format": "detailed" | "summary" | "graph",
    "export_format": "json" | "graphviz" | "mermaid",
    "thresholds": {
      "coupling": 0.7,
      "cohesion": 0.3
    },
    "include_suggestions": true
  }
}
```

**Examples**:
```javascript
// Detect circular dependencies
analyze.dependencies("circular", { type: "workspace" }, {
  include_suggestions: true  // suggests how to break cycles
})

// Analyze coupling between modules
analyze.dependencies("coupling", { type: "directory", path: "src/handlers" }, {
  thresholds: { coupling: 0.5 }
})

// Generate dependency graph
analyze.dependencies("graph", { type: "workspace" }, {
  format: "graph",
  export_format: "mermaid"
})
```

**Replaces**:
- `analyze_imports` ✅ (implemented)
- `analyze_dependencies` ⚠️ (via kind="graph", not yet implemented)
- `find_circular_dependencies` ⚠️ (via kind="circular", not yet implemented)
- `find_coupling` ⚠️ (via kind="coupling", not yet implemented)
- `find_cohesion` ⚠️ (via kind="cohesion", not yet implemented)
- `analyze_dependency_depth` ⚠️ (via kind="depth", not yet implemented)

---

### 4. Structure Analysis

**Command**: `analyze.structure(kind, scope, options)` → `StructureReport`

**Supported `kind` Values** (LOCKED):
- `"symbols"` - Hierarchical symbol tree (LSP-based)
- `"hierarchy"` - Class/type hierarchy analysis
- `"interfaces"` - Interface usage and adoption patterns
- `"inheritance"` - Inheritance depth and breadth
- `"modules"` - Module organization and structure

**Arguments**:
```json
{
  "kind": "symbols" | "hierarchy" | "interfaces" | "inheritance" | "modules",
  "scope": {
    "type": "workspace" | "directory" | "file" | "symbol",
    "path": "src/",
    "symbol_name": "MyClass",
    "include": ["*.rs"],
    "exclude": ["tests/"]
  },
  "options": {
    "depth": 5,
    "include_private": false,
    "symbol_kinds": ["function", "class", "interface"],
    "format": "detailed" | "summary" | "tree",
    "include_metrics": true
  }
}
```

**Examples**:
```javascript
// Get all symbols in workspace
analyze.structure("symbols", { type: "workspace" }, {
  symbol_kinds: ["function", "class"],
  format: "tree"
})

// Analyze class hierarchy
analyze.structure("hierarchy", { type: "file", path: "src/models.rs" }, {
  depth: 3,
  include_metrics: true
})

// Find interface implementations
analyze.structure("interfaces", { type: "workspace" }, {
  format: "detailed"
})
```

**Replaces**:
- `get_document_symbols` ✅
- `analyze_inheritance` ⚠️ (via kind="hierarchy")
- `analyze_interface_usage` ⚠️ (via kind="interfaces")

**Does NOT replace**:
- `search_workspace_symbols` - **Kept as navigation command** (see Navigation Commands section)
- `find_definition` - Point-query, not bulk analysis
- `find_references` - Point-query, not bulk analysis
- `find_implementations` - Point-query, not bulk analysis

**Note**: Navigation commands are fundamentally different from bulk analysis. They accept specific queries/positions and return targeted results, not workspace-wide findings.

---

### 5. Documentation Analysis

**Command**: `analyze.documentation(kind, scope, options)` → `DocumentationReport`

**Supported `kind` Values** (LOCKED):
- `"coverage"` - Documentation coverage metrics
- `"quality"` - Documentation quality assessment
- `"missing"` - Undocumented public APIs
- `"outdated"` - Comments contradicting code
- `"todos"` - TODO/FIXME/HACK markers

**Arguments**:
```json
{
  "kind": "coverage" | "quality" | "missing" | "outdated" | "todos",
  "scope": {
    "type": "workspace" | "directory" | "file",
    "path": "src/",
    "include": ["*.rs"],
    "exclude": ["tests/", "examples/"]
  },
  "options": {
    "visibility": "public" | "all",
    "require_examples": false,
    "min_comment_ratio": 0.2,
    "format": "detailed" | "summary",
    "include_suggestions": true
  }
}
```

**Examples**:
```javascript
// Find undocumented public APIs
analyze.documentation("missing", { type: "workspace" }, {
  visibility: "public",
  include_suggestions: true
})

// Calculate documentation coverage
analyze.documentation("coverage", { type: "directory", path: "src/handlers" })

// Extract all TODO comments
analyze.documentation("todos", { type: "workspace" }, {
  format: "detailed"
})
```

**Replaces**:
- `analyze_comment_ratio` ⚠️ (via kind="coverage", embedded in current system)
- `find_undocumented_exports` ⚠️ (via kind="missing", not yet implemented)
- `find_outdated_comments` ⚠️ (via kind="outdated", not yet implemented)
- `find_todo_comments` ⚠️ (via kind="todos", not yet implemented)

---

### 6. Test Analysis

**Command**: `analyze.tests(kind, scope, options)` → `TestReport`

**Supported `kind` Values** (LOCKED):
- `"coverage"` - Test coverage percentage per file/function
- `"untested"` - Functions/modules without tests
- `"quality"` - Test quality metrics (assertions, mocks, etc.)
- `"smells"` - Test smells (slow tests, fragile tests, etc.)

**Arguments**:
```json
{
  "kind": "coverage" | "untested" | "quality" | "smells",
  "scope": {
    "type": "workspace" | "directory" | "file",
    "path": "src/",
    "include": ["*.rs"],
    "exclude": ["tests/"]
  },
  "options": {
    "coverage_format": "lcov" | "cobertura" | "jacoco",
    "coverage_file": ".coverage/lcov.info",
    "min_coverage": 0.8,
    "include_private": false,
    "format": "detailed" | "summary",
    "include_suggestions": true
  }
}
```

**Examples**:
```javascript
// Parse coverage report and find gaps
analyze.tests("coverage", { type: "workspace" }, {
  coverage_format: "lcov",
  coverage_file: "coverage/lcov.info",
  min_coverage: 0.8
})

// Find untested public functions
analyze.tests("untested", { type: "workspace" }, {
  include_private: false,
  include_suggestions: true  // suggests test templates
})

// Detect test smells
analyze.tests("smells", { type: "directory", path: "tests/" })
```

**Replaces**:
- `analyze_test_coverage` ⚠️ (via kind="coverage", not yet implemented)
- `find_untested_code` ⚠️ (via kind="untested", not yet implemented)
- `analyze_test_quality` ⚠️ (via kind="quality", not yet implemented)
- `find_test_smells` ⚠️ (via kind="smells", not yet implemented)

---

## Navigation Commands (Separate from Analysis)

**The following commands remain as dedicated navigation tools** (not replaced by `analyze.*`):

### `search_workspace_symbols(query, options)` → `SymbolList`

**Why separate**: String-based symbol search is a point-query operation, fundamentally different from bulk workspace analysis.

**Arguments**:
```json
{
  "query": "processOrder",
  "match_mode": "substring" | "fuzzy" | "exact",
  "kind_filter": ["function", "class", "interface"],
  "limit": 100
}
```

**Example**:
```javascript
search_workspace_symbols("process", { match_mode: "fuzzy", kind_filter: ["function"] })
```

### Other Navigation Commands (Unchanged)

- `find_definition(file_path, position)` → Location
- `find_references(file_path, position)` → LocationList
- `find_implementations(file_path, position)` → LocationList

**These are point-queries for IDE workflows, not bulk analysis operations.**

---

## Actionable Suggestions

All analysis results include `suggestions` that link directly to refactoring operations:

```json
{
  "findings": [{
    "kind": "complexity_hotspot",
    "location": { "file_path": "src/app.rs", "range": {...} },
    "suggestions": [
      {
        "action": "extract_function",
        "description": "Extract nested block to reduce complexity",
        "refactor_call": {
          "command": "extract.plan",
          "arguments": {
            "kind": "function",
            "source": {
              "file_path": "src/app.rs",
              "range": { "start": { "line": 15, "character": 4 }, "end": { "line": 23, "character": 5 } },
              "name": "validate_order"
            }
          }
        },
        "estimated_impact": "reduces complexity from 25 to 17"
      },
      {
        "action": "inline_variable",
        "description": "Inline temporary variable 'temp'",
        "refactor_call": {
          "command": "inline.plan",
          "arguments": {
            "kind": "variable",
            "target": { "file_path": "src/app.rs", "position": { "line": 12, "character": 8 } }
          }
        },
        "estimated_impact": "reduces complexity by 1 point"
      }
    ]
  }]
}
```

**Benefits**:
- AI agents can **directly execute** suggested refactorings
- Users get **actionable next steps**, not just metrics
- **Closed-loop workflow**: analyze → suggest → refactor → re-analyze

---

## Batch Analysis

For workflows that need multiple analyses, support batch queries:

```javascript
analyze.batch(queries) → BatchAnalysisResult
```

**Example**:
```javascript
analyze.batch([
  { command: "analyze.quality", kind: "complexity", scope: { type: "workspace" } },
  { command: "analyze.dead_code", kind: "unused_symbols", scope: { type: "workspace" } },
  { command: "analyze.dependencies", kind: "circular", scope: { type: "workspace" } }
])
```

**Result**:
```json
{
  "results": [
    { "command": "analyze.quality", "result": { /* QualityReport */ } },
    { "command": "analyze.dead_code", "result": { /* DeadCodeReport */ } },
    { "command": "analyze.dependencies", "result": { /* DependencyReport */ } }
  ],
  "summary": {
    "total_findings": 45,
    "total_files_analyzed": 123,
    "analysis_time_ms": 456
  },
  "optimization": {
    "shared_parsing": true,      // AST parsed once, reused across analyses
    "cache_hits": 42,            // number of cached results reused
    "sequential_execution": true // queries run sequentially to maximize cache sharing
  }
}
```

**Optimization Strategy**:
- Files are parsed once, AST reused across all analyses in the batch
- Symbol tables and LSP queries cached between analyses
- Queries executed sequentially (not parallel) to maximize cache sharing
- Cache strategy configurable via `batch_optimization` option

---

## Implementation Approach

**No migration needed**: This is a beta product with no external users.

**Direct implementation**:
1. Implement all 6 `analyze.*` commands with unified `AnalysisResult` structure
2. Add actionable suggestions linking to refactoring commands
3. Implement `analyze.batch` with shared parsing optimization
4. Remove all 37 legacy commands immediately
5. Update all internal callsites to use new API
6. Update documentation

**No deprecation period, no legacy wrappers, no telemetry tracking.**

---

## Command Reduction Summary

| Analysis Category | Old Commands | New Commands | Reduction |
|------------------|-------------|--------------|-----------|
| Quality/Complexity | 10 | 1 | -90% |
| Dead Code | 6 | 1 | -83% |
| Dependencies | 6 | 1 | -83% |
| Structure | 7 | 1 | -86% |
| Documentation | 4 | 1 | -75% |
| Test Coverage | 4 | 1 | -75% |
| **TOTAL** | **37** | **6** | **-84%** |

**Plus**: 1 batch command for multi-analysis workflows

**Navigation commands preserved** (not counted in reduction):
- `search_workspace_symbols`
- `find_definition`
- `find_references`
- `find_implementations`

**Legend**:
- ✅ = Already implemented in current system
- ⚠️ = Covered by new API via `kind` parameter, implementation pending

**All 37 legacy commands are covered** - zero regressions. Commands marked ⚠️ require implementing the corresponding `kind` value, but the API design supports them.

---

## Benefits

### 1. Consistency
- Every analysis follows identical pattern
- Uniform result structure across all categories
- Consistent filtering and threshold options

### 2. Actionability
- Every finding includes refactoring suggestions
- Direct links to `*.plan` commands
- Estimated impact for each suggestion

### 3. Composability
- Batch analysis for workflows
- Results can be filtered, sorted, merged
- AI agents can reason about findings

### 4. Simplicity
- 84% fewer commands to learn
- Single result format to parse
- Clear categorization by analysis type

### 5. Extensibility
- New analysis `kind` values added without new commands
- Options extend without breaking changes
- Language-specific features via `kind` + `options`

### 6. Discoverability
- `kind` parameter self-documents available analyses
- Shared structure across all categories
- Better IDE autocomplete and validation

### 7. Integration
- Bridges seamlessly to refactoring API
- Closed-loop: analyze → refactor → re-analyze
- Workflow automation-ready

---

## Integration with Refactoring API

Analysis and refactoring APIs work together:

```javascript
// 1. Analyze code quality
const quality = await analyze.quality("complexity", { type: "workspace" }, {
  thresholds: { cyclomatic_complexity: 20 },
  severity_filter: "high",
  include_suggestions: true
});

// 2. Pick a suggestion
const suggestion = quality.findings[0].suggestions[0];

// 3. Execute the suggested refactoring
const plan = await extract.plan(
  suggestion.refactor_call.arguments.kind,
  suggestion.refactor_call.arguments.source
);

// 4. Preview and apply
if (plan.warnings.length === 0) {
  await workspace.apply_edit(plan);
}

// 5. Re-analyze to verify improvement
const newQuality = await analyze.quality("complexity", {
  type: "file",
  path: quality.findings[0].location.file_path
});

console.log(`Complexity reduced from ${quality.findings[0].metrics.cyclomatic_complexity}
             to ${newQuality.findings[0].metrics.cyclomatic_complexity}`);
```

---

## Design Decisions

### 1. Explicit `kind` Enumerations (LOCKED)
**Decision**: All `kind` values explicitly documented per category.

**Rationale**:
- Clients know exactly what values are valid
- Better IDE autocomplete and validation
- No ambiguity about available analysis types
- Each section lists supported kinds as string literals

### 2. Defaults & Pagination (LOCKED)
**Decision**: Default `limit=1000`, `offset=0`, `severity_filter=null`.

**Rationale**:
- 1000 findings sufficient for most use cases
- `offset` enables pagination for larger result sets
- `null` severity filter includes all findings by default
- `has_more` flag in summary indicates additional results available

### 3. Batch Resource Sharing (LOCKED)
**Decision**: Batch queries share AST parsing, execute sequentially.

**Rationale**:
- Massive performance win for multi-analysis workflows
- Sequential execution maximizes cache hits
- `optimization` object in result provides transparency
- Configurable via `batch_optimization` option

### 4. Suggestion Validation (LOCKED)
**Decision**: CI validates all `suggestion.refactor_call` references.

**Rationale**:
- Prevents broken suggestions from reaching production
- Ensures refactor commands exist and accept correct parameters
- CI test runs all analyzers, validates suggestion structure
- Regression protection as refactoring API evolves

### 5. Suggestion Ranking (LOCKED)
**Decision**: Suggestions ordered by estimated impact, highest first.

**Rationale**:
- Users see most valuable suggestions first
- Optional `priority` field for manual override
- `estimated_impact` required for all suggestions
- AI agents can pick top suggestion without extra logic

### 6. Project-Level Thresholds (DEFERRED)
**Decision**: Phase 2+ feature, support `.codebuddy/analysis.json` config.

**Rationale**:
- Inline options sufficient for MVP
- Config file enables per-project defaults
- Not critical for initial rollout
- Can add without breaking existing API

---

## Success Criteria

- [ ] All 6 `analyze.*` commands implemented and tested
- [ ] Unified `AnalysisResult` structure used consistently
- [ ] Actionable suggestions generated for all finding types
- [ ] `analyze.batch` supports multi-analysis workflows with shared parsing
- [ ] All 37 legacy commands removed from codebase
- [ ] Integration tests cover all analysis kinds
- [ ] All internal callsites updated to new API
- [ ] Documentation shows analyze → refactor workflows
- [ ] CI validates suggestion `refactor_call` references valid commands

---

## Conclusion

This unified analysis API reduces complexity by 84% while providing actionable insights that bridge directly into refactoring workflows. The consistent result structure and suggestion system enable AI agents to reason about code quality and automatically apply improvements.

**Recommendation**: Approve and coordinate with Refactoring API implementation (PROPOSAL_UNIFIED_REFACTORING_API.md) for Phase 1 rollout.
