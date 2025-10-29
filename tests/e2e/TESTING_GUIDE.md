# Data-Driven Testing Guide

## Architecture Overview

The test suite uses a **data-driven architecture** that separates test logic from test data, making it incredibly easy to add support for new languages.

### Three-Layer Architecture

1. **Fixtures** (`../../crates/mill-test-support/src/harness/test_fixtures.rs`)
   - Language-specific test data
   - Contains code snippets, file names, and expected outcomes
   - Static data structures

2. **Runners** (`../../apps/mill/tests/lsp_feature_runners.rs`)
   - Generic test logic
   - One runner function per LSP feature
   - Language-agnostic implementation

3. **Test Declarations** (`../../apps/mill/tests/lsp_features.rs`)
   - Minimal test file
   - Connects fixtures with runners
   - Automatically generates test matrix

## Adding a New Language (e.g., Go)

To add test support for Go, you only need to edit **one file**:

### Step 1: Add Test Cases to Fixtures

Edit `../../crates/mill-test-support/src/harness/test_fixtures.rs`:

```rust
pub const GO_TO_DEFINITION_TESTS: &[GoToDefinitionTestCase] = &[
    // TypeScript Case
    GoToDefinitionTestCase {
        language_id: "ts",
        files: &[
            ("main.ts", "import { util } from './util';\nutil();"),
            ("util.ts", "export function util() {}"),
        ],
        trigger_point: ("main.ts", 0, 9),
        expected_location: ("util.ts", 0, 17),
    },
    // Python Case
    GoToDefinitionTestCase {
        language_id: "py",
        files: &[
            ("main.py", "from helper import func\nfunc()"),
            ("helper.py", "def func():\n    return 42"),
        ],
        trigger_point: ("main.py", 0, 19),
        expected_location: ("helper.py", 0, 4),
    },
    // Go Case - NEW!
    GoToDefinitionTestCase {
        language_id: "go",
        files: &[
            ("main.go", "package main\n\nimport \"./helper\"\n\nfunc main() {\n    helper.DoWork()\n}"),
            ("helper/helper.go", "package helper\n\nfunc DoWork() {}"),
        ],
        trigger_point: ("main.go", 5, 11),
        expected_location: ("helper/helper.go", 2, 5),
    },
];
```text
**That's it!** The test infrastructure will automatically:
- Run the new test case for Go
- Use the same test logic as TypeScript and Python
- Generate both mock and real LSP tests

### Step 2: Run the Tests

```bash
# Run mock tests (fast, no dependencies)
cargo nextest run --test lsp_features

# Run with output to see all languages being tested
cargo nextest run --test lsp_features test_go_to_definition_mock --no-capture

# Output:
# Running mock go-to-definition test 1/3 for language: ts
# Running mock go-to-definition test 2/3 for language: py
# Running mock go-to-definition test 3/3 for language: go  ← New!
```text
## Adding a New LSP Feature

To add tests for a new LSP feature (e.g., "call hierarchy"):

### Step 1: Define Fixture Struct

Add to `../../crates/mill-test-support/src/harness/test_fixtures.rs`:

```rust
# [derive(Debug, Clone)]
pub struct CallHierarchyTestCase {
    pub language_id: &'static str,
    pub files: &'static [(&'static str, &'static str)],
    pub trigger_point: (&'static str, u32, u32),
    pub expected_calls: usize,
}

pub const CALL_HIERARCHY_TESTS: &[CallHierarchyTestCase] = &[
    CallHierarchyTestCase {
        language_id: "ts",
        files: &[("test.ts", "function foo() { bar(); }\nfunction bar() {}")],
        trigger_point: ("test.ts", 0, 9),
        expected_calls: 1,
    },
];
```text
### Step 2: Implement Runner Function

Add to `../../apps/mill/tests/lsp_feature_runners.rs`:

```rust
pub async fn run_call_hierarchy_test(case: &CallHierarchyTestCase, use_real_lsp: bool) {
    // Implementation similar to other runners
    // - Build test workspace
    // - Send LSP request
    // - Verify response
}
```text
### Step 3: Declare Tests

Add to `../../apps/mill/tests/lsp_features.rs`:

```rust
# [tokio::test]
async fn test_call_hierarchy_mock() {
    for (idx, case) in CALL_HIERARCHY_TESTS.iter().enumerate() {
        println!("Running mock call-hierarchy test {}/{} for language: {}",
                 idx + 1, CALL_HIERARCHY_TESTS.len(), case.language_id);
        run_call_hierarchy_test(case, false).await;
    }
}

# [tokio::test]
# [ignore]
async fn test_call_hierarchy_real() {
    for (idx, case) in CALL_HIERARCHY_TESTS.iter().enumerate() {
        println!("Running real call-hierarchy test {}/{} for language: {}",
                 idx + 1, CALL_HIERARCHY_TESTS.len(), case.language_id);
        run_call_hierarchy_test(case, true).await;
    }
}
```text
## Benefits

