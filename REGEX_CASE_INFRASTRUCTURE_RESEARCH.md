# Codebase Regex Handling and Case-Related Utilities Research

## Executive Summary

The codebase has mature regex infrastructure and some basic case conversion utilities, but **lacks case preservation/detection logic** for find-and-replace operations. Key infrastructure exists for pattern matching, but no mechanism to detect or preserve case styles (snake_case, camelCase, PascalCase, kebab-case) during batch replacements.

---

## 1. Existing Regex Infrastructure

### 1.1 Regex Libraries Used

**Standard regex crate (1.10)**
- Used in: `cb-lang-rust`, `mill-lang-typescript`, `mill-lang-common`, `mill-handlers`, `mill-ast`
- Purpose: Basic regex matching without lookahead/lookbehind

**fancy-regex (0.13)** 
- Used in: `cb-lang-rust`, `mill-lang-markdown`, `mill-lang-toml`
- Features: Lookahead, lookbehind, word boundaries
- Current use: Comment updates with word boundary assertions

### 1.2 Glob/Pattern Matching Libraries

**glob (0.3)**
- Used in: `apps/codebuddy`, `crates/mill-foundation`, `crates/mill-config`
- Purpose: File glob pattern matching
- Method: `glob::Pattern::new()` for simple glob compilation

**globset (0.4.10)**
- Used in: `crates/mill-handlers`, `crates/mill-services`
- Purpose: Efficient multi-pattern glob matching
- Current use: File exclusion patterns in rename operations

---

## 2. Existing Regex-Based Find/Replace Operations

### 2.1 String Literal Path Rewriting (Most Advanced)
**File**: `/workspace/crates/cb-lang-rust/src/string_literal_support.rs`

Features:
- Detects and updates path-like string literals during renames
- Handles both regular strings and raw strings (r"...", r#"..."#, etc.)
- Conservative heuristic: only matches strings with `/`, `\`, or file extensions
- Preserves non-path prose text (important!)
- Handles nested renames with idempotency checks
- Prevents false positives on URLs, version numbers

Regex patterns used:
```rust
// Regular strings: "..." (skips raw strings)
r#""([^"\\]*(\\.[^"\\]*)*)""#

// Raw strings with hashes
r"...", r#"..."#, r##"..."##, etc. (multiple patterns)
```

### 2.2 Markdown Link Updates
**File**: `/workspace/crates/mill-lang-markdown/src/import_support_impl.rs`

Regex patterns:
- Inline links: `!?\[([^]]+)\]\(([^)]+)\)` - handles both `[text](path)` and `![alt](path)`
- Reference definitions: `(?m)^\s*\[([^]]+)\]:\s*(\S+)` - multi-line mode
- Autolinks: `<([^>]+)>` - paths in angle brackets
- Inline code: `` `([^`]+[/\\][^`]*)` `` - backtick code containing paths
- Prose paths: `([a-zA-Z0-9_-]+/[a-zA-Z0-9_/.-]*)` - basic path pattern

### 2.3 TOML Path Updates
**File**: `/workspace/crates/mill-lang-toml/src/lib.rs`

Uses fancy-regex with lookahead/lookbehind:
```rust
// Word boundary assertion for comment updates
let pattern = format!(
    r"(?<![a-zA-Z0-9]){}(?![a-zA-Z0-9])",
    fancy_regex::escape(old_basename)
);
```

### 2.4 Import Statement Detection
**File**: `/workspace/crates/mill-handlers/src/handlers/tools/analysis/dependencies.rs`

Simple regex for Rust imports:
```rust
let import_regex = Regex::new(r"use\s+.*;").unwrap();
```

### 2.5 Inline Crate References
**File**: `/workspace/crates/mill-ast/src/import_updater/reference_finder.rs`

Pattern matching without regex - uses string methods:
- Finds `crate_name::` patterns
- Word boundary detection via character inspection
- Extracts full qualified paths

---

## 3. Case Conversion Utilities

### 3.1 Naming Convention Converter (CLI-Focused)
**File**: `/workspace/apps/codebuddy/src/cli/conventions.rs`

**Supported Case Styles**:
- `kebab-case` - splits on `-`, joins with `-`
- `snake_case` - splits on `_`, joins with `_`
- `camelCase` - splits on uppercase letters, first word lowercase
- `PascalCase` - splits on uppercase letters, all words capitalized

**Algorithm**:
1. `split_by_convention()` - Parse source case style to words
2. `join_by_convention()` - Reconstruct in target case style

**Key Functions**:
```rust
pub fn convert_filename(filename: &str, from: &str, to: &str) -> Option<String>
fn split_by_convention(s: &str, convention: &str) -> Option<Vec<String>>
fn join_by_convention(words: &[String], convention: &str) -> String
fn capitalize_first(s: &str) -> String
```

**Limitations**:
- Works on filenames (extracts stem, preserves extension)
- Simple camelCase/PascalCase splitting on capital letters only
- No support for acronyms (e.g., "XMLParser")
- Only used in CLI for explicit conversions, NOT in refactoring operations

### 3.2 Where Case Conversion Is NOT Used

**Rename operations** (`rename.plan`):
- No case preservation/detection during path renames
- String literal updates use literal replacement only
- Import updates use simple string replacement

**Symbol renaming**:
- No automatic case style adjustments
- User provides new name explicitly

---

## 4. Glob Pattern Infrastructure

### 4.1 Glob Pattern Usage
**File**: `/workspace/crates/mill-foundation/src/core/rename_scope.rs`

```rust
pub fn should_include_file(&self, path: &Path) -> bool {
    // Check exclude patterns first
    let path_str = path.to_string_lossy();
    for pattern in &self.exclude_patterns {
        if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
            if glob_pattern.matches(&path_str) {
                return false;
            }
        }
    }
    // ... extension-based checks
}
```

Supports:
- Simple glob patterns: `**/test_*`, `**/fixtures/**`
- File exclusion during batch operations
- Error handling for invalid patterns

### 4.2 File Discovery with Globset
**File**: `/workspace/crates/mill-services/src/services/file_service/basic_ops.rs`

Uses `ignore::WalkBuilder` for efficient directory traversal:
- Respects `.gitignore` patterns
- Efficient multi-pattern matching via globset
- Used in batch analysis

---

## 5. Current Refactoring Architecture

### 5.1 Rename Scope Configuration
**File**: `/workspace/crates/mill-foundation/src/core/rename_scope.rs`

Configuration controls what gets updated:
- `update_code` - Import statements and symbol references
- `update_string_literals` - Path strings in code
- `update_docs` - Markdown file links
- `update_configs` - TOML, YAML configurations
- `update_comments` - Code comments (opt-in, experimental)
- `update_markdown_prose` - Prose text paths (opt-in, high false positive risk)
- `update_exact_matches` - Non-path strings in configs (opt-in)
- `exclude_patterns` - Glob patterns for exclusions

### 5.2 Import Helper Utilities
**File**: `/workspace/crates/mill-lang-common/src/import_helpers.rs`

Primitives for import manipulation:
```rust
pub fn find_last_matching_line<F>(content: &str, predicate: F) -> Option<usize>
pub fn insert_line_at(content: &str, line_index: usize, new_line: &str) -> String
pub fn remove_lines_matching<F>(content: &str, predicate: F) -> (String, usize)
pub fn replace_in_lines(content: &str, pattern: &str, replacement: &str) -> (String, usize)
```

These are **basic string operations**, not regex-based.

---

## 6. What's Missing: Case Preservation/Detection

### 6.1 The Gap

There is **NO infrastructure** for:

1. **Case Style Detection**
   - Cannot detect what case style a string uses
   - No heuristic to identify snake_case vs camelCase vs PascalCase

2. **Automatic Case Preservation**
   - When renaming `user_name` → `user_id`, can't automatically convert to `userId` in camelCase contexts
   - String replacements are literal only

3. **Case-Aware Pattern Matching**
   - No regex patterns that match across case variations
   - Can't find "UserName", "user_name", "user-name" with single pattern

4. **Batch Case Conversion**
   - The `conventions.rs` case converter is CLI-only
   - Not integrated into refactoring pipelines
   - No batch operation over multiple files

### 6.2 Why This Matters

Current behavior:
```rust
// In string: "user_name"
// Rename to: "user_id"
// Result: "user_id" (correct)

// But in camelCase variable: "userName"
// Rename operation: "user_name" → "user_id"
// Result: "userName" (unchanged - still points to old name!)
```

---

## 7. Recommended Infrastructure for Implementation

### 7.1 New Utilities Needed

**Case Detection**:
```rust
fn detect_case_style(s: &str) -> CaseStyle {
    // Returns: Snake, Kebab, Camel, Pascal, Identifier, Unknown
}

enum CaseStyle {
    Snake,      // user_name
    Kebab,      // user-name
    Camel,      // userName
    Pascal,     // UserName
    Identifier, // userName (for when we can't convert)
    Unknown,    // various_Mix-Case patterns
}
```

**Case-Aware Find**:
```rust
fn find_case_variants(base_name: &str) -> Vec<String> {
    // Returns: ["user_name", "userName", "UserName", "user-name"]
}

fn build_case_insensitive_pattern(base_name: &str, preserve_case: bool) -> String {
    // Build regex pattern matching all case variations
    // If preserve_case: track which variant matched and apply same case to replacement
}
```

**Replace with Case Preservation**:
```rust
fn replace_with_case_preservation(
    content: &str,
    old_name: &str,
    new_name: &str,
) -> (String, usize) {
    // 1. Detect case style of old_name
    // 2. Find all case variants
    // 3. For each match, apply detected case style to new_name
    // 4. Return updated content and match count
}
```

### 7.2 Integration Points

1. **String literal updates**: Extend `string_literal_support.rs`
2. **Import statements**: Extend language plugins' import support
3. **CLI batch operations**: Could use `conventions.rs` converter

### 7.3 Complexity Considerations

- **Acronyms**: "XMLParser" → split as "xml", "parser" or "x", "m", "l", "parser"?
- **Consecutive capitals**: "HTTPServer" vs "HttpServer"
- **Numbers**: "user2name" → snake_case, camelCase?
- **Special chars**: "user_name-test" → hybrid case (keep as-is, don't convert?)
- **Rare cases**: "a", "A", "AB", single/double letter handling

---

## 8. Summary of Available Infrastructure

### ✅ Present
- Regex library ecosystem (regex, fancy-regex)
- Glob/globset for pattern matching
- String literal update logic with smart heuristics
- Markdown/TOML/Rust-specific path updating
- Import manipulation primitives
- CLI case conversion utilities
- Rename scope configuration system

### ❌ Missing
- Case style detection function
- Case-aware regex patterns
- Case preservation during batch operations
- Integration of case conversion into refactoring pipelines
- Batch rename with case variation handling

### ⚠️ Experimental/Opt-In
- Comment updates with fancy-regex (experimental)
- Markdown prose path updates (opt-in due to false positives)
- Exact identifier matches in config files (opt-in)

---

## 9. Crate Organization

**Where to implement new utilities**:

1. **Case detection**: 
   - New module: `crates/mill-lang-common/src/case_detection.rs`
   - Or extend: `apps/codebuddy/src/cli/conventions.rs` with detection logic

2. **Case-aware find/replace**:
   - New module: `crates/mill-ast/src/case_aware_replace.rs`
   - Or in: `crates/cb-lang-common/` (shared language utilities)

3. **Integration points**:
   - Update `crates/mill-lang-rust/src/string_literal_support.rs`
   - Update language plugin import support interfaces

---

## References

Key files analyzed:
- `/workspace/apps/codebuddy/src/cli/conventions.rs` - Case conversion logic
- `/workspace/crates/cb-lang-rust/src/string_literal_support.rs` - Smart literal updates
- `/workspace/crates/mill-lang-markdown/src/import_support_impl.rs` - Markdown regex patterns
- `/workspace/crates/mill-lang-toml/src/lib.rs` - TOML updates with fancy-regex
- `/workspace/crates/mill-foundation/src/core/rename_scope.rs` - Scope configuration
- `/workspace/crates/mill-lang-common/src/import_helpers.rs` - Import manipulation primitives

