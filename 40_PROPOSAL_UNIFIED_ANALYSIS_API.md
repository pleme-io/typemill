# Proposal: Unified Analysis API

**Status**: Draft
**Author**: Project Team
**Date**: 2025-10-10
**Formal Spec**: [docs/design/unified_api_contracts.md](docs/design/unified_api_contracts.md)

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

### Pillar 2: Analysis Primitives (Code Understanding)

These building blocks deliver insight and precision before refactoring happens:

- **Linting** – surface style violations and simple correctness bugs early.
- **Complexity Analysis** – highlight high-risk functions or modules as they grow unwieldy.
- **Dead Code Detection** – identify unused or unreachable symbols so the codebase can be reclaimed.
- **Code Smell Detection** – spot maintainability red flags (long methods, god objects, magic numbers, etc.).
- **Dependency Analysis** – map relationships and cycles across files, modules, and packages.

Together, these analysis primitives establish the foundation for understanding code health and guiding subsequent refactors.

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
          "estimated_impact": "reduces complexity by ~8 points",
          "safety": "requires_review",
          "confidence": 0.85,
          "reversible": true
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
        "safety": "requires_review",
        "confidence": 0.85,
        "reversible": true,
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
        "safety": "safe",
        "confidence": 0.95,
        "reversible": true,
        "refactor_call": {
          "command": "inline.plan",
          "arguments": {
            "kind": "variable",
            "target": { "file_path": "src/app.rs", "position": { "line": 12, "character": 8 } }
          }
        },
        "estimated_impact": "reduces complexity by 1 point"
      },
      {
        "action": "delete_unused_import",
        "description": "Remove unused import 'std::collections::HashMap'",
        "safety": "safe",
        "confidence": 0.98,
        "reversible": true,
        "refactor_call": {
          "command": "delete.plan",
          "arguments": {
            "kind": "unused_imports",
            "target": { "file_path": "src/app.rs" }
          }
        },
        "estimated_impact": "no complexity change, improves code cleanliness"
      }
    ]
  }]
}
```

**Suggestion metadata fields**:
- **`safety`**: Risk level for applying the suggestion
  - `"safe"` - No logic changes, preserves semantics exactly (auto-apply safe)
  - `"requires_review"` - Logic changes, preserves intent but needs human verification
  - `"experimental"` - Significant changes, requires thorough testing
- **`confidence`**: Algorithm confidence score (0.0 to 1.0) in suggestion correctness
- **`reversible`**: Whether the refactoring can be undone without information loss
- **`estimated_impact`**: Human-readable description of expected improvement

**Benefits**:
- AI agents can **autonomously apply safe suggestions** (safety="safe", confidence > 0.9)
- AI agents know when to **request human review** (safety="requires_review" or low confidence)
- Users get **risk-assessed actionable next steps**, not just metrics
- **Closed-loop workflow**: analyze → suggest → refactor → re-analyze
- **Progressive automation**: safe → requires_review → experimental

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

**No long-term legacy support**: This is a beta product with no external users. We will not maintain dual APIs long-term.

**Phased implementation** (see [35_IMPLEMENTATION_SEQUENCING.md](35_IMPLEMENTATION_SEQUENCING.md) for detailed timeline):

### Phase 0: Foundation (PREREQUISITE)
- **Self-registration system** for plugin capability discovery
- Registry descriptors enable dynamic validation of `kind` values
- **Blocks**: All unified API work until complete
- **Timeline**: 2-3 weeks

### Phase 2A: Core Analysis (3-4 weeks, staged by category)
For each analysis category:
1. Implement `analyze.<category>` with all `kind` variants
2. Verify each `kind` produces correct results (tests pass)
3. Add basic suggestions linking to refactoring commands
4. **No config/safety metadata yet** - inline options only

### Phase 2B: Configuration (1-2 weeks, parallel with 2C)
1. `.codebuddy/analysis.toml` loader
2. Preset resolution with overrides
3. Config validation against registry

### Phase 2C: Safety Metadata (2-3 weeks, parallel with 2B)
1. Safety classification logic per suggestion type
2. Confidence scoring algorithms
3. Reversibility analysis
4. Safety-first ranking algorithm
5. CI validation of metadata

### Phase 3: Batch Operations (2-3 weeks)
1. `analyze.batch` with shared AST parsing
2. Cache optimization
3. Performance benchmarks

### Legacy Removal (Per Category)
Only after Phase 2A completes for a category:
1. Remove legacy commands for that category
2. Update documentation
3. Verify no regressions

**Critical dependency**: Phase 0 (self-registration) must complete before Phase 2A.

### Suggested Implementation Order

**1. Quality Analysis** (remove 10 legacy commands)
- Implement: `analyze.quality` with kinds: complexity, smells, maintainability, readability
- Remove: analyze_complexity, find_complexity_hotspots, suggest_refactoring, find_god_classes, etc.

**2. Dead Code Analysis** (remove 6 legacy commands)
- Implement: `analyze.dead_code` with kinds: unused_symbols, unused_imports, unreachable_code, unused_parameters, unused_types, unused_variables
- Remove: find_dead_code, find_unused_imports, find_unused_parameters, etc.

**3. Dependency Analysis** (remove 6 legacy commands)
- Implement: `analyze.dependencies` with kinds: imports, graph, circular, coupling, cohesion, depth
- Remove: analyze_imports, find_circular_dependencies, etc.

**4. Structure Analysis** (remove 7 legacy commands)
- Implement: `analyze.structure` with kinds: symbols, hierarchy, interfaces, inheritance, modules
- Remove: get_document_symbols, analyze_inheritance, etc.
- Note: Keep navigation commands (search_workspace_symbols, find_definition, etc.)

**5. Documentation Analysis** (remove 4 legacy commands)
- Implement: `analyze.documentation` with kinds: coverage, quality, missing, outdated, todos
- Remove: analyze_comment_ratio, find_undocumented_exports, etc.

**6. Test Analysis** (remove 4 legacy commands)
- Implement: `analyze.tests` with kinds: coverage, untested, quality, smells
- Remove: analyze_test_coverage, find_untested_code, etc.

**7. Batch Support** (add new capability)
- Implement: `analyze.batch` with shared parsing optimization

### Timeline

**No fixed timeline** - we're the only users. Implement at comfortable pace, verify each category works before removing legacy.

**Key principle**: Never remove a legacy command until its replacement is implemented and tested.

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
**Decision**: CI validates all `suggestion.refactor_call` references and safety metadata.

**Rationale**:
- Prevents broken suggestions from reaching production
- Ensures refactor commands exist and accept correct parameters
- Validates safety level and confidence score are present and reasonable
- CI test runs all analyzers, validates suggestion structure
- Regression protection as refactoring API evolves

**Validation checks**:
- `refactor_call.command` references valid refactoring command
- `refactor_call.arguments` match command schema
- `safety` is one of: "safe", "requires_review", "experimental"
- `confidence` is float between 0.0 and 1.0
- `reversible` is boolean
- `estimated_impact` is non-empty string

### 5. Suggestion Ranking (LOCKED)
**Decision**: Suggestions ordered by safety (safe first), then confidence, then estimated impact.

**Rationale**:
- AI agents should see safest, highest-confidence suggestions first
- Enables progressive automation: apply all "safe" suggestions, then ask for human review on others
- Optional `priority` field for manual override
- `estimated_impact` required for all suggestions

**Ranking algorithm**:
1. Primary: Safety level ("safe" > "requires_review" > "experimental")
2. Secondary: Confidence score (higher is better)
3. Tertiary: Estimated impact (parsed heuristically from string)

### 6. Project-Level Configuration (PROMOTED TO PHASE 1)
**Decision**: Support `.codebuddy/analysis.toml` for preset configurations.

**Rationale**:
- Dramatically improves DX by eliminating repetitive option passing
- Ensures consistency across team members and AI agents
- Config file serves as living documentation of project quality standards
- Can be overridden per-call when needed

**Configuration format**:
```toml
# .codebuddy/analysis.toml
[presets.strict]
thresholds = { cyclomatic_complexity = 10, nesting_depth = 3, parameter_count = 4 }
severity_filter = "high"
limit = 100

