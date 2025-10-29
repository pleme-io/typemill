# Analysis Tools

**Comprehensive code analysis with unified kind/scope API**

All analysis tools follow a consistent `analyze.<category>(kind, scope, options)` pattern, returning standardized `AnalysisResult` structures with findings, metrics, and actionable suggestions. The unified API design makes it easy to perform different types of analysis across files, directories, or entire workspaces.

**Tool count:** 8 tools
**Related categories:** [Refactoring](refactoring.md) (suggestions link to refactoring commands), [Workspace](workspace.md) (module dependency analysis)

**Common API Pattern:**
```json
{
  "kind": "complexity",        // What to analyze
  "scope": {                   // Where to analyze
    "type": "file",           // "file" | "directory" | "workspace" | "symbol"
    "path": "src/app.rs"
  },
  "options": {                 // How to analyze (optional)
    "thresholds": {},
    "severity_filter": "high",
    "include_suggestions": true
  }
}
```text
**Benefits:**
- Consistent result format across all analysis types
- Actionable suggestions with safety metadata
- Configurable thresholds and filtering
- AST caching for batch operations
- Integration with refactoring API (suggestions → commands)

## Table of Contents

- [Tools](#tools)
  - [analyze.quality](#analyzequality)
  - [analyze.dead_code](#analyzedead_code)
  - [analyze.dependencies](#analyzedependencies)
  - [analyze.structure](#analyzestructure)
  - [analyze.documentation](#analyzedocumentation)
  - [analyze.tests](#analyzetests)
  - [analyze.batch](#analyzebatch)
  - [analyze.module_dependencies](#analyzemodule_dependencies)
- [Common Patterns](#common-patterns)
  - [Unified kind/scope API](#unified-kindscope-api)
  - [Batch operations with AST caching](#batch-operations-with-ast-caching)
  - [Actionable suggestions](#actionable-suggestions)
  - [Performance considerations](#performance-considerations)

---

## Tools

### analyze.quality

**Purpose:** Analyze code quality metrics including complexity, code smells, maintainability, and readability.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| kind | string | **Yes** | Analysis type: `"complexity"` \| `"smells"` \| `"maintainability"` \| `"readability"` |
| scope | object | **Yes** | Analysis scope (see Scope Types below) |
| scope.type | string | **Yes** | Scope granularity: `"file"` \| `"directory"` \| `"workspace"` \| `"symbol"` |
| scope.path | string | **Yes** | Absolute path to file, directory, or symbol location |
| options | object | No | Optional configuration (see Options below) |
| options.thresholds | object | No | Custom complexity thresholds (see Default Thresholds below) |
| options.severity_filter | string | No | Filter by severity: `"high"` \| `"medium"` \| `"low"` \| `null` (default: null = all) |
| options.limit | number | No | Maximum number of findings to return (default: 1000) |
| options.include_suggestions | boolean | No | Include actionable refactoring suggestions (default: true) |
| options.fix | array | No | List of fixers to apply (e.g., `["auto_toc", "trailing_whitespace"]`) |
| options.apply | boolean | No | Apply fixes (false = preview with diffs, true = write files) (default: false) |
| options.fix_options | object | No | Per-fixer configuration options (key = fixer_id, value = config object) |

**Supported Kinds:**
- `"complexity"` - Cyclomatic and cognitive complexity analysis (MVP available)
- `"smells"` - Code smell detection: long methods, god classes, magic numbers
- `"maintainability"` - Overall maintainability metrics
- `"readability"` - Readability issues: nesting, parameter count, length
- `"markdown_structure"` - Markdown structural issues: heading hierarchy, duplicates, empty sections
- `"markdown_formatting"` - Markdown formatting issues: missing alt text, bare URLs, table consistency

**Scope Types:**
- `"file"` - Analyze single file
- `"directory"` - Analyze all files in directory (recursive)
- `"workspace"` - Analyze entire workspace
- `"symbol"` - Analyze specific symbol (requires symbol name in path)

**Default Thresholds:**
```json
{
  "cyclomatic_complexity": 15,
  "cognitive_complexity": 10,
  "nesting_depth": 4,
  "parameter_count": 5,
  "line_count": 50
}
```text
**CLI Alternative:**
```bash
mill analyze complexity --path src/handlers.rs --threshold 20
```text
**Returns:**

`AnalysisResult` structure with:
- `findings[]` - Array of quality issues found
  - `id` - Unique identifier
  - `kind` - Finding type (e.g., "complexity_hotspot", "long_method")
  - `severity` - "high" | "medium" | "low"
  - `location` - File path, range, symbol name
  - `metrics` - Complexity scores (cyclomatic, cognitive, nesting, parameters, line count)
  - `message` - Human-readable description
  - `suggestions[]` - Refactoring suggestions with safety metadata
- `summary` - Statistics (total/returned findings, by_severity, files/symbols analyzed, time)
- `metadata` - Category, kind, scope, language, timestamp, thresholds

**Example:**

```json
// MCP request
{
  "method": "tools/call",
  "params": {
    "name": "analyze.quality",
    "arguments": {
      "kind": "complexity",
      "scope": {
        "type": "file",
        "path": "src/handlers.rs"
      },
      "options": {
        "thresholds": {
          "cyclomatic_complexity": 15,
          "cognitive_complexity": 10,
          "nesting_depth": 4,
          "parameter_count": 5
        },
        "severity_filter": "high",
        "include_suggestions": true
      }
    }
  }
}

// Response
{
  "result": {
    "findings": [
      {
        "id": "complexity-1",
        "kind": "complexity_hotspot",
        "severity": "high",
        "location": {
          "file_path": "src/handlers.rs",
          "range": {
            "start": {"line": 45, "character": 0},
            "end": {"line": 98, "character": 1}
          },
          "symbol": "processOrder",
          "symbol_kind": "function"
        },
        "metrics": {
          "cyclomatic_complexity": 25,
          "cognitive_complexity": 18,
          "nesting_depth": 5,
          "parameter_count": 8,
          "line_count": 53
        },
        "message": "Function 'processOrder' has high cyclomatic complexity (25)",
        "suggestions": [
          {
            "action": "extract_function",
            "description": "Extract nested conditional block to separate function",
            "estimated_impact": "reduces complexity by ~8 points",
            "safety": "requires_review",
            "confidence": 0.85,
            "reversible": true,
            "refactor_call": {
              "command": "extract",
              "arguments": {
                "kind": "function",
                "source": {
                  "file_path": "src/handlers.rs",
                  "range": {
                    "start": {"line": 60, "character": 4},
                    "end": {"line": 75, "character": 5}
                  },
                  "name": "validateOrder"
                }
              }
            }
          }
        ]
      }
    ],
    "summary": {
      "total_findings": 3,
      "returned_findings": 3,
      "has_more": false,
      "by_severity": {"high": 2, "medium": 1, "low": 0},
      "files_analyzed": 1,
      "symbols_analyzed": 12,
      "analysis_time_ms": 234
    },
    "metadata": {
      "category": "quality",
      "kind": "complexity",
      "scope": {"type": "file", "path": "src/handlers.rs"},
      "language": "rust",
      "timestamp": "2025-10-22T12:00:00Z",
      "thresholds": {
        "cyclomatic_complexity": 15,
        "cognitive_complexity": 10
      }
    }
  }
}
```text
**Notes:**
- Reuses proven complexity analysis from `mill-ast::complexity`
- Suggestions include safety metadata (`safe` | `requires_review` | `experimental`)
- All findings link to refactoring commands for closed-loop workflow
- Default thresholds: cyclomatic=15, cognitive=10, nesting=4, params=5, length=50
- Language support: Rust, TypeScript/JavaScript (AST-based)

**Markdown Auto-Fixes:**

For `markdown_structure` and `markdown_formatting` kinds, auto-fix capabilities are available:

**Available Fixers:**
- `auto_toc` - Generate/update table of contents (for `markdown_structure`)
- `trailing_whitespace` - Remove trailing spaces/tabs (for `markdown_formatting`)
- `missing_code_language_tag` - Add language tags to code blocks (for `markdown_formatting`)
- `malformed_heading` - Fix missing space after # (for `markdown_structure`)
- `reversed_link_syntax` - Fix `(url)[text]` → `[text](url)` (for `markdown_formatting`)

**Auto-Fix Options:**

```json
{
  "kind": "markdown_structure",
  "scope": {"type": "file", "path": "README.md"},
  "options": {
    "fix": ["auto_toc"],           // List of fixers to run
    "apply": false,                 // false = preview (default), true = write files
    "fix_options": {                // Per-fixer configuration
      "auto_toc": {
        "marker": "## Table of Contents",  // TOC marker to search for
        "max_depth": 3,                     // Max heading level (1-6)
        "include_h1": false,                // Include H1 headings
        "exclude_patterns": ["^TOC$", "^Contents$"]  // Regex patterns to exclude
      }
    }
  }
}
```

**Auto-Fix Workflow:**

1. **Preview mode** (`apply: false`): Returns diffs without modifying files
   - Response includes `fix_actions.previews` with unified diff format
   - Files remain unchanged
   - Use for validation before applying

2. **Execute mode** (`apply: true`): Applies fixes with conflict detection
   - Response includes `fix_actions.files_modified` count
   - SHA-256 optimistic locking prevents concurrent edit conflicts
   - All fixes applied atomically (all succeed or all rollback)

**Example - Preview TOC Updates:**

```json
{
  "method": "tools/call",
  "params": {
    "name": "analyze.quality",
    "arguments": {
      "kind": "markdown_structure",
      "scope": {"type": "file", "path": "README.md"},
      "options": {
        "fix": ["auto_toc"],
        "apply": false  // Preview only
      }
    }
  }
}

// Response includes:
{
  "result": {
    "summary": {
      "fix_actions": {
        "preview_only": true,
        "applied": false,
        "previews": 1,
        "files_modified": 0,
        "diffs": {
          "README.md": "--- a/README.md\n+++ b/README.md\n@@ -3,7 +3,8 @@\n ## Table of Contents\n \n-Old content\n+- [Section 1](#section-1)\n+- [Section 2](#section-2)"
        }
      }
    }
  }
}
```

**Example - Apply All Markdown Fixes:**

```json
{
  "kind": "markdown_formatting",
  "scope": {"type": "directory", "path": "docs"},
  "options": {
    "fix": ["trailing_whitespace", "missing_code_language_tag", "reversed_link_syntax"],
    "apply": true  // Execute fixes
  }
}

// Response includes:
{
  "result": {
    "summary": {
      "fix_actions": {
        "preview_only": false,
        "applied": true,
        "files_modified": 15,
        "total_edits": 47
      }
    }
  }
}
```

**CLI Examples:**

```bash
# Preview TOC update
mill tool analyze.quality '{"kind": "markdown_structure", "scope": {"type": "file", "path": "README.md"}, "options": {"fix": ["auto_toc"], "apply": false}}'

# Apply TOC update
mill tool analyze.quality '{"kind": "markdown_structure", "scope": {"type": "file", "path": "README.md"}, "options": {"fix": ["auto_toc"], "apply": true}}'

# Fix all formatting issues in directory
mill tool analyze.quality '{"kind": "markdown_formatting", "scope": {"type": "directory", "path": "docs"}, "options": {"fix": ["trailing_whitespace", "missing_code_language_tag"], "apply": true}}'
```

See [Markdown Auto-Fixes](../features/markdown-auto-fixes.md) for complete fixer documentation.

---

### analyze.dead_code

**Purpose:** Detect unused code including imports, symbols, parameters, variables, types, and unreachable code.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| kind | string | **Yes** | Detection type: `"unused_imports"` \| `"unused_symbols"` \| `"unused_parameters"` \| `"unused_variables"` \| `"unused_types"` \| `"unreachable"` |
| scope | object | **Yes** | Analysis scope (see Scope Types below) |
| scope.type | string | **Yes** | Scope granularity: `"file"` \| `"directory"` \| `"workspace"` |
| scope.path | string | **Yes** | Absolute path to file, directory, or workspace root |
| options | object | No | Optional configuration |
| options.severity_filter | string | No | Filter by severity: `"high"` \| `"medium"` \| `"low"` \| `null` (default: null = all) |
| options.limit | number | No | Maximum number of findings to return (default: 1000) |
| options.include_suggestions | boolean | No | Include removal suggestions (default: true) |

**Supported Kinds:**
- `"unused_imports"` - Imports declared but never referenced
- `"unused_symbols"` - Functions, classes, variables defined but never used
- `"unused_parameters"` - Function parameters that are never referenced
- `"unused_variables"` - Local variables assigned but never read
- `"unused_types"` - Type definitions with no references
- `"unreachable"` - Unreachable code after return/break/continue

**Scope Types:**
- `"file"` - Analyze single file
- `"directory"` - Analyze all files in directory (recursive)
- `"workspace"` - Analyze entire workspace

**CLI Alternative:**
```bash
mill analyze dead-code --kind unused_imports --path src
```text
**Error Messages:**
- Missing `kind`: "Invalid request: Missing 'kind' parameter"
- Invalid `kind`: "Unsupported kind 'invalid'. Valid: unused_imports, unused_symbols, unused_parameters, unused_variables, unused_types, unreachable"

**Returns:**

`AnalysisResult` structure with findings describing unused code locations and removal suggestions.

**Example:**

```json
// MCP request
{
  "method": "tools/call",
  "params": {
    "name": "analyze.dead_code",
    "arguments": {
      "kind": "unused_imports",
      "scope": {
        "type": "file",
        "path": "src/components.ts"
      }
    }
  }
}

// Response
{
  "result": {
    "findings": [
      {
        "id": "unused-import-1",
        "kind": "unused_import",
        "severity": "low",
        "location": {
          "file_path": "src/components.ts",
          "range": {
            "start": {"line": 3, "character": 0},
            "end": {"line": 3, "character": 45}
          },
          "symbol": "useEffect"
        },
        "metrics": {
          "module_path": "react",
          "imported_symbols": ["useEffect"],
          "import_type": "named"
        },
        "message": "Import 'useEffect' from 'react' is unused",
        "suggestions": [
          {
            "action": "remove_import",
            "description": "Remove unused import statement",
            "estimated_impact": "removes 1 line of code",
            "safety": "safe",
            "confidence": 0.95,
            "reversible": true
          }
        ]
      }
    ],
    "summary": {
      "total_findings": 2,
      "returned_findings": 2,
      "has_more": false,
      "by_severity": {"high": 0, "medium": 0, "low": 2},
      "files_analyzed": 1,
      "symbols_analyzed": 15,
      "analysis_time_ms": 123
    },
    "metadata": {
      "category": "dead_code",
      "kind": "unused_imports",
      "scope": {"type": "file", "path": "src/components.ts"},
      "language": "typescript",
      "timestamp": "2025-10-22T12:00:00Z"
    }
  }
}
```text
**Notes:**
- Uses heuristic: symbol appearing once = declaration, >1 = usage
- Conservative approach avoids false negatives
- Language-specific import patterns (Rust `use`, TypeScript `import`, etc.)
- Unused symbols detection limited to private/internal symbols
- Public exports assumed to be used externally

---

### analyze.dependencies

**Purpose:** Analyze dependency patterns including imports, dependency graphs, circular dependencies, coupling, and cohesion.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| kind | string | **Yes** | Analysis type: `"imports"` \| `"graph"` \| `"circular"` \| `"coupling"` \| `"cohesion"` \| `"depth"` |
| scope | object | **Yes** | Analysis scope (see Scope Types below) |
| scope.type | string | **Yes** | Scope granularity: `"file"` \| `"directory"` \| `"workspace"` |
| scope.path | string | **Yes** | Absolute path to file, directory, or workspace root |
| options | object | No | Optional configuration |
| options.severity_filter | string | No | Filter by severity: `"high"` \| `"medium"` \| `"low"` \| `null` (default: null = all) |
| options.limit | number | No | Maximum number of findings to return (default: 1000) |
| options.include_suggestions | boolean | No | Include refactoring suggestions (default: true) |

**Supported Kinds:**
- `"imports"` - Categorize all import statements (external, internal, relative)
- `"graph"` - Full dependency graph with nodes and edges
- `"circular"` - Circular dependency detection with cycle paths
- `"coupling"` - Module coupling strength (afferent/efferent coupling)
- `"cohesion"` - Module cohesion metrics (LCOM)
- `"depth"` - Dependency depth and chain analysis

**Scope Types:**
- `"file"` - Analyze single file dependencies
- `"directory"` - Analyze directory dependency structure
- `"workspace"` - Analyze workspace-wide dependencies

**CLI Alternative:**
```bash
mill analyze cycles --path . --fail-on-cycles
mill analyze deps --path src/service.ts
```text
**Error Messages:**
- Missing `kind`: "Invalid request: Missing 'kind' parameter"
- Invalid `kind`: "Unsupported kind 'invalid'. Valid: imports, graph, circular, coupling, cohesion, depth"

**Returns:**

`AnalysisResult` structure with dependency findings and categorization metrics.

**Example:**

```json
// MCP request
{
  "method": "tools/call",
  "params": {
    "name": "analyze.dependencies",
    "arguments": {
      "kind": "imports",
      "scope": {
        "type": "file",
        "path": "src/service.ts"
      }
    }
  }
}

// Response
{
  "result": {
    "findings": [
      {
        "id": "import-1",
        "kind": "import",
        "severity": "low",
        "location": {
          "file_path": "src/service.ts",
          "range": {
            "start": {"line": 1, "character": 0},
            "end": {"line": 1, "character": 38}
          }
        },
        "metrics": {
          "source_module": "react",
          "imported_symbols": ["useState", "useEffect"],
          "import_category": "external",
          "import_type": "named"
        },
        "message": "Import from external module 'react'",
        "suggestions": []
      },
      {
        "id": "import-2",
        "kind": "import",
        "severity": "low",
        "location": {
          "file_path": "src/service.ts",
          "range": {
            "start": {"line": 2, "character": 0},
            "end": {"line": 2, "character": 35}
          }
        },
        "metrics": {
          "source_module": "./utils",
          "imported_symbols": ["formatDate"],
          "import_category": "relative",
          "import_type": "named"
        },
        "message": "Import from relative module './utils'",
        "suggestions": []
      }
    ],
    "summary": {
      "total_findings": 5,
      "returned_findings": 5,
      "has_more": false,
      "by_severity": {"high": 0, "medium": 0, "low": 5},
      "files_analyzed": 1,
      "symbols_analyzed": 8,
      "analysis_time_ms": 98
    },
    "metadata": {
      "category": "dependencies",
      "kind": "imports",
      "scope": {"type": "file", "path": "src/service.ts"},
      "language": "typescript",
      "timestamp": "2025-10-22T12:00:00Z"
    }
  }
}
```text
**Notes:**
- Import categorization: external (packages), internal (project), relative (./...)
- Plugin-based parsing for accurate AST extraction
- Circular dependency detection requires `analysis-circular-deps` feature
- Graph analysis builds dependency tree with metrics
- Coupling/cohesion analysis for architectural insights

---

### analyze.structure

**Purpose:** Analyze code structure including symbols, hierarchy, interfaces, inheritance, and module organization.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| kind | string | **Yes** | Analysis type: `"symbols"` \| `"hierarchy"` \| `"interfaces"` \| `"inheritance"` \| `"modules"` |
| scope | object | **Yes** | Analysis scope (see Scope Types below) |
| scope.type | string | **Yes** | Scope granularity: `"file"` \| `"directory"` \| `"workspace"` |
| scope.path | string | **Yes** | Absolute path to file, directory, or workspace root |
| options | object | No | Optional configuration |
| options.severity_filter | string | No | Filter by severity: `"high"` \| `"medium"` \| `"low"` \| `null` (default: null = all) |
| options.limit | number | No | Maximum number of findings to return (default: 1000) |
| options.include_suggestions | boolean | No | Include refactoring suggestions (default: true) |

**Supported Kinds:**
- `"symbols"` - Extract and categorize all symbols by kind (functions, classes, etc.)
- `"hierarchy"` - Analyze class/module hierarchy structure
- `"interfaces"` - Find interface/trait definitions
- `"inheritance"` - Track inheritance chains and depth
- `"modules"` - Analyze module organization patterns

**Scope Types:**
- `"file"` - Analyze single file structure
- `"directory"` - Analyze directory structure
- `"workspace"` - Analyze workspace structure

**CLI Alternative:**
```bash
mill tool analyze.structure '{
  "kind": "symbols",
  "scope": {"type": "file", "path": "src/models.ts"}
}'
```text
**Error Messages:**
- Missing `kind`: "Invalid request: Missing 'kind' parameter"
- Invalid `kind`: "Unsupported kind 'invalid'. Valid: symbols, hierarchy, interfaces, inheritance, modules"

**Returns:**

`AnalysisResult` structure with structural findings and categorization metrics.

**Example:**

```json
// MCP request
{
  "method": "tools/call",
  "params": {
    "name": "analyze.structure",
    "arguments": {
      "kind": "symbols",
      "scope": {
        "type": "file",
        "path": "src/models.ts"
      }
    }
  }
}

// Response
{
  "result": {
    "findings": [
      {
        "id": "symbols-1",
        "kind": "symbols",
        "severity": "low",
        "location": {
          "file_path": "src/models.ts"
        },
        "metrics": {
          "total_symbols": 12,
          "symbols_by_kind": {
            "Function": 5,
            "Class": 2,
            "Interface": 3,
            "Type": 2
          },
          "visibility_breakdown": {
            "public": 8,
            "private": 4
          }
        },
        "message": "Symbol analysis: 12 total symbols (8 public, 4 private) across 4 categories",
        "suggestions": []
      }
    ],
    "summary": {
      "total_findings": 1,
      "returned_findings": 1,
      "has_more": false,
      "by_severity": {"high": 0, "medium": 0, "low": 1},
      "files_analyzed": 1,
      "symbols_analyzed": 12,
      "analysis_time_ms": 87
    },
    "metadata": {
      "category": "structure",
      "kind": "symbols",
      "scope": {"type": "file", "path": "src/models.ts"},
      "language": "typescript",
      "timestamp": "2025-10-22T12:00:00Z"
    }
  }
}
```text
**Notes:**
- Uses language plugin SymbolKind enum for categorization
- Visibility detection based on naming conventions (MVP)
- Hierarchy analysis tracks class/module relationships
- Inheritance depth helps identify complex hierarchies
- Future: AST-based visibility analysis, unused symbol detection

---

### analyze.documentation

**Purpose:** Analyze documentation quality including coverage, quality, style, examples, and TODOs.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| kind | string | **Yes** | Analysis type: `"coverage"` \| `"quality"` \| `"style"` \| `"examples"` \| `"todos"` |
| scope | object | **Yes** | Analysis scope (see Scope Types below) |
| scope.type | string | **Yes** | Scope granularity: `"file"` \| `"directory"` \| `"workspace"` |
| scope.path | string | **Yes** | Absolute path to file, directory, or workspace root |
| options | object | No | Optional configuration |
| options.thresholds | object | No | Coverage thresholds (default: see below) |
| options.severity_filter | string | No | Filter by severity: `"high"` \| `"medium"` \| `"low"` \| `null` (default: null = all) |
| options.limit | number | No | Maximum number of findings to return (default: 1000) |

**Supported Kinds:**
- `"coverage"` - Documentation coverage percentage (documented vs undocumented)
- `"quality"` - Documentation quality (length, clarity, completeness)
- `"style"` - Documentation style consistency
- `"examples"` - Presence and quality of code examples
- `"todos"` - TODO/FIXME/HACK comments tracking

**Scope Types:**
- `"file"` - Analyze single file documentation
- `"directory"` - Analyze directory documentation
- `"workspace"` - Analyze workspace documentation

**Default Coverage Thresholds:**
- < 50% = high severity (poor documentation)
- 50-80% = medium severity (needs improvement)
- > 80% = low severity (good documentation)

**CLI Alternative:**
```bash
mill tool analyze.documentation '{
  "kind": "coverage",
  "scope": {"type": "file", "path": "src/api.ts"}
}'
```text
**Error Messages:**
- Missing `kind`: "Invalid request: Missing 'kind' parameter"
- Invalid `kind`: "Unsupported kind 'invalid'. Valid: coverage, quality, style, examples, todos"

**Returns:**

`AnalysisResult` structure with documentation findings and coverage/quality metrics.

**Example:**

```json
// MCP request
{
  "method": "tools/call",
  "params": {
    "name": "analyze.documentation",
    "arguments": {
      "kind": "coverage",
      "scope": {
        "type": "file",
        "path": "src/api.ts"
      }
    }
  }
}

// Response
{
  "result": {
    "findings": [
      {
        "id": "coverage-1",
        "kind": "coverage",
        "severity": "medium",
        "location": {
          "file_path": "src/api.ts"
        },
        "metrics": {
          "coverage_percentage": 60.0,
          "documented_count": 3,
          "undocumented_count": 2,
          "total_symbols": 5
        },
        "message": "Documentation coverage is 60% (3/5 symbols documented)",
        "suggestions": [
          {
            "action": "add_documentation",
            "description": "Document undocumented functions",
            "estimated_impact": "improves coverage to 100%",
            "safety": "safe",
            "confidence": 1.0,
            "reversible": true
          }
        ]
      }
    ],
    "summary": {
      "total_findings": 1,
      "returned_findings": 1,
      "has_more": false,
      "by_severity": {"high": 0, "medium": 1, "low": 0},
      "files_analyzed": 1,
      "symbols_analyzed": 5,
      "analysis_time_ms": 45
    },
    "metadata": {
      "category": "documentation",
      "kind": "coverage",
      "scope": {"type": "file", "path": "src/api.ts"},
      "language": "typescript",
      "timestamp": "2025-10-22T12:00:00Z"
    }
  }
}
```text
**Notes:**
- Coverage: percentage of documented functions/classes/modules
- Quality: checks doc length, clarity, parameter descriptions
- Style: consistency with project conventions (JSDoc, rustdoc, etc.)
- TODO detection: finds TODO/FIXME/HACK/XXX comments
- Severity based on coverage thresholds: <50% high, 50-80% medium, >80% low

---

### analyze.tests

**Purpose:** Analyze test quality including coverage, quality, assertions, and organization.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| kind | string | **Yes** | Analysis type: `"coverage"` \| `"quality"` \| `"assertions"` \| `"organization"` |
| scope | object | **Yes** | Analysis scope (see Scope Types below) |
| scope.type | string | **Yes** | Scope granularity: `"file"` \| `"directory"` \| `"workspace"` |
| scope.path | string | **Yes** | Absolute path to file, directory, or workspace root |
| options | object | No | Optional configuration |
| options.thresholds | object | No | Coverage ratio thresholds (default: see below) |
| options.severity_filter | string | No | Filter by severity: `"high"` \| `"medium"` \| `"low"` \| `null` (default: null = all) |
| options.limit | number | No | Maximum number of findings to return (default: 1000) |

**Supported Kinds:**
- `"coverage"` - Test coverage ratio (tests to functions ratio)
- `"quality"` - Test quality (empty tests, trivial assertions, test smells)
- `"assertions"` - Assertion strength and count per test
- `"organization"` - Test file organization and naming

**Scope Types:**
- `"file"` - Analyze single file tests
- `"directory"` - Analyze directory test coverage
- `"workspace"` - Analyze workspace test coverage

**Default Coverage Ratio Thresholds:**
- < 0.5 (50%) = high severity (insufficient tests)
- 0.5-0.8 (50-80%) = medium severity (needs more tests)
- > 0.8 (80%) = low severity (good coverage)

**Note:** This analyzes test-to-function ratio, not line coverage. For line coverage, use `cargo tarpaulin` or similar tools.

**CLI Alternative:**
```bash
mill tool analyze.tests '{
  "kind": "coverage",
  "scope": {"type": "file", "path": "src/calculator.ts"}
}'
```text
**Error Messages:**
- Missing `kind`: "Invalid request: Missing 'kind' parameter"
- Invalid `kind`: "Unsupported kind 'invalid'. Valid: coverage, quality, assertions, organization"

**Returns:**

`AnalysisResult` structure with test findings and coverage/quality metrics.

**Example:**

```json
// MCP request
{
  "method": "tools/call",
  "params": {
    "name": "analyze.tests",
    "arguments": {
      "kind": "coverage",
      "scope": {
        "type": "file",
        "path": "src/calculator.ts"
      }
    }
  }
}

// Response
{
  "result": {
    "findings": [
      {
        "id": "coverage-1",
        "kind": "coverage",
        "severity": "high",
        "location": {
          "file_path": "src/calculator.ts"
        },
        "metrics": {
          "coverage_ratio": 0.4,
          "total_tests": 2,
          "total_functions": 5,
          "test_to_function_ratio": 0.4
        },
        "message": "Low test coverage: 2 tests for 5 functions (40% ratio)",
        "suggestions": [
          {
            "action": "add_tests",
            "description": "Add tests for uncovered functions",
            "estimated_impact": "improves coverage to recommended 1:1 ratio",
            "safety": "safe",
            "confidence": 1.0,
            "reversible": true
          }
        ]
      }
    ],
    "summary": {
      "total_findings": 1,
      "returned_findings": 1,
      "has_more": false,
      "by_severity": {"high": 1, "medium": 0, "low": 0},
      "files_analyzed": 1,
      "symbols_analyzed": 7,
      "analysis_time_ms": 56
    },
    "metadata": {
      "category": "tests",
      "kind": "coverage",
      "scope": {"type": "file", "path": "src/calculator.ts"},
      "language": "typescript",
      "timestamp": "2025-10-22T12:00:00Z"
    }
  }
}
```text
**Notes:**
- Coverage: test-to-function ratio (not line coverage)
- Quality: detects empty tests, trivial assertions, missing assertions
- Assertion strength: checks for meaningful assertions
- Severity: <0.5 ratio = high, 0.5-0.8 = medium, >0.8 = low
- Test smell detection: long tests, multiple concerns per test

---

### analyze.batch

**Purpose:** Execute multiple analysis queries in a single batch for optimized performance with shared AST caching.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| queries | array | **Yes** | Array of analysis query objects (see structure below) |
| noSuggestions | boolean | No | Disable suggestion generation (default: false) |
| maxSuggestions | number | No | Maximum number of suggestions to return |

**Query Object Structure:**

Each query in the array must have:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| command | string | **Yes** | Analysis command: `"analyze.quality"` \| `"analyze.dead_code"` \| `"analyze.dependencies"` \| `"analyze.structure"` \| `"analyze.documentation"` \| `"analyze.tests"` |
| kind | string | **Yes** | Specific analysis kind (depends on command - see individual tool docs) |
| scope | object | **Yes** | Analysis scope object |
| scope.type | string | **Yes** | Scope type: `"file"` \| `"directory"` \| `"workspace"` |
| scope.path | string | **Yes** | Absolute path to analyze |
| options | object | No | Optional configuration (command-specific) |

**Performance Tip:** Use the same `scope.path` for all queries to maximize AST cache reuse!

**Returns:**

`BatchAnalysisResult` with array of results for each query, plus summary and metadata.

**Example:**

```json
// MCP request
{
  "method": "tools/call",
  "params": {
    "name": "analyze.batch",
    "arguments": {
      "queries": [
        {
          "command": "analyze.quality",
          "kind": "complexity",
          "scope": {"type": "file", "path": "src/main.rs"}
        },
        {
          "command": "analyze.dead_code",
          "kind": "unused_imports",
          "scope": {"type": "file", "path": "src/main.rs"}
        },
        {
          "command": "analyze.dependencies",
          "kind": "imports",
          "scope": {"type": "file", "path": "src/main.rs"}
        },
        {
          "command": "analyze.structure",
          "kind": "symbols",
          "scope": {"type": "file", "path": "src/main.rs"}
        },
        {
          "command": "analyze.documentation",
          "kind": "todos",
          "scope": {"type": "file", "path": "src/main.rs"}
        },
        {
          "command": "analyze.tests",
          "kind": "coverage",
          "scope": {"type": "file", "path": "src/main.rs"}
        }
      ]
    }
  }
}

// Response
{
  "result": {
    "results": [
      {
        "command": "analyze.quality",
        "kind": "complexity",
        "result": {
          "findings": [...],
          "summary": {...},
          "metadata": {...}
        }
      },
      {
        "command": "analyze.dead_code",
        "kind": "unused_imports",
        "result": {
          "findings": [...],
          "summary": {...},
          "metadata": {...}
        }
      }
      // ... 4 more results
    ],
    "summary": {
      "total_queries": 6,
      "successful_queries": 6,
      "failed_queries": 0,
      "total_findings": 15,
      "total_suggestions": 8,
      "total_analysis_time_ms": 456
    },
    "suggestions": [
        {
            "message": "Extract helper methods to reduce complexity",
            "safety": "requires_review",
            "confidence": 0.85,
            "reversible": true,
            "estimated_impact": "High",
            "refactor_call": {
                "tool": "extract",
                "arguments": {
                    "file_path": "src/main.rs",
                    "start_line": 4,
                    "end_line": 64
                }
            }
        }
    ],
    "metadata": {
      "batch_id": "batch-1234",
      "timestamp": "2025-10-22T12:00:00Z",
      "ast_cache_hits": 5,
      "ast_cache_misses": 1
    }
  }
}
```text
**Notes:**
- Shared AST parsing: parses each file once, reuses for all queries
- Significant performance improvement for multi-category analysis
- All queries must use same file path for optimal caching
- Each result follows standard `AnalysisResult` format
- Failed queries return error details in result object
- Cache statistics show AST reuse efficiency

---

### analyze.module_dependencies

**Purpose:** Analyze Rust module dependencies for crate extraction - determines external crates, workspace dependencies, and standard library modules required.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| target | object | **Yes** | Target specification (see structure below) |
| target.kind | string | **Yes** | Target type: `"file"` \| `"directory"` |
| target.path | string | **Yes** | Absolute path to Rust file or directory |
| options | object | No | Optional configuration (see options below) |
| options.include_dev_dependencies | boolean | No | Include dev dependencies (default: false) |
| options.include_workspace_deps | boolean | No | Include workspace dependencies (default: true) |
| options.resolve_features | boolean | No | Resolve cargo features (default: true) |

**Target Kinds:**
- `"file"` - Analyze single Rust file dependencies
- `"directory"` - Analyze all .rs files in directory (recursive)

**Options Defaults:**
```json
{
  "include_dev_dependencies": false,
  "include_workspace_deps": true,
  "resolve_features": true
}
```text
**Language Support:** Rust only (.rs files)

**CLI Alternative:**
```bash
mill tool analyze.module_dependencies '{
  "target": {
    "kind": "directory",
    "path": "crates/mill-ast/src"
  },
  "options": {
    "include_workspace_deps": true
  }
}'
```text
**Error Messages:**
- Missing `target`: "Invalid request: Missing 'target' parameter"
- Invalid `target.kind`: "Unsupported kind 'invalid'. Valid: file, directory"
- Non-Rust path: "Path does not contain any Rust (.rs) files"

**Returns:**

Dependency analysis result with:
- `external_dependencies` - Third-party crates with version/features
- `workspace_dependencies` - Internal workspace crates
- `std_dependencies` - Standard library modules
- `import_analysis` - Summary statistics
- `files_analyzed` - List of scanned files

**Example:**

```json
// MCP request
{
  "method": "tools/call",
  "params": {
    "name": "analyze.module_dependencies",
    "arguments": {
      "target": {
        "kind": "directory",
        "path": "crates/mill-ast/src"
      },
      "options": {
        "include_workspace_deps": true,
        "resolve_features": true
      }
    }
  }
}

// Response
{
  "result": {
    "external_dependencies": {
      "serde": {
        "version": "1.0",
        "features": ["derive"],
        "optional": false,
        "source": "workspace"
      },
      "tokio": {
        "version": "1.28",
        "features": ["full", "macros"],
        "optional": false,
        "source": "crates.io"
      },
      "anyhow": {
        "version": "1.0",
        "features": [],
        "optional": false,
        "source": "crates.io"
      }
    },
    "workspace_dependencies": [
      "mill-foundation",
      "mill-plugin-api",
      "mill-types"
    ],
    "std_dependencies": [
      "std::collections",
      "std::fs",
      "std::path",
      "std::io"
    ],
    "import_analysis": {
      "total_imports": 42,
      "external_crates": 3,
      "workspace_crates": 3,
      "std_crates": 4,
      "unresolved_imports": []
    },
    "files_analyzed": [
      "crates/mill-ast/src/lib.rs",
      "crates/mill-ast/src/parser.rs",
      "crates/mill-ast/src/complexity/mod.rs",
      "crates/mill-ast/src/complexity/metrics.rs"
    ]
  }
}
```text
**Notes:**
- Rust-specific: parses `use` statements from .rs files
- Cross-references with workspace Cargo.toml for versions
- Distinguishes workspace deps vs external deps
- Feature detection (e.g., `serde = { version = "1.0", features = ["derive"] }`)
- Essential for `workspace.extract_dependencies` tool
- Reports unresolved imports for manual investigation
- Use cases: pre-extraction analysis, dependency auditing, Cargo.toml generation

---

## CLI vs MCP Tool Interfaces

Some analysis tools can be invoked two different ways:

**Via CLI subcommand** (supports flags):
```bash
# Complexity analysis
mill analyze complexity --path src/handlers.rs --threshold 20

# Circular dependency detection
mill analyze cycles --path . --fail-on-cycles

# Dead code detection
mill analyze dead-code --kind unused_imports --path src

# Dependency analysis
mill analyze deps --path src/service.ts
```text
**Via MCP tool** (requires JSON):
```bash
# Same complexity analysis via MCP tool interface
mill tool analyze.quality '{
  "kind": "complexity",
  "scope": {"type": "file", "path": "src/handlers.rs"},
  "options": {"thresholds": {"cyclomatic_complexity": 20}}
}'

# Same circular dependency detection
mill tool analyze.dependencies '{
  "kind": "circular",
  "scope": {"type": "workspace", "path": "."}
}'
```text
**When to use each:**
- **CLI subcommands**: Faster for manual/interactive use, supports flags, better error messages
- **MCP tool interface**: Required for programmatic/AI agent use, more flexible, consistent with other tools

Both interfaces call the same underlying analysis tools and return the same results.

---

## Common Patterns

### Unified kind/scope API

All analysis tools follow the same parameter pattern:

```json
{
  "kind": "specific_analysis",  // What to analyze
  "scope": {                     // Where to analyze
    "type": "file",             // Scope granularity
    "path": "src/file.rs"       // Target path
  },
  "options": {                   // Optional configuration
    "thresholds": {},
    "severity_filter": null,
    "limit": 1000,
    "include_suggestions": true
  }
}
```text
**Scope Types:**
- `file` - Single file analysis
- `directory` - Recursive directory analysis
- `workspace` - Full workspace analysis
- `symbol` - Specific symbol analysis (requires symbol name)

**Benefits:**
- Consistent API across all analysis categories
- Easy to learn: master one pattern, use all tools
- Composable: combine multiple analyses in batch operations
- Predictable: same result structure for all tools

### Batch operations with AST caching

Use `analyze.batch` for multi-category analysis:

```json
{
  "queries": [
    {
      "command": "analyze.quality",
      "kind": "complexity",
      "scope": {"type": "file", "path": "src/app.rs"}
    },
    {
      "command": "analyze.dead_code",
      "kind": "unused_imports",
      "scope": {"type": "file", "path": "src/app.rs"}
    }
  ]
}
```text
**Performance optimization:**
- AST parsed once per file
- Shared across all queries in batch
- Significant speedup for multi-category analysis
- Cache hits/misses reported in metadata

### Actionable suggestions

All findings include actionable suggestions:

```json
{
  "suggestions": [
    {
      "action": "extract_function",
      "description": "Extract nested block to function",
      "estimated_impact": "reduces complexity by ~8 points",
      "safety": "requires_review",
      "confidence": 0.85,
      "reversible": true,
      "refactor_call": {
        "command": "extract",
        "arguments": {...}
      }
    }
  ]
}
```text
**Safety levels:**
- `safe` - No logic changes, preserves semantics
- `requires_review` - Logic changes, needs verification
- `experimental` - Significant changes, thorough testing required

**Integration with refactoring:**
- `refactor_call` provides ready-to-use refactoring command
- Closed-loop workflow: analyze → suggest → refactor
- Confidence scores guide automated vs manual review

### Performance considerations

**AST caching:**
- Analyses reuse parsed AST within same file
- Batch operations optimize cache utilization
- Cache statistics in batch metadata
- Disable with `TYPEMILL_DISABLE_AST_CACHE=1`

**Language support:**
- Full support: Rust, TypeScript/JavaScript
- AST-based parsing via language plugins
- Future: Python, Go, Java (git tag `pre-language-reduction`)

**Thresholds:**
- Configurable per analysis type
- Default values tuned for general use
- Adjust for project-specific standards
- Severity computed based on threshold violations

---

**Language Support:** TypeScript/JavaScript (.ts, .tsx, .js, .jsx), Rust (.rs)
**Additional languages** (Python, Go, Java, Swift, C#) available in git tag `pre-language-reduction`

**Related Documentation:**
- [Refactoring Tools](refactoring.md) - Apply suggestions from analysis
- [Workspace Tools](workspace.md) - Use module_dependencies for crate extraction
- [API Contracts](../architecture/api_contracts.md) - JSON schemas and validation
- **[Actionable Suggestions](../features/actionable_suggestions.md)** - How analysis suggestions work
- **[Cache Configuration](../operations/cache_configuration.md)** - Performance tuning for analysis

---

**Last Updated:** 2025-10-22
**API Version:** 1.0.0-rc4