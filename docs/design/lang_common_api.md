# Language Plugin Common API

## Scope
- Defines reusable line-level primitives for import manipulation in `cb-lang-common`.
- Applicable to core languages (TypeScript, Rust) and external language plugins.
- Extractable utilities proven across multiple language implementations.

## Plugin Architecture
- **Core plugins** (bundled): TypeScript, Rust
- **External plugins** (community): Access via `cb-lang-common` dependency
- All utilities are language-agnostic line-level primitives
- Plugin architecture unchanged - supports any language with proper implementation

## Design Principles
1. **Extract from proven patterns** - Functions appearing in multiple plugin implementations
2. **Primitives over frameworks** - Simple, composable utilities
3. **Conservative scope** - Better to add later than remove unused code
4. **Zero abstraction penalty** - As fast as hand-written code
5. **100% test coverage** - Every function thoroughly tested

---

## Module Structure

### Current (Preserved)
```
cb-lang-common/src/
├── import_parsing.rs     # Existing utilities
├── import_graph.rs       # ImportGraphBuilder
└── lib.rs                # Public exports
```

### Addition
```
cb-lang-common/src/
└── import_helpers.rs     # NEW: Line-level operations (4 functions, ~100 lines)
```

---

## API Reference

### `find_last_matching_line`

```rust
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

**Purpose:** Find 0-based index of last line matching predicate. Returns `None` if no match.

**Complexity:** O(n) - single pass

**Usage:**
```rust
// TypeScript: find last import
find_last_matching_line(content, |line| line.trim().starts_with("import "))

// Rust: find last use statement
find_last_matching_line(content, |line| line.trim().starts_with("use "))
```

---

### `insert_line_at`

```rust
pub fn insert_line_at(content: &str, position: usize, new_line: &str) -> String {
    let mut lines: Vec<&str> = content.lines().collect();

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

**Purpose:** Insert line at 0-based position. Appends if position beyond end.

**Complexity:** O(n) - split, insert, join

**Edge cases:**
- Position beyond end → appends to end
- Empty content → returns new_line only

---

### `remove_lines_matching`

```rust
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

**Purpose:** Filter out all lines where predicate returns `true`.

**Complexity:** O(n) - single pass filter

**Usage:**
```rust
// Remove TypeScript imports
remove_lines_matching(content, |line| {
    line.trim() == format!("import {}", module)
})

// Remove Rust use statements
remove_lines_matching(content, |line| {
    let t = line.trim();
    t.starts_with("use ") && t.contains(&format!("use {}", module))
})
```

---

### `find_first_non_matching_line`

```rust
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

**Purpose:** Find first line that does NOT match predicate. Useful for finding end of import blocks.

**Complexity:** O(n) - early exit on first non-match

**Usage:**
```rust
// Find where imports end (Python-style)
find_first_non_matching_line(content, |line| {
    let t = line.trim();
    t.starts_with("import ") || t.starts_with("from ") || t.is_empty()
}).unwrap_or(0)
```

---

## Modifications to Existing Code

### `import_parsing.rs` Changes

**Remove (unused):**
- `split_import_list(text: &str) -> Vec<(String, Option<String>)>` - No plugin uses comma-separated import parsing
- `ExternalDependencyDetector` struct - No plugin uses builder pattern

**Keep (proven usage):**
- `parse_import_alias(text: &str) -> (String, Option<String>)` - Multiple plugins use
- `extract_package_name(path: &str) -> String` - TypeScript and others use
- `normalize_import_path(path: &str) -> String` - All plugins normalize paths

**Deprecation:**
```rust
#[deprecated(since = "0.2.0", note = "Unused by any plugin, will be removed in 0.3.0")]
pub fn split_import_list(text: &str) -> Vec<(String, Option<String>)> { /* ... */ }
```

---

## Migration Pattern

### Before (manual iteration)
```rust
let mut lines: Vec<&str> = content.lines().collect();
let mut last_import_idx = None;

for (idx, line) in lines.iter().enumerate() {
    let trimmed = line.trim();
    if trimmed.starts_with("import ") {
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

### After (using primitives)
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

**Impact:** 15 lines → 6 lines (60% reduction), clearer intent

---

## Performance Requirements

**Target benchmarks:**
- 10K line files: < 1ms per operation
- No allocation penalty vs hand-written code
- Linear scaling O(n) with content size

**Benchmark suite:**
```rust
#[bench]
fn bench_find_last_import_10k_lines(b: &mut Bencher) {
    let content = generate_test_file(10_000);
    b.iter(|| {
        find_last_matching_line(&content, |line| line.trim().starts_with("import "))
    });
}
```

---

## Testing Requirements

**Unit tests (per function):**
- Happy path (3+ tests per function)
- Edge cases: empty content, no matches, all matches
- Performance tests: 10K line files
- Property-based tests with `proptest`
- Doc examples must compile and pass

**Integration tests:**
```rust
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
}
```

---

## Design Decisions

**Q: Return `Result<T, E>` or `Option<T>`?**
**A:** `Option<T>` - "not found" is not an error condition

**Q: String allocation vs `Cow<str>`?**
**A:** Always allocate - simpler API, negligible cost for import operations

**Q: Async functions?**
**A:** Synchronous - content is small, already in memory, async overhead unnecessary

**Q: Line ending normalization?**
**A:** Always use `\n` - LSP protocol standard

---

## Out of Scope

### Excluded from cb-lang-common

**Language-specific logic:**
```rust
// ❌ Python-specific
fn skip_docstrings_and_shebang(content: &str) -> usize

// ❌ Go-specific
fn parse_import_block(content: &str) -> Vec<String>
```

**Complex parsers:**
```rust
// ❌ Requires AST parsing - keep in plugins
pub struct ImportRewriter {
    language: Language,
    style: ImportStyle,
}
```

**Anticipatory features:**
```rust
// ❌ No plugin uses yet
pub fn sort_imports_by_group(imports: Vec<String>) -> Vec<String>

// ❌ Only 1 plugin needs
pub fn detect_relative_import(path: &str) -> bool
```

**AST-based parsing:**
- Each language has unique AST structure
- Keep `syn`, `tree-sitter`, `swc` in language plugins

---

## Future Considerations

**Add if 3+ plugins develop similar patterns:**
- Import grouping/sorting utilities
- Import statement normalization
- Multi-line import handling helpers

**Possible custom Clippy lint:**
```rust
#[warn(manual_import_iteration)]
// Suggests: use cb_lang_common::import_helpers::find_last_matching_line
```

---

## Contract Summary

**4 core utilities:**
1. `find_last_matching_line` - Find last import for insertion
2. `insert_line_at` - Insert line at position
3. `remove_lines_matching` - Filter out import lines
4. `find_first_non_matching_line` - Find end of import block

**Scope:** ~100-120 lines of truly reusable code

**Adoption:** Purely additive, no breaking changes, external plugins can adopt incrementally

**Quality gates:** 100% test coverage, zero Clippy warnings, all doc examples pass, benchmarks show zero overhead