### 1. Extremely DRY
- Test logic written once, used for all languages
- No code duplication

### 2. Easy to Scale
- Adding Go: Just add data entries
- Adding Python to an existing test: Just add one data entry

### 3. Consistent Testing
- All languages tested with identical logic
- Ensures uniform behavior across language servers

### 4. Clear Separation of Concerns
- Data (fixtures) is separate from logic (runners)
- Easy to understand and maintain

### 5. Type Safety
- Compile-time guarantees from Rust
- Can't forget required fields

## Example: Current Test Coverage

```bash
$ cargo nextest run --test lsp_features -- --list

test test_completion_mock
test test_completion_real (ignored)
test test_document_symbols_mock
test test_document_symbols_real (ignored)
test test_find_references_mock
test test_find_references_real (ignored)
test test_go_to_definition_mock
test test_go_to_definition_real (ignored)
test test_hover_mock
test test_hover_real (ignored)
test test_rename_mock
test test_rename_real (ignored)
test test_workspace_symbols_mock
test test_workspace_symbols_real (ignored)
```text
Each test runs for multiple languages automatically!

## Running Tests

```bash
# Run all mock tests (fast, no LSP servers needed)
cargo nextest run --test lsp_features

# Run with verbose output to see language coverage
cargo nextest run --test lsp_features --no-capture

# Run a specific test
cargo nextest run --test lsp_features test_go_to_definition_mock

# Run real LSP tests (requires LSP servers installed)
cargo nextest run --features lsp-tests --test lsp_features --status-level skip --test-threads=1
```text
## Test Feature Flags

The test suite uses Cargo feature flags to categorize tests, allowing you to run subsets of the test suite for faster iteration.

-   `fast-tests` (default): Runs mock-based unit and integration tests that do not require external dependencies like LSP servers. These are very fast and are run by default with `cargo nextest run`.
-   `lsp-tests`: Enables tests that require real LSP servers to be installed and available in the environment. Use this to validate real-world integration.
-   `e2e-tests`: End-to-end workflow tests that may be slower and require a more complete environment setup.
-   `heavy-tests`: Includes performance benchmarks and property-based tests that are very slow and not typically run during development.

**How to use them:**

```bash
# Run only the fast tests (default behavior)
cargo nextest run --workspace

# Run fast tests and the LSP integration tests
cargo nextest run --workspace --features lsp-tests

# Run the full test suite, including all categories
cargo nextest run --workspace --all-features --status-level skip
```text
## Testing Workflow Execution

The `e2e_workflow_execution.rs` test suite verifies the workflow executor and planner, which orchestrate complex multi-step operations.

### Workflow Test Categories

1. **Simple Workflows** - Single-step operations
2. **Complex Workflows with Dependencies** - Multi-step operations with dependency resolution
3. **Failure Handling** - Error scenarios and graceful degradation
4. **Dry-Run Mode** - Preview changes without execution
5. **Rollback on Failure** - Atomic operations with rollback
6. **Batch Operations** - Multiple operations in a single workflow
7. **Dependency Resolution** - Analyzing and updating dependencies
8. **Workflow Planning** - Complex operation planning and execution

### Adding a Workflow Test

```rust
# [tokio::test]
async fn test_my_workflow() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Setup test files
    let file = workspace.path().join("test.ts");
    std::fs::write(&file, "export function test() {}").unwrap();

    // Wait for LSP initialization
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // Execute workflow operation
    let response = client
        .call_tool("tool_name", json!({ "params": "values" }))
        .await;

    // Verify workflow results
    assert!(response.is_ok());
    // Add specific assertions
}
```text
### Running Workflow Tests

```bash
# Run all workflow tests
cargo nextest run --test e2e_workflow_execution

# Run specific workflow test
cargo nextest run --test e2e_workflow_execution test_execute_simple_workflow

# Run with output
cargo nextest run --test e2e_workflow_execution --no-capture
```text
## Testing Code Analysis Tools

The `e2e_analysis_features.rs` test suite now includes tests for:

- `analyze.quality` - Project-wide complexity analysis
- `find_complexity_hotspots` - Identify most complex code
- `analyze.dead_code` - Detect unused code

### Example: Testing Complexity Analysis

```rust
# [tokio::test]
async fn test_analyze_quality_typescript() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create files with varying complexity
    let simple = workspace.path().join("simple.ts");
    std::fs::write(&simple, "export function simple() { return 1; }").unwrap();

    let complex = workspace.path().join("complex.ts");
    std::fs::write(&complex, "export function complex(a, b) { /* complex logic */ }").unwrap();

    // Call analysis tool
    let response = client.call_tool("analyze.quality", json!({
        "kind": "complexity",
        "scope": {"type": "workspace"}
    })).await;

    // Verify results
    assert!(response.is_ok());
    let result = response.unwrap();
    assert!(result["result"]["files"].as_array().unwrap().len() >= 2);
}
```text
## Current Test Coverage

### LSP Feature Tests (Data-Driven)
- **Languages Covered**: TypeScript, Python, Go, Rust
- **Features**: Go to Definition, Find References, Hover, Document Symbols, Workspace Symbols, Completion, Rename

### E2E Integration Tests
- **Analysis Features**: 9 tests (analyze.dead_code, analyze.quality, find_complexity_hotspots)
- **Workflow Execution**: 10 tests (simple workflows, complex workflows, failure handling, dry-run, rollback, batch operations)
- **File Operations**: Tests for create, read, write, delete, rename
- **Refactoring**: Cross-language refactoring tests
- **Workspace Operations**: Directory rename, consolidation, dependency updates
- **Error Scenarios**: Resilience and error handling
- **Performance**: Load and stress testing
- **Server Lifecycle**: LSP server management

### Running All Tests

```bash
# Run all unit tests
cargo nextest run --lib

# Run all integration tests
cargo nextest run --test '*'

# Run specific test suite
cargo nextest run --test lsp_features
cargo nextest run --test e2e_analysis_features
cargo nextest run --test e2e_workflow_execution

# Run with verbose output
cargo nextest run --no-capture

# Run ignored tests (real LSP servers)
cargo nextest run --status-level skip --test-threads=1
```text
## Test Infrastructure: TestClient and Binaries

### Binary Architecture

The codebase has **two server binaries**:

1. **`mill`** (../../apps/mill) - CLI wrapper with lifecycle management
   - Uses a global PID lock file at `/tmp/mill.pid`
   - Prevents multiple instances via file locking
   - Provides commands: `start`, `stop`, `status`, `serve`, etc.
   - **Not suitable for parallel tests** due to lock conflicts

2. **`mill-server`** (../../crates/mill-server) - Core MCP server
   - No PID lock file
   - Can run multiple instances in parallel
   - **Used by TestClient for integration tests**
   - Automatically built by cargo when running tests

### Why TestClient Uses `mill-server`

The `TestClient` (in `../../crates/mill-test-support/src/harness/client.rs`) spawns `mill-server` instead of `mill` to allow tests to run in parallel without PID lock conflicts. Each test gets its own isolated server instance.

**Important**: `mill-server` is **always available** when running tests because:
- It's part of the workspace dependencies
- Cargo automatically builds it when running `cargo test`
- No additional setup required

If you see test failures about "server already running", it means the test is incorrectly trying to use `mill` instead of `mill-server`.

## Test Organization

```text
tests/
├── src/
│   └── harness/           # Test infrastructure
│       ├── test_fixtures.rs    # Language-specific test data
│       ├── test_helpers.rs     # Helper functions
│       ├── test_builder.rs     # Test workspace builder
│       └── ...
├── tests/                 # Integration test files
│   ├── lsp_features.rs         # Data-driven LSP tests
│   ├── lsp_feature_runners.rs  # Test runners
│   ├── e2e_analysis_features.rs    # Analysis tool tests
│   ├── e2e_workflow_execution.rs   # Workflow tests
│   ├── e2e_refactoring_cross_language.rs
│   ├── e2e_workspace_operations.rs
│   └── ...
└── test-fixtures/         # Static test data
```text
## Writing Helper-Based E2E Tests

The E2E test suite (tests/e2e/src/) uses **shared helper functions** to eliminate boilerplate and ensure consistency. After migration (completed Week 3), the suite reduced from **~16,226 LOC to 10,940 LOC** (32% reduction, -5,286 lines).

### Core Test Helpers

All helpers are in `tests/e2e/src/test_helpers.rs`:

#### 1. Standard Test Pattern: `run_tool_test`
```rust
use crate::test_helpers::*;

# [tokio::test]
async fn test_rename_file() {
    run_tool_test(
        &[("old.rs", "pub fn test() {}")],  // Initial files
        "rename",                       // Tool name
        |ws| build_rename_params(ws, "old.rs", "new.rs", "file"),  // Params builder
        |ws| {                              // Verification closure
            assert!(ws.file_exists("new.rs"));
            assert!(!ws.file_exists("old.rs"));
            Ok(())
        }
    ).await.unwrap();
}
```text
**What it does:**
- Creates fresh `TestWorkspace` and `TestClient` (auto-cleanup via Drop)
- Builds params with workspace context (absolute paths)
- Generates plan → applies plan → runs verifications
- Ensures test isolation (no state bleed between tests)

#### 2. Plan Validation Pattern: `run_tool_test_with_plan_validation`
```rust
# [tokio::test]
async fn test_rename_with_metadata_check() {
    run_tool_test_with_plan_validation(
        &[("file.rs", "content")],
        "rename",
        |ws| build_rename_params(ws, "file.rs", "renamed.rs", "file"),
        |plan| {                            // Plan validator
            assert_eq!(plan.get("planType").and_then(|v| v.as_str()), Some("renamePlan"));
            assert!(plan.get("metadata").is_some());
            Ok(())
        },
        |ws| {                              // Result validator
            assert!(ws.file_exists("renamed.rs"));
            Ok(())
        }
    ).await.unwrap();
}
```text
**Use when:** You need to inspect the plan structure/metadata before applying.

#### 3. Dry-Run Pattern: `run_dry_run_test`
```rust
# [tokio::test]
async fn test_rename_dry_run() {
    run_dry_run_test(
        &[("original.rs", "content")],
        "rename",
        |ws| build_rename_params(ws, "original.rs", "renamed.rs", "file"),
        |ws| {                              // Verify no changes
            assert!(ws.file_exists("original.rs"), "Original should still exist");
            assert!(!ws.file_exists("renamed.rs"), "New should NOT exist");
            Ok(())
        }
    ).await.unwrap();
}
```text
**Use when:** Testing that `dryRun: true` doesn't modify the workspace.

#### 4. Mutation Pattern: `run_tool_test_with_mutation`
```rust
# [tokio::test]
async fn test_checksum_validation() {
    run_tool_test_with_mutation(
        &[("file.rs", "original")],
        "rename",
        |ws| build_rename_params(ws, "file.rs", "renamed.rs", "file"),
        |ws, _plan| {                       // Mutation hook (between plan and apply)
            ws.create_file("file.rs", "MODIFIED");  // Corrupt checksum
        },
        |ws| {                              // Verify apply failed
            assert!(ws.file_exists("file.rs"), "Should fail validation, stay unchanged");
            Ok(())
        }
    ).await.unwrap();
}
```text
**Use when:** Testing checksum validation or other plan→apply state changes.

### Parameter Builders

Use these helpers to build tool parameters with absolute paths:

```rust
// Rename operations
build_rename_params(ws, "old.rs", "new.rs", "file")

// Move operations
build_move_params(ws, "src/file.rs", "lib/file.rs", "file")

// Delete operations
build_delete_params(ws, "unused.rs", "file")
```text
**Why closures?** The params builder runs AFTER workspace creation, so paths are always absolute and correct.

### Migration Impact (Week 3 Completion)

**Before (Week 1):** 16,226 LOC
**After (Week 3):** 10,940 LOC
**Reduction:** -5,286 lines (32%)

**Affected files:** 36 test files migrated
- `test_rename_integration.rs`: 357 lines → ~140 lines (61% reduction)
- `test_move_with_imports.rs`: 314 lines → ~176 lines (44% reduction)
- `dry_run_integration.rs`: 747 lines → ~195 lines (74% reduction)
- `test_workspace_apply_integration.rs`: 611 lines → ~195 lines (68% reduction)

**Test performance:**
- Suite runtime: ~2.0s for 198 tests (100% passing)
- Parallel execution maintained via fresh instances

### Best Practices

1. **Always use helpers for refactoring tests** - Avoid manual `TestWorkspace::new()` + `TestClient::new()` patterns
2. **Use closure-based param builders** - Never hardcode paths before workspace creation
3. **One assertion focus per test** - Keep tests simple and readable
4. **Leverage fixtures for complex setups** - See `mill-test-support/src/harness/mcp_fixtures.rs`
5. **Test isolation is automatic** - Each helper creates fresh workspace/client with Drop cleanup

### When NOT to Use Helpers

Some tests intentionally skip helpers:
- **LSP-dependent tests** (extract, inline, transform) - Require error handling for LSP unavailability
- **Complex workspace validation** (consolidation) - Need custom Cargo.toml inspection
- **Analysis tests** - Use different request/response patterns

## Next Steps

To expand test coverage:

1. **Add Java to LSP fixtures** - Extend test coverage to Java language
2. **Add more edge cases** - Test error conditions, multi-file scenarios
3. **Add property-based tests** - Use proptest for fuzzing
4. **Add performance crates/mill-bench** - Track performance over time
5. **Add more workflow scenarios** - Complex multi-step operations

The architecture makes all of these expansions straightforward!