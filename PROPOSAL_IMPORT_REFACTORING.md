# Proposal: Consolidate Import Support Logic Using cb-lang-common

**Author**: Analysis Bot
**Date**: 2025-10-08
**Status**: Proposal
**Priority**: High
**Estimated Effort**: 10-15 hours total

---

## Executive Summary

All 5 language plugins (Rust, Python, Go, TypeScript, Swift) currently duplicate significant import handling logic. While `cb-lang-common` provides tested utilities for common import operations, **they are barely being used**. This proposal outlines a refactoring initiative to consolidate duplicated code, reduce complexity, and establish sustainable patterns for future language plugins.

### Key Findings

| Language | Import Support SLOC | Current Complexity | cb-lang-common Usage |
|----------|--------------------|--------------------|---------------------|
| Rust     | 262 lines          | 98 avg cognitive   | Minimal (ImportGraphBuilder only) |
| Python   | 424 lines          | 204 avg cognitive  | Minimal (LineExtractor only) |
| Go       | 327 lines          | 132 avg cognitive  | Moderate (SubprocessAstTool) |
| TypeScript | 524 lines        | 214 peak cognitive | Minimal (LineExtractor only) |
| Swift    | 173 lines          | 28 avg cognitive   | Minimal (ErrorBuilder only) |
| **Total** | **1,710 lines**   | **~135 avg**       | **<5% utilization** |

### Impact Potential

- **Estimated Code Reduction**: 400-600 lines (25-35% of current import support code)
- **Complexity Reduction**: 30-40% reduction in cognitive complexity
- **Future ROI**: Every new language plugin gets 40% less code to write
- **Bug Surface**: Centralized import utilities mean bugs fixed once benefit all languages

---

## Problem Statement

### 1. Massive Code Duplication

Each language plugin reimplements similar patterns:

**Pattern: Parse "X as Y" alias**
- ❌ Python: Custom parsing in `contains_import()` lines 118-124
- ❌ Go: String splitting logic for aliases
- ❌ TypeScript: Regex/string parsing for ES6 imports
- ✅ **Available but unused**: `cb_lang_common::import_parsing::parse_import_alias()`

**Pattern: Detect external vs relative imports**
- ❌ Python: Custom logic to detect relative imports (`.`, `..`)
- ❌ Go: Package detection logic
- ❌ TypeScript: Complex path resolution for `./`, `../`, `@/` patterns
- ✅ **Available but unused**: `cb_lang_common::import_parsing::ExternalDependencyDetector`

**Pattern: Normalize import paths**
- ❌ Python: Manual quote stripping in multiple functions
- ❌ TypeScript: Quote normalization scattered throughout
- ❌ Go: Import path cleaning
- ✅ **Available but unused**: `cb_lang_common::import_parsing::normalize_import_path()`

**Pattern: Extract package names from scoped imports**
- ❌ TypeScript: Custom logic for `@types/node`, `@scope/package`
- ❌ Go: Domain-based package extraction (`github.com/user/repo`)
- ✅ **Available but unused**: `cb_lang_common::import_parsing::extract_package_name()`

**Pattern: Path-to-module conversion**
- ❌ Python: `path_to_python_module()` function (31 lines, lines 278-308)
- ❌ TypeScript: Similar file path → module path logic
- ✅ **Available but unused**: `cb_lang_common::io::file_path_to_module()`

### 2. Inconsistent Behavior Across Languages

Each plugin handles edge cases differently:
- **Empty files**: Python checks, Swift doesn't
- **Docstrings**: Python has complex docstring skipping logic (50+ lines), other languages don't need it
- **Multi-line imports**: Handled inconsistently
- **Indentation preservation**: Rust uses `LineExtractor`, others use manual offset calculation

### 3. High Complexity in Production Code

From project complexity analysis:
- Python import_support: **204 avg cognitive complexity** (2,660 SLOC total)
- TypeScript workspace_support: **214 peak cognitive complexity**
- Go import_support: **132 avg cognitive complexity**

**Test code has complexity 291** - that's acceptable. **Production import handling at 200+** - that's a red flag.

### 4. Future Cost Multiplier

Swift was recently added (173 lines). Did it:
- ✅ Copy-paste from another language?
- ❌ Use cb-lang-common utilities?
- ❌ Follow a documented pattern?

**Every new language = another 200-400 lines of duplicated logic**.

---

## Detailed Analysis by Language

