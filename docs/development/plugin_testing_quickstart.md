# Language Plugin Testing Quick Start

Quick reference for testing TypeMill language plugins.

## Minimum Requirements

Every plugin needs **≥11 tests**:
- 1 metadata test
- 1 manifest test
- 3 parsing tests (valid, invalid, empty)
- 2 edge case tests (unicode, long lines)
- 2 performance tests
- 2 integration tests

## Setup (30 seconds)

### 1. Add dependency in Cargo.toml:

```toml
[dev-dependencies]
mill-test-support = { path = "../../crates/mill-test-support" }
tempfile = "3.10"
tokio = { version = "1.35", features = ["full"] }
```

### 2. Import in tests:

```rust
use mill_test_support::harness::{
    IntegrationTestHarness,
    edge_cases,
    fixtures,
    *,
};
```

## Common Patterns

### Metadata Test (1 test)

```rust
#[test]
fn test_plugin_metadata() {
    let plugin = YourLanguagePlugin::new();
    assert_eq!(plugin.name(), "YourLanguage");
    assert!(plugin.supported_extensions().contains(&"ext"));
}
```

### Manifest Test (1 test)

```rust
#[test]
fn test_manifest_parsing() {
    let manifest_content = r#"
    [package]
    name = "test"
    "#;

    let result = parse_manifest(manifest_content);
    assert!(result.is_ok());
}
```

### Parsing Tests (3 tests)

```rust
#[test]
fn test_parse_valid_code() {
    let source = r#"fn main() { println!("hello"); }"#;
    let result = parse(source);
    assert!(result.is_ok());
}

#[test]
fn test_parse_invalid_syntax() {
    let source = "fn main() {";  // Missing closing brace
    let result = parse(source);
    assert!(result.is_err());  // Should error gracefully
}

#[test]
fn test_parse_empty_file() {
    let source = "";
    let result = parse(source);
    // Should not panic
    assert!(result.is_ok() || result.is_err());
}
```

### Edge Case Tests (2 tests)

```rust
#[test]
fn test_edge_unicode() {
    let source = edge_cases::unicode_identifiers();
    let result = parse(source);
    assert!(result.is_ok());
}

#[test]
fn test_edge_long_lines() {
    let source = edge_cases::extremely_long_line();
    let result = parse(source);
    assert!(result.is_ok());
}
```

### Performance Tests (2 tests)

```rust
#[test]
fn test_performance_large_file() {
    let source = fixtures::large_file_template("your_language", 5000);
    let start = std::time::Instant::now();

    let result = parse(&source);

    assert!(result.is_ok());
    assert_performance(start.elapsed(), 5);  // Must complete in 5 seconds
}

#[test]
fn test_performance_references() {
    let source = fixtures::large_file_template("your_language", 1000);
    let start = std::time::Instant::now();

    let _ = scan_references(&source, "someSymbol");

    assert_performance(start.elapsed(), 5);
}
```

### Integration Tests (2 tests)

```rust
#[tokio::test]
async fn test_integration_parse_modify_verify() {
    let harness = IntegrationTestHarness::new().expect("harness");

    let source = "fn add(a: i32, b: i32) -> i32 { a + b }";
    harness.test_parse_modify_verify(source, |content| {
        content.replace("add", "sum")
    }).expect("workflow");
}

#[tokio::test]
async fn test_integration_multi_file_workflow() {
    let harness = IntegrationTestHarness::new().expect("harness");

    // Create file structure
    harness.create_directory("src").expect("mkdir");
    harness.create_source_file("src/lib.rs", "pub fn helper() {}").expect("create");
    harness.create_source_file("main.rs", "use lib::helper;").expect("create");

    // Verify files exist
    let main = harness.read_file("main.rs").expect("read");
    assert_contains_all(&main, &["use", "helper"]);
}
```

## Running Tests

```bash
# Your plugin
cargo test -p mill-lang-your-plugin

# Just lib tests (no integration)
cargo test -p mill-lang-your-plugin --lib

# Single test
cargo test -p mill-lang-your-plugin --lib test_parse_valid_code

# All plugins
cargo test --workspace --lib
```

## Test Utils Reference

### Edge Cases (built-in)

```rust
use mill_test_support::harness::edge_cases;

let empty = edge_cases::empty_file();
let whitespace = edge_cases::whitespace_only_file();
let unicode = edge_cases::unicode_identifiers();
let long_line = edge_cases::extremely_long_line();
let no_newline = edge_cases::no_final_newline();
let mixed_endings = edge_cases::mixed_line_endings();
let regex_chars = edge_cases::special_regex_characters();
let null_bytes = edge_cases::null_bytes_content();
```

### Fixtures (built-in)

```rust
use mill_test_support::harness::fixtures;

let rust_file = fixtures::large_file_template("rust", 5000);
let python_file = fixtures::large_file_template("python", 5000);
let ts_file = fixtures::large_file_template("typescript", 5000);
```

### Assertions (built-in)

```rust
use mill_test_support::harness::*;

// Check timing
assert_performance(duration, max_seconds);

// Check content
assert_contains_all(&content, &["expected", "strings"]);
assert_contains_any(&content, &["one", "or", "other"]);
assert_not_contains(&content, "unexpected");
```

### Integration Harness (built-in)

```rust
use mill_test_support::harness::IntegrationTestHarness;

let harness = IntegrationTestHarness::new()?;

// File operations
harness.create_source_file("test.ext", "content")?;
harness.create_directory("src")?;
let content = harness.read_file("test.ext")?;
harness.delete_file("test.ext")?;

// Workflows
harness.test_parse_modify_verify(source, modifier_fn)?;
```

## Troubleshooting

### Test timeouts?
- Reduce the size in `large_file_template()`: use 1000 instead of 5000
- Check if parsing is O(n²) - optimize first, then adjust test thresholds

### Tests panicking?
- Add `.expect("msg")` or proper error handling
- Use `should_panic` attribute for tests that should panic
- Check for `unwrap()` calls that might panic

### Edge cases failing?
- Look at example implementations in:
  - Rust: `languages/mill-lang-rust/src/lib.rs`
  - TypeScript: `languages/mill-lang-typescript/src/lib.rs`
  - Python: `languages/mill-lang-python/src/lib.rs`

### Can't find mill-test-support?
- Verify path in `Cargo.toml`: `{ path = "../../crates/mill-test-support" }`
- Run `cargo build -p mill-test-support` first

## See Also

- [Full Testing Standards](/workspace/docs/development/testing_standards.md)
- [mill-test-support Documentation](/workspace/crates/mill-test-support/src/lib.rs)
- [Contributing Guide](/workspace/contributing.md)
