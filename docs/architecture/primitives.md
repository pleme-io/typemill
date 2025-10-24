# Code Primitives: The Foundation of Codebuddy

## Overview

Codebuddy's design philosophy is built on two foundational pillars that work together to provide comprehensive code intelligence and transformation capabilities. These primitives represent the "DNA" of developer tooling — minimal, composable building blocks that combine to solve complex software engineering challenges.

This document explains the conceptual framework underlying Codebuddy's tool design and how these primitives map to actual MCP tools in the system.

---

## The Two Pillars

### Pillar 1: Refactoring Primitives (Code Transformation)

Refactoring primitives are atomic operations for restructuring or improving code without changing its external behavior. Each primitive represents a single, focused transformation that can be composed with others to achieve complex refactoring goals.

### Pillar 2: Analysis Primitives (Code Understanding)

Analysis primitives power insight and precision before refactoring happens. These tools help understand code health, structure, and relationships, enabling intelligent decision-making about what transformations to apply.

Together, these pillars form a complete code intelligence ecosystem: **analysis informs refactoring, and refactoring builds on analysis**.

---

## Pillar 1: Refactoring Primitives

### Core Primitive Operations

#### 1. Rename
**Concept**: Change the name of a symbol (variable, function, class, module, or file) throughout the codebase.

**Implemented Tools**:
- `rename.plan` - Plan renaming of symbols, files, or directories (unified API)
- `workspace.apply_edit` - Execute rename plans with atomic application
- `rename_file` - Legacy direct file rename with automatic import updates
- `rename_directory` - Legacy direct directory rename with automatic import updates

**Key Characteristics**:
- Scope-aware (local vs. global scope)
- Reference tracking (find all usages)
- Import/export updates
- Cross-file consistency

**Example Use Cases**:
- Improving code clarity with better naming
- Aligning with naming conventions
- Refactoring API surface areas

---

#### 2. Extract
**Concept**: Pull a block of code into its own function, file, or module for better organization and reusability.

**Implemented Tools**:
- `extract.plan` - Plan code extraction (function, variable, constant) (unified API)
- `workspace.apply_edit` - Execute extraction plans with atomic application
- `extract_module_to_package` - Extract module into a separate package (workspace-level)

**Key Characteristics**:
- Scope preservation (captures necessary parameters)
- Return value detection
- Dependency analysis
- Import generation

**Example Use Cases**:
- Breaking down large functions
- Eliminating code duplication
- Creating reusable components
- Modularizing monolithic codebases

---

#### 3. Inject / Insert
**Concept**: Add code to an existing structure (imports, function parameters, class members).

**Implemented Tools**:
- `apply_edits` - Apply multiple edits atomically across files
- `write_file` - Write new content to files
- Code actions from `get_code_actions` - Add missing imports, implement interfaces

**Key Characteristics**:
- Position-aware insertion
- Syntax preservation
- Conflict detection
- Multi-file atomicity

**Example Use Cases**:
- Adding missing imports
- Implementing interface requirements
- Adding logging statements
- Inserting type annotations

---

#### 4. Move
**Concept**: Relocate code between files or directories while maintaining functionality.

**Implemented Tools**:
- `move.plan` - Plan moving of symbols, files, or directories (unified API)
- `workspace.apply_edit` - Execute move plans with atomic application
- `rename_file` - Legacy direct file move with automatic import updates
- `rename_directory` - Legacy direct directory move with automatic import updates

**Key Characteristics**:
- Import/export rewiring
- Reference updating via `ReferenceUpdater` service
- Namespace preservation
- Dependency tracking

**Example Use Cases**:
- Reorganizing project structure
- Creating feature modules
- Consolidating related code
- Migrating to new architecture

---

#### 5. Inline
**Concept**: Replace a reference with its value or implementation, reducing indirection.

**Implemented Tools**:
- `inline.plan` - Plan inlining of variables or functions (unified API)
- `workspace.apply_edit` - Execute inlining plans with atomic application

**Key Characteristics**:
- Scope-aware replacement
- Single/multiple occurrence handling
- Side effect preservation
- Type safety maintenance

**Example Use Cases**:
- Eliminating unnecessary variables
- Simplifying overly abstract code
- Performance optimization
- Removing dead abstractions

---

#### 6. Reorder
**Concept**: Change the sequence of code elements for clarity or convention compliance.

**Implemented Tools**:
- `reorder.plan` - Plan reordering of parameters, imports, etc. (unified API)
- `workspace.apply_edit` - Execute reordering plans with atomic application
- Code actions from `get_code_actions` - Quick fixes for import organization
- `format_document` - Format code according to style guidelines

**Key Characteristics**:
- Semantic preservation
- Convention awareness
- Dependency respect
- Style guide compliance

**Example Use Cases**:
- Organizing imports alphabetically
- Grouping related methods
- Following language conventions
- Improving readability

---

#### 7. Transform
**Concept**: Modify code structure while preserving behavior (control flow, data structures).

**Implemented Tools**:
- `transform.plan` - Plan code transformations (async conversion, etc.) (unified API)
- `workspace.apply_edit` - Execute transformation plans with atomic application
- `get_code_actions` - Provides transformation suggestions via LSP
- `format_document` - Apply formatting transformations
- `apply_edits` - Execute complex multi-file transformations

**Key Characteristics**:
- Behavior preservation
- Pattern recognition
- Idiomatic conversion
- Type preservation

**Example Use Cases**:
- Converting loops to functional patterns
- Modernizing syntax
- Applying design patterns
- Refactoring for performance

---

### Optional: Delete and Duplicate

#### Delete
**Implemented Tools**:
- `delete.plan` - Plan deletion of unused code (imports, dead code) (unified API)
- `workspace.apply_edit` - Execute deletion plans with atomic application
- `delete_file` - Remove files from the workspace (file-level)
- Code actions - Quick fixes for removing unused imports

**Key Characteristics**:
- Dependency detection
- Safe removal validation
- Cascade cleanup

---

#### Duplicate
**Concept**: Copy code snippets or structures.

**Implemented Through**:
- `read_file` + `write_file` combinations
- `apply_edits` for targeted duplication

**Key Characteristics**:
- Conflict avoidance
- Namespace collision detection
- Reference independence

---

## Pillar 2: Analysis Primitives

Analysis primitives provide the intelligence layer that informs refactoring decisions. These tools scan, measure, and report on code structure, quality, and relationships.

### Core Analysis Operations

#### 1. Linting
**Concept**: Enforce style and detect simple errors.

**Implemented Tools**:
- `get_diagnostics` - Real-time error and warning detection
- `get_code_actions` - Quick fixes for linting issues

**Key Characteristics**:
- Real-time feedback
- Configurable rule sets
- Actionable suggestions
- Integration with language servers

**Example Use Cases**:
- Enforcing code style
- Detecting common mistakes
- Ensuring type safety
- Maintaining code quality

---

#### 2. Complexity Analysis
**Concept**: Measure how complicated a function or module is (cyclomatic complexity, nesting depth).

**Implemented Tools**:
- `get_document_symbols` - Analyze code structure
- `prepare_call_hierarchy` - Understand call complexity
- `find_references` - Measure usage complexity

**Key Characteristics**:
- Quantitative metrics
- Threshold-based warnings
- Hotspot identification
- Refactoring prioritization

**Example Use Cases**:
- Identifying refactoring candidates
- Code review guidance
- Technical debt measurement
- Maintainability tracking

---

#### 3. Dead Code Detection
**Concept**: Find unused or unreachable code.

**Implemented Tools**:
- `analyze.dead_code` - Identify unused exports and functions
- `find_references` - Verify symbol usage
- `analyze.dependencies` - Detect unused imports

**Key Characteristics**:
- Whole-program analysis
- Export tracking
- Import validation
- Safe removal suggestions

**Example Use Cases**:
- Cleaning up legacy code
- Reducing bundle size
- Improving compile times
- Eliminating technical debt

---

#### 4. Code Smell Detection
**Concept**: Identify patterns suggesting poor structure.

**Implemented Tools**:
- `get_diagnostics` - Detect anti-patterns
- `get_code_actions` - Suggest improvements
- `analyze.dead_code` - Identify unused code smell

**Common Code Smells**:
- Long functions (extract function candidate)
- Duplicate code (extract to shared function)
- Large classes (split into modules)
- Deep nesting (flatten control flow)

**Key Characteristics**:
- Pattern recognition
- Heuristic-based detection
- Refactoring suggestions
- Context-aware analysis

---

#### 5. Dependency Analysis
**Concept**: Map out relationships between modules, functions, and files.

**Implemented Tools**:
- `analyze.dependencies` - Build dependency graphs
- `find_references` - Track symbol dependencies
- `prepare_call_hierarchy` - Analyze function call relationships
- `get_call_hierarchy_incoming_calls` / `get_call_hierarchy_outgoing_calls` - Detailed call graphs

**Key Characteristics**:
- Graph construction
- Circular dependency detection
- Impact analysis
- Layering validation

**Example Use Cases**:
- Refactoring impact assessment
- Architectural analysis
- Breaking circular dependencies
- Module boundary definition

---

## Primitive Composition

The power of this framework comes from composing primitives to achieve complex goals.

### Example: Safe Module Extraction

**Goal**: Extract a large file into multiple smaller modules.

**Primitive Sequence**:
1. **Analyze Dependencies** (`analyze.dependencies`) - Understand current structure
2. **Detect Complexity** (`get_document_symbols`) - Identify extraction candidates
3. **Extract Functions** (`extract.plan` + `workspace.apply_edit`) - Pull out logical units
4. **Move to New Files** (`move.plan` + `workspace.apply_edit`) - Create new module structure
5. **Update Imports** (automatic via unified API) - Maintain references
6. **Verify No Dead Code** (`analyze.dead_code`) - Ensure clean migration
7. **Format All Files** (`format_document`) - Apply consistent style

---

### Example: Performance Optimization Refactor

**Goal**: Optimize a slow function while maintaining behavior.

**Primitive Sequence**:
1. **Analyze Complexity** (`prepare_call_hierarchy`) - Identify hot paths
2. **Inline Hot Variables** (`inline.plan` + `workspace.apply_edit`) - Reduce overhead
3. **Extract Reusable Parts** (`extract.plan` + `workspace.apply_edit`) - Enable caching
4. **Verify References** (`find_references`) - Ensure no breaking changes
5. **Run Diagnostics** (`get_diagnostics`) - Check for introduced errors
6. **Transform Patterns** (`transform.plan` + `workspace.apply_edit`) - Apply optimization patterns

---

## The Unified Refactoring API Pattern

Codebuddy implements a consistent, safe `plan -> apply` pattern for all refactoring operations. This two-step approach enhances safety by allowing preview and validation before making changes.

### Two-Step Process

1. **Planning Phase** (`*.plan` commands)
   - **Always read-only** - Never modifies files
   - Generates a detailed refactoring plan
   - Includes checksums for affected files
   - Returns warnings about potential issues
   - Available for: `rename.plan`, `extract.plan`, `inline.plan`, `move.plan`, `reorder.plan`, `transform.plan`, `delete.plan`

2. **Application Phase** (`workspace.apply_edit`)
   - **Single execution command** for all refactoring types
   - Validates checksums to prevent stale edits
   - Atomic execution (all changes succeed or all rollback)
   - Optional validation command execution (e.g., `cargo check`)
   - Automatic rollback on failure

### Safety Features

**Checksum Validation**:
- Each plan includes SHA-256 hashes of files to be modified
- `workspace.apply_edit` verifies files haven't changed since plan creation
- Prevents applying stale plans to modified code

**Atomic Application**:
- All file changes succeed together or rollback together
- No partial application that leaves code in broken state
- Transaction-like semantics for multi-file refactorings

**Optional Validation**:
- Run build/test commands after applying changes
- Automatic rollback if validation fails
- Example: `{"validation": {"command": "cargo check", "timeout_seconds": 60}}`

### Example Workflow

```bash
# Step 1: Generate rename plan (read-only, safe to explore)
PLAN=$(codebuddy tool rename.plan '{
  "target": {
    "kind": "symbol",
    "path": "src/app.ts",
    "selector": { "position": { "line": 15, "character": 8 } }
  },
  "newName": "newUser"
}')

# Step 2: Inspect plan (optional)
echo "$PLAN" | jq '.edits | length'  # See number of changes
echo "$PLAN" | jq '.warnings'        # Check for warnings

# Step 3: Apply plan with validation
codebuddy tool workspace.apply_edit "{
  \"plan\": $PLAN,
  \"options\": {
    \"validate_checksums\": true,
    \"rollback_on_error\": true,
    \"validation\": {
      \"command\": \"npm test\",
      \"timeout_seconds\": 120
    }
  }
}"
```

### Benefits Over Legacy Tools

**Before (Legacy tools, now removed)**:
- Direct execution with no preview capability
- Limited rollback support
- Inconsistent safety features across tools
- Difficult to inspect changes before applying

**After (Unified API)**:
- Consistent `plan -> apply` pattern
- Preview changes before application
- Uniform checksum validation
- Centralized atomic execution
- Single apply command for all refactoring types

### Coexistence with Legacy Tools

The unified API coexists with legacy file/directory operations:
- **Use unified API** for: symbol renaming, code extraction, inlining, transformations
- **Use legacy tools** for: simple file operations (`rename_file`, `delete_file`)
- Legacy file tools may be migrated to unified API in future versions

---

## Design Principles

### 1. Atomicity
Each primitive represents a **single, focused operation**. This ensures:
- Clear semantics
- Easy testing
- Predictable composition
- Minimal side effects

### 2. Composability
Primitives **combine to solve complex problems**. This enables:
- Flexible workflows
- Reusable building blocks
- Incremental refactoring
- Custom automation

### 3. Language Independence
Primitives are **conceptually universal** across programming languages. This supports:
- Consistent user experience
- Plugin architecture
- Multi-language projects
- Transferable knowledge

### 4. Safety First
All primitives **preserve correctness**. This guarantees:
- No breaking changes
- Type safety preservation
- Reference integrity
- Atomic transactions

---

## Mapping to Codebuddy Tools

### Refactoring Primitives → MCP Tools

| Primitive | MCP Tools | Handler |
|-----------|-----------|---------|
| **Rename** | `rename.plan`, `workspace.apply_edit`, `rename_file`, `rename_directory` | RefactoringHandler, FileOpsHandler, WorkspaceHandler |
| **Extract** | `extract.plan`, `workspace.apply_edit`, `extract_module_to_package` | RefactoringHandler, WorkspaceHandler |
| **Inject/Insert** | `apply_edits`, `write_file`, code actions | EditingHandler, FileOpsHandler |
| **Move** | `move.plan`, `workspace.apply_edit`, `rename_file`, `rename_directory` | RefactoringHandler, FileOpsHandler, WorkspaceHandler |
| **Inline** | `inline.plan`, `workspace.apply_edit` | RefactoringHandler |
| **Reorder** | `reorder.plan`, `workspace.apply_edit`, `format_document`, code actions | RefactoringHandler, EditingHandler |
| **Transform** | `transform.plan`, `workspace.apply_edit`, `get_code_actions`, `apply_edits` | RefactoringHandler, EditingHandler |
| **Delete** | `delete.plan`, `workspace.apply_edit`, `delete_file`, code actions | RefactoringHandler, FileOpsHandler |

### Analysis Primitives → MCP Tools

| Primitive | MCP Tools | Handler |
|-----------|-----------|---------|
| **Linting** | `get_diagnostics`, `get_code_actions` | NavigationHandler, EditingHandler |
| **Complexity** | `get_document_symbols`, `prepare_call_hierarchy` | NavigationHandler |
| **Dead Code** | `analyze.dead_code`, `find_references`, `analyze.dependencies` | WorkspaceHandler, NavigationHandler |
| **Code Smells** | `get_diagnostics`, `get_code_actions` | NavigationHandler, EditingHandler |
| **Dependencies** | `analyze.dependencies`, `find_references`, call hierarchy tools | WorkspaceHandler, NavigationHandler |

---

## Future Primitive Extensions

As Codebuddy evolves, additional primitives may be added:

### Potential Refactoring Primitives
- **Merge** - Combine multiple functions/modules
- **Split** - Break one entity into multiple
- **Wrap** - Add abstraction layer (e.g., try/catch, logging)
- **Unwrap** - Remove abstraction layer

### Potential Analysis Primitives
- **Performance Profiling** - Runtime hotspot detection
- **Security Analysis** - Vulnerability scanning
- **Test Coverage** - Coverage gap identification
- **Documentation Quality** - Comment/doc completeness

---

## Related Documentation

- **[docs/tools/README.md](../tools/README.md)** - Complete MCP tool API reference
- **[ARCHITECTURE.md](overview.md)** - System architecture and design
- **[workflows.md](../development/workflows.md)** - Intent-based workflow automation
- **[contributing.md](../../contributing.md)** - Adding new tools and primitives

---

## Summary

Codebuddy's primitive-based architecture provides:

1. **Clear Mental Model** - Easy to understand tool capabilities
2. **Composable Operations** - Build complex workflows from simple parts
3. **Language Agnostic** - Universal concepts across programming languages
4. **Safety Guarantees** - Correctness-preserving transformations
5. **Extensible Design** - Easy to add new primitives

By organizing all 44+ MCP tools into these two pillars (Refactoring and Analysis), Codebuddy provides a complete foundation for AI-assisted code intelligence and transformation.
