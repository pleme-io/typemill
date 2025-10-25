# cb-ast

AST parsing and code analysis crate for TypeMill.

## Overview

Provides TypeScript/JavaScript AST parsing using SWC, dependency graph analysis, code complexity metrics, and refactoring utilities.

## Features

### TypeScript/JavaScript Parsing (SWC)
- Production-grade parsing via `swc_ecma_parser` v24
- ES modules, CommonJS, dynamic imports, type-only imports
- Graceful fallback to regex parsing for malformed code
- Full AST traversal with `swc_ecma_visit`

### Code Complexity Analysis
- Cyclomatic complexity (control flow branches)
- Cognitive complexity (human understanding difficulty)
- Per-function metrics with detailed breakdowns
- Supports TypeScript, JavaScript, Python, Rust, Go

### Dependency Graph Analysis
- Project-wide import/dependency graphs
- Circular dependency detection
- Import relationship analysis
- Uses `petgraph` for graph operations

### Refactoring Support
- AST-powered symbol renaming
- Import path updates across files
- Edit plan generation for refactoring operations
- Thread-safe caching with `dashmap`

## Language Support

**TypeScript/JavaScript**: Native SWC parser in this crate

**Other Languages**: Handled by language plugins in `crates/languages/`:
- Python → `cb-lang-python`
- Go → `cb-lang-go`
- Rust → `cb-lang-rust`

See [crates/languages/README.md](../languages/README.md) for plugin details.

## API

### Import Analysis

```rust
use cb_ast::build_import_graph;

// Parse source and build import graph
let graph = build_import_graph(source, path)?;
```

### Dependency Graphs

```rust
use cb_ast::build_dependency_graph;

// Build project-wide dependency graph
let dep_graph = build_dependency_graph(&import_graphs);

// Detect circular dependencies
if dep_graph.has_cycles() { /* ... */ }
```

### Complexity Analysis

```rust
use cb_ast::complexity::analyze_complexity;

// Get complexity metrics for a file
let metrics = analyze_complexity(source, language)?;

for func in metrics.functions {
    println!("{}: cyclomatic={}, cognitive={}",
        func.name, func.cyclomatic_complexity, func.cognitive_complexity);
}
```

### Refactoring

```rust
use cb_ast::plan_refactor;

// Generate edit plan for refactoring
let plan = plan_refactor(&intent_spec, file_path)?;
```

## Implementation

- **parser.rs**: Import graph building with SWC
- **complexity.rs**: Code complexity analysis
- **refactoring.rs**: AST-powered refactoring operations
- **dependency_graph.rs**: Dependency analysis with petgraph

Parser version: `0.3.0-swc`

## Testing

```bash
cargo test -p cb-ast
```

## Documentation

For complete architecture details, see [docs/architecture/overview.md](../../docs/architecture/overview.md).
