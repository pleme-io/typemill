# Analysis Platform Foundation

**Status:** Proposed
**Goal:** Build the foundational infrastructure for whole-program static analysis capabilities

---

## Overview

Extend Codebuddy beyond real-time code intelligence (LSP) into **batch whole-program analysis** - deep insights that require cross-file understanding and are too expensive for interactive tools.

## Why This Matters

**Current limitations of LSP/linters:**
- Single-file context only
- No call graph analysis
- Can't trace data flow across modules
- Miss architectural issues

**What whole-program analysis enables:**
- Cross-file dependency tracking
- Call graph construction and traversal
- Data flow analysis across module boundaries
- Architecture-level insights

## Core Infrastructure Components

### 1. Dependency Graph Construction

**Purpose:** Build and maintain a graph of module/file dependencies across the codebase.

**Capabilities:**
- Parse import/use/require statements from all source files
- Track both direct and transitive dependencies
- Support language-specific module systems (ES6, CommonJS, Python imports, Rust crates, etc.)
- Handle dynamic imports and conditional dependencies

**Data Structure:**
```rust
pub struct DependencyGraph {
    /// Nodes: file paths or module identifiers
    nodes: HashMap<NodeId, ModuleNode>,

    /// Edges: import relationships
    edges: HashMap<NodeId, Vec<Dependency>>,

    /// Reverse index for "what imports this module?"
    reverse_edges: HashMap<NodeId, Vec<NodeId>>,
}

pub struct ModuleNode {
    pub id: NodeId,
    pub path: PathBuf,
    pub language: Language,
    pub exports: Vec<Symbol>,
}

pub struct Dependency {
    pub source: NodeId,
    pub target: NodeId,
    pub kind: DependencyKind,  // Import, Export, Re-export
    pub symbols: Vec<String>,  // Specific imported symbols
}
```

**Implementation:**
- Reuse existing AST parsing from `cb-ast`
- Extract import statements via language plugins
- Build graph incrementally as files are analyzed
- Persist graph to disk for fast startup

### 2. Call Graph Construction

**Purpose:** Map function/method calls across the entire codebase.

**Capabilities:**
- Track function definitions and their call sites
- Handle method calls on objects/structs
- Support indirect calls (function pointers, callbacks)
- Integrate with LSP for accurate type resolution

**Data Structure:**
```rust
pub struct CallGraph {
    /// Function definitions
    functions: HashMap<FunctionId, FunctionNode>,

    /// Call edges: caller -> callee
    calls: HashMap<FunctionId, Vec<CallSite>>,

    /// Reverse: callee -> callers
    callers: HashMap<FunctionId, Vec<FunctionId>>,
}

pub struct FunctionNode {
    pub id: FunctionId,
    pub name: String,
    pub location: Location,
    pub signature: FunctionSignature,
    pub visibility: Visibility,  // Public, private, internal
}

pub struct CallSite {
    pub caller: FunctionId,
    pub callee: FunctionId,
    pub location: Location,
    pub call_type: CallType,  // Direct, Indirect, Virtual
}
```

**Implementation:**
- Build language-specific indexers that operate on parsed ASTs or compiler APIs (e.g., rust-analyzer query engine, TypeScript compiler program)
- Use LSP responses only as an optional fallback for languages without richer APIs, with rate-limiting safeguards
- Handle polymorphism via type information resolved by the indexers
- Support cross-language calls (FFI, RPC) through dedicated plugins

### 3. Incremental Update Strategy

**Purpose:** Update graphs efficiently when files change, without full rebuild.

**Strategy:**
- Track file modification timestamps
- Invalidate affected subgraphs on file change
- Rebuild only modified nodes and their dependents
- Use persistent storage for unchanged portions

**Invalidation Rules:**
```rust
pub struct GraphInvalidation {
    /// Files modified since last analysis
    modified_files: HashSet<PathBuf>,

    /// Nodes that need reanalysis
    invalidated_nodes: HashSet<NodeId>,

    /// Edges that need recomputation
    invalidated_edges: HashSet<EdgeId>,
}

impl GraphInvalidation {
    /// Mark a file as modified and invalidate dependent nodes
    pub fn invalidate_file(&mut self, path: &Path, graph: &mut DependencyGraph) {
        // Invalidate the node itself
        if let Some(node_id) = graph.find_node_by_path(path) {
            self.invalidated_nodes.insert(node_id);

            // Invalidate all nodes that depend on this one
            for dependent in graph.get_dependents(node_id) {
                self.invalidated_nodes.insert(dependent);
            }
        }
    }
}
```

### 4. Graph Persistence & Caching

**Purpose:** Store computed graphs to disk for fast restart and incremental updates.

