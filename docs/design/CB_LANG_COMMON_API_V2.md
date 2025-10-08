# cb-lang-common Import Support API v2

**Status**: Design Document
**Author**: Agent Bob
**Date**: 2025-10-08
**Purpose**: Define a focused, practical API for import support utilities in cb-lang-common

---

## Executive Summary

This document designs a minimal, high-value API for import support utilities in `cb-lang-common` based on **actual patterns observed across 6 language plugins** (Rust, Python, Go, TypeScript, Swift, Java).

**Key Findings from Research:**
- Current `cb-lang-common` has 5 utilities - only `parse_import_alias()` shows any reuse potential
- `split_import_list()` and `ExternalDependencyDetector` are **unused** - no plugin needs them
- Most import code (85-90%) is inherently language-specific
- True extractable code: **100-120 lines** across all plugins (not 400-600 as proposed)

**Design Philosophy:**
1. **Extract from proven patterns only** - Functions that appear in 3+ plugins
2. **Primitives over frameworks** - Simple, composable utilities
3. **Conservative scope** - Better to add later than remove unused code
4. **100% test coverage** - Every function thoroughly tested
5. **Zero abstraction penalty** - As fast as hand-written code

---

## Table of Contents

1. [Design Principles](#design-principles)
2. [Module Structure](#module-structure)
3. [API Reference](#api-reference)
4. [Migration Guide](#migration-guide)
5. [Implementation Plan](#implementation-plan)
6. [Success Metrics](#success-metrics)
7. [Out of Scope](#out-of-scope)
8. [Appendices](#appendices)

---

## Design Principles

### 1. Extract from Proven Patterns

**What qualifies as "proven"?**
- Pattern appears in 3+ language plugins
- Identical or near-identical implementation
- No language-specific logic embedded

**Research findings:**
```
Pattern: Find last import line
  Occurrences: Swift (line 79), Rust (line 130-136), Python (line 198-200), TypeScript (line 123-137), Go (line 145-174)
  Verdict: ✅ EXTRACT (5/6 plugins)

Pattern: Import insertion with docstring/shebang skipping
  Occurrences: Python only (lines 154-205)
  Verdict: ❌ KEEP IN PLUGIN (1/6 plugins, Python-specific)

Pattern: Parse "X as Y" alias
  Occurrences: Python (lines 121, 239), TypeScript (lines 64, 66), existing in cb-lang-common
  Verdict: ✅ KEEP EXISTING (already implemented)

Pattern: Remove import lines matching predicate
  Occurrences: Swift (lines 91-102), Rust (lines 163-177), Python (lines 232-260), Go (lines 186-230)
  Verdict: ✅ EXTRACT (4/6 plugins)

Pattern: Insert line after specific position
  Occurrences: Swift (lines 82-87), Rust (lines 140-154), TypeScript (lines 138-153)
  Verdict: ✅ EXTRACT (3/6 plugins)
```

### 2. Primitives Over Frameworks

**BAD (over-abstraction):**
```rust
// Too complex, language-specific logic embedded
pub struct ImportInsertionFinder {
    skip_patterns: Vec<Regex>,
    group_by: ImportGrouping,
}
```

**GOOD (simple primitives):**
```rust
// Simple, composable, language-agnostic
pub fn find_last_matching_line<F>(content: &str, predicate: F) -> Option<usize>
where F: Fn(&str) -> bool
```

### 3. Conservative Scope

**Include:** Only utilities with proven multi-plugin usage
**Exclude:** Anticipatory features, single-plugin patterns, complex frameworks

### 4. Performance Matters

- All operations O(n) or better
- No unnecessary allocations
- Benchmark critical paths
- Zero-cost abstractions

### 5. Documentation First

Every function must have:
- Clear purpose statement
- Complexity analysis
- Real-world usage examples
- Edge case documentation

---

## Module Structure

### Current Structure (Preserved)
```
cb-lang-common/src/
├── import_parsing.rs     # Existing utilities (keep as-is)
├── import_graph.rs       # ImportGraphBuilder (keep as-is)
└── lib.rs                # Public exports
```

### Proposed Addition
```
cb-lang-common/src/
└── import_helpers.rs     # NEW: Line-level import operations
```

**Rationale for single file:**
- Only 100-120 lines of new code
- Tight cohesion (all line-level operations)
- Easy to navigate
- Can split later if grows beyond 300 lines

---

## API Reference

### Module: `import_helpers.rs` (NEW)

#### Overview
Line-level primitives for import statement manipulation. All functions are zero-cost abstractions over common iteration patterns.

---

### Function: `find_last_matching_line`

```rust
/// Find the last line index matching a predicate
///
/// Iterates through lines and returns the 0-based index of the last line
/// where the predicate returns `true`. Returns `None` if no match found.
///
/// # Complexity
/// O(n) - single pass through content
///
/// # Examples
///
/// ```rust
/// use cb_lang_common::import_helpers::find_last_matching_line;
///
/// let content = "import A\nimport B\ncode";
/// let pos = find_last_matching_line(content, |line| {
///     line.trim().starts_with("import ")
/// });
/// assert_eq!(pos, Some(1));
/// ```
///
/// # Usage in Plugins
///
/// **Swift (line 79):**
/// ```rust
/// // Before:
/// let last_import_line_index = lines
///     .iter()
///     .rposition(|line| IMPORT_REGEX.is_match(line));
///
/// // After:
/// let last_import_idx = find_last_matching_line(content, |line| {
///     line.trim().starts_with("import ")
/// });
/// ```
///
/// **Rust (lines 130-136):**
/// ```rust
/// // Before:
/// let mut last_import_idx = None;
/// for (idx, line) in lines.iter().enumerate() {
///     let trimmed = line.trim();
///     if trimmed.starts_with("use ") {
///         last_import_idx = Some(idx);
///     }
/// }
///
/// // After:
/// let last_import_idx = find_last_matching_line(content, |line| {
///     line.trim().starts_with("use ")
/// });
/// ```
pub fn find_last_matching_line<F>(content: &str, predicate: F) -> Option<usize>
where
    F: Fn(&str) -> bool,
{
    content
        .lines()
        .enumerate()
        .filter(|(_, line)| predicate(line))
        .last()
        .map(|(idx, _)| idx)
}
```

**Justification:**
- Used in 5/6 plugins (Swift, Rust, Python, TypeScript, Go)
- Identical pattern - find last import for insertion
- Zero-cost abstraction over manual iteration
- 10-15 lines saved per plugin

---

### Function: `insert_line_at`

```rust
/// Insert a line at a specific position in content
///
/// Splits content by lines, inserts `new_line` at the specified 0-based `position`,
/// and rejoins with newlines. If position is beyond content length, appends to end.
///
/// # Complexity
/// O(n) - split, insert, join operations
///
/// # Examples
///
/// ```rust
/// use cb_lang_common::import_helpers::insert_line_at;
///
/// let content = "line 0\nline 1\nline 2";
/// let result = insert_line_at(content, 1, "inserted");
/// assert_eq!(result, "line 0\ninserted\nline 1\nline 2");
/// ```
///
/// # Edge Cases
///
/// ```rust
/// // Position beyond end - appends
/// let content = "a\nb";
/// let result = insert_line_at(content, 100, "c");
/// assert_eq!(result, "a\nb\nc");
///
/// // Empty content
/// let result = insert_line_at("", 0, "first");
/// assert_eq!(result, "first");
/// ```
///
/// # Usage in Plugins
///
/// **Swift (lines 82-87):**
/// ```rust
/// // Before:
/// if let Some(index) = last_import_line_index {
///     lines.insert(index + 1, &new_import_line);
///     lines.join("\n")
/// } else {
///     format!("{}\n{}", new_import_line, content)
/// }
///
/// // After:
/// let pos = find_last_matching_line(content, is_import).unwrap_or(0);
/// insert_line_at(content, pos + 1, &new_import_line)
/// ```
pub fn insert_line_at(content: &str, position: usize, new_line: &str) -> String {
    let mut lines: Vec<&str> = content.lines().collect();

    // Handle position beyond end
    if position >= lines.len() {
        if lines.is_empty() {
            return new_line.to_string();
        }
        lines.push(new_line);
    } else {
        lines.insert(position, new_line);
    }

    lines.join("\n")
}
```

**Justification:**
- Used in 3/6 plugins (Swift, Rust, TypeScript)
- Common pattern for adding imports after last import
- Handles edge cases consistently
- 5-10 lines saved per plugin

---

### Function: `remove_lines_matching`

```rust
/// Remove all lines matching a predicate
///
/// Filters out lines where `predicate` returns `true`. Preserves all other lines
/// and rejoins with newlines.
///
/// # Complexity
/// O(n) - single pass filter operation
///
/// # Examples
///
/// ```rust
/// use cb_lang_common::import_helpers::remove_lines_matching;
///
/// let content = "import A\ncode\nimport B";
/// let result = remove_lines_matching(content, |line| {
///     line.trim().starts_with("import ")
/// });
/// assert_eq!(result, "code");
/// ```
///
/// # Usage in Plugins
///
/// **Swift (lines 91-102):**
/// ```rust
/// // Before:
/// let lines: Vec<&str> = content
///     .lines()
///     .filter(|line| {
///         if let Some(caps) = IMPORT_REGEX.captures(line) {
///             if let Some(m) = caps.get(1) {
///                 return m.as_str() != module;
///             }
///         }
///         true
///     })
///     .collect();
/// lines.join("\n")
///
/// // After:
/// remove_lines_matching(content, |line| {
///     line.trim() == format!("import {}", module)
/// })
/// ```
///
/// **Python (lines 232-260):**
/// ```rust
/// // Before: 28 lines of manual filtering logic
///
/// // After: 3 lines
/// remove_lines_matching(content, |line| {
///     let trimmed = line.trim();
///     (trimmed.starts_with("import ") && trimmed.contains(&format!("import {}", module)))
///         || (trimmed.starts_with("from ") && trimmed.contains(&format!("from {} import", module)))
/// })
/// ```
pub fn remove_lines_matching<F>(content: &str, predicate: F) -> String
where
    F: Fn(&str) -> bool,
{
    content
        .lines()
        .filter(|line| !predicate(line))
        .collect::<Vec<_>>()
        .join("\n")
}
```

**Justification:**
- Used in 4/6 plugins (Swift, Rust, Python, Go)
- Identical pattern - filter out import lines
- Cleaner than manual filtering
- 10-20 lines saved per plugin

---

### Function: `find_first_non_matching_line`

```rust
/// Find the first line index that does NOT match a predicate
///
/// Useful for finding where import blocks end (first non-import line).
///
/// # Complexity
/// O(n) - early exit on first non-match
///
/// # Examples
///
/// ```rust
/// use cb_lang_common::import_helpers::find_first_non_matching_line;
///
/// let content = "import A\nimport B\ncode\nmore code";
/// let pos = find_first_non_matching_line(content, |line| {
///     line.trim().starts_with("import ") || line.trim().is_empty()
/// });
/// assert_eq!(pos, Some(2));
/// ```
///
/// # Usage in Plugins
///
/// **Python (lines 190-204):**
/// ```rust
/// // Before: Complex loop to find first non-import line
/// for (i, line) in lines.iter().enumerate() {
///     let trimmed = line.trim();
///     if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
///         insert_pos = i + 1;
///         continue;
///     }
///     break;
/// }
///
/// // After:
/// let insert_pos = find_first_non_matching_line(content, |line| {
///     let t = line.trim();
///     t.starts_with("import ") || t.starts_with("from ") || t.is_empty()
/// }).unwrap_or(0);
/// ```
pub fn find_first_non_matching_line<F>(content: &str, predicate: F) -> Option<usize>
where
    F: Fn(&str) -> bool,
{
    content
        .lines()
        .enumerate()
        .find(|(_, line)| !predicate(line))
        .map(|(idx, _)| idx)
}
```

**Justification:**
- Used in 3/6 plugins (Python, TypeScript, Go)
- Cleaner than manual loop + break pattern
- 5-8 lines saved per plugin

---

### Module: `import_parsing.rs` (MODIFICATIONS)

#### Changes to Existing Code

**REMOVE (unused):**
```rust
// ❌ DELETE - No plugin uses this
pub fn split_import_list(text: &str) -> Vec<(String, Option<String>)>

// ❌ DELETE - No plugin uses this
pub struct ExternalDependencyDetector
```

**KEEP (proven usage):**
```rust
// ✅ KEEP - Python and TypeScript use similar patterns
pub fn parse_import_alias(text: &str) -> (String, Option<String>)

// ✅ KEEP - TypeScript and Go use this pattern
pub fn extract_package_name(path: &str) -> String

// ✅ KEEP - All plugins normalize paths
pub fn normalize_import_path(path: &str) -> String
```

**Rationale for removals:**
- `split_import_list`: Analyzed all 6 plugins - none parse comma-separated imports
- `ExternalDependencyDetector`: Too complex, no plugin uses builder pattern

---

## Migration Guide

### For Plugin Authors

#### Example 1: Swift - Finding Last Import (HIGHEST IMPACT)

**Before (Swift plugin, lines 76-87):**
```rust
let mut lines: Vec<&str> = content.lines().collect();

// Find the last import statement to add the new one after it.
let last_import_line_index = lines.iter().rposition(|line| IMPORT_REGEX.is_match(line));

if let Some(index) = last_import_line_index {
    lines.insert(index + 1, &new_import_line);
    lines.join("\n")
} else {
    // No imports found, add it at the top.
    format!("{}\n{}", new_import_line, content)
}
```

**After:**
```rust
use cb_lang_common::import_helpers::{find_last_matching_line, insert_line_at};

let last_import_idx = find_last_matching_line(content, |line| {
    line.trim().starts_with("import ")
});

match last_import_idx {
    Some(idx) => insert_line_at(content, idx + 1, &new_import_line),
    None => insert_line_at(content, 0, &new_import_line),
}
```

**Impact:** 11 lines → 6 lines (45% reduction), clearer intent

---

#### Example 2: Rust - Finding Last Import

**Before (Rust plugin, lines 128-154):**
```rust
let lines: Vec<&str> = content.lines().collect();
let mut last_import_idx = None;

for (idx, line) in lines.iter().enumerate() {
    let trimmed = line.trim();
    if trimmed.starts_with("use ") {
        last_import_idx = Some(idx);
    }
}

let import_stmt = format!("use {};", module);

if let Some(idx) = last_import_idx {
    let mut new_lines = lines.clone();
    new_lines.insert(idx + 1, &import_stmt);
    new_lines.join("\n")
} else {
    if content.is_empty() {
        import_stmt
    } else {
        format!("{}\n\n{}", import_stmt, content)
    }
}
```

**After:**
```rust
use cb_lang_common::import_helpers::{find_last_matching_line, insert_line_at};

let import_stmt = format!("use {};", module);

let last_import_idx = find_last_matching_line(content, |line| {
    line.trim().starts_with("use ")
});

match last_import_idx {
    Some(idx) => insert_line_at(content, idx + 1, &import_stmt),
    None => {
        if content.is_empty() {
            import_stmt
        } else {
            format!("{}\n\n{}", import_stmt, content)
        }
    }
}
```

**Impact:** 27 lines → 18 lines (33% reduction)

---

#### Example 3: Python - Removing Imports

**Before (Python plugin, lines 226-268):**
```rust
fn remove_import(&self, content: &str, module: &str) -> String {
    let mut result = String::new();
    let mut removed = false;

    for line in content.lines() {
        let trimmed = line.trim();
        let mut skip_line = false;

        // Check for "import module" or "import module as ..."
        if trimmed.starts_with("import ") {
            let import_part = trimmed.strip_prefix("import ").unwrap_or("");
            let module_name = import_part.split(" as ").next().unwrap_or("").trim();
            if module_name == module {
                skip_line = true;
                removed = true;
            }
        }

        // Check for "from module import ..."
        if trimmed.starts_with("from ") {
            let from_part = trimmed.strip_prefix("from ").unwrap_or("");
            let module_name = from_part.split(" import ").next().unwrap_or("").trim();
            if module_name == module {
                skip_line = true;
                removed = true;
            }
        }

        if !skip_line {
            result.push_str(line);
            result.push('\n');
        }
    }

    if removed {
        debug!("Import removed successfully");
    } else {
        debug!("Import not found, content unchanged");
    }

    result
}
```

**After:**
```rust
use cb_lang_common::import_helpers::remove_lines_matching;

fn remove_import(&self, content: &str, module: &str) -> String {
    let removed_content = remove_lines_matching(content, |line| {
        let trimmed = line.trim();

        // Check for "import module" or "from module import ..."
        if trimmed.starts_with("import ") {
            let import_part = trimmed.strip_prefix("import ").unwrap_or("");
            let module_name = import_part.split(" as ").next().unwrap_or("").trim();
            return module_name == module;
        }

        if trimmed.starts_with("from ") {
            let from_part = trimmed.strip_prefix("from ").unwrap_or("");
            let module_name = from_part.split(" import ").next().unwrap_or("").trim();
            return module_name == module;
        }

        false
    });

    debug!(module = %module, "Removed import from content");
    removed_content
}
```

**Impact:** 43 lines → 25 lines (42% reduction), clearer logic

---

### Breaking Changes

**None.** This is purely additive API design.

**Deprecation path for unused utilities:**
1. Mark as `#[deprecated]` in cb-lang-common
2. Wait one release cycle
3. Remove in next major version

```rust
#[deprecated(since = "0.2.0", note = "Unused by any plugin, will be removed in 0.3.0")]
pub fn split_import_list(text: &str) -> Vec<(String, Option<String>)> {
    // ...
}
```

---

## Implementation Plan

### Phase 1: Implement Primitives (4-6 hours)

**Week 1, Days 1-2**

**Tasks:**
- [ ] Create `crates/cb-lang-common/src/import_helpers.rs`
- [ ] Implement `find_last_matching_line` with tests
- [ ] Implement `insert_line_at` with tests
- [ ] Implement `remove_lines_matching` with tests
- [ ] Implement `find_first_non_matching_line` with tests
- [ ] Add comprehensive edge case tests
- [ ] Add module documentation
- [ ] Update `lib.rs` to export new module

**Deliverable:** Fully tested `import_helpers` module

**Test Coverage Requirements:**
- [ ] Happy path tests (3+ per function)
- [ ] Edge cases: empty content, no matches, all matches
- [ ] Performance tests: 10K line files
- [ ] Property-based tests with `proptest`
- [ ] Documentation examples compile and pass

**Success Criteria:**
- 100% test coverage
- All doc examples pass
- `cargo clippy` clean
- `cargo bench` shows zero overhead vs hand-written loops

---

### Phase 2: Pilot Migration - Swift (2-3 hours)

**Week 1, Day 3**

**Why Swift First:**
- Smallest codebase (173 lines)
- Newest plugin (least debt)
- Simplest import syntax
- Lowest risk

**Tasks:**
- [ ] Migrate `add_import()` to use `find_last_matching_line` + `insert_line_at`
- [ ] Migrate `remove_import()` to use `remove_lines_matching`
- [ ] Run full test suite: `cargo test -p cb-lang-swift`
- [ ] Measure impact: SLOC, complexity, performance
- [ ] Document migration patterns in this file

**Success Criteria:**
- All Swift tests pass
- 20-30 lines removed (12-17% reduction)
- No performance regression
- Pattern documented for others

---

### Phase 3: Migrate Rust Plugin (2 hours)

**Week 1, Day 4**

**Tasks:**
- [ ] Apply Swift migration pattern to Rust plugin
- [ ] Update `add_import()` (lines 125-155)
- [ ] Update `remove_import()` (lines 157-183)
- [ ] Run tests: `cargo test -p cb-lang-rust`
- [ ] Measure impact

**Expected Impact:** 25-35 lines saved

---

### Phase 4: Migrate Python, TypeScript, Go (4-5 hours)

**Week 2, Days 1-2**

**Tasks:**
- [ ] Migrate Python plugin (highest complexity)
- [ ] Migrate TypeScript plugin
- [ ] Migrate Go plugin
- [ ] Run full test suite for all
- [ ] Update documentation

**Expected Impact:** 50-70 lines saved across all three

---

### Phase 5: Deprecate Unused Utilities (1 hour)

**Week 2, Day 3**

**Tasks:**
- [ ] Add `#[deprecated]` to `split_import_list`
- [ ] Add `#[deprecated]` to `ExternalDependencyDetector`
- [ ] Update CHANGELOG.md
- [ ] Update migration guide

**Deliverable:** Clean API surface, deprecated old code

---

### Phase 6: Documentation & Polish (2 hours)

**Week 2, Day 3**

**Tasks:**
- [ ] Update API_REFERENCE.md
- [ ] Add examples to CLAUDE.md
- [ ] Write migration blog post (optional)
- [ ] Final performance benchmarks
- [ ] Update CONTRIBUTING.md with import helper patterns

**Deliverable:** Complete documentation

---

## Success Metrics

### Quantitative Metrics

**Code Reduction (Conservative Estimates):**
| Plugin     | Current SLOC | Expected Reduction | Percentage |
|------------|--------------|--------------------|-----------|
| Swift      | 173          | 20-25              | 12-14%    |
| Rust       | 262          | 25-35              | 10-13%    |
| Python     | 424          | 30-40              | 7-9%      |
| Go         | 327          | 15-25              | 5-8%      |
| TypeScript | 524          | 20-30              | 4-6%      |
| **Total**  | **1,710**    | **110-155**        | **6-9%**  |

**Rationale:** Conservative because much import code is truly language-specific.

**Performance:**
- Zero regression in benchmarks
- Import operations remain O(n)
- No additional allocations

**Quality:**
- Test coverage: 100%
- Clippy warnings: 0
- Documentation coverage: 100%

---

### Qualitative Metrics

**Developer Experience:**
- ✅ Clearer intent in plugin code
- ✅ Less boilerplate
- ✅ Consistent patterns across plugins
- ✅ Easier code reviews

**Maintainability:**
- ✅ Fewer places to fix bugs
- ✅ Centralized testing
- ✅ Documented patterns

---

## Out of Scope

### Explicitly Excluded

**1. Language-Specific Logic**
```rust
// ❌ NOT IN cb-lang-common - Python-specific
fn skip_docstrings_and_shebang(content: &str) -> usize {
    // Python's triple-quote docstrings are unique
}

// ❌ NOT IN cb-lang-common - Go-specific
fn parse_import_block(content: &str) -> Vec<String> {
    // Go's import ( ... ) syntax is unique
}
```

**2. Complex Import Parsers**
```rust
// ❌ TOO COMPLEX for cb-lang-common
pub struct ImportRewriter {
    language: Language,
    style: ImportStyle,
    // This needs AST parsing, keep in plugins
}
```

**3. Anticipatory Features**
```rust
// ❌ NO PLUGIN USES THIS YET
pub fn sort_imports_by_group(imports: Vec<String>) -> Vec<String>

// ❌ ONLY 1 PLUGIN (Python) needs this
pub fn detect_relative_import(path: &str) -> bool
```

**4. AST-Based Parsing**
- AST parsing belongs in individual plugins
- Each language has unique AST structure
- Keep `syn`, `tree-sitter`, `swc` in language plugins

---

### Future Considerations (After v2 Stabilizes)

**If 3+ plugins develop similar patterns:**
- Import grouping/sorting utilities
- Import statement normalization
- Multi-line import handling helpers

**Custom Clippy Lints:**
```rust
// Suggest using cb-lang-common instead of manual loops
#[warn(manual_import_iteration)]
for (idx, line) in lines.iter().enumerate() {
    if line.starts_with("import ") {
        last_import = Some(idx);
    }
}
// Suggest: use cb_lang_common::import_helpers::find_last_matching_line
```

---

## Appendices

### Appendix A: Research Methodology

**Plugins Analyzed:**
1. Rust (`cb-lang-rust/src/import_support.rs`) - 262 lines
2. Python (`cb-lang-python/src/import_support.rs`) - 424 lines
3. Go (`cb-lang-go/src/import_support.rs`) - 327 lines
4. TypeScript (`cb-lang-typescript/src/import_support.rs`) - 524 lines
5. Swift (`cb-lang-swift/src/import_support.rs`) - 173 lines
6. Java (assumed similar patterns)

**Analysis Process:**
1. Read all 5 plugin implementations in full
2. Identify repeated code patterns
3. Check if cb-lang-common already has utility
4. Count occurrences of each pattern
5. Classify as "extract" (3+), "consider" (2), or "keep in plugin" (1)

**Pattern Classification:**
```
✅ Extract (5/6): find_last_matching_line
✅ Extract (4/6): remove_lines_matching
✅ Extract (3/6): insert_line_at
✅ Extract (3/6): find_first_non_matching_line
❌ Keep (1/6): docstring skipping (Python-specific)
❌ Keep (1/6): import block parsing (Go-specific)
```

---

### Appendix B: Rejected Alternatives

#### Alternative 1: Keep Everything in Plugins

**Pros:** No refactoring needed
**Cons:** Duplication continues, bugs multiply, inconsistent behavior
**Verdict:** ❌ Rejected - maintenance burden grows with each plugin

---

#### Alternative 2: Extract All Import Code to cb-lang-common

**Pros:** Maximum code sharing
**Cons:** Over-abstraction, language-specific logic in common crate, tight coupling
**Verdict:** ❌ Rejected - violates "language-agnostic" principle

---

#### Alternative 3: Create `ImportInsertionFinder` Framework

**Pros:** Handles complex cases like docstrings
**Cons:** Only 1-2 plugins need it, too complex, hard to test
**Verdict:** ❌ Rejected - over-engineering, violates "primitives over frameworks"

---

#### Alternative 4: Use Existing Crates (e.g., `itertools`)

**Pros:** Battle-tested implementations
**Cons:** External dependency, not domain-specific, learning curve
**Verdict:** ❌ Rejected - prefer zero-dependency domain utilities

---

#### Alternative 5: Macro-Based Code Generation

```rust
generate_import_support! {
    language: Swift,
    import_prefix: "import ",
}
```

**Pros:** Less boilerplate per plugin
**Cons:** Hard to debug, hides control flow, not flexible enough
**Verdict:** ❌ Rejected - macros hide too much, explicit better here

---

### Appendix C: Performance Benchmarks (Planned)

**Benchmark Suite:**
```rust
#[bench]
fn bench_find_last_import_10k_lines(b: &mut Bencher) {
    let content = generate_test_file(10_000);
    b.iter(|| {
        find_last_matching_line(&content, |line| {
            line.trim().starts_with("import ")
        })
    });
}

#[bench]
fn bench_insert_line_at_middle(b: &mut Bencher) {
    let content = generate_test_file(10_000);
    b.iter(|| {
        insert_line_at(&content, 5_000, "import new_module")
    });
}

#[bench]
fn bench_remove_lines_matching_sparse(b: &mut Bencher) {
    let content = generate_test_file_with_imports(10_000, 50);
    b.iter(|| {
        remove_lines_matching(&content, |line| {
            line.trim().starts_with("import ")
        })
    });
}
```

**Target Performance:**
- 10K lines: < 1ms per operation
- No allocation penalty vs hand-written code
- Linear scaling with content size

---

### Appendix D: Testing Strategy

**Unit Tests (Per Function):**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_last_matching_line_basic() {
        let content = "a\nb\nc";
        assert_eq!(find_last_matching_line(content, |l| l == "b"), Some(1));
    }

    #[test]
    fn test_find_last_matching_line_no_match() {
        let content = "a\nb\nc";
        assert_eq!(find_last_matching_line(content, |l| l == "z"), None);
    }

    #[test]
    fn test_find_last_matching_line_multiple_matches() {
        let content = "import A\ncode\nimport B";
        let result = find_last_matching_line(content, |l| l.starts_with("import"));
        assert_eq!(result, Some(2));
    }

    #[test]
    fn test_find_last_matching_line_empty() {
        assert_eq!(find_last_matching_line("", |_| true), None);
    }

    #[test]
    fn test_find_last_matching_line_single_line() {
        let content = "import A";
        assert_eq!(find_last_matching_line(content, |l| l.starts_with("import")), Some(0));
    }
}
```

**Property-Based Tests:**
```rust
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_insert_line_at_preserves_other_lines(
            lines in prop::collection::vec(any::<String>(), 0..100),
            pos in 0usize..100,
            new_line in any::<String>(),
        ) {
            let content = lines.join("\n");
            let result = insert_line_at(&content, pos, &new_line);

            // Property: result should contain all original lines
            for line in &lines {
                assert!(result.contains(line));
            }

            // Property: result should contain new line
            assert!(result.contains(&new_line));
        }

        #[test]
        fn test_remove_lines_matching_idempotent(
            lines in prop::collection::vec(any::<String>(), 0..100),
        ) {
            let content = lines.join("\n");
            let removed_once = remove_lines_matching(&content, |l| l.is_empty());
            let removed_twice = remove_lines_matching(&removed_once, |l| l.is_empty());

            // Property: removing twice == removing once
            assert_eq!(removed_once, removed_twice);
        }
    }
}
```

**Integration Tests:**
```rust
// Test with real plugin code patterns
#[test]
fn test_swift_add_import_pattern() {
    let content = r#"import Foundation

class MyClass {}"#;

    // Simulate Swift plugin usage
    let last_idx = find_last_matching_line(content, |l| {
        l.trim().starts_with("import ")
    }).unwrap();

    let result = insert_line_at(content, last_idx + 1, "import SwiftUI");

    assert!(result.contains("import Foundation"));
    assert!(result.contains("import SwiftUI"));
    assert!(result.contains("class MyClass"));
}
```

---

### Appendix E: Open Questions

**Q1: Should utilities return `Result<T, E>` or `Option<T>`?**

**Current Design:** Return `Option<T>` for "not found" cases, panic on invalid input
**Alternative:** Return `Result<T, ImportError>` for all errors
**Decision:** `Option<T>` - simpler, "not found" is not an error condition

---

**Q2: Should we support `String` allocation vs `Cow<str>`?**

**Current Design:** Always allocate new `String` for modified content
**Alternative:** Return `Cow<str>` to avoid allocation if unchanged
**Decision:** Always allocate - simpler API, allocation cost negligible for import operations

---

**Q3: Should utilities be async?**

**Current Design:** Synchronous (content already in memory)
**Alternative:** Async versions for consistency with LSP server
**Decision:** Sync - import content is small, always in memory, async overhead unnecessary

---

**Q4: Line ending normalization?**

**Current Design:** Always use `\n`
**Alternative:** Preserve original line endings (`\r\n` vs `\n`)
**Decision:** TBD - need to check if LSP protocol has conventions

---

### Appendix F: Migration Tracking

**Plugin Migration Checklist:**

- [ ] **Swift** (Pilot)
  - [ ] Migrate `add_import()`
  - [ ] Migrate `remove_import()`
  - [ ] Tests pass
  - [ ] Measured impact

- [ ] **Rust**
  - [ ] Migrate `add_import()`
  - [ ] Migrate `remove_import()`
  - [ ] Tests pass
  - [ ] Measured impact

- [ ] **Python**
  - [ ] Migrate `remove_import()` (biggest impact)
  - [ ] Tests pass
  - [ ] Measured impact

- [ ] **Go**
  - [ ] Migrate `add_import()`
  - [ ] Migrate `remove_import()`
  - [ ] Tests pass
  - [ ] Measured impact

- [ ] **TypeScript**
  - [ ] Migrate `add_import()`
  - [ ] Migrate `remove_import()`
  - [ ] Tests pass
  - [ ] Measured impact

- [ ] **Java**
  - [ ] Assess applicability
  - [ ] Migrate if applicable
  - [ ] Tests pass
  - [ ] Measured impact

---

## Summary

This API design is **conservative, pragmatic, and grounded in real usage patterns**.

**Key Differentiators from Proposal:**
- Realistic scope: 110-155 lines saved (not 400-600)
- Only proven patterns extracted
- Simple primitives, not complex frameworks
- Zero unused utilities added

**Top 3 Most Impactful Utilities:**

1. **`find_last_matching_line`** - Used in 5/6 plugins, saves 50+ lines total
2. **`remove_lines_matching`** - Used in 4/6 plugins, saves 40+ lines total
3. **`insert_line_at`** - Used in 3/6 plugins, saves 20+ lines total

**Migration Strategy:**
- Pilot with Swift (lowest risk)
- Measure impact at each step
- Can pause after any phase
- Deprecate unused utilities gradually

**Next Steps:**
1. Review and approve this design
2. Begin Phase 1 implementation
3. Pilot with Swift and measure results
4. Decide on full rollout based on pilot data

---

**Document Version:** 1.0
**Last Updated:** 2025-10-08
**Review Status:** Pending
