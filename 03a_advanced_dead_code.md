# Advanced Dead Code Analysis

**Status:** Phase 1 Implemented ✅
**Goal:** Detect unused types, interfaces, constants, and other symbols beyond function-level dead code detection

---

## Implementation Status

### ✅ Phase 1 Complete (Merged: commit 7c9cfd18)

**What was implemented:**
- **Graph-based analysis infrastructure**: New `cb-analysis-deep-dead-code` crate using dependency graphs
- **LSP integration**: Workspace-wide symbol discovery via `workspace/symbol` and `find_references`
- **CLI command**: `codebuddy analyze dead-code` with `--include-public` flag
- **MCP tool**: `analyze.dead_code` with `kind: "deep"` parameter
- **Public/private filtering**: Conservative mode (default) excludes public symbols; aggressive mode includes them
- **Cross-file analysis**: Leverages LSP for tracking symbol usage across the workspace
- **Integration tests**: Basic test coverage for default and aggressive modes

**Architecture:**
- `analysis/cb-analysis-deep-dead-code/` - Core analysis engine
- `analysis/cb-analysis-common/src/graph.rs` - Shared dependency graph infrastructure
- `crates/cb-handlers/src/handlers/tools/analysis/dead_code.rs` - MCP handler integration with both regex-based (file scope) and LSP-based (workspace scope) implementations
- `apps/codebuddy/src/cli.rs` - CLI command implementation

**Current Capabilities:**
- Detects unused functions, classes, and other symbols via LSP
- Works across files within a workspace
- Respects public/private visibility (with configuration)
- Returns structured findings compatible with unified analysis API
- File-level analysis uses regex heuristics for: unused imports, unused symbols, unreachable code, unused parameters, unused types, unused variables

### 🚧 Phase 2 Planned (Future Work)

**Remaining from original proposal:**
- **AST-based type extraction**: Direct parsing of types, interfaces, constants, enums using language-specific AST parsers (`syn` for Rust, TypeScript compiler API) for more comprehensive detection
- **Generic/template handling**: Track both generic parameters and concrete instantiations
- **Enhanced symbol categorization**: Explicit detection of Type, Interface, Trait, Constant, Enum (currently relies on LSP's SymbolKind mapping)
- **Unused type parameters**: Detect generic parameters that are declared but never used
- **Exported symbol strategy**: More sophisticated heuristics beyond simple public/private (e.g., check if exported in package.json, Cargo.toml `pub use`, documented as public API)
- **Performance optimizations**: Symbol caching, incremental analysis, parallel file parsing (see "Performance" section)
- **Advanced filtering**: Min size thresholds, exclude patterns by path, symbol type filtering

**Gap Analysis:**

The current implementation provides a **solid foundation** for dead code detection using LSP as the source of truth for workspace-wide analysis. However, the original proposal envisioned more granular detection using direct AST parsing:

- **LSP approach (current)**: Relies on language server's symbol reporting - comprehensive for functions, classes, but may miss some categories like unused constants or type parameters. Excellent for cross-file analysis.
- **Regex heuristics (current)**: File-level analysis uses pattern matching for quick detection - good for common cases, may have false positives/negatives.
- **AST approach (future)**: Direct parsing gives complete control over symbol extraction and categorization - will enable detection of all symbol types mentioned in original proposal.

**See "Future Enhancements" section below for details.**

---

## Problem Statement

**Existing dead code detection is limited:**
- Most tools only detect unused functions
- Missing: unused types, interfaces, constants, enums, traits
- Can't track usage through generics/templates
- No cross-file analysis

**Real-world impact:**
- Production codebases contain 20-40% dead code
- Accumulated technical debt from features removed but code left behind
- Maintenance burden: developers waste time reading/updating unused code
- Build times increase with unused dependencies
- Code review overhead (reviewing dead code)

**Manual detection is unreliable:**
- Hard to distinguish "currently unused" from "intentionally exported for future use"
- Generics make usage tracking complex
- Cross-file references are easy to miss

## Solution Overview

Extend existing function-level dead code detection to cover:
1. **Types/Structs/Classes** - unused data structures
2. **Interfaces/Traits** - unused abstractions
3. **Constants/Enums** - unused configuration values
4. **Type Parameters** - unused generics
5. **Exported Symbols** - unused public APIs

**Output:**
```
Found 42 unused symbols:

Types (12):
  src/models/legacy-user.ts:15 - interface LegacyUser
  src/types/old-config.ts:8 - type OldConfig
  crates/cb-types/src/deprecated.rs:22 - struct DeprecatedError

Interfaces/Traits (5):
  src/interfaces/unused.ts:3 - interface UnusedService
  crates/cb-protocol/src/old.rs:10 - trait OldProtocol

Constants (18):
  src/config/constants.ts:45 - const MAX_RETRIES_V1
  crates/cb-core/src/constants.rs:12 - const DEPRECATED_TIMEOUT

Enums (7):
  src/types/status.ts:20 - enum ObsoleteStatus
  crates/cb-types/src/enums.rs:5 - enum LegacyMode

Total: 42 unused symbols (estimate: 3,200 lines can be deleted)
```

## Implementation

**Prerequisite:** Build on the shared dependency and call graph infrastructure so symbol reachability queries stay consistent with other analyses.

### 1. Symbol Extraction

**For each language, extract:**
- Type definitions (struct, class, interface, type alias)
- Trait/protocol definitions
- Constant declarations
- Enum definitions
- Generic type parameters

**Rust Example:**
```rust
// Extract from AST
pub struct SymbolExtractor;

impl SymbolExtractor {
    pub fn extract_types(&self, source: &str) -> Vec<TypeSymbol> {
        let ast: syn::File = syn::parse_file(source)?;
        let mut types = Vec::new();

        for item in ast.items {
            match item {
                Item::Struct(s) => types.push(TypeSymbol {
                    name: s.ident.to_string(),
                    kind: SymbolKind::Struct,
                    location: get_location(&s),
                    visibility: get_visibility(&s.vis),
                }),
                Item::Enum(e) => types.push(TypeSymbol {
                    name: e.ident.to_string(),
                    kind: SymbolKind::Enum,
                    location: get_location(&e),
                    visibility: get_visibility(&e.vis),
                }),
                Item::Type(t) => types.push(TypeSymbol {
                    name: t.ident.to_string(),
                    kind: SymbolKind::TypeAlias,
                    location: get_location(&t),
                    visibility: get_visibility(&t.vis),
                }),
                Item::Trait(tr) => types.push(TypeSymbol {
                    name: tr.ident.to_string(),
                    kind: SymbolKind::Trait,
                    location: get_location(&tr),
                    visibility: get_visibility(&tr.vis),
                }),
                Item::Const(c) => types.push(TypeSymbol {
                    name: c.ident.to_string(),
                    kind: SymbolKind::Constant,
                    location: get_location(&c),
                    visibility: get_visibility(&c.vis),
                }),
                _ => {}
            }
        }

        types
    }
}
```

**TypeScript Example:**
```typescript
// Using TypeScript compiler API
import * as ts from 'typescript';

function extractTypes(sourceFile: ts.SourceFile): TypeSymbol[] {
    const types: TypeSymbol[] = [];

    function visit(node: ts.Node) {
        if (ts.isInterfaceDeclaration(node)) {
            types.push({
                name: node.name.text,
                kind: 'interface',
                location: getLocation(node),
                exported: hasExportModifier(node),
            });
        }
        if (ts.isTypeAliasDeclaration(node)) {
            types.push({
                name: node.name.text,
                kind: 'type',
                location: getLocation(node),
                exported: hasExportModifier(node),
            });
        }
        if (ts.isEnumDeclaration(node)) {
            types.push({
                name: node.name.text,
                kind: 'enum',
                location: getLocation(node),
                exported: hasExportModifier(node),
            });
        }

        ts.forEachChild(node, visit);
    }

    visit(sourceFile);
    return types;
}
```

### 2. Usage Tracking

**Track all locations where symbols are used:**
- Type annotations (`let user: User`)
- Generic parameters (`Vec<User>`, `List<Order>`)
- Implements clauses (`struct Foo implements Bar`)
- Inheritance (`class Child extends Parent`)
- Type assertions/casts (`value as MyType`)
- Import statements (`import { User } from './types'`)

**Implementation:** Use a fully qualified symbol identity (`QualifiedSymbolId = { module_path, symbol_name, signature }`) so that similarly named symbols in different scopes do not collide.
```rust
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct QualifiedSymbolId {
    pub module: ModulePath,
    pub name: String,
    pub signature: Option<String>, // e.g., generics or type params
}

pub struct UsageTracker {
    /// Symbol definitions: fully-qualified id -> definition metadata
    definitions: HashMap<QualifiedSymbolId, SymbolDefinition>,

    /// Symbol usages: fully-qualified id -> [usage locations]
    usages: HashMap<QualifiedSymbolId, Vec<UsageLocation>>,
}

impl UsageTracker {
    /// Track a type usage in source code
    pub fn track_usage(&mut self, symbol: QualifiedSymbolId, location: Location, context: UsageContext) {
        self.usages
            .entry(symbol.clone())
            .or_default()
            .push(UsageLocation { location, context });
    }

    /// Find all symbols with zero usages
    pub fn find_unused(&self) -> Vec<QualifiedSymbolId> {
        self.definitions
            .keys()
            .filter(|qid| {
                self.usages.get(*qid).map_or(true, |uses| uses.is_empty())
            })
            .cloned()
            .collect()
    }
}

pub enum UsageContext {
    TypeAnnotation,      // let x: Type
    GenericParameter,    // Vec<Type>
    Implements,          // struct Foo: Type
    Extends,             // class Foo extends Type
    Import,              // import { Type } from ...
    TypeAssertion,       // value as Type
    FunctionParameter,   // fn foo(x: Type)
    FunctionReturn,      // fn foo() -> Type
}
```

**Type resolution:** Hook into compiler-grade language services to resolve `QualifiedSymbolId`s (e.g., TypeScript's `Program`, rust-analyzer's salsa queries, mypy daemon). Plain text search remains a fallback only for languages without richer support.

### 3. Generic/Template Handling

**Challenge:** Generics can hide usage.

**Example (Rust):**
```rust
// Definition
pub struct Container<T> {
    value: T,
}

// Usage (T is used, but what concrete types?)
let c = Container { value: 42 };  // T = i32
let d = Container { value: "hi" }; // T = &str
```

**Solution:** Track both generic and concrete usage:
```rust
pub struct GenericUsage {
    /// Generic type parameter name
    parameter: String,

    /// Concrete types it was instantiated with
    instantiations: Vec<String>,
}
```

**Mark as unused only if:**
- Generic type is never instantiated, OR
- All instantiations are with unused types

### 4. Exported Symbol Handling

**Challenge:** Public APIs might be unused internally but used by external consumers.

**Strategy:**
1. **Default: Conservative** - Don't mark public exports as unused
2. **Optional: Aggressive** - Mark as unused unless:
   - Documented as public API
   - Used in published examples
   - Listed in package exports

**Configuration:**
```json
{
  "dead_code_analysis": {
    "check_public_exports": false,  // Safe default
    "exclude_documented": true,     // Keep items with doc comments
    "exclude_patterns": [
      "src/public-api/**",          // Explicit public API directory
      "**/index.ts"                 // Entry points
    ]
  }
}
```

### 5. Cross-File Analysis

**Leverage dependency graph:**
- Build call graph (from platform foundation)
- Track symbol usage across module boundaries
- Handle re-exports (`pub use other_mod::Type`)

**Example:**
```rust
// crates/cb-types/src/lib.rs
pub struct PublicType;  // Exported

// crates/cb-core/src/lib.rs
use cb_types::PublicType;  // Usage

// Result: PublicType is used (via cross-crate import)
```

## Language Support

### Rust
- Extract: structs, enums, traits, type aliases, constants
- Track: type annotations, generic bounds, impl blocks
- Special: handle `#[derive]` (usage), `#[cfg]` (conditional compilation)

### TypeScript/JavaScript
- Extract: interfaces, types, enums, classes, constants
- Track: type annotations, generic parameters, extends/implements
- Special: handle declaration files (`.d.ts`), ambient declarations

### Go
- Extract: structs, interfaces, constants, type aliases
- Track: type annotations, embedded types, interface satisfaction
- Special: Go doesn't have explicit "implements", track structural typing

### Python
- Extract: classes, TypedDict, Protocol, constants, enums (enum.Enum)
- Track: type hints, inheritance, protocol implementation
- Special: runtime usage (harder to detect, require type stubs)

## MCP Tool Definition

```json
{
  "name": "analyze_dead_code",
  "description": "Find unused types, interfaces, constants, and other symbols",
  "inputSchema": {
    "type": "object",
    "properties": {
      "scope": {
        "type": "string",
        "enum": ["workspace", "directory", "file"],
        "description": "Scope of analysis"
      },
      "path": {
        "type": "string",
        "description": "Path to analyze (for directory/file scope)"
      },
      "symbol_types": {
        "type": "array",
        "items": {
          "type": "string",
          "enum": ["types", "interfaces", "constants", "enums", "all"]
        },
        "default": ["all"],
        "description": "Which symbol types to check"
      },
      "include_public": {
        "type": "boolean",
        "default": false,
        "description": "Check public exports (aggressive mode)"
      },
      "min_size": {
        "type": "integer",
        "description": "Minimum lines to report (filter small symbols)"
      }
    }
  }
}
```

**Example usage:**
```json
{
  "method": "tools/call",
  "params": {
    "name": "analyze_dead_code",
    "arguments": {
      "scope": "workspace",
      "symbol_types": ["types", "constants"],
      "include_public": false
    }
  }
}
```

## CLI Command

```bash
# Find all dead code
codebuddy analyze dead-code

# Only check types
codebuddy analyze dead-code --types

# Only check constants
codebuddy analyze dead-code --constants

# Aggressive mode (include public exports)
codebuddy analyze dead-code --aggressive

# Output with estimated line savings
codebuddy analyze dead-code --show-lines

# Generate deletion script
codebuddy analyze dead-code --generate-script > delete-dead-code.sh

# Dry run (show what would be deleted)
codebuddy analyze dead-code --dry-run
```

## Output Format

### Console (Human-Readable)
```
Dead Code Analysis Results
==========================

Types (12 unused):
  ✗ src/models/legacy-user.ts:15-45 (30 lines)
    interface LegacyUser
    Last used: 6 months ago (git blame)
    Suggested action: Delete

  ✗ src/types/old-config.ts:8-25 (17 lines)
    type OldConfig
    Suggested action: Delete

Constants (18 unused):
  ✗ src/config/constants.ts:45 (1 line)
    const MAX_RETRIES_V1 = 5
    Suggested action: Delete

Summary:
  Total symbols: 156
  Unused: 42 (26.9%)
  Estimated deletable lines: 3,200
  Estimated file size reduction: 12.4%
```

### JSON (Machine-Readable)
```json
{
  "unused_symbols": [
    {
      "name": "LegacyUser",
      "kind": "interface",
      "location": {
        "file": "src/models/legacy-user.ts",
        "start_line": 15,
        "end_line": 45
      },
      "lines": 30,
      "visibility": "public",
      "last_used": "2024-04-01T00:00:00Z",
      "suggested_action": "delete"
    }
  ],
  "summary": {
    "total_symbols": 156,
    "unused_count": 42,
    "unused_percentage": 26.9,
    "deletable_lines": 3200,
    "size_reduction_percentage": 12.4
  }
}
```

## AI-Assisted Cleanup

**When integrated with MCP, AI can:**

1. **Confirm deletion safety:**
   - Check git history for last usage
   - Search codebase for string references (comments, docs)
   - Verify no dynamic usage (reflection, `eval()`)

2. **Generate deletion PR:**
   - Create branch
   - Delete dead symbols
   - Update imports
   - Generate commit message with context

3. **Prioritize deletions:**
   - Start with private symbols (safer)
   - Delete low-value items first (constants, small types)
   - Defer complex deletions (heavily generic types)

**Example conversation:**
```
User: "Clean up dead code in src/models/"
Assistant: [calls analyze_dead_code]
"Found 12 unused types in src/models/:
- LegacyUser (30 lines, unused for 6 months)
- OldConfig (17 lines, unused for 3 months)
- DeprecatedStatus (5 lines, unused for 1 year)

Safe to delete all 12 (no references found).
Estimated deletion: 245 lines.

Shall I create a PR to remove them?"
```

## Performance

### Benchmark Targets

| Codebase Size | Symbols | Analysis Time | Memory |
|---------------|---------|---------------|--------|
| Small (1k LOC) | 200 | <500ms | <20MB |
| Medium (10k LOC) | 2k | <5s | <100MB |
| Large (100k LOC) | 20k | <30s | <500MB |

### Optimization Strategies

**Symbol Extraction:**
- Parallel file parsing (Rayon)
- Cache extracted symbols (invalidate on file change)
- Skip test files (optional)

**Usage Tracking:**
- Precompute symbol index (name -> definition)
- Use Aho-Corasick for multi-pattern search
- Skip comments and strings (unlikely to be real usage)

**Incremental Analysis:**
- Reuse previous analysis results
- Only reanalyze changed files and their dependents
- Mark symbols as "maybe unused" on first pass, confirm on second

## Success Metrics

**Accuracy:**
- [ ] Zero false positives (all reported symbols are truly unused)
- [ ] >95% recall (find most unused symbols, miss <5%)
- [ ] Handles generics correctly (no false positives from templates)

**Performance:**
- [ ] Analyzes 100k LOC in <30 seconds
- [ ] Incremental analysis after 1-file change in <3 seconds
- [ ] Memory usage <500MB for 100k LOC

**Usability:**
- [ ] Clear output showing deletable lines
- [ ] AI assistant can auto-generate deletion PRs
- [ ] Safe mode (conservative) has zero production incidents

## Future Enhancements

**Git Integration:**
- Show when symbol was last used (git blame)
- Track symbol age (how long it's been unused)
- Suggest deletion confidence based on history

**Code Coverage Integration:**
- Cross-reference with test coverage
- Mark symbols as "tested but unused" (high confidence for deletion)

**Dependency Analysis:**
- Find entire dependency chains that are unused
- "This type is only used by this function, which is also unused"

**Interactive Mode:**
- Review each unused symbol with AI
- Accept/reject deletions
- Generate incremental PRs

**Trend Tracking:**
- Dashboard showing dead code over time
- Alert when dead code percentage increases
- Gamify: "Your team deleted 5,000 lines this month!"

---

## References

**Algorithms:**
- Reachability analysis: [Wikipedia](https://en.wikipedia.org/wiki/Reachability)
- Call graph construction: Used for tracing usage

**Tools for Comparison:**
- `ts-prune` (TypeScript) - [npm](https://www.npmjs.com/package/ts-prune)
- `cargo-udeps` (Rust) - Finds unused dependencies
- `vulture` (Python) - [PyPI](https://pypi.org/project/vulture/)
- `UCDetector` (Java) - Eclipse plugin

**Rust Libraries:**
- `syn` - Rust AST parsing
- `swc` - TypeScript/JavaScript AST parsing
- `tree-sitter` - Multi-language parsing