**Format:**
- Use MessagePack or bincode for efficient binary serialization
- Store metadata: build timestamp, watched file list, language versions
- Maintain a per-file hash index so only touched files require recomputation
- Validate cache on load with a fast metadata check, then fall back to hash verification for changed files

**Cache Structure:**
```
.codebuddy/cache/
├── dependency_graph.bin
├── call_graph.bin
├── metadata.json
└── file_index.json  # per-file digests for targeted validation
```

**Cache Validation:**
```rust
pub struct CacheMetadata {
    pub build_timestamp: SystemTime,
    pub watched_files: Vec<PathBuf>,
    pub language_plugin_versions: HashMap<String, String>,
    pub metadata_hash_index: HashMap<PathBuf, FileDigest>,
}

impl Cache {
    pub fn is_valid(&self) -> bool {
        // Fast path: rely on file metadata deltas
        if files_unchanged_since(&self.watched_files, self.build_timestamp) {
            return true;
        }

        // Slow path: verify hashed chunks for modified files only
        diff_and_verify_changed_files(&self.watched_files, &self.metadata_hash_index)
    }
}
```

### 5. Analysis Query API

**Purpose:** Provide a high-level API for analysis algorithms to query graphs.

**Example API:**
```rust
pub trait GraphQuery {
    /// Find all paths from source to target (for circular dependency detection)
    fn find_paths(&self, source: NodeId, target: NodeId) -> Vec<Vec<NodeId>>;

    /// Get all transitive dependencies of a node
    fn transitive_dependencies(&self, node: NodeId) -> HashSet<NodeId>;

    /// Find strongly connected components (for cycle detection)
    fn strongly_connected_components(&self) -> Vec<Vec<NodeId>>;

    /// Get all call sites for a function (for impact analysis)
    fn get_call_sites(&self, function: FunctionId) -> Vec<CallSite>;

    /// Find all functions reachable from an entry point
    fn reachable_functions(&self, entry: FunctionId) -> HashSet<FunctionId>;
}
```

## Delivery Mechanisms

### MCP Tool Integration

**Design:** Expose analysis capabilities as MCP tools callable from AI assistants.

**Example Tools:**
- `analyze_dependencies` - Build dependency graph for current project
- `query_call_graph` - Find callers or callees of a function
- `invalidate_cache` - Force rebuild of analysis graphs

**Benefits:**
- AI assistants can trigger analyses on-demand
- Results are streamed back to conversation
- AI can suggest fixes based on analysis results

### CLI Commands

**Design:** Provide standalone CLI for CI/CD integration.

**Example Commands:**
```bash
# Build dependency graph
codebuddy analyze build-graph

# Query graph
codebuddy analyze query --find-cycles
codebuddy analyze query --callers "myFunction"

# Invalidate cache
codebuddy analyze invalidate

# Export graph (for visualization)
codebuddy analyze export --format dot > graph.dot
```

### CI/CD Integration

**GitHub Actions Example:**
```yaml
- name: Run Codebuddy Analysis
  run: |
    codebuddy analyze build-graph
    codebuddy analyze query --find-cycles --fail-on-cycles
```

**Pre-commit Hook:**
```bash
#!/bin/bash
# .git/hooks/pre-commit
codebuddy analyze build-graph --incremental
codebuddy analyze query --find-cycles --quiet
```

## Language Support

### Cross-Language Strategy

**Approach:** Leverage existing language plugins for parsing, but build language-agnostic graph representation.

**Per-Language Requirements:**
- Parse import/export statements → Dependency graph
- Parse function definitions and calls → Call graph
- Provide symbol resolution → Accurate linking

**Supported Languages (Initial):**
- **Rust:** Full support (use existing `cb-lang-rust` plugin)
- **TypeScript/JavaScript:** Full support (use existing `cb-lang-typescript` plugin)
- **Go:** Full support (use existing `cb-lang-go` plugin)
- **Python:** Full support (use existing `cb-lang-python` plugin)

**Cross-Language Boundaries:**
- FFI calls (Rust ↔ C)
- RPC/gRPC interfaces (TypeScript ↔ Go)
- Subprocess invocations (Python ↔ shell scripts)

### Language Plugin Interface

**Extension to `LanguagePlugin` trait:**
```rust
pub trait LanguagePlugin {
    // ... existing methods ...

    /// Extract dependency information from source
    fn extract_dependencies(&self, source: &str) -> Result<Vec<ModuleDependency>>;

    /// Extract function call information
    fn extract_calls(&self, source: &str) -> Result<Vec<FunctionCall>>;

    /// Resolve a symbol to its definition
    fn resolve_symbol(&self, symbol: &str, context: &Path) -> Result<SymbolLocation>;
}
```

## Performance Considerations

### Graph Construction Performance