[presets.permissive]
thresholds = { cyclomatic_complexity = 25, nesting_depth = 6, parameter_count = 8 }
severity_filter = "medium"
limit = 500

[presets.ci]
thresholds = { cyclomatic_complexity = 15, nesting_depth = 4 }
severity_filter = "high"
limit = 1000
include_suggestions = true

[defaults]
# Applied to all analysis commands unless overridden
scope = { type = "workspace", exclude = ["tests/", "examples/", "benches/"] }
limit = 1000
format = "detailed"
include_suggestions = true
```

**Usage**:
```javascript
// Use preset
analyze.quality("complexity", { preset: "strict" })

// Preset with scope override
analyze.quality("complexity", { preset: "strict", scope: { type = "file", path = "src/lib.rs" } })

// Override specific options
analyze.quality("complexity", { preset: "strict", thresholds: { cyclomatic_complexity: 12 } })
```

---

## Success Criteria

**Per-category completion** (repeat for each of 6 categories):
- [ ] `analyze.<category>` command implemented with all `kind` variants
- [ ] All `kind` values for category produce correct results
- [ ] Tests pass for all kinds in category
- [ ] Actionable suggestions generated with safety/confidence/reversible metadata
- [ ] Suggestions ordered by safety, then confidence, then impact
- [ ] Internal callsites updated to use new API
- [ ] Legacy commands for category removed
- [ ] Documentation updated for category

**Overall completion**:
- [ ] All 6 `analyze.*` commands implemented and tested
- [ ] Unified `AnalysisResult` structure used consistently across all categories
- [ ] Project-level configuration (`.codebuddy/analysis.toml`) with preset support
- [ ] All suggestions include safety, confidence, and reversible fields
- [ ] Suggestion ranking algorithm implemented (safety → confidence → impact)
- [ ] `analyze.batch` supports multi-analysis workflows with shared parsing
- [ ] All 37 legacy commands removed from codebase (staged by category)
- [ ] Integration tests cover all analysis kinds (24 total kind values)
- [ ] Integration tests cover preset loading and override behavior
- [ ] Documentation shows analyze → refactor workflows with safety examples
- [ ] CI validates suggestion `refactor_call` references valid commands
- [ ] CI validates safety metadata (valid levels, confidence range, etc.)
- [ ] CI validates suggestion ranking order
- [ ] Navigation commands preserved (search_workspace_symbols, find_definition, etc.)

**Key milestone**: Can complete categories in any order. Each category is independently shippable.

---

## Conclusion

This unified analysis API reduces complexity by 84% while providing actionable insights that bridge directly into refactoring workflows. The consistent result structure and suggestion system enable AI agents to reason about code quality and automatically apply improvements.

**Implementation strategy**: Build first, remove second. Each category is implemented and tested before removing its legacy commands. No functionality gaps, no regressions.

**Recommendation**: Approve and begin with Quality Analysis category (easiest, most used). Coordinate with Refactoring API implementation (30_PROPOSAL_UNIFIED_REFACTORING_API.md) for end-to-end workflows.

---

## Implementation Checklist: Complete File Manifest

This section provides a comprehensive checklist of all files that need to be created, edited, or removed when implementing this proposal.

### Files to CREATE (31 new files)

#### Protocol Types (1 file)
- [ ] `crates/cb-protocol/src/analysis_result.rs` - Unified analysis result structures
  - `AnalysisResult`, `QualityReport`, `DeadCodeReport`, `DependencyReport`, `StructureReport`, `DocumentationReport`, `TestReport`
  - `Finding`, `AnalysisSummary`, `AnalysisMetadata`, `Suggestion`

#### Configuration (1 file)
- [ ] `crates/cb-core/src/analysis_config.rs` - Configuration loader for `.codebuddy/analysis.toml`
  - `AnalysisConfig`, `AnalysisPreset`, `AnalysisDefaults`
  - Preset loading and application logic

#### Handler Files (7 files)
- [ ] `crates/cb-handlers/src/handlers/analysis/quality_handler.rs` - `analyze.quality` implementation
- [ ] `crates/cb-handlers/src/handlers/analysis/dead_code_handler.rs` - `analyze.dead_code` implementation
- [ ] `crates/cb-handlers/src/handlers/analysis/dependency_handler.rs` - `analyze.dependencies` implementation
- [ ] `crates/cb-handlers/src/handlers/analysis/structure_handler.rs` - `analyze.structure` implementation
- [ ] `crates/cb-handlers/src/handlers/analysis/documentation_handler.rs` - `analyze.documentation` implementation
- [ ] `crates/cb-handlers/src/handlers/analysis/test_handler.rs` - `analyze.tests` implementation
- [ ] `crates/cb-handlers/src/handlers/analysis/batch_handler.rs` - `analyze.batch` implementation

#### Analysis Engine Implementations (14 files)
- [ ] `analysis/cb-analysis-quality/Cargo.toml` - Quality analysis package manifest
- [ ] `analysis/cb-analysis-quality/src/lib.rs` - Quality analysis engine
- [ ] `analysis/cb-analysis-quality/src/complexity.rs` - Complexity analyzer
- [ ] `analysis/cb-analysis-quality/src/smells.rs` - Code smell detector
- [ ] `analysis/cb-analysis-quality/src/maintainability.rs` - Maintainability metrics
- [ ] `analysis/cb-analysis-quality/src/readability.rs` - Readability analyzer

- [ ] `analysis/cb-analysis-dependency/Cargo.toml` - Dependency analysis package manifest
- [ ] `analysis/cb-analysis-dependency/src/lib.rs` - Dependency analysis engine
- [ ] `analysis/cb-analysis-dependency/src/circular.rs` - Circular dependency detector
- [ ] `analysis/cb-analysis-dependency/src/coupling.rs` - Coupling analyzer

- [ ] `analysis/cb-analysis-structure/Cargo.toml` - Structure analysis package manifest
- [ ] `analysis/cb-analysis-structure/src/lib.rs` - Structure analysis engine
- [ ] `analysis/cb-analysis-structure/src/hierarchy.rs` - Hierarchy analyzer

- [ ] `analysis/cb-analysis-documentation/Cargo.toml` - Documentation analysis package manifest
- [ ] `analysis/cb-analysis-documentation/src/lib.rs` - Documentation analysis engine

- [ ] `analysis/cb-analysis-test/Cargo.toml` - Test analysis package manifest
- [ ] `analysis/cb-analysis-test/src/lib.rs` - Test analysis engine

#### Integration Tests (7 files)
- [ ] `integration-tests/src/test_unified_analysis_quality.rs` - Quality analysis tests
- [ ] `integration-tests/src/test_unified_analysis_dead_code.rs` - Dead code analysis tests
- [ ] `integration-tests/src/test_unified_analysis_dependencies.rs` - Dependency analysis tests
- [ ] `integration-tests/src/test_unified_analysis_structure.rs` - Structure analysis tests
- [ ] `integration-tests/src/test_unified_analysis_documentation.rs` - Documentation analysis tests
- [ ] `integration-tests/src/test_unified_analysis_tests.rs` - Test coverage analysis tests
- [ ] `integration-tests/src/test_unified_analysis_batch.rs` - Batch analysis tests

---

### Files to EDIT (24 existing files)

#### Core Registration & Routing (5 files)
- [ ] `crates/cb-handlers/src/handlers/mod.rs` - Add `pub mod analysis;` directory
- [ ] `crates/cb-protocol/src/lib.rs` - Export `analysis_result` module
- [ ] `crates/cb-core/src/lib.rs` - Export `analysis_config` module
- [ ] `crates/cb-handlers/src/handlers/tool_registry.rs` - Register 6 new `analyze.*` tools
- [ ] `crates/cb-handlers/src/handlers/plugin_dispatcher.rs` - Route analysis commands to handlers
  - Add analysis handler registration via `register_handlers_with_logging!`

#### Cargo Configuration (2 files)
- [ ] `Cargo.toml` - Add new analysis crates to workspace members:
  - `analysis/cb-analysis-quality`
  - `analysis/cb-analysis-dependency`
  - `analysis/cb-analysis-structure`
  - `analysis/cb-analysis-documentation`
  - `analysis/cb-analysis-test`

- [ ] `crates/cb-handlers/Cargo.toml` - Add dependencies on new analysis crates

#### Test Infrastructure (1 file)
- [ ] `crates/cb-test-support/src/harness/mcp_fixtures.rs` - Add analysis test fixtures

#### Documentation (7 files)
- [ ] `API_REFERENCE.md` - Replace 37 legacy commands with 6 unified commands
  - Document each `kind` value per category
  - Add result structure examples
  - Show analyze → refactor workflows

- [ ] `QUICK_REFERENCE.md` - Update with new analysis commands

- [ ] `CLAUDE.md` - Update AI agent instructions
  - Remove legacy analysis commands
  - Add unified analysis API guidance

- [ ] `AGENTS.md` - Same as CLAUDE.md (synchronized)

- [ ] `docs/TOOLS_QUICK_REFERENCE.md` - Update tool reference

- [ ] `CONTRIBUTING.md` - Document new analysis handler patterns

- [ ] `CHANGELOG.md` - Document unified analysis API release

#### Build Configuration (1 file)
- [ ] `Makefile` - Add analysis test targets:
  - `make test-analysis` - Run all analysis tests
  - `make test-analysis-quality` - Per-category testing
  - (Additional per-category targets)

#### Existing Analysis Infrastructure (7 files - migrate/adapt)
- [ ] `analysis/cb-analysis-dead-code/src/detector.rs` - Adapt to unified `AnalysisResult`
- [ ] `analysis/cb-analysis-dead-code/src/types.rs` - Migrate to `analysis_result.rs` structures
- [ ] `analysis/cb-analysis-dead-code/src/config.rs` - Merge into `analysis_config.rs`
- [ ] `analysis/cb-analysis-graph/src/dependency.rs` - Adapt for `analyze.dependencies`
- [ ] `crates/cb-ast/src/complexity/analyzer.rs` - Integrate with `analyze.quality`
- [ ] `crates/cb-lang-typescript/src/import_support.rs` - Extend for dependency analysis
- [ ] `crates/cb-lang-rust/src/parser.rs` - Extend for structure analysis

---

### Files to REMOVE (7 files - after migration complete)

#### Legacy Handler Files
- [ ] `crates/cb-handlers/src/handlers/analysis_handler.rs` - Replaced by category handlers
- [ ] `crates/cb-handlers/src/handlers/dependency_handler.rs` - Replaced by `analysis/dependency_handler.rs`
- [ ] `crates/cb-handlers/src/handlers/tools/analysis/code.rs` - Replaced by `quality_handler.rs`
- [ ] `crates/cb-handlers/src/handlers/tools/analysis/project.rs` - Replaced by `quality_handler.rs`
- [ ] `crates/cb-handlers/src/handlers/tools/analysis/unused_imports.rs` - Replaced by `dead_code_handler.rs`

#### Legacy Test Files
- [ ] `integration-tests/src/test_analysis_features.rs` - Replace with category-specific tests (after migration)

#### Legacy Config
- [ ] `analysis/cb-analysis-dead-code/src/config.rs` - Merged into unified `analysis_config.rs`

---

### Implementation Effort Estimate

| Phase | New Files | Edited Files | Removed Files | Estimated Effort |
|-------|-----------|--------------|---------------|------------------|
| Protocol & Config | 2 | 3 | 0 | 2-3 days |
| Quality Analysis | 6 | 4 | 2 | 1-2 weeks |
| Dead Code Analysis | 1 | 3 | 2 | 1 week |
| Dependency Analysis | 4 | 3 | 1 | 1-2 weeks |
| Structure Analysis | 3 | 2 | 0 | 1-2 weeks |
| Documentation Analysis | 2 | 2 | 0 | 1 week |
| Test Analysis | 2 | 2 | 0 | 1 week |
| Batch Support | 2 | 2 | 0 | 1 week |
| Documentation | 0 | 7 | 1 | 3-4 days |
| Testing & CI | 7 | 2 | 1 | 1 week |
| **TOTAL** | **29** | **30** | **7** | **8-11 weeks** |

**Total Files Changed: ~66 files**

---

### Implementation Order (Recommended)

1. **Protocol Foundation** (Days 1-3)
   - Create `analysis_result.rs` with all types
   - Create `analysis_config.rs` with preset system
   - Update module exports

2. **Quality Analysis** (Weeks 1-2) - Highest value, most used
   - Implement `analyze.quality` with all 4 kinds
   - Write integration tests
   - Remove legacy complexity commands

3. **Dead Code Analysis** (Week 3) - Already partially implemented
   - Implement `analyze.dead_code` with all 6 kinds
   - Migrate existing `find_dead_code` logic
   - Remove legacy dead code commands

4. **Dependency Analysis** (Weeks 4-5)
   - Implement `analyze.dependencies` with all 6 kinds
   - Write integration tests
   - Remove legacy dependency commands

5. **Structure Analysis** (Weeks 6-7)
   - Implement `analyze.structure` with all 5 kinds
   - Write integration tests
   - Remove legacy structure commands

6. **Documentation Analysis** (Week 8)
   - Implement `analyze.documentation` with all 5 kinds
   - Write integration tests

7. **Test Analysis** (Week 9)
   - Implement `analyze.tests` with all 4 kinds
   - Write integration tests

8. **Batch Support** (Week 10)
   - Implement `analyze.batch` with shared AST parsing
   - Performance benchmarks
   - Cache optimization tests

9. **Documentation & Cleanup** (Week 11)
   - Update all documentation
   - Final verification
   - CI validation setup

---

### Validation Checklist

Use this checklist to verify completeness during implementation:

#### Per-Category Completion
For each of 6 analysis categories, verify:
- [ ] Handler file created with all `kind` implementations
- [ ] Analysis engine crate created (if needed)
- [ ] Integration tests passing for all kinds
- [ ] Suggestions include safety/confidence/reversible metadata
- [ ] Suggestion ranking implemented (safety → confidence → impact)
- [ ] Legacy commands removed for this category
- [ ] Documentation updated
- [ ] Tool registered in `tool_registry.rs`
- [ ] Handler registered in `plugin_dispatcher.rs`

#### Overall Completion
- [ ] All 6 `analyze.*` commands working end-to-end
- [ ] `analyze.batch` supports multi-analysis with shared parsing
- [ ] `.codebuddy/analysis.toml` configuration loading works
- [ ] Preset system with overrides functional
- [ ] All 37 legacy analysis commands removed
- [ ] All 7 integration test files passing
- [ ] Navigation commands preserved (search_workspace_symbols, find_definition, etc.)
- [ ] API_REFERENCE.md fully updated
- [ ] All documentation synchronized
- [ ] CI validates suggestion metadata
- [ ] Build passes with zero warnings in new code

---

This checklist ensures no files are missed during implementation and provides clear tracking for the 8-11 week implementation timeline.
