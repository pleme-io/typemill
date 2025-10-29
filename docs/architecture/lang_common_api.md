# Language Plugin Common API

## Scope
- Defines canonical contracts for line-level import primitives in `mill-lang-common`.
- Applies to core plugins (TypeScript, Rust) and external language plugins.
- Overrides any conflicting implementation details; code must conform to this spec.

## Module Structure

**Current (preserved):**
```text
mill-lang-common/src/
├── import_parsing.rs
├── import_graph.rs
└── lib.rs
```text
**Addition:**
```text
mill-lang-common/src/
└── import_helpers.rs     # 4 functions, ~100 lines
```text
---

## API Contracts

### `find_last_matching_line`

```rust
pub fn find_last_matching_line<F>(content: &str, predicate: F) -> Option<usize>
where F: Fn(&str) -> bool
```text
**Contract:**
- **Input**: UTF-8 string content, predicate function
- **Output**: `Some(index)` for last matching line (0-based), `None` if no match
- **Complexity**: O(n) single pass, no early exit
- **Panics**: Never
- **Thread safety**: Safe (immutable inputs)

**Invariants:**
- Line indices are 0-based and count from start
- Empty content always returns `None`
- Predicate receives each line exactly once in order
- Return value < total line count when `Some`

**Edge cases:**
```rust
find_last_matching_line("", |_| true)           // → None
find_last_matching_line("a\nb", |l| l == "b")  // → Some(1)
find_last_matching_line("a\na", |l| l == "a")  // → Some(1) (last occurrence)
```text
---

### `insert_line_at`

```rust
pub fn insert_line_at(content: &str, position: usize, new_line: &str) -> String
```text
**Contract:**
- **Input**: UTF-8 content, 0-based position, UTF-8 line to insert
- **Output**: New string with line inserted
- **Complexity**: O(n) - split, insert, join
- **Panics**: Never
- **Allocation**: Always allocates new `String`

**Behavior:**
- `position < line_count`: Insert at exact position (shifts existing lines down)
- `position >= line_count`: Append to end
- Empty content: Returns `new_line` only
- Line separator: Always `\n` (LF)

**Invariants:**
- Result line count = original count + 1
- All original lines preserved in order
- No trailing newline added

**Edge cases:**
```rust
insert_line_at("", 0, "x")              // → "x"
insert_line_at("", 999, "x")            // → "x"
insert_line_at("a\nb", 1, "x")          // → "a\nx\nb"
insert_line_at("a\nb", 999, "x")        // → "a\nb\nx"
```text
---

### `remove_lines_matching`

```rust
pub fn remove_lines_matching<F>(content: &str, predicate: F) -> String
where F: Fn(&str) -> bool
```text
**Contract:**
- **Input**: UTF-8 content, predicate function
- **Output**: New string with matching lines removed
- **Complexity**: O(n) single pass filter
- **Panics**: Never
- **Allocation**: Always allocates new `String`

**Behavior:**
- Lines where `predicate(line) == true` are excluded
- Preserved lines maintain original order
- Line separator: Always `\n` (LF)
- All matches removed (no partial filtering)

**Invariants:**
- Result line count ≤ original line count
- No line appears in result if predicate returned `true` for it
- All non-matching lines preserved exactly

**Edge cases:**
```rust
remove_lines_matching("", |_| true)                 // → ""
remove_lines_matching("a\nb\nc", |_| false)        // → "a\nb\nc"
remove_lines_matching("a\nb\nc", |_| true)         // → ""
remove_lines_matching("a\nb", |l| l == "b")        // → "a"
```text
---

### `find_first_non_matching_line`

```rust
pub fn find_first_non_matching_line<F>(content: &str, predicate: F) -> Option<usize>
where F: Fn(&str) -> bool
```text
**Contract:**
- **Input**: UTF-8 content, predicate function
- **Output**: `Some(index)` for first non-matching line (0-based), `None` if all match
- **Complexity**: O(n) with early exit on first non-match
- **Panics**: Never
- **Thread safety**: Safe (immutable inputs)