**Target:** Analyze 100k LOC codebase in <10 seconds (cold start), <2 seconds (incremental).

**Optimizations:**
- Parallel file parsing (use Rayon)
- Lazy evaluation (build subgraphs on-demand)
- Bloom filters for "does this node exist?" queries
- Pre-compute frequently used queries (SCCs, transitive closures)

### Memory Usage

**Target:** Keep full graph in memory for <500MB (100k LOC codebase).

**Strategies:**
- Use interned strings for identifiers
- Store only essential data in hot path
- Offload detailed AST to disk, reference by ID
- Use compact data structures (petgraph, id-arena)

## Graph Algorithms

### Core Algorithms Needed

**Cycle Detection (Tarjan's Algorithm):**
- Find strongly connected components (SCCs)
- O(V + E) time complexity
- Used by: Circular dependency detection

**Transitive Closure:**
- Compute "what can reach what?"
- Warshall's algorithm or DFS-based
- Used by: Dead code analysis, impact analysis

**Shortest Path (Dijkstra/BFS):**
- Find shortest dependency chain
- Used by: Explaining why X depends on Y

**Call Tree Traversal:**
- DFS from entry points
- Mark reachable functions
- Used by: Dead code detection

### Algorithm Library

**Reuse existing Rust crates:**
- `petgraph` - Graph data structures and algorithms
- `rustc-hash` - Fast hashing for graph nodes
- `rayon` - Parallel iteration

## Data Model

### Graph Schema

**Nodes:**
```rust
pub enum GraphNode {
    Module(ModuleNode),
    Function(FunctionNode),
    Type(TypeNode),
    Constant(ConstantNode),
}
```

**Edges:**
```rust
pub enum GraphEdge {
    Import { from: NodeId, to: NodeId, symbols: Vec<String> },
    Call { caller: FunctionId, callee: FunctionId, location: Location },
    TypeDependency { user: TypeId, dependency: TypeId },
    DataFlow { source: NodeId, sink: NodeId },
}
```

**Metadata:**
```rust
pub struct GraphMetadata {
    pub build_time: SystemTime,
    pub total_nodes: usize,
    pub total_edges: usize,
    pub languages: HashSet<Language>,
    pub entry_points: Vec<NodeId>,
}
```

## Testing Strategy

### Unit Tests
- Graph construction from mock imports
- Incremental update correctness
- Cache invalidation logic
- Algorithm correctness (cycle detection, etc.)

### Integration Tests
- Full graph construction on real codebases
- Performance benchmarks (time, memory)
- Cross-language graph accuracy
- Cache hit/miss ratios

### Test Fixtures
```
integration-tests/fixtures/analysis/
├── circular-deps/         # Intentional cycles
├── dead-code/            # Unused functions/types
├── large-codebase/       # Performance test (10k+ files)
└── cross-language/       # Rust calling Python, etc.
```

## Success Metrics

**Performance:**
- [ ] Build dependency graph for 100k LOC in <10s
- [ ] Incremental rebuild after 1-file change in <2s
- [ ] Memory usage <500MB for 100k LOC graph

**Accuracy:**
- [ ] Dependency graph matches manual inspection (100% precision)
- [ ] Call graph includes >95% of actual call sites
- [ ] Cache invalidation has zero false negatives (no stale data)

**Usability:**
- [ ] MCP tools integrate seamlessly with AI assistants
- [ ] CLI commands work in CI/CD pipelines
- [ ] Graph export visualizes correctly in Graphviz

## Future Enhancements

**Data Flow Analysis:**
- Track variable assignments across functions
- Taint analysis for security

**Type Dependency Graph:**
- Track which types depend on which
- Impact analysis for type changes

**Cross-Repository Analysis:**
- Multi-repo dependency graphs
- Monorepo subgraph partitioning

**Distributed Graph Storage:**
- Store graph in database (PostgreSQL, SQLite)
- Enable team-wide sharing of analysis results

---

## References

**Graph Algorithms:**
- Tarjan's SCC algorithm: [Wikipedia](https://en.wikipedia.org/wiki/Tarjan%27s_strongly_connected_components_algorithm)
- Call graph construction: [LLVM's CallGraph](https://llvm.org/doxygen/classllvm_1_1CallGraph.html)

**Rust Crates:**
- `petgraph` - Graph data structures: [docs.rs](https://docs.rs/petgraph)
- `rayon` - Parallel iterators: [docs.rs](https://docs.rs/rayon)
- `dashmap` - Concurrent HashMap: [docs.rs](https://docs.rs/dashmap)

**Industry Tools (for comparison):**
- Sourcetrail - Interactive code visualization
- CodeScene - Behavioral code analysis
- Structure101 - Architecture analysis
