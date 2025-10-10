# Language Plugin Common API

**Status**: Design Document
**Author**: Agent Bob
**Date**: 2025-10-10
**Purpose**: Define reusable primitives for import support utilities in cb-lang-common

---

## Executive Summary

This document defines minimal, high-value API for import support utilities in `cb-lang-common`, applicable to both **core languages** (TypeScript, Rust - bundled by default) and **external language plugins** (community-maintained).

**Key Findings:**
- Core primitives extracted from proven cross-language patterns
- 4 high-value utilities identified: `find_last_matching_line`, `insert_line_at`, `remove_lines_matching`, `find_first_non_matching_line`
- Conservative scope: ~100-120 lines of truly reusable code
- Most import code (85-90%) remains inherently language-specific

**Design Philosophy:**
1. **Extract from proven patterns** - Functions appearing in multiple plugin implementations
2. **Primitives over frameworks** - Simple, composable utilities
3. **Conservative scope** - Better to add later than remove unused code
4. **100% test coverage** - Every function thoroughly tested
5. **Zero abstraction penalty** - As fast as hand-written code

---

## Plugin Architecture Scope

- **Core plugins** (bundled): TypeScript, Rust
- **External plugins** (community): Can leverage these same utilities via cb-lang-common dependency
- All utilities are language-agnostic line-level primitives
- Plugin architecture unchanged - supports any language with proper implementation

---

## Design Principles

### 1. Extract from Proven Patterns

**What qualifies as "proven"?**
- Pattern appears in multiple language plugin implementations
- Identical or near-identical implementation
- No language-specific logic embedded

**Proven patterns identified:**
- Find last import line (multiple languages)
- Remove import lines matching predicate (multiple languages)
- Insert line after specific position (multiple languages)
- Find first non-matching line (multiple languages)

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

**Rationale:**
- Only 100-120 lines of new code
- Tight cohesion (all line-level operations)
- Easy to navigate
- Can split later if grows beyond 300 lines

---

## API Reference

### Module: `import_helpers.rs` (NEW)

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
/// # Usage Pattern (TypeScript)
/// ```rust
/// // Find last import to insert after it
/// let last_import_idx = find_last_matching_line(content, |line| {
///     line.trim().starts_with("import ")
/// });
/// ```
///
/// # Usage Pattern (Rust)
/// ```rust
/// // Find last use statement
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
- Used across multiple plugin implementations
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
/// # Usage Pattern (TypeScript)
/// ```rust
/// remove_lines_matching(content, |line| {
///     line.trim() == format!("import {}", module)
/// })
/// ```
///
/// # Usage Pattern (Rust)
/// ```rust
/// remove_lines_matching(content, |line| {
///     let trimmed = line.trim();
///     trimmed.starts_with("use ") && trimmed.contains(&format!("use {}", module))
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
- Identical pattern across multiple plugins - filter out import lines
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
/// # Usage Pattern (Python-style)
/// ```rust
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
// ✅ KEEP - Multiple plugins use similar patterns
pub fn parse_import_alias(text: &str) -> (String, Option<String>)

// ✅ KEEP - TypeScript and other plugins use this pattern
pub fn extract_package_name(path: &str) -> String

// ✅ KEEP - All plugins normalize paths
pub fn normalize_import_path(path: &str) -> String
```

**Rationale for removals:**
- `split_import_list`: No plugin implementation uses comma-separated import parsing
- `ExternalDependencyDetector`: Too complex, no plugin uses builder pattern

---

## Migration Guide

### For Plugin Authors

#### Generic Pattern: Finding and Inserting After Last Import

**Before (manual iteration):**
```rust
let mut lines: Vec<&str> = content.lines().collect();
let mut last_import_idx = None;

for (idx, line) in lines.iter().enumerate() {
    let trimmed = line.trim();
    if trimmed.starts_with("import ") { // or "use " for Rust
        last_import_idx = Some(idx);
    }
}

if let Some(idx) = last_import_idx {
    lines.insert(idx + 1, &new_import_line);
    lines.join("\n")
} else {
    format!("{}\n{}", new_import_line, content)
}
```

**After (using primitives):**
```rust
use cb_lang_common::import_helpers::{find_last_matching_line, insert_line_at};

let last_import_idx = find_last_matching_line(content, |line| {
    line.trim().starts_with("import ") // or "use " for Rust
});

match last_import_idx {
    Some(idx) => insert_line_at(content, idx + 1, &new_import_line),
    None => insert_line_at(content, 0, &new_import_line),
}
```

**Impact:** 11+ lines → 6 lines (~45% reduction), clearer intent

---

#### Generic Pattern: Removing Imports

**Before (manual filtering):**
```rust
let mut result = String::new();

for line in content.lines() {
    let trimmed = line.trim();
    let mut skip_line = false;

    if trimmed.starts_with("import ") {
        // Language-specific parsing logic
        if /* matches module to remove */ {
            skip_line = true;
        }
    }

    if !skip_line {
        result.push_str(line);
        result.push('\n');
    }
}

result
```

**After (using primitives):**
```rust
use cb_lang_common::import_helpers::remove_lines_matching;

remove_lines_matching(content, |line| {
    let trimmed = line.trim();
    // Language-specific matching logic
    trimmed.starts_with("import ") && /* matches module to remove */
})
```

**Impact:** 15+ lines → 5 lines (~65% reduction), clearer logic

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

## Success Metrics

### Qualitative Metrics

**Developer Experience:**
- ✅ Clearer intent in plugin code
- ✅ Less boilerplate
- ✅ Consistent patterns across plugins (core and external)
- ✅ Easier code reviews

**Maintainability:**
- ✅ Fewer places to fix bugs
- ✅ Centralized testing
- ✅ Documented patterns

**Performance:**
- Zero regression in benchmarks
- Import operations remain O(n)
- No additional allocations

**Quality:**
- Test coverage: 100%
- Clippy warnings: 0
- Documentation coverage: 100%

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

// ❌ ONLY 1 PLUGIN needs this
pub fn detect_relative_import(path: &str) -> bool
```

**4. AST-Based Parsing**
- AST parsing belongs in individual plugins
- Each language has unique AST structure
- Keep `syn`, `tree-sitter`, `swc` in language plugins

---

## Future Considerations

**If multiple plugins develop similar patterns:**
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

## Performance Benchmarks

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

## Testing Strategy

**Unit Tests (Per Function):**
- Happy path tests (3+ per function)
- Edge cases: empty content, no matches, all matches
- Performance tests: 10K line files
- Property-based tests with `proptest`
- Documentation examples compile and pass

**Integration Tests:**
```rust
// Test with real plugin code patterns
#[test]
fn test_typescript_add_import_pattern() {
    let content = r#"import { foo } from 'bar';

const x = 1;"#;

    let last_idx = find_last_matching_line(content, |l| {
        l.trim().starts_with("import ")
    }).unwrap();

    let result = insert_line_at(content, last_idx + 1, "import { baz } from 'qux';");

    assert!(result.contains("import { foo } from 'bar';"));
    assert!(result.contains("import { baz } from 'qux';"));
    assert!(result.contains("const x = 1;"));
}

#[test]
fn test_rust_add_use_pattern() {
    let content = r#"use std::collections::HashMap;

fn main() {}"#;

    let last_idx = find_last_matching_line(content, |l| {
        l.trim().starts_with("use ")
    }).unwrap();

    let result = insert_line_at(content, last_idx + 1, "use std::fs;");

    assert!(result.contains("use std::collections::HashMap;"));
    assert!(result.contains("use std::fs;"));
    assert!(result.contains("fn main()"));
}
```

---

## Design Decisions

**Q1: Should utilities return `Result<T, E>` or `Option<T>`?**
**Decision:** `Option<T>` - simpler, "not found" is not an error condition

**Q2: Should we support `String` allocation vs `Cow<str>`?**
**Decision:** Always allocate - simpler API, allocation cost negligible for import operations

**Q3: Should utilities be async?**
**Decision:** Sync - import content is small, always in memory, async overhead unnecessary

**Q4: Line ending normalization?**
**Decision:** Always use `\n` - LSP protocol standard

---

## Summary

This API design is **conservative, pragmatic, and grounded in real usage patterns**.

**Key Principles:**
- Realistic scope: 100-120 lines of shared utilities
- Only proven patterns extracted from multiple implementations
- Simple primitives, not complex frameworks
- Zero unused utilities added
- Applicable to both core and external plugins

**Top 4 Utilities:**
1. **`find_last_matching_line`** - Find last import for insertion
2. **`remove_lines_matching`** - Filter out import lines
3. **`insert_line_at`** - Insert line at specific position
4. **`find_first_non_matching_line`** - Find end of import block

**Adoption Strategy:**
- Purely additive (no breaking changes)
- Deprecate unused utilities gradually
- External plugins can adopt incrementally
- Performance benchmarks ensure zero overhead

---

**Document Version:** 2.0
**Last Updated:** 2025-10-10
**Review Status:** Active