**Invariants:**
- Returns first line where `predicate(line) == false`
- Empty content always returns `None`
- If returned, all lines before index matched predicate
- Return value < total line count when `Some`

**Edge cases:**
```rust
find_first_non_matching_line("", |_| true)              // → None
find_first_non_matching_line("a\nb", |_| true)         // → None (all match)
find_first_non_matching_line("a\nb", |l| l == "a")     // → Some(1)
find_first_non_matching_line("x\ny", |l| l == "a")     // → Some(0) (first non-match)
```text
---

## Modifications to `import_parsing.rs`

### Immediate Removals (No Deprecation)

**Delete:**
- `split_import_list(text: &str) -> Vec<(String, Option<String>)>` - Unused by any plugin
- `ExternalDependencyDetector` struct - Unused by any plugin

**Rationale:** No deprecation window needed (beta product, no external users)

### Preserved Functions

**Keep:**
- `parse_import_alias(text: &str) -> (String, Option<String>)` - Used by TypeScript, Python plugins
- `extract_package_name(path: &str) -> String` - Used by TypeScript plugin
- `normalize_import_path(path: &str) -> String` - Used by all plugins

---

## Performance Requirements

### Acceptance Criteria (CI Gates)

**Benchmarks must pass:**
```rust
// Gate 1: 10K line file operations
bench_find_last_import_10k_lines:  max 1.0ms  (fail if > 1ms)
bench_insert_line_at_middle:       max 1.5ms  (fail if > 1.5ms)
bench_remove_lines_matching:       max 1.2ms  (fail if > 1.2ms)
bench_find_first_non_matching:     max 0.8ms  (fail if > 0.8ms)

// Gate 2: Allocation overhead
All functions: max 5% overhead vs hand-written loops (fail if > 5%)

// Gate 3: Scaling
100K line files: max 10x of 10K baseline (fail if > 10x)
```text
**Failure action:** Block merge until optimized

**Benchmark suite (required):**
```rust
# [bench]
fn bench_find_last_import_10k_lines(b: &mut Bencher) {
    let content = generate_test_file(10_000);
    b.iter(|| {
        find_last_matching_line(&content, |line| line.trim().starts_with("import "))
    });
}

# [bench]
fn bench_insert_line_at_middle(b: &mut Bencher) {
    let content = generate_test_file(10_000);
    b.iter(|| {
        insert_line_at(&content, 5_000, "import new")
    });
}

# [bench]
fn bench_remove_lines_matching(b: &mut Bencher) {
    let content = generate_test_file_with_imports(10_000, 50);
    b.iter(|| {
        remove_lines_matching(&content, |l| l.trim().starts_with("import "))
    });
}
```text
---

## Testing Requirements

### CI Gates (Mandatory)

**Unit tests (per function):**
- ✅ Happy path: 3+ tests per function (fail if < 3)
- ✅ Edge cases: empty content, no matches, all matches (fail if missing)
- ✅ Boundary: first line, last line, middle (fail if missing)
- ✅ Property-based: `proptest` with 1000 cases (fail if < 1000)

**Integration tests:**
- ✅ TypeScript import pattern (fail if missing)
- ✅ Rust use statement pattern (fail if missing)
- ✅ Cross-function composition (fail if missing)

**Documentation:**
- ✅ All doc examples compile (fail if any error)
- ✅ All doc examples pass `cargo test --doc` (fail if any failure)

**Coverage:**
- ✅ Line coverage: 100% (fail if < 100%)
- ✅ Branch coverage: 100% (fail if < 100%)

**Quality:**
- ✅ `cargo clippy`: 0 warnings (fail if any)
- ✅ `cargo fmt --check`: no changes (fail if dirty)

### Required Test Cases

