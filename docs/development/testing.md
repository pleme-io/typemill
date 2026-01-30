# Testing Guide

Fast reference for test organization, execution, and best practices.

## Test Pyramid (4 Layers)

| Layer | Location | Purpose | Speed | Count | Feature Flag |
|-------|----------|---------|-------|-------|--------------|
| **Unit** | `crates/*/src/` | Individual functions, business logic | ⚡ <100ms | 100s | default |
| **Integration** | `tests/e2e/src/` | Tool handlers with mocks | ⚡ <5s | 81 | default |
| **E2E** | `../../apps/mill/tests/e2e_*.rs` | Complete workflows | 🐌 <30s | 58 | default |
| **Smoke** | `../../apps/mill/tests/smoke/` | Protocol connectivity | 🐌 <1min | 5 | `#[ignore]` |

**Total:** 244+ tests across 93+ files

## When to Use Each Layer

| Test Type | Use When | Don't Use When |
|-----------|----------|----------------|
| **Unit** | Testing pure functions, data transformations, algorithms | Testing with external dependencies |
| **Integration** | Testing tool handlers, service APIs, parameter validation | Testing individual functions or workflows |
| **E2E** | Testing critical multi-step workflows, error recovery | Testing simple functions or single tools |
| **Smoke** | Adding NEW protocol layer (WebSocket, gRPC) | Testing individual features (use integration) |

## Running Tests

| Command | Purpose | Time |
|---------|---------|------|
| `cargo nextest run --workspace` | Fast tests (unit + integration) | ~10s |
| `cargo nextest run --workspace --features lsp-tests` | + LSP server tests | ~60s |
| `cargo nextest run --workspace --all-features` | + Heavy tests (benchmarks) | ~80s |
| `cargo nextest run --workspace --ignored --features lsp-tests` | Smoke tests only | ~1min |
| `cargo nextest run -p mill --test e2e_*` | E2E tests only | ~30s |
| `cargo nextest run --workspace --no-capture` | With test output | varies |

## Test Coverage by Category

### LSP Features (Data-Driven)

| Feature | Languages | Mock Tests | Real Tests |
|---------|-----------|------------|------------|
| Go to Definition | TS, Py, Go, Rust | ✅ | ✅ (#[ignore]) |
| Find References | TS, Py, Go, Rust | ✅ | ✅ (#[ignore]) |
| Hover | TS, Py, Go, Rust | ✅ | ✅ (#[ignore]) |
| Document Symbols | TS, Py, Go, Rust | ✅ | ✅ (#[ignore]) |
| Workspace Symbols | TS, Py, Go, Rust | ✅ | ✅ (#[ignore]) |
| Completion | TS, Py, Go, Rust | ✅ | ✅ (#[ignore]) |
| Rename (rename_all) | TS, Py, Go, Rust | ✅ | ✅ (#[ignore]) |

### E2E Features

| Category | Tests | Coverage |
|----------|-------|----------|
| Workflow Execution | 10 | simple, complex, failure, dry-run, rollback, batch |
| File Operations | 6 | create, read, write, prune, rename_all, list |
| Refactoring | 8 | Cross-language, imports, symbols |
| Workspace Operations | 7 | Directory rename_all, consolidation, dependencies |
| Error Scenarios | 5 | Resilience, recovery, validation |
| Performance | 3 | Load testing, stress testing |
| Server Lifecycle | 10 | LSP management, restart, crash recovery |

## Data-Driven Test Architecture

### Three-Layer Pattern

| Layer | File | Purpose | Languages |
|-------|------|---------|-----------|
| **Fixtures** | `../crates/mill-test-support/src/harness/test_fixtures.rs` | Test data (code snippets, expected results) | TS, Py, Go, Rust |
| **Runners** | `../../apps/mill/tests/lsp_feature_runners.rs` | Generic test logic (language-agnostic) | All |
| **Declarations** | `../../apps/mill/tests/lsp_features.rs` | Test matrix (connects fixtures + runners) | All |

### Adding a New Language

| Step | Action | File | Lines Changed |
|------|--------|------|---------------|
| 1 | Add test case data | `test_fixtures.rs` | +10 lines |
| 2 | Run tests | - | - |

**That's it!** Test infrastructure automatically:
- Runs new cases for the new language
- Uses same logic as existing languages
- Generates mock + real LSP tests

## Feature Flags

| Flag | Purpose | Requirements | Enabled By |
|------|---------|--------------|------------|
| `fast-tests` | Unit + integration (mocks) | None | Default |
| `lsp-tests` | Real LSP server tests | LSP servers in PATH | `--features lsp-tests` |
| `e2e-tests` | End-to-end workflows | None | Default |
| `heavy-tests` | Benchmarks, property tests | None (slow) | `--features heavy-tests` |
| `all-features` | Complete suite | LSP servers | `--all-features` |

## Test Infrastructure

### Binary Architecture

| Binary | Location | PID Lock | Parallel Tests | Used By |
|--------|----------|----------|----------------|---------|
| `mill` | `../../apps/mill` | ✅ `/tmp/mill.pid` | ❌ Conflicts | CLI, users |
| `mill-server` | `../crates/mill-server` | ❌ No lock | ✅ Isolated instances | TestClient, CI |

**Important:** TestClient uses `mill-server` (not `mill`) to enable parallel test execution.

### Test Helpers

| Helper | Purpose | Example Usage |
|--------|---------|---------------|
| `TestWorkspace` | Isolated temp workspace | `TestWorkspace::new()` |
| `TestClient` | MCP tool invocation | `client.call_tool("rename_all", args)` |
| `test_fixtures.rs` | Language-specific test data | See data-driven architecture |

## Test Organization

```
workspace/
├── crates/*/src/           # Unit tests (inline #[test])
├── tests/
│   ├── src/harness/        # Test infrastructure
│   │   ├── test_fixtures.rs      # Language data
│   │   ├── test_helpers.rs       # Helper functions
│   │   └── test_builder.rs       # Workspace builder
│   └── tests/              # Integration tests
│       ├── lsp_features.rs       # Data-driven LSP tests
│       └── lsp_feature_runners.rs # Test runners
└── ../../apps/mill/tests/   # E2E and smoke tests
    ├── e2e_*.rs            # E2E workflows
    └── smoke/              # Protocol tests (#[ignore])
```
## Best Practices

### ✅ DO

| Practice | Rationale |
|----------|-----------|
| Follow test pyramid | Many unit tests, fewer integration, fewer E2E, minimal smoke |
| Test behavior, not implementation | Tests survive refactoring |
| Make tests fast | Fast feedback = happy developers |
| Use descriptive names | `test_rename_updates_all_references` not `test_1` |
| Test error cases | Happy path + errors = robust code |
| Keep tests isolated | Tests run in any order |
| Use test helpers | `TestWorkspace`, `TestClient` = cleaner tests |

### ❌ DON'T

| Anti-Pattern | Why Not | Alternative |
|-------------|---------|-------------|
| Test same logic twice | Wastes time, maintenance burden | Test once in lowest layer |
| Test implementation details | Breaks on refactoring | Test public API only |
| Tests depend on each other | Breaks parallel execution | Isolate tests |
| Use sleeps | Flaky tests | Use proper synchronization |
| Skip test layers | Wrong layer = slow tests | Use decision tree |
| Test third-party code | Not your responsibility | Trust LSP servers work |
| Add smoke tests for features | Wrong layer | Use integration tests |

## Timing Expectations

| Layer | Expected | Acceptable Max | Acceptable Timeout |
|-------|----------|----------------|-------------------|
| Unit | <100ms | 500ms | 1s |
| Integration | <5s | 30s | 1min |
| E2E | <30s | 2min | 5min |
| Smoke | <1min | 5min | 10min |

**Total suite:** ~2-3 minutes (without smoke tests)

## CI/CD Test Matrix

| Stage | Trigger | Tests | Duration |
|-------|---------|-------|----------|
| **Fast** | Every commit | Unit + Integration + Lint | ~30s |
| **Full** | PRs | Fast + E2E + Heavy | ~3min |
| **Smoke** | Manual | Protocol connectivity | ~5min |

## Troubleshooting

| Problem | Cause | Solution |
|---------|-------|----------|
| Tests timeout | Heavy tests running | Use `cargo nextest run --workspace` (no `--all-features`) |
| LSP tests fail | LSP servers not installed | Install: `npm i -g typescript-language-server`, `rustup component add rust-analyzer` |
| LSP servers not found | Not in PATH | Check: `which typescript-language-server` |
| Smoke tests skip | Marked `#[ignore]` | Run: `cargo nextest run --ignored --features lsp-tests` |
| Server already running | Using `mill` not `mill-server` | TestClient should use `mill-server` |
| Tests fail intermittently | Race conditions | Fix synchronization, avoid sleeps |

## Warning Signs

| Sign | Problem | Action |
|------|---------|--------|
| Test suite >5min | Too many slow tests | Move to E2E or optimize |
| Many ignored tests | Broken or redundant | Fix or remove |
| Tests fail randomly | Race conditions | Fix sync |
| New tests wrong layer | Misunderstanding pyramid | Review this guide |
| Smoke tests test features | Testing wrong thing | Move to integration |

## Examples

See real test implementations in the codebase:
- **Unit tests**: Inline in `crates/*/src/` with `#[test]` or `#[cfg(test)]`
- **Integration tests**: `tests/e2e/src/` - Cross-crate workflows with test harness
- **E2E tests**: `apps/mill/tests/` - Complete workflows with real components
- **Test fixtures**: `crates/mill-test-support/fixtures/` - Shared test data

## Next Steps to Expand Coverage

| Task | Effort | Impact |
|------|--------|--------|
| Add Java to LSP fixtures | Low | High (new language) |
| Add edge cases | Medium | Medium (robustness) |
| Add property-based tests | Medium | High (fuzzing) |
| Add performance benchmarks | Medium | Medium (regression detection) |
| Add workflow scenarios | High | High (user workflows) |

---

## Language Plugin Testing Standards

> Last updated: 2025-11-15
> Status: ✅ All 13 plugins compliant

This section defines the minimum testing standards for all TypeMill language plugins. These standards ensure consistent quality, maintainability, and reliability across the plugin ecosystem.

### Minimum Test Requirements

Every language plugin MUST have at least **11 tests** covering:

#### Required Test Categories

1. **Metadata Tests (1 test minimum)**
   - Plugin name, version, file extensions
   - Example: `test_plugin_basic_metadata()`

2. **Manifest/Configuration Tests (1 test minimum)**
   - Package manifest parsing (Cargo.toml, package.json, pom.xml, etc.)
   - Example: `test_manifest_parsing()`

3. **Parsing Tests (3 tests minimum)**
   - Valid source code parsing
   - Invalid syntax handling
   - Empty file handling
   - Example: `test_parse_valid_code()`, `test_parse_invalid_syntax()`, `test_parse_empty_file()`

4. **Edge Case Tests (2 tests minimum)**
   - Unicode identifiers
   - Extremely long lines (15,000+ characters)
   - Example: `test_edge_unicode_identifiers()`, `test_edge_extremely_long_lines()`

5. **Performance Tests (2 tests minimum)**
   - Large file parsing (5,000+ items)
   - Reference scanning performance
   - Example: `test_performance_parse_large_file()`

6. **Integration Tests (2 tests minimum)**
   - Multi-file workflows
   - Parse → modify → verify patterns
   - Example: `test_integration_parse_modify_verify()`

**Total**: Minimum 11 tests per plugin

### Shared Test Infrastructure

All plugins MUST use `mill-test-support` for common testing patterns.

#### Adding the Dependency

In your plugin's `Cargo.toml`:

```toml
[dev-dependencies]
mill-test-support = { path = "../../crates/mill-test-support" }
```

#### Using the Harness

```rust
use mill_test_support::harness::{
    IntegrationTestHarness,
    edge_cases,
    fixtures,
    *,
};

#[tokio::test]
async fn test_integration_example() {
    let harness = IntegrationTestHarness::new().expect("harness");

    // Create test files
    harness.create_source_file("test.rs", "fn main() {}").expect("create");

    // Verify structure
    let content = harness.read_file("test.rs").expect("read");
    assert_contains_all(&content, &["fn", "main"]);
}
```

### Test Naming Conventions

Use clear, descriptive test names following this pattern:

```
test_[category]_[scenario]_[expected_outcome]
```

**Examples**:
- `test_parse_valid_rust_code()`
- `test_edge_unicode_identifiers()`
- `test_performance_parse_large_file()`
- `test_integration_create_parse_refactor()`

### Edge Case Checklist

All plugins should handle these edge cases:

- ✅ Empty files
- ✅ Whitespace-only files
- ✅ Unicode identifiers (non-ASCII characters)
- ✅ Extremely long lines (15,000+ characters)
- ✅ No newlines in source
- ✅ Mixed line endings (CRLF/LF)
- ✅ Special regex characters in strings
- ✅ Null bytes in content

**Implementation**: Use `edge_cases` module from `mill-test-support`

### Performance Testing

Performance tests MUST:
- Generate files with 5,000+ items (functions, classes, etc.)
- Complete within 5 seconds
- Use relaxed timing for CI environments

**Implementation**: Use `fixtures::large_file_template()` and `assertions::assert_performance()`

### Integration Testing Patterns

#### Pattern 1: Parse → Modify → Verify

```rust
#[tokio::test]
async fn test_integration_parse_modify_verify() {
    let harness = IntegrationTestHarness::new().expect("harness");
    let source = "fn add(a: i32, b: i32) -> i32 { a + b }";

    harness.test_parse_modify_verify(source, |content| {
        content.replace("add", "sum")
    }).expect("workflow");
}
```

#### Pattern 2: Multi-File Workflow

```rust
#[tokio::test]
async fn test_integration_move_file_references() {
    let harness = IntegrationTestHarness::new().expect("harness");

    // Create file structure
    harness.create_directory("src").expect("create dir");
    harness.create_source_file("src/utils.rs", "pub fn helper() {}").expect("create file");
    harness.create_source_file("main.rs", "use utils::helper;").expect("create file");

    // Verify references
    let main = harness.read_file("main.rs").expect("read");
    assert_contains_all(&main, &["use", "helper"]);
}
```

#### Pattern 3: Manifest Workflow

```rust
#[tokio::test]
async fn test_integration_manifest_dependencies() {
    let harness = IntegrationTestHarness::new().expect("harness");

    // Create manifest
    harness.create_source_file("Cargo.toml",
        "[package]\nname = \"test\"\n\n[dependencies]\nserde = \"1.0\""
    ).expect("create");

    // Verify parsing
    let content = harness.read_file("Cargo.toml").expect("read");
    assert_contains_all(&content, &["package", "dependencies", "serde"]);
}
```

### Current Plugin Status

All 13 language plugins meet baseline standards:

| Plugin | Tests | Integration | Edge Cases | Performance | Status |
|--------|-------|-------------|------------|-------------|--------|
| Rust | 29 | 0 (0%) | 8 (28%) | 2 | ✅ |
| TypeScript | 15 | 2 (13%) | 8 (53%) | 2 | ✅ |
| Python | 17 | 3 (18%) | 0 (0%) | 2 | ✅ |
| Go | 33 | 3 (9%) | 0 (0%) | 3 | ✅ |
| C# | 18 | 3 (17%) | 0 (0%) | 2 | ✅ |
| C++ | 17 | 2 (12%) | 8 (47%) | 2 | ✅ |
| C | 12 | 2 (17%) | 2 (17%) | 1 | ✅ |
| Java | 15 | 1 (7%) | 2 (13%) | 2 | ⚠️ (4 pre-existing failures) |
| Swift | 63 | 5 (8%) | 0 (0%) | 2 | ✅ |
| Markdown | 6 | 1 (17%) | 0 (0%) | 0 | ✅ |
| TOML | 11 | 2 (18%) | 2 (18%) | 0 | ✅ |
| YAML | 11 | 2 (18%) | 1 (9%) | 0 | ✅ |
| Gitignore | 6 | 2 (33%) | 1 (17%) | 0 | ✅ |

**Overall Summary**:
- Total Tests: 253
- Integration Tests: 28 (11% coverage)
- Edge Case Tests: 32 (13% coverage)
- Performance Tests: 18
- Success Rate: 12/13 plugins ✅

### Adding a New Language Plugin

When creating a new language plugin, follow these steps:

#### 1. Add Test Infrastructure

In `Cargo.toml`:
```toml
[dev-dependencies]
mill-test-support = { path = "../../crates/mill-test-support" }
tempfile = "3.10"
tokio = { version = "1.35", features = ["full"] }
```

#### 2. Create Minimum Test Suite

Use this template in `src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mill_test_support::harness::{
        IntegrationTestHarness,
        edge_cases,
        fixtures,
        *,
    };

    // 1. Metadata test
    #[test]
    fn test_plugin_metadata() {
        let plugin = LanguagePlugin::new();
        assert_eq!(plugin.name(), "YourLanguage");
    }

    // 2. Manifest test
    #[test]
    fn test_manifest_parsing() {
        // Test manifest parsing
    }

    // 3. Parsing tests (3)
    #[test]
    fn test_parse_valid_code() { }

    #[test]
    fn test_parse_invalid_syntax() { }

    #[test]
    fn test_parse_empty_file() {
        let source = edge_cases::empty_file();
        // Verify no panic
    }

    // 4. Edge case tests (2)
    #[test]
    fn test_edge_unicode() {
        let source = edge_cases::unicode_identifiers();
        // Test Unicode handling
    }

    #[test]
    fn test_edge_long_lines() {
        let source = edge_cases::extremely_long_line();
        // Test long line handling
    }

    // 5. Performance tests (2)
    #[test]
    fn test_performance_large_file() {
        let source = fixtures::large_file_template("your_language", 5000);
        let start = std::time::Instant::now();
        // Parse source
        assert_performance(start.elapsed(), 5);
    }

    // 6. Integration tests (2)
    #[tokio::test]
    async fn test_integration_parse_modify_verify() {
        let harness = IntegrationTestHarness::new().expect("harness");
        // Implement workflow
    }

    #[tokio::test]
    async fn test_integration_manifest_workflow() {
        let harness = IntegrationTestHarness::new().expect("harness");
        // Implement manifest workflow
    }
}
```

#### 3. Run and Verify

```bash
cargo test -p mill-lang-your-language
```

Ensure all 11+ tests pass before submitting.

### Maintenance

This document should be updated when:
- Minimum test requirements change
- New testing patterns are identified
- Shared utilities are enhanced
- Plugin test counts significantly increase

---

## Key References

- [Test Pyramid Pattern](https://martinfowler.com/articles/practical-test-pyramid.html)
- [contributing.md](../../contributing.md) - Development setup
- [tests/e2e/](../../tests/e2e/) - Integration test suite

## Related Documentation

- **[Contributor Overview](overview.md)** - Quick start guide for contributors
- **[CI/CD Integration](../operations/cicd.md)** - Running tests in CI pipelines
