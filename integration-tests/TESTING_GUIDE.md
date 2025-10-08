# Data-Driven Testing Guide

## Architecture Overview

The test suite uses a **data-driven architecture** that separates test logic from test data, making it incredibly easy to add support for new languages.

### Three-Layer Architecture

1. **Fixtures** (`integration-tests/src/harness/test_fixtures.rs`)
   - Language-specific test data
   - Contains code snippets, file names, and expected outcomes
   - Static data structures

2. **Runners** (`integration-tests/tests/lsp_feature_runners.rs`)
   - Generic test logic
   - One runner function per LSP feature
   - Language-agnostic implementation

3. **Test Declarations** (`integration-tests/tests/lsp_features.rs`)
   - Minimal test file
   - Connects fixtures with runners
   - Automatically generates test matrix

## Adding a New Language (e.g., Go)

To add test support for Go, you only need to edit **one file**:

### Step 1: Add Test Cases to Fixtures

Edit `integration-tests/src/harness/test_fixtures.rs`:

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
```

**That's it!** The test infrastructure will automatically:
- Run the new test case for Go
- Use the same test logic as TypeScript and Python
- Generate both mock and real LSP tests

### Step 2: Run the Tests

```bash
# Run mock tests (fast, no dependencies)
cargo test --test lsp_features

# Run with output to see all languages being tested
cargo test --test lsp_features test_go_to_definition_mock -- --nocapture

# Output:
# Running mock go-to-definition test 1/3 for language: ts
# Running mock go-to-definition test 2/3 for language: py
# Running mock go-to-definition test 3/3 for language: go  ← New!
```

## Adding a New LSP Feature

To add tests for a new LSP feature (e.g., "call hierarchy"):

### Step 1: Define Fixture Struct

Add to `integration-tests/src/harness/test_fixtures.rs`:

```rust
#[derive(Debug, Clone)]
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
```

### Step 2: Implement Runner Function

Add to `integration-tests/tests/lsp_feature_runners.rs`:

```rust
pub async fn run_call_hierarchy_test(case: &CallHierarchyTestCase, use_real_lsp: bool) {
    // Implementation similar to other runners
    // - Build test workspace
    // - Send LSP request
    // - Verify response
}
```

### Step 3: Declare Tests

Add to `integration-tests/tests/lsp_features.rs`:

```rust
#[tokio::test]
async fn test_call_hierarchy_mock() {
    for (idx, case) in CALL_HIERARCHY_TESTS.iter().enumerate() {
        println!("Running mock call-hierarchy test {}/{} for language: {}",
                 idx + 1, CALL_HIERARCHY_TESTS.len(), case.language_id);
        run_call_hierarchy_test(case, false).await;
    }
}

#[tokio::test]
#[ignore]
async fn test_call_hierarchy_real() {
    for (idx, case) in CALL_HIERARCHY_TESTS.iter().enumerate() {
        println!("Running real call-hierarchy test {}/{} for language: {}",
                 idx + 1, CALL_HIERARCHY_TESTS.len(), case.language_id);
        run_call_hierarchy_test(case, true).await;
    }
}
```

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
$ cargo test --test lsp_features -- --list

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
```

Each test runs for multiple languages automatically!

## Running Tests

```bash
# Run all mock tests (fast, no LSP servers needed)
cargo test --test lsp_features

# Run with verbose output to see language coverage
cargo test --test lsp_features -- --nocapture

# Run a specific test
cargo test --test lsp_features test_go_to_definition_mock

# Run real LSP tests (requires LSP servers installed)
cargo test --test lsp_features -- --ignored --test-threads=1
```

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
#[tokio::test]
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
```

### Running Workflow Tests

```bash
# Run all workflow tests
cargo test --test e2e_workflow_execution

# Run specific workflow test
cargo test --test e2e_workflow_execution test_execute_simple_workflow

# Run with output
cargo test --test e2e_workflow_execution -- --nocapture
```

## Testing Code Analysis Tools

The `e2e_analysis_features.rs` test suite now includes tests for:

- `analyze_project_complexity` - Project-wide complexity analysis
- `find_complexity_hotspots` - Identify most complex code
- `find_dead_code` - Detect unused code

### Example: Testing Complexity Analysis

```rust
#[tokio::test]
async fn test_analyze_project_complexity_typescript() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create files with varying complexity
    let simple = workspace.path().join("simple.ts");
    std::fs::write(&simple, "export function simple() { return 1; }").unwrap();

    let complex = workspace.path().join("complex.ts");
    std::fs::write(&complex, "export function complex(a, b) { /* complex logic */ }").unwrap();

    // Call analysis tool
    let response = client.call_tool("analyze_project_complexity", json!({})).await;

    // Verify results
    assert!(response.is_ok());
    let result = response.unwrap();
    assert!(result["result"]["files"].as_array().unwrap().len() >= 2);
}
```

## Current Test Coverage

### LSP Feature Tests (Data-Driven)
- **Languages Covered**: TypeScript, Python, Go, Rust
- **Features**: Go to Definition, Find References, Hover, Document Symbols, Workspace Symbols, Completion, Rename

### E2E Integration Tests
- **Analysis Features**: 9 tests (find_dead_code, analyze_project_complexity, find_complexity_hotspots)
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
cargo test --lib

# Run all integration tests
cargo test --test '*'

# Run specific test suite
cargo test --test lsp_features
cargo test --test e2e_analysis_features
cargo test --test e2e_workflow_execution

# Run with verbose output
cargo test -- --nocapture

# Run ignored tests (real LSP servers)
cargo test -- --ignored --test-threads=1
```

## Test Organization

```
integration-tests/
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
```

## Next Steps

To expand test coverage:

1. **Add Java to LSP fixtures** - Extend test coverage to Java language
2. **Add more edge cases** - Test error conditions, multi-file scenarios
3. **Add property-based tests** - Use proptest for fuzzing
4. **Add performance crates/cb-bench** - Track performance over time
5. **Add more workflow scenarios** - Complex multi-step operations

The architecture makes all of these expansions straightforward!
