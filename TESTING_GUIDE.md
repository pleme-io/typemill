# Data-Driven Testing Guide

## Architecture Overview

The test suite uses a **data-driven architecture** that separates test logic from test data, making it incredibly easy to add support for new languages.

### Three-Layer Architecture

1. **Fixtures** (`rust/crates/tests/src/harness/test_fixtures.rs`)
   - Language-specific test data
   - Contains code snippets, file names, and expected outcomes
   - Static data structures

2. **Runners** (`rust/crates/tests/tests/lsp_feature_runners.rs`)
   - Generic test logic
   - One runner function per LSP feature
   - Language-agnostic implementation

3. **Test Declarations** (`rust/crates/tests/tests/lsp_features.rs`)
   - Minimal test file
   - Connects fixtures with runners
   - Automatically generates test matrix

## Adding a New Language (e.g., Go)

To add test support for Go, you only need to edit **one file**:

### Step 1: Add Test Cases to Fixtures

Edit `rust/crates/tests/src/harness/test_fixtures.rs`:

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

Add to `rust/crates/tests/src/harness/test_fixtures.rs`:

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

Add to `rust/crates/tests/tests/lsp_feature_runners.rs`:

```rust
pub async fn run_call_hierarchy_test(case: &CallHierarchyTestCase, use_real_lsp: bool) {
    // Implementation similar to other runners
    // - Build test workspace
    // - Send LSP request
    // - Verify response
}
```

### Step 3: Declare Tests

Add to `rust/crates/tests/tests/lsp_features.rs`:

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

## Next Steps

To expand test coverage:

1. **Add more languages**: Add Go, Rust, Java cases to fixture arrays
2. **Add more features**: Implement formatting, code actions, etc.
3. **Enhance assertions**: Add more specific validation in runners
4. **Add edge cases**: Test error conditions, multi-file scenarios

The architecture makes all of these expansions straightforward!