### Rust (`crates/cb-lang-rust/src/import_support.rs`)

**Current**: 262 lines, 98 avg cognitive complexity

**What's duplicated**:
- ✅ **Lines 64-66**: Manual indentation preservation
  - **Replace with**: `cb_lang_common::LineExtractor` (already imported but underutilized)
- ✅ **Lines 117-123**: Manual import checking with `contains()`
  - **Replace with**: `cb_lang_common::import_parsing` utilities
- ✅ **Lines 140-154**: Manual import insertion logic
  - **Extract to**: `cb_lang_common::import_parsing::add_import_statement()`

**Unique to Rust**:
- ❌ AST-based `syn` parsing (lines 58-62, 169) - **Keep (language-specific)**
- ❌ `quote!` macro usage (lines 70, 171) - **Keep (language-specific)**

**Extraction opportunity**: ~40-60 lines (15-23%)

**Current cb-lang-common usage**:
```rust
use cb_lang_common::ImportGraphBuilder;
use cb_lang_common::LineExtractor;  // Imported but barely used!
```

---

### Python (`crates/cb-lang-python/src/import_support.rs`)

**Current**: 424 lines, 204 avg cognitive complexity

**What's duplicated**:
- ✅ **Lines 118-137**: `contains_import()` with manual "as" parsing
  - **Replace with**: `cb_lang_common::import_parsing::parse_import_alias()`
- ✅ **Lines 278-308**: `path_to_python_module()` helper (31 lines!)
  - **Replace with**: `cb_lang_common::io::file_path_to_module()`
- ✅ **Lines 155-225**: Complex docstring + shebang + comment handling in `add_import()`
  - **Extract pattern to**: `cb_lang_common::import_parsing::find_import_insertion_point()`
- ✅ **Lines 69**: Simple string `replace()` in imports
  - **Enhance with**: `cb_lang_common::import_parsing::rewrite_import_path()`

**Unique to Python**:
- ❌ Docstring detection (lines 165-188) - **Keep (Python-specific)**
- ❌ Shebang handling (lines 159-162) - **Keep (Python-specific)**
- ❌ Triple-quote patterns - **Keep (Python-specific)**

**Extraction opportunity**: ~100-150 lines (24-35%)

**Current cb-lang-common usage**:
```rust
use cb_lang_common::LineExtractor;  // Only used in one place
use cb_lang_common::read_manifest;   // Manifest operations only
```

**Missing opportunities**: All `import_parsing` utilities unused!

---

### Go (`crates/cb-lang-go/src/import_support.rs`)

**Current**: 327 lines, 132 avg cognitive complexity

**What's duplicated**:
- ✅ Import block detection and manipulation
- ✅ Package name extraction from imports
- ✅ Path normalization for Go module paths
- ✅ Alias detection in import statements

**Unique to Go**:
- ❌ Import block grouping (parentheses)
- ❌ Domain-based packages (`github.com/...`)

**Extraction opportunity**: ~70-100 lines (21-31%)

**Current cb-lang-common usage**:
```rust
use cb_lang_common::{SubprocessAstTool, run_ast_tool, parse_with_fallback, ImportGraphBuilder};
```