**`find_last_matching_line`:**
```rust
# [test] fn returns_none_for_empty_content()
# [test] fn returns_none_when_no_match()
# [test] fn returns_last_match_when_multiple()
# [test] fn returns_only_match_when_single()
```text
**`insert_line_at`:**
```rust
# [test] fn inserts_at_beginning()
# [test] fn inserts_in_middle()
# [test] fn appends_when_position_beyond_end()
# [test] fn handles_empty_content()
```text
**`remove_lines_matching`:**
```rust
# [test] fn removes_all_matching_lines()
# [test] fn preserves_non_matching_lines()
# [test] fn returns_empty_when_all_match()
# [test] fn returns_unchanged_when_none_match()
```text
**`find_first_non_matching_line`:**
```rust
# [test] fn returns_first_non_match()
# [test] fn returns_none_when_all_match()
# [test] fn returns_zero_when_first_non_match()
```text
---

## Error Handling

**Contract:** All functions are infallible (no `Result` return types).

**Rationale:**
- "Not found" is not an error (return `Option::None`)
- Invalid position handled gracefully (append to end)
- Empty content is valid input

**Panic policy:** Functions must never panic. Violations constitute contract breach.

---

## Thread Safety

**Contract:** All functions are `Send + Sync` safe.

**Guarantees:**
- Immutable input references only
- No shared mutable state
- No interior mutability
- Safe for concurrent execution from multiple threads

---

## Migration Path

### For Plugin Authors

**Before:**
```rust
let mut lines: Vec<&str> = content.lines().collect();
let mut last_import_idx = None;
for (idx, line) in lines.iter().enumerate() {
    if line.trim().starts_with("import ") {
        last_import_idx = Some(idx);
    }
}
if let Some(idx) = last_import_idx {
    lines.insert(idx + 1, &new_import_line);
    lines.join("\n")
} else {
    format!("{}\n{}", new_import_line, content)
}
```text
**After:**
```rust
use cb_lang_common::import_helpers::{find_last_matching_line, insert_line_at};

let last_idx = find_last_matching_line(content, |l| l.trim().starts_with("import "));
match last_idx {
    Some(idx) => insert_line_at(content, idx + 1, &new_import_line),
    None => insert_line_at(content, 0, &new_import_line),
}
```text
**Benefits:** 15 lines → 6 lines, no manual index tracking, guaranteed correctness

---

## Out of Scope (Explicitly Excluded)

**Language-specific logic:**
- Python docstring skipping
- Go import block parsing
- Any AST-based manipulation

**Complex abstractions:**
- Builder patterns
- Import rewriters
- Style formatters

**Anticipatory features:**
- Import sorting/grouping
- Multi-line handling
- Relative import detection

**Rationale:** Keep primitives language-agnostic. Complex logic belongs in individual plugins.

---

## Design Decisions (Locked)

**Q: Return `Result<T, E>` or `Option<T>`?**
**A:** `Option<T>` - "not found" is not an error condition. Locked.

**Q: String allocation vs `Cow<str>`?**
**A:** Always allocate - simpler API, 3-5% overhead acceptable for import ops. Locked.

**Q: Async functions?**
**A:** Synchronous - content in memory, async overhead (20-30%) not justified. Locked.

**Q: Line ending normalization?**
**A:** Always `\n` - LSP protocol standard, simplifies cross-platform. Locked.

---

## Contract Validation

**CI must enforce:**
- All benchmarks pass thresholds
- All test gates pass (100% coverage)
- Zero Clippy warnings
- All doc examples compile and pass
- Functions never panic (verified via `#[should_panic]` absence)

**Breaking changes:**
- Require major version bump
- Must document migration path
- Beta product: no backwards compatibility guarantees

---

## Summary

**4 infallible primitives:**
1. `find_last_matching_line(content, predicate) -> Option<usize>`
2. `insert_line_at(content, position, new_line) -> String`
3. `remove_lines_matching(content, predicate) -> String`
4. `find_first_non_matching_line(content, predicate) -> Option<usize>`

**Guarantees:**
- O(n) or better complexity
- Thread-safe (Send + Sync)
- Never panic
- 100% test coverage
- Performance gates enforced

**Scope:** ~100 lines of production code, language-agnostic primitives only