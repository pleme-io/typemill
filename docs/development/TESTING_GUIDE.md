# Testing Guide

This guide explains the test structure, when to use each type of test, and how to run the test suite.

---

## Test Pyramid

Codebuddy follows a 4-layer test pyramid to ensure comprehensive coverage while maintaining fast feedback loops:

```
                    /\
                   /  \
                  /Smoke\          Layer 4: Protocol connectivity
                 /______\          2 files, 5 tests (#[ignore])
                /        \
               /   E2E    \        Layer 3: Full workflows
              /____________\       7 files, 58 tests
             /              \
            /  Integration   \     Layer 2: Handler logic
           /__________________\    18 files, 81 tests
          /                    \
         /     Unit Tests        \ Layer 1: Business logic
        /________________________\ 68+ files, 100s of tests
```

### Layer 1: Unit Tests

**Location:** `crates/*/src/` (inline with `#[test]` or `#[cfg(test)]`)

**Purpose:** Test individual functions, structs, methods in isolation

**Characteristics:**
- ⚡ Very fast (milliseconds)
- 🎯 Focused on single units of code
- 🔒 No external dependencies
- 📊 Comprehensive coverage

**What to test:**
- Data transformations
- Business logic
- Error handling
- Edge cases
- Algorithm correctness

**Example:**
```rust
#[test]
fn test_parse_symbol_name() {
    let input = "fn main()";
    let symbol = parse_symbol(input);
    assert_eq!(symbol.kind, SymbolKind::Function);
    assert_eq!(symbol.name, "main");
}
```

**Run:**
```bash
cargo test --workspace --lib
```

---

### Layer 2: Integration Tests

**Location:** `integration-tests/src/`

**Purpose:** Test tool handlers with mocked LSP servers

**Characteristics:**
- ⚡ Fast (seconds)
- 🔧 Tests handlers with mock dependencies
- ✅ Validates API contracts
- 📝 Tests parameter validation and response format

**What to test:**
- Tool handler logic
- Parameter validation
- Response structure
- Error scenarios
- All public MCP tools

**Example:**
```rust
#[tokio::test]
async fn test_rename_tool_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("test.ts", "function foo() {}");

    let response = client
        .call_tool(
            "rename.plan",
            json!({
                "symbol": "foo",
                "new_name": "bar",
                "file_path": workspace.absolute_path("test.ts")
            }),
        )
        .await
        .expect("rename.plan should succeed");

    assert!(response.get("result").is_some());
}
```

**Run:**
```bash
cargo nextest run --workspace -p integration-tests
```

---

### Layer 3: E2E Tests

**Location:** `apps/codebuddy/tests/e2e_*.rs`

**Purpose:** Test complete workflows with real components

**Characteristics:**
- 🐌 Slow (minutes)
- 🌐 Tests full system integration
- 🔄 Tests multi-step workflows
- 🚨 Tests error recovery

**What to test:**
- Critical user workflows
- Multi-tool sequences
- Cross-component integration
- Error recovery scenarios
- Performance characteristics

**Example:**
```rust
#[tokio::test]
async fn test_refactoring_workflow_end_to_end() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Setup workspace with multiple files
    workspace.create_file("src/main.ts", "...");
    workspace.create_file("src/helper.ts", "...");

    // Step 1: Plan the refactoring
    let plan = client.call_tool("rename.plan", ...).await.unwrap();

    // Step 2: Apply the plan
    let result = client.call_tool("workspace.apply_edit", ...).await.unwrap();

    // Step 3: Verify changes
    assert!(result.get("success").unwrap().as_bool().unwrap());

    // Step 4: Verify files were modified correctly
    let content = std::fs::read_to_string(workspace.path().join("src/main.ts")).unwrap();
    assert!(content.contains("new_name"));
}
```

**Run:**
```bash
cargo nextest run --workspace -p codebuddy --test e2e_*
```

---

### Layer 4: Smoke Tests

**Location:** `apps/codebuddy/tests/smoke/`

**Purpose:** Test protocol connectivity ONLY

**Characteristics:**
- 🐌 Slow (requires external services)
- 🔌 Tests protocol layers
- ⏭️ Always `#[ignore]` (manual run)
- 🎯 Minimal coverage (just connectivity)

**What to test:**
- Protocol initialization
- Message format (JSON-RPC, LSP)
- Basic routing (2-3 requests prove it works)
- Connection reuse

**What NOT to test:**
- ❌ Business logic (use unit tests)
- ❌ Feature implementations (use integration tests)
- ❌ Every tool/feature (redundant)

**Example:**
```rust
#[tokio::test]
#[ignore] // Requires LSP servers installed
#[cfg(feature = "lsp-tests")]
async fn test_lsp_protocol_connectivity() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("test.ts", "function foo() {}");

    // Test 1: find_definition (proves LSP textDocument/definition works)
    let response = client.call_tool("find_definition", ...).await;
    assert!(response.is_ok(), "LSP protocol should work");

    // Test 2: find_references (proves routing works for different requests)
    let response = client.call_tool("find_references", ...).await;
    assert!(response.is_ok(), "LSP routing should work");

    // Don't test every LSP feature - that's what unit/integration tests do!
}
```

**Run:**
```bash
# Run all smoke tests
cargo nextest run --workspace --ignored --features lsp-tests

# Run only MCP smoke tests
cargo nextest run --workspace --ignored smoke::mcp

# Run only LSP smoke tests
cargo nextest run --workspace --ignored --features lsp-tests smoke::lsp
```

---

## Feature Gates

Codebuddy uses feature flags to control which tests run:

### `lsp-tests`

**Purpose:** Tests requiring LSP servers installed

**Requirements:**
- TypeScript: `npm install -g typescript-language-server`
- Rust: `rustup component add rust-analyzer`

**Run:**
```bash
cargo nextest run --workspace --features lsp-tests
```

**What it enables:**
- LSP smoke tests
- Any tests marked with `#[cfg(feature = "lsp-tests")]`

---

### `heavy-tests`

**Purpose:** Performance benchmarks and property-based testing

**Requirements:** None (just takes longer to run)

**Run:**
```bash
cargo nextest run --workspace --features heavy-tests
```

**What it enables:**
- Performance benchmark tests
- Property-based testing (proptest)
- Large dataset tests

---

### `all-features`

**Purpose:** Run complete test suite including all feature-gated tests

**Run:**
```bash
cargo nextest run --workspace --all-features
```

---

## Adding New Tests

### When to Use Each Layer

Use this decision tree:

```
Are you testing a pure function or data structure?
  └─ YES → Unit Test (Layer 1)

Are you testing a tool handler or service?
  └─ YES → Integration Test (Layer 2)

Are you testing a complete user workflow?
  └─ YES → E2E Test (Layer 3)

Are you testing protocol connectivity?
  └─ YES → Smoke Test (Layer 4) - RARE!
```

---

### Adding a Unit Test

**When:** Testing individual functions, business logic, data transformations

**Where:** In the same file as the code, in a `#[cfg(test)]` module

**Example:**
```rust
// src/parser.rs
pub fn parse_imports(source: &str) -> Vec<Import> {
    // implementation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_imports_basic() {
        let source = "import { foo } from 'bar';";
        let imports = parse_imports(source);
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].name, "foo");
    }

    #[test]
    fn test_parse_imports_empty() {
        let source = "";
        let imports = parse_imports(source);
        assert!(imports.is_empty());
    }
}
```

---

### Adding an Integration Test

**When:** Testing a new MCP tool handler or service

**Where:** `integration-tests/src/test_<tool_name>_integration.rs`

**Example:**
```rust
// integration-tests/src/test_my_new_tool.rs
use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

#[tokio::test]
async fn test_my_new_tool_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("test.ts", "...");

    let response = client
        .call_tool(
            "my_new_tool",
            json!({
                "param": "value"
            }),
        )
        .await
        .expect("Tool should succeed");

    assert!(response.get("result").is_some());
}
```

---

### Adding an E2E Test

**When:** Testing a critical user workflow involving multiple tools

**Where:** `apps/codebuddy/tests/e2e_*.rs` (add to existing file or create new one)

**Example:**
```rust
// apps/codebuddy/tests/e2e_my_workflow.rs
use cb_test_support::harness::{TestClient, TestWorkspace};
use serde_json::json;

#[tokio::test]
async fn test_my_workflow_end_to_end() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Setup
    workspace.create_file("main.ts", "...");

    // Execute multi-step workflow
    let step1 = client.call_tool("tool1", ...).await.unwrap();
    let step2 = client.call_tool("tool2", ...).await.unwrap();
    let step3 = client.call_tool("tool3", ...).await.unwrap();

    // Verify end result
    assert!(step3.get("success").unwrap().as_bool().unwrap());
}
```

---

### Adding a Smoke Test (RARE)

**When:** Adding a NEW protocol layer (e.g., adding WebSocket MCP support)

**Where:** `apps/codebuddy/tests/smoke/<protocol>.rs`

**Important:** Only add smoke tests for NEW protocol layers. Do NOT add smoke tests for individual features!

**Example:**
```rust
// apps/codebuddy/tests/smoke/websocket.rs
#[tokio::test]
#[ignore] // Requires WebSocket server running
async fn test_websocket_protocol_connectivity() {
    // Test basic WebSocket connection
    // Test 2-3 different message types (proves routing works)
    // Test connection reuse

    // Do NOT test every tool - that's what integration tests do!
}
```

---

## Anti-Patterns

### ❌ DON'T: Test Same Logic Twice

**Bad:**
```rust
// Testing same business logic in two places
#[tokio::test]
async fn test_rename_mock() {
    // Tests rename business logic
}

#[tokio::test]
#[ignore]
async fn test_rename_real() {
    // Tests SAME rename business logic via real LSP
    // This is redundant!
}
```

**Good:**
```rust
// Test business logic once (mock)
#[tokio::test]
async fn test_rename_mock() {
    // Tests rename business logic
}

// Test protocol connectivity separately (smoke test)
#[tokio::test]
#[ignore]
async fn test_lsp_protocol_connectivity() {
    // Tests LSP works (2-3 requests, any tool)
    // Does NOT test rename logic specifically
}
```

---

### ❌ DON'T: Test Business Logic in Smoke Tests

**Bad:**
```rust
#[tokio::test]
#[ignore]
async fn test_mcp_rename_feature() {
    // Testing rename feature logic via MCP
    // This belongs in integration test!
}
```

**Good:**
```rust
// Integration test (tests rename logic)
#[tokio::test]
async fn test_rename_integration() {
    // Tests rename business logic with mocks
}

// Smoke test (tests protocol only)
#[tokio::test]
#[ignore]
async fn test_mcp_protocol_connectivity() {
    // Tests MCP protocol with ANY 2-3 tools
    // Just proves protocol works
}
```

---

### ❌ DON'T: Add Tests to Wrong Layer

**Bad:**
```rust
// E2E test that should be a unit test
#[tokio::test]
async fn test_parse_import_statement() {
    // This is testing a pure function
    // Should be a unit test!
}
```

**Good:**
```rust
// Unit test
#[test]
fn test_parse_import_statement() {
    // Fast, isolated, no async needed
}
```

---

## Running Tests

### Quick Reference

```bash
# Fast tests only (default - unit + integration)
cargo nextest run --workspace

# With LSP tests (requires LSP servers)
cargo nextest run --workspace --features lsp-tests

# Full suite with heavy tests
cargo nextest run --workspace --all-features

# Smoke tests only (requires external services)
cargo nextest run --workspace --ignored --features lsp-tests

# Specific test file
cargo nextest run -p codebuddy --test e2e_workflow_execution

# Specific test function
cargo nextest run --workspace test_rename_integration

# With test output
cargo nextest run --workspace --no-capture

# Watch mode (run tests on file change)
cargo watch -x "nextest run --workspace"
```

---

### Test Timing Expectations

| Layer       | Expected Time | Acceptable Max |
|-------------|---------------|----------------|
| Unit        | < 100ms       | 500ms          |
| Integration | < 5s          | 30s            |
| E2E         | < 30s         | 2min           |
| Smoke       | < 1min        | 5min           |

**Total test suite:** ~2-3 minutes (without smoke tests)

---

## Continuous Integration

### CI Test Matrix

Our CI runs tests in stages:

1. **Fast Stage** (runs on every commit)
   - Unit tests
   - Integration tests
   - Lint (clippy)
   - Format (rustfmt)

2. **Full Stage** (runs on PRs)
   - Fast stage
   - E2E tests
   - Heavy tests

3. **Manual Stage** (manual trigger)
   - Smoke tests (requires infrastructure)

---

## Troubleshooting

### Tests Timing Out

**Problem:** Tests take too long or timeout

**Solutions:**
1. Check if you're running heavy tests by accident: `cargo nextest run --workspace` (no `--all-features`)
2. Check if LSP servers are slow to start (increase timeout in test)
3. Move test to E2E layer if it requires heavy setup

---

### LSP Tests Failing

**Problem:** Tests with `#[cfg(feature = "lsp-tests")]` fail

**Solutions:**
1. Ensure LSP servers installed:
   - TypeScript: `npm install -g typescript-language-server`
   - Rust: `rustup component add rust-analyzer`
2. Check LSP servers in PATH: `which typescript-language-server`
3. Increase initialization timeout in test

---

### Smoke Tests Always Skipped

**Problem:** Smoke tests don't run

**Solution:** Smoke tests are marked `#[ignore]` and must be run explicitly:
```bash
cargo nextest run --workspace --ignored --features lsp-tests
```

---

## Best Practices

### ✅ DO

1. **Follow the test pyramid:** Many unit tests, fewer integration, even fewer E2E, minimal smoke
2. **Test behavior, not implementation:** Test what code does, not how it does it
3. **Make tests fast:** Fast tests = fast feedback = happy developers
4. **Use descriptive test names:** `test_rename_updates_all_references`, not `test_rename_1`
5. **Test error cases:** Happy path + error cases = robust code
6. **Keep tests isolated:** Each test should be independent
7. **Use test helpers:** `TestWorkspace`, `TestClient` make tests cleaner

---

### ❌ DON'T

1. **Don't test implementation details:** Test public API, not private functions
2. **Don't make tests depend on each other:** Tests should run in any order
3. **Don't use sleeps:** Use proper synchronization or polling
4. **Don't skip test layers:** Don't write E2E test for something that should be unit test
5. **Don't test third-party code:** Trust that LSP servers work, test YOUR code
6. **Don't add smoke tests for features:** Smoke tests are for PROTOCOL layers only

---

## Maintenance

### Regular Tasks

**Weekly:**
- Review failed tests
- Check test timing (slow tests?)
- Update fixtures if needed

**Monthly:**
- Review test coverage
- Look for redundant tests
- Update this guide if needed

**Quarterly:**
- Full test suite audit
- Check for test pyramid drift
- Refactor slow tests

---

### Warning Signs

Watch for these issues:

| Sign                          | Problem                    | Solution                        |
|-------------------------------|----------------------------|---------------------------------|
| Test suite > 5min             | Too many slow tests        | Move to E2E or optimize         |
| Many ignored tests            | Tests broken or redundant  | Fix or remove                   |
| Tests fail intermittently     | Race conditions            | Fix synchronization             |
| New tests in wrong layer      | Misunderstanding pyramid   | Review this guide               |
| Smoke tests test features     | Testing wrong thing        | Move to integration layer       |

---

## Summary

**Remember:**
- **Layer 1 (Unit):** Test individual functions (fast, many)
- **Layer 2 (Integration):** Test handlers with mocks (fast, comprehensive)
- **Layer 3 (E2E):** Test critical workflows (slow, selective)
- **Layer 4 (Smoke):** Test protocol connectivity (slow, minimal)

**Golden Rule:** Test behavior at the lowest possible layer of the pyramid!

---

## References

- [Test Pyramid Pattern](https://martinfowler.com/articles/practical-test-pyramid.html)
- [Integration Tests Guide](../../integration-tests/TESTING_GUIDE.md)
- [CLAUDE.md - Development Commands](../../CLAUDE.md#development-commands)
- [CONTRIBUTING.md](../../CONTRIBUTING.md)

---

**Questions?** Open an issue or ask in the project discussions!