**Better than others**, but still missing:
- `import_parsing::extract_package_name()` (handles Go's domain-based packages!)
- `import_parsing::normalize_import_path()`

---

### TypeScript (`crates/cb-lang-typescript/src/workspace_support.rs`)

**Current**: 524 lines, 214 peak cognitive complexity

**What's duplicated**:
- ✅ ES6 import/export parsing (multiple patterns)
- ✅ Scoped package detection (`@types/node`, `@scope/package`)
- ✅ Relative vs absolute import detection
- ✅ Path alias resolution (`@/components`)

**Unique to TypeScript**:
- ❌ `tsconfig.json` path mapping
- ❌ ES6 destructuring in imports
- ❌ Type-only imports (`import type`)

**Extraction opportunity**: ~120-160 lines (23-31%)

**Current cb-lang-common usage**:
```rust
use cb_lang_common::LineExtractor;
use cb_lang_common::read_manifest;
```

**Massive missed opportunities**:
- `ExternalDependencyDetector` with patterns for `./`, `../`, `@/`
- `extract_package_name()` for scoped packages
- `split_import_list()` for destructured imports

---

### Swift (`crates/cb-lang-swift/src/import_support.rs`)

**Current**: 173 lines, 28 avg cognitive complexity

**What's duplicated**:
- ✅ Simple import parsing (`import Module`)
- ✅ Import statement construction
- ✅ Contains/add/remove logic

**Unique to Swift**:
- ❌ `@testable import` handling
- ❌ Framework vs module distinction

**Extraction opportunity**: ~30-50 lines (17-29%)

**Current cb-lang-common usage**:
```rust
use cb_lang_common::ErrorBuilder;  // Only for error construction!
```

**Swift is the newest plugin** - perfect opportunity to establish the pattern!

---

## Available but Unused cb-lang-common Utilities

### From `import_parsing.rs` (212 lines, fully tested)

```rust
// ✅ Ready to use, zero languages using these:

pub fn parse_import_alias(text: &str) -> (String, Option<String>)
// Handles: "foo as bar" → ("foo", Some("bar"))
// Languages: Python, TypeScript, Go all have this pattern

pub fn split_import_list(text: &str) -> Vec<(String, Option<String>)>
// Handles: "foo, bar as b, baz" → [("foo", None), ("bar", Some("b")), ...]
// Languages: TypeScript destructuring, Python multiple imports

pub struct ExternalDependencyDetector
// Configurable detector for external vs relative imports
// Languages: ALL need this (TypeScript most urgent)

pub fn extract_package_name(path: &str) -> String
// Handles: "@types/node/fs" → "@types/node", "lodash/fp" → "lodash"
// Languages: TypeScript (scoped), Go (domain-based)

pub fn normalize_import_path(path: &str) -> String
// Strips quotes, whitespace from import paths
// Languages: ALL do this manually
```

### From `io.rs`

```rust
pub fn file_path_to_module(path: &Path, root: &Path, extension: &str) -> String
// Converts file paths to module paths
// Languages: Python, TypeScript both need this
```

### From `LineExtractor` (already imported!)

```rust
pub fn preserve_indentation(original: &str, new_content: &str) -> String
pub fn extract_line_range(content: &str, start: usize, end: usize) -> String
// Languages: Rust imports but underuses, others reimplement
```

---

## Proposed Solution

### Phase 1: Extend cb-lang-common (2-3 hours)

Add missing utilities that languages need but don't exist yet:

**New utilities to add**:

```rust
// crates/cb-lang-common/src/import_parsing.rs

/// Find the best insertion point for a new import statement
///
/// Handles:
/// - Skipping docstrings (Python), shebangs (Python), license headers
/// - Inserting after existing imports (all languages)
/// - Grouping by import type (standard lib, external, internal)
pub struct ImportInsertionFinder {
    skip_patterns: Vec<Regex>,  // e.g., shebang, docstring markers
    group_by: ImportGrouping,   // Standard, External, Internal
}

impl ImportInsertionFinder {
    pub fn new() -> Self;
    pub fn with_skip_pattern(self, pattern: &str) -> Self;
    pub fn find_insertion_point(&self, content: &str) -> usize;
}

/// Rewrite an import path during rename/move operations
pub fn rewrite_import_path(
    import_line: &str,
    old_path: &str,
    new_path: &str,
    style: ImportStyle,  // Module, Package, Relative
) -> Option<String>;

/// Build an import statement for a language
pub enum ImportStyle {
    Python,      // "from X import Y"
    Rust,        // "use X::Y"
    Go,          // "import \"X/Y\""
    ES6,         // "import { Y } from 'X'"
    Swift,       // "import X"
}

pub fn build_import_statement(
    module: &str,
    symbols: &[(String, Option<String>)],  // (name, alias)
    style: ImportStyle,
) -> String;
```

**New module**: `import_graph_operations.rs`

```rust
// Common graph operations all languages use

pub fn find_unused_imports(
    graph: &ImportGraph,
    used_symbols: &HashSet<String>,
) -> Vec<String>;

pub fn sort_imports_by_group(
    imports: Vec<String>,
    detector: &ExternalDependencyDetector,
) -> Vec<String>;
```

### Phase 2: Refactor Swift (Pilot) (2-3 hours)

**Why Swift first?**
- ✅ Smallest codebase (173 lines)
- ✅ Newest plugin (least technical debt)
- ✅ Simplest import syntax
- ✅ Lowest risk (if we break it, minimal impact)
- ✅ Sets pattern for others

**Refactoring steps**:

1. Update `SwiftImportSupport::contains_import()`:
```rust
// BEFORE (lines 65-69):
fn contains_import(&self, content: &str, module: &str) -> bool {
    content.lines().any(|line| {
        line.trim().starts_with("import ") && line.contains(module)
    })
}

// AFTER:
use cb_lang_common::import_parsing::{normalize_import_path, ExternalDependencyDetector};

fn contains_import(&self, content: &str, module: &str) -> bool {
    content.lines().any(|line| {
        let trimmed = line.trim();
        if trimmed.starts_with("import ") {
            let import_path = normalize_import_path(
                trimmed.strip_prefix("import ").unwrap_or("")
            );
            import_path == module || import_path.starts_with(&format!("{}.", module))
        } else {
            false
        }
    })
}
```

2. Update `SwiftImportSupport::add_import()` to use `ImportInsertionFinder`:
```rust
// BEFORE: Manual logic to find insertion point (lines 70-90)

// AFTER:
use cb_lang_common::import_parsing::ImportInsertionFinder;

fn add_import(&self, content: &str, module: &str) -> String {
    if self.contains_import(content, module) {
        return content.to_string();
    }

    let finder = ImportInsertionFinder::new()
        .with_skip_pattern(r"^//")  // Skip comments
        .with_skip_pattern(r"^/\*"); // Skip block comments

    let insert_pos = finder.find_insertion_point(content);
    let import_stmt = format!("import {}", module);

    // Insert at position (implementation in cb_lang_common)
    insert_line_at_position(content, &import_stmt, insert_pos)
}
```

**Success criteria**:
- ✅ All Swift tests pass
- ✅ SLOC reduced by 20-30 lines (12-17%)
- ✅ Complexity reduced (28 → 20-22)
- ✅ Pattern documented for other languages

### Phase 3: Refactor Python (High ROI) (3-4 hours)

**Why Python second?**
- 🎯 Highest complexity (204 avg)
- 🎯 Most duplicated code (424 lines)
- 🎯 Biggest complexity reduction potential

**Key refactorings**:

1. **Replace `path_to_python_module()` (31 lines)**:
```rust
// DELETE lines 278-308, replace with:
use cb_lang_common::io::file_path_to_module;

// In rewrite_imports_for_move():
let old_module = file_path_to_module(old_path, root, "py");
let new_module = file_path_to_module(new_path, root, "py");
```

2. **Replace docstring detection with `ImportInsertionFinder`**:
```rust
// DELETE lines 155-225 (70 lines!), replace with:
use cb_lang_common::import_parsing::ImportInsertionFinder;

let finder = ImportInsertionFinder::new()
    .with_skip_pattern(r"^#!")          // Shebang
    .with_skip_pattern(r#"^["']{3}"#)   // Docstrings
    .with_skip_pattern(r"^#");          // Comments

let insert_pos = finder.find_insertion_point(content);
```

3. **Use `parse_import_alias()` in `contains_import()`**:
```rust
// ENHANCE lines 118-137:
use cb_lang_common::import_parsing::parse_import_alias;

// When parsing "import foo as bar":
let (name, alias) = parse_import_alias(import_part);
if name == module || name.starts_with(&format!("{}.", module)) {
    return true;
}
```

**Expected impact**:
- Lines removed: 100-120 (24-28%)
- Complexity reduction: 204 → 140-150 (26-31%)

### Phase 4: Refactor Rust, Go, TypeScript (4-6 hours)

Apply proven patterns to remaining languages in order of complexity:

1. **TypeScript** (524 lines, 214 peak) - 2 hours
   - Use `ExternalDependencyDetector` for path aliases
   - Use `extract_package_name()` for scoped packages
   - Use `split_import_list()` for destructured imports

2. **Go** (327 lines, 132 avg) - 1.5 hours
   - Use `extract_package_name()` for domain-based packages
   - Use `ImportInsertionFinder` for import blocks

3. **Rust** (262 lines, 98 avg) - 1 hour
   - Better utilize `LineExtractor` (already imported!)
   - Use `ImportInsertionFinder` for use statements

---

## Implementation Checklist

### Phase 1: Extend cb-lang-common ✅

- [ ] Add `ImportInsertionFinder` struct with patterns
- [ ] Add `rewrite_import_path()` function
- [ ] Add `build_import_statement()` with `ImportStyle` enum
- [ ] Add `insert_line_at_position()` helper
- [ ] Create `import_graph_operations.rs` module
- [ ] Add comprehensive tests for all new utilities
- [ ] Update `lib.rs` re-exports
- [ ] Document all new functions with examples

**Deliverable**: Extended `cb-lang-common` with ~200 new lines (utilities + tests)

### Phase 2: Refactor Swift (Pilot) ✅

- [ ] Update `contains_import()` to use `normalize_import_path()`
- [ ] Update `add_import()` to use `ImportInsertionFinder`
- [ ] Update `remove_import()` to use common utilities
- [ ] Run full test suite (`cargo test -p cb-lang-swift`)
- [ ] Measure before/after: SLOC, complexity, test coverage
- [ ] Document pattern in `SWIFT_REFACTORING_NOTES.md`

**Deliverable**: Refactored Swift plugin, pattern documentation

### Phase 3: Refactor Python ✅

- [ ] Replace `path_to_python_module()` with `file_path_to_module()`
- [ ] Replace docstring detection with `ImportInsertionFinder`
- [ ] Use `parse_import_alias()` in import parsing
- [ ] Use `build_import_statement()` for statement construction
- [ ] Run full test suite (`cargo test -p cb-lang-python`)
- [ ] Measure impact: expect 100-120 line reduction

**Deliverable**: Refactored Python plugin with 24-28% code reduction

### Phase 4: Refactor Rust ✅

- [ ] Enhance `LineExtractor` usage for indentation
- [ ] Use `ImportInsertionFinder` for use statement insertion
- [ ] Extract common patterns to cb-lang-common
- [ ] Run tests, measure impact

**Deliverable**: Refactored Rust plugin

### Phase 5: Refactor Go ✅

- [ ] Use `extract_package_name()` for domain packages
- [ ] Use `ImportInsertionFinder` for import blocks
- [ ] Run tests, measure impact

**Deliverable**: Refactored Go plugin

### Phase 6: Refactor TypeScript ✅

- [ ] Use `ExternalDependencyDetector` with path patterns
- [ ] Use `extract_package_name()` for scoped packages
- [ ] Use `split_import_list()` for destructuring
- [ ] Run tests, measure impact

**Deliverable**: Refactored TypeScript plugin

---

## Expected Results

### Quantitative Impact (Conservative Estimates)

| Language | Current SLOC | Est. Removed | Current Complexity | Est. Reduction |
|----------|--------------|--------------|-------------------|----------------|
| Swift    | 173          | 25-30 (15%)  | 28 avg           | 20-22 (-21%)  |
| Python   | 424          | 100-120 (25%)| 204 avg          | 145-155 (-25%)|
| Rust     | 262          | 40-50 (17%)  | 98 avg           | 75-80 (-19%)  |
| Go       | 327          | 65-80 (22%)  | 132 avg          | 100-105 (-21%)|
| TypeScript| 524         | 120-140 (25%)| 214 peak         | 160-165 (-23%)|
| **Total**| **1,710**    | **350-420**  | **~135 avg**     | **~22% avg**  |

### Qualitative Impact

**Immediate Benefits**:
- ✅ Easier code reviews (less code per plugin)
- ✅ Faster bug fixes (fix once in cb-lang-common)
- ✅ Consistent behavior across languages
- ✅ Lower barrier to entry for new contributors

**Long-term Benefits**:
- ✅ New language plugins 40% faster to implement
- ✅ Import handling becomes "solved problem"
- ✅ Testing surface area reduced
- ✅ Complexity metrics improve across project

**Maintenance Benefits**:
- ✅ Single source of truth for import operations
- ✅ Centralized testing
- ✅ Documentation in one place
- ✅ Easier to add new features (add once, all languages benefit)

---

## Risk Assessment

### Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Breaking existing functionality | Medium | High | Comprehensive test suite per language, pilot with Swift first |
| Over-abstraction | Low | Medium | Phase 1 analysis validates extraction, pilot proves pattern |
| Time investment without ROI | Low | Medium | Can stop after any phase, each delivers value |
| Regression in edge cases | Medium | Medium | Keep language-specific tests, add integration tests |
| Team resistance to change | Low | Low | Show pilot results, incremental rollout |

### Risk Mitigation Strategy

1. **Pilot-first approach**: Swift refactoring proves pattern with minimal risk
2. **Comprehensive testing**: Every refactoring maintains 100% test coverage
3. **Incremental rollout**: Can pause after any phase
4. **Rollback plan**: Git branches allow easy rollback per language
5. **Documentation**: Pattern docs help reviewers understand changes

---

## Timeline & Effort

| Phase | Effort | Dependencies | Deliverable |
|-------|--------|-------------|-------------|
| 1. Extend cb-lang-common | 2-3 hours | None | New utilities + tests |
| 2. Pilot (Swift) | 2-3 hours | Phase 1 | Refactored Swift + pattern docs |
| 3. Python refactor | 3-4 hours | Phase 2 | Refactored Python (highest ROI) |
| 4. Rust refactor | 1.5-2 hours | Phase 2 | Refactored Rust |
| 5. Go refactor | 1.5-2 hours | Phase 2 | Refactored Go |
| 6. TypeScript refactor | 2-3 hours | Phase 2 | Refactored TypeScript |
| **Total** | **12-17 hours** | - | 5 refactored plugins + utilities |

**Can pause after any phase** and still deliver value.

### Recommended Approach

**Week 1**: Phases 1-2 (4-6 hours)
- Extend cb-lang-common
- Pilot with Swift
- **Decision point**: Continue or adjust based on pilot results

**Week 2**: Phase 3 (3-4 hours)
- Refactor Python (highest complexity reduction)
- **Decision point**: ROI validated, continue with others

**Week 3**: Phases 4-6 (5-7 hours)
- Refactor remaining languages
- Final documentation

---

## Success Metrics

### Code Metrics
- ✅ Total SLOC reduced by 350-420 lines (20-25%)
- ✅ Average cognitive complexity reduced by 20-25%
- ✅ Test coverage maintained at 100%
- ✅ No regression in functionality

### Developer Experience
- ✅ New language plugin template uses cb-lang-common (50% less code)
- ✅ Code review time reduced (less code to review per PR)
- ✅ Import-related bugs centralized to cb-lang-common

### Long-term Impact
- ✅ Pattern established for all language plugins
- ✅ Documentation shows "before/after" for future reference
- ✅ cb-lang-common becomes go-to place for plugin utilities

---

## Alternative Approaches Considered

### Alternative 1: Leave as-is
**Pros**: No effort required
**Cons**: Duplication continues, complexity grows with each language, technical debt compounds
**Verdict**: ❌ Rejected - cost multiplies with each new language

### Alternative 2: Extract only most duplicated code
**Pros**: Lower effort (~5 hours), some value
**Cons**: Doesn't solve root problem, half-measures
**Verdict**: ⚠️ Possible fallback if full refactor too ambitious

### Alternative 3: Rewrite from scratch
**Pros**: Clean slate, ideal architecture
**Cons**: Very high risk, 30+ hours, breaks everything
**Verdict**: ❌ Rejected - too risky

### Alternative 4: Gradual extraction over time
**Pros**: Low risk, no big refactor
**Cons**: Never actually happens, "later" becomes "never"
**Verdict**: ❌ Rejected - needs dedicated focus

**Selected approach**: Phased refactoring with pilot (Alternative 4 + structure)

---

## Appendix A: Detailed Duplication Examples

### Example 1: Import Alias Parsing

**Duplicated across 4 languages**:

```rust
// Python (lines 118-124):
let module_name = import_part.split(" as ").next().unwrap_or("").trim();

// TypeScript (similar pattern):
const parts = importLine.split(' as ');
const moduleName = parts[0].trim();

// Go (similar pattern):
parts := strings.Split(importLine, " as ")
module := strings.TrimSpace(parts[0])
```

**Available in cb-lang-common**:
```rust
use cb_lang_common::import_parsing::parse_import_alias;
let (module, alias) = parse_import_alias(import_part);
```

**Impact**: 4 languages × 5-10 lines each = 20-40 lines removed

---

### Example 2: Path Normalization

**Duplicated across all languages**:

```rust
// Python: Manual quote stripping
let path = import_path.trim().trim_matches('"').trim_matches('\'');

// TypeScript: Similar logic
const normalized = path.trim().replace(/^["']|["']$/g, '');

// Go: Multiple trim operations
trimmed := strings.Trim(strings.TrimSpace(path), "\"")
```

**Available in cb-lang-common**:
```rust
use cb_lang_common::import_parsing::normalize_import_path;
let path = normalize_import_path(import_path);
```

**Impact**: 5 languages × 2-3 lines each = 10-15 lines removed

---

### Example 3: File Path to Module

**Duplicated in Python and TypeScript**:

```rust
// Python (31 lines, 278-308):
fn path_to_python_module(path: &Path) -> String {
    let path_no_ext = path.with_extension("");
    let components: Vec<_> = path_no_ext
        .components()
        .filter_map(|c| {
            if let std::path::Component::Normal(s) = c {
                s.to_str()
            } else {
                None
            }
        })
        .filter(|s| *s != "src")
        .collect();

    let mut module = components.join(".");
    if module.ends_with(".__init__") {
        module = module.strip_suffix(".__init__").unwrap_or(&module).to_string();
    }
    module
}

// TypeScript: Similar logic (different separators)
```

**Available in cb-lang-common**:
```rust
use cb_lang_common::io::file_path_to_module;
let module = file_path_to_module(path, root, "py");
```

**Impact**: 2 languages × 30-35 lines each = 60-70 lines removed

---

## Appendix B: cb-lang-common Utilities Reference

### Already Available (Unused)

From `import_parsing.rs`:
- `parse_import_alias(text: &str) -> (String, Option<String>)`
- `split_import_list(text: &str) -> Vec<(String, Option<String>)>`
- `ExternalDependencyDetector` struct with builder pattern
- `extract_package_name(path: &str) -> String`
- `normalize_import_path(path: &str) -> String`

From `io.rs`:
- `file_path_to_module(path: &Path, root: &Path, ext: &str) -> String`
- `read_manifest(path: &Path) -> Result<String>`
- `find_source_files(root: &Path, extensions: &[&str]) -> Result<Vec<PathBuf>>`

From `refactoring.rs`:
- `LineExtractor` - preserves indentation
- `CodeRange` - source location utilities
- `IndentationDetector` - detects indentation style

### To Be Added (Phase 1)

- `ImportInsertionFinder` - find insertion point for imports
- `rewrite_import_path()` - rewrite imports during rename
- `build_import_statement()` - construct import statements
- `insert_line_at_position()` - helper for line insertion
- `find_unused_imports()` - graph-based unused import detection
- `sort_imports_by_group()` - sort by external/internal/std

---

## Appendix C: Testing Strategy

### Test Coverage Requirements

**All refactored code must maintain 100% test coverage**:

1. **Unit tests** (each function):
   - Happy path
   - Edge cases (empty input, malformed input)
   - Language-specific quirks

2. **Integration tests** (per language):
   - Parse → Modify → Verify round-trip
   - Import addition with existing code
   - Import removal without breaking code
   - Rename operations across multiple files

3. **Regression tests**:
   - Keep all existing tests
   - Add tests for any bugs found during refactoring
   - Edge cases from production usage

### Test Execution

**Before each merge**:
```bash
# Full test suite per language
cargo test -p cb-lang-swift
cargo test -p cb-lang-python
cargo test -p cb-lang-rust
cargo test -p cb-lang-go
cargo test -p cb-lang-typescript

# Common utilities tests
cargo test -p cb-lang-common

# Integration tests
cargo test -p integration-tests
```

**Continuous Integration**:
- All tests must pass before merge
- Code coverage reports required
- Complexity metrics tracked

---

## Approval & Next Steps

### Required Approvals

- [ ] Technical Lead (architecture approval)
- [ ] Language Plugin Maintainers (impact assessment)
- [ ] QA Lead (testing strategy approval)

### Next Steps

1. **Review this proposal** - Team discussion
2. **Approve Phase 1** - Extend cb-lang-common
3. **Execute Pilot** - Refactor Swift, measure results
4. **Decision point** - Continue or adjust based on pilot
5. **Execute rollout** - Refactor remaining languages

### Questions for Discussion

1. Should we do all languages or just high-complexity ones (Python, TypeScript)?
2. Timeline acceptable (2-3 weeks part-time)?
3. Any concerns about breaking changes?
4. Should we involve language plugin maintainers in refactoring their plugins?

---

## Conclusion

This refactoring initiative will:

✅ **Remove 350-420 lines of duplicated code** (20-25% reduction)
✅ **Reduce cognitive complexity by 20-25%** across all plugins
✅ **Establish sustainable patterns** for future language plugins
✅ **Improve maintainability** through centralized utilities
✅ **Reduce bug surface** by fixing issues once in cb-lang-common

The pilot-first approach minimizes risk while proving value early. Each phase delivers incremental value, allowing us to pause or adjust at any point.

**Recommendation**: Approve Phase 1 (extend cb-lang-common) and Phase 2 (Swift pilot) to validate the approach with minimal risk.
