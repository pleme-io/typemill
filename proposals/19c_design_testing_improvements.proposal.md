# Proposal 19c: Design Consistency & Testing Improvements

**Status**: Ready for Implementation
**Scope**: C#, Go, Swift - Standardize design patterns and expand test coverage
**Priority**: MEDIUM

## Problem

The C#, Go, and Swift language plugins contain **20 design inconsistencies and test coverage gaps** that reduce maintainability and confidence:

**Design Inconsistencies (8 issues)**:

```rust
// C# and Swift use the macro:
define_language_plugin! {
    struct: CsharpPlugin,
    name: "csharp",
    // ... generates boilerplate
}

// Go manually implements everything (inconsistent):
pub const METADATA: LanguageMetadata = LanguageMetadata { ... };
pub const CAPABILITIES: PluginCapabilities = PluginCapabilities { ... };
pub struct GoPlugin {
    import_support: import_support::GoImportSupport,
    // Manual field definition
}
```

**Test Coverage Gaps (12 issues)**:
- No error path testing (what happens with malformed input?)
- No Unicode handling tests (日本語, русский, عربي module names)
- No large file performance tests (>1MB source code)
- No concurrent access tests
- No boundary condition tests (empty files, single char, no newlines)

**Impact**:
- More code to maintain (Go has ~70 extra lines of boilerplate)
- Inconsistent patterns make cross-plugin changes harder
- Unknown behavior with edge cases
- Potential performance issues undiscovered

## Solution

Standardize Go plugin design to match C# and Swift, extract hardcoded values to constants, improve pattern matching to reduce false positives, and expand test coverage to catch edge cases.

**All tasks should be completed in one implementation session** to ensure comprehensive quality improvements.

## Checklists

### Standardize Go Plugin Design

- [ ] Migrate Go to use `define_language_plugin!` macro
- [ ] Remove manual `METADATA` and `CAPABILITIES` constants
- [ ] Align field structure with C# and Swift
- [ ] Update tests to match new structure
- [ ] Verify behavior unchanged (30/30 tests still pass)
- [ ] Verify ~70 lines of boilerplate eliminated

### Extract Constants Across All Plugins

- [ ] Create `constants.rs` in each plugin (C#, Go, Swift)
- [ ] Extract all hardcoded regex patterns to constants
- [ ] Extract version numbers to constants
- [ ] Document constant meanings with rustdoc
- [ ] Use `const` or `lazy_static!` as appropriate

### Improve Pattern Matching

#### C# Pattern Fixes
- [ ] Add regex word boundaries to `using` statement matcher (lib.rs:170)
- [ ] Exclude matches inside comments (// and /* */)
- [ ] Exclude matches inside string literals
- [ ] Handle using aliases: `using Alias = Namespace;`
- [ ] Add tests for false positive cases

#### Go Pattern Fixes
- [ ] Add context-aware import statement detection
- [ ] Improve qualified path matching to avoid strings
- [ ] Test with edge cases (multiline imports, comments)

#### Swift Pattern Fixes
- [ ] Improve import statement regex precision
- [ ] Handle Swift-specific import syntax (`import class`, `import func`)
- [ ] Add tests for Unicode module names

### Standardize Error Messages

- [ ] Define error message format standard across plugins
- [ ] Align error messages to follow consistent pattern
- [ ] Add error codes for categorization (optional)
- [ ] Document all error conditions in rustdoc

### Expand Test Coverage

#### Error Path Tests
- [ ] C#: Test invalid source code handling
- [ ] C#: Test malformed .csproj parsing
- [ ] Go: Test malformed go.mod parsing
- [ ] Go: Test invalid module names
- [ ] Swift: Test invalid Package.swift
- [ ] Swift: Test malformed package definitions
- [ ] All: Test file read failures
- [ ] All: Test empty file handling

#### Edge Case Tests
- [ ] Test Unicode module names (日本語, русский, عربي)
- [ ] Test extremely long lines (>10,000 chars)
- [ ] Test files with no newlines
- [ ] Test files with only whitespace
- [ ] Test boundary conditions (line 0, column 0)
- [ ] Test files with mixed line endings (CRLF/LF)

#### Performance Tests
- [ ] Test large files (>1MB source code)
- [ ] Test scanning with >10,000 references
- [ ] Benchmark pattern matching performance
- [ ] Add performance regression guards
- [ ] Document expected performance characteristics

#### Robustness Tests
- [ ] Test with invalid UTF-8 sequences
- [ ] Test with binary files (should error gracefully)
- [ ] Test with symbolic links
- [ ] Test with read-only files

### Documentation Updates

- [ ] Document all error conditions in rustdoc
- [ ] Add safety guarantees to public APIs
- [ ] Document panic conditions (should be zero after 19a)
- [ ] Add performance characteristics
- [ ] Document Unicode support status
- [ ] Add troubleshooting guide for common issues

### Verification

- [ ] Run `cargo clippy --all-targets -- -D warnings`
- [ ] Verify all existing tests still pass (64/64)
- [ ] Run new edge case tests
- [ ] Run performance benchmarks
- [ ] Verify no performance regression >5%
- [ ] Check code coverage metrics (aim for >85%)

## Success Criteria

- [ ] Go plugin uses `define_language_plugin!` macro (consistent with C#/Swift)
- [ ] All hardcoded values extracted to named constants
- [ ] Pattern matching excludes comments and strings
- [ ] Consistent error message format across plugins
- [ ] +30 new tests added (error paths, edge cases, performance)
- [ ] All 64 existing tests continue to pass
- [ ] No performance regression >5%
- [ ] Complete rustdoc coverage for public APIs

## Benefits

- **Maintainability**: Consistent design patterns across all plugins (~70 lines less boilerplate)
- **Correctness**: Pattern matching improvements reduce false positives
- **Robustness**: Edge case tests catch unexpected behavior before production
- **Performance**: Benchmark tests prevent regression
- **Developer Experience**: Better error messages and complete documentation
- **Confidence**: Comprehensive test coverage enables safe refactoring

## Implementation Notes

### Use define_language_plugin! Macro for Go

**Before** (manual implementation):
```rust
pub const METADATA: LanguageMetadata = LanguageMetadata {
    name: "go",
    extensions: vec!["go".to_string()],
    // ... ~40 lines of boilerplate
};

pub const CAPABILITIES: PluginCapabilities = PluginCapabilities {
    // ... ~30 lines more
};

pub struct GoPlugin {
    import_support: import_support::GoImportSupport,
    // ... manual fields
}
```

**After** (macro-generated):
```rust
define_language_plugin! {
    struct: GoPlugin,
    name: "go",
    extensions: ["go"],
    manifest: "go.mod",
    lsp_command: "gopls",
    capabilities: [with_imports, with_project_factory, with_workspace],
    fields: {
        import_support: import_support::GoImportSupport,
        project_factory: project_factory::GoProjectFactory,
        workspace_support: workspace_support::GoWorkspaceSupport,
        lsp_installer: lsp_installer::GoLspInstaller,
    },
}
```

### Extract Constants

**Before**:
```rust
let re = regex::Regex::new(r"(?m)^\s*(func|class|struct|...)").unwrap();
```

**After**:
```rust
// constants.rs
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    pub static ref SYMBOL_PATTERN: Regex = Regex::new(
        r"(?m)^\s*(func|class|struct|enum|protocol|extension)\s+([a-zA-Z0-9_]+)"
    ).expect("Valid regex at compile time");
}

pub const DEFAULT_VERSION: &str = "1.21";
pub const PARSER_VERSION: &str = "0.20.0";
```

### Improve Pattern Matching

**Before** (false positives):
```rust
let pattern = format!("using {};", module_name);
if let Some(col) = line.find(&pattern) {
    // ❌ Matches inside comments, strings, aliases
}
```

**After** (context-aware):
```rust
// Exclude comments
let line = line.split("//").next().unwrap_or(line);

// Use regex with word boundaries
let pattern = format!(r"^\s*using\s+{}\s*;", regex::escape(module_name));
let re = Regex::new(&pattern)?;

if let Some(m) = re.find(line) {
    // ✅ Only matches actual using statements
}
```

### Add Edge Case Tests

**Error Path Testing**:
```rust
#[test]
fn test_invalid_source_handling() {
    let plugin = CsharpPlugin::default();
    let invalid_source = "this is not valid C# code {{{";

    // Should return error, not panic
    let result = plugin.parse_source(invalid_source);
    assert!(result.is_err());
}
```

**Unicode Testing**:
```rust
#[test]
fn test_unicode_module_names() {
    let content = "using System.日本語;\nusing Модуль.Русский;";
    let references = scan_references(content, "日本語");

    assert_eq!(references.len(), 1);
    assert_eq!(references[0].line, 1);
}
```

**Performance Testing**:
```rust
#[test]
fn test_large_file_performance() {
    let huge_content = "using System.Text;\n".repeat(100_000);

    let start = std::time::Instant::now();
    let result = scan_references(&huge_content, "Text");
    let duration = start.elapsed();

    // Should complete within reasonable time
    assert!(duration.as_secs() < 5);
    assert_eq!(result.len(), 100_000);
}
```

**Boundary Conditions**:
```rust
#[test]
fn test_empty_file() {
    let plugin = GoPlugin::default();
    let result = plugin.parse_source("");
    assert!(result.is_ok());
}

#[test]
fn test_single_char_file() {
    let result = parse_source("x");
    assert!(result.is_ok());
}
```

## References

- Rust testing guide: https://doc.rust-lang.org/book/ch11-00-testing.html
- define_language_plugin! macro: `crates/mill-lang-common/src/macros.rs`
- Analysis document: `.debug/parity-refinement-proposal-2025-10-31.md`

## Detailed Issue Catalog

**Design Inconsistencies**:
1. Go plugin doesn't use `define_language_plugin!` macro (~70 lines extra)
2. Hardcoded Go version "1.21" (lib.rs:145)
3. Hardcoded C# parser version "0.20.0" (lib.rs:256)
4. C# `using` pattern too simplistic (lib.rs:170)
5. Go qualified path matching in strings
6. Swift import regex lacks precision
7. No constant extraction across plugins
8. Inconsistent error message formats

**Test Coverage Gaps**:
1. No invalid source code tests
2. No malformed manifest tests (go.mod, .csproj, Package.swift)
3. No Unicode module name tests
4. No large file performance tests
5. No file read failure tests
6. No empty file tests
7. No long line tests (>10K chars)
8. No mixed line ending tests
9. No invalid UTF-8 tests
10. No binary file tests
11. No concurrent access tests
12. No boundary condition tests (line 0, column 0)
