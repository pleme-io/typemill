# Cross-Language Testing Guide

Learn how to test refactoring operations across multiple languages using our parameterized testing framework. Write once, test everywhere.

## Table of Contents
- [Overview](#overview)
- [Architecture](#architecture)
- [Design Philosophy](#design-philosophy)
- [Usage](#usage)
- [Adding a New Scenario](#adding-a-new-scenario)
- [Feature Matrix](#feature-matrix)
- [Best Practices](#best-practices)

## Overview

Instead of writing separate test files for each language (Python, TypeScript, Rust, Go), we use a **single parameterized test** that runs the same logical operation across all languages with language-specific syntax.

## Architecture

### Components

1. **Refactoring Harness** (`crates/cb-test-support/src/harness/refactoring_harness.rs`)
   - `Language` enum - Supported languages with metadata
   - `RefactoringOperation` enum - Operations that can be tested
   - `LanguageFixture` - Language-specific code fixtures
   - `ExpectedBehavior` - Expected test outcomes
   - `RefactoringScenarios` - Predefined equivalent scenarios

2. **Cross-Language Tests** (`apps/codebuddy/tests/e2e_refactoring_cross_language.rs`)
   - Parameterized test functions
   - Language-agnostic test logic
   - Per-language result reporting

## Design Philosophy

### DRY Principle

**❌ Old Approach** (duplicated logic):
```
tests/
├── e2e_python_refactoring.rs       (120 lines)
├── e2e_typescript_refactoring.rs   (120 lines)
├── e2e_rust_refactoring.rs         (120 lines)
└── e2e_go_refactoring.rs           (120 lines)
```
Total: 480 lines of duplicated test logic

**✅ New Approach** (parameterized):
```
tests/
└── e2e_refactoring_cross_language.rs  (300 lines)
```
Total: 300 lines covering all 4 languages

### Consistency

All languages tested identically:
- Same refactoring operation
- Same validation logic
- Same success criteria
- Clear feature matrix

## Usage

### Running Tests

```bash
# Run all cross-language refactoring tests
cargo nextest run -p integration-tests --test e2e_refactoring_cross_language

# Run specific scenario
cargo nextest run -p integration-tests --test e2e_refactoring_cross_language test_extract_simple_expression

# Run with output to see per-language results
cargo nextest run -p integration-tests --test e2e_refactoring_cross_language --no-capture
```

### Example Output

```
=== Testing: extract_simple_expression ===

Testing Python...
[Python] ✓ Refactoring succeeded

Testing TypeScript...
[TypeScript] ✓ Refactoring succeeded

Testing Rust...
[Rust] Not supported - skipping

Testing Go...
[Go] Not supported - skipping

=== Results: 2/2 supported languages passed ===
```

## Adding a New Scenario

### Step 1: Define Language-Equivalent Fixtures

```rust
// In crates/cb-test-support/src/harness/refactoring_harness.rs

impl RefactoringScenarios {
    pub fn your_new_scenario() -> RefactoringTestCase {
        RefactoringTestCase::new("your_scenario_name").with_all_languages(|lang| {
            let (source, operation, behavior) = match lang {
                Language::Python => (
                    "def example():\n    # Python code here\n",
                    RefactoringOperation::ExtractVariable {
                        variable_name: "result".to_string(),
                        start_line: 1,
                        start_char: 4,
                        end_line: 1,
                        end_char: 10,
                    },
                    ExpectedBehavior::Success,
                ),
                Language::TypeScript => (
                    "function example() {\n    // TypeScript code here\n}\n",
                    RefactoringOperation::ExtractVariable {
                        variable_name: "result".to_string(),
                        start_line: 1,
                        start_char: 4,
                        end_line: 1,
                        end_char: 10,
                    },
                    ExpectedBehavior::Success,
                ),
                Language::Rust => (
                    "fn example() {\n    // Rust code here\n}\n",
                    RefactoringOperation::ExtractVariable {
                        variable_name: "result".to_string(),
                        start_line: 1,
                        start_char: 4,
                        end_line: 1,
                        end_char: 10,
                    },
                    ExpectedBehavior::NotSupported, // Not yet implemented
                ),
                Language::Go => (
                    "func example() {\n    // Go code here\n}\n",
                    RefactoringOperation::ExtractVariable {
                        variable_name: "result".to_string(),
                        start_line: 1,
                        start_char: 4,
                        end_line: 1,
                        end_char: 10,
                    },
                    ExpectedBehavior::NotSupported, // Not yet implemented
                ),
            };

            (
                LanguageFixture {
                    language: lang,
                    source_code: source,
                    operation,
                },
                behavior,
            )
        })
    }
}
```

### Step 2: Add to All Scenarios List

```rust
impl RefactoringScenarios {
    pub fn all() -> Vec<RefactoringTestCase> {
        vec![
            Self::extract_simple_expression(),
            Self::extract_multiline_function(),
            Self::inline_simple_variable(),
            Self::your_new_scenario(),  // Add here
        ]
    }
}
```

### Step 3: Create Parameterized Test

```rust
// In apps/codebuddy/tests/e2e_refactoring_cross_language.rs

#[tokio::test]
async fn test_your_new_scenario_cross_language() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let scenario = RefactoringScenarios::your_new_scenario();

    eprintln!("\n=== Testing: {} ===", scenario.scenario_name);

    let mut success_count = 0;
    let mut total_supported = 0;

    for fixture in &scenario.fixtures {
        let expected = scenario
            .expected
            .get(&fixture.language)
            .expect("Expected behavior must be defined");

        eprintln!("\nTesting {:?}...", fixture.language);

        let succeeded = run_single_language_test(
            &workspace,
            &mut client,
            fixture.language,
            fixture.source_code,
            &fixture.operation,
            expected,
        )
        .await;

        if matches!(expected, ExpectedBehavior::Success) {
            total_supported += 1;
            if succeeded {
                success_count += 1;
            }
        }
    }

    eprintln!(
        "\n=== Results: {}/{} supported languages passed ===\n",
        success_count, total_supported
    );

    assert!(
        success_count >= 1,
        "At least 1 language should support your_scenario"
    );
}
```

## Adding a New Language

### Step 1: Add to Language Enum

```rust
// In crates/cb-test-support/src/harness/refactoring_harness.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Python,
    TypeScript,
    Rust,
    Go,
    YourNewLanguage,  // Add here
}

impl Language {
    pub fn all() -> Vec<Language> {
        vec![
            Language::Python,
            Language::TypeScript,
            Language::Rust,
            Language::Go,
            Language::YourNewLanguage,  // Add here
        ]
    }

    pub fn file_extension(&self) -> &'static str {
        match self {
            Language::Python => "py",
            Language::TypeScript => "ts",
            Language::Rust => "rs",
            Language::Go => "go",
            Language::YourNewLanguage => "ext",  // Add here
        }
    }

    pub fn supports_refactoring(&self) -> bool {
        match self {
            Language::Python | Language::TypeScript => true,
            Language::Rust | Language::Go => false,
            Language::YourNewLanguage => true,  // Set based on implementation
        }
    }
}
```

### Step 2: Add Fixtures to All Scenarios

Update each scenario in `RefactoringScenarios` to include your new language:

```rust
Language::YourNewLanguage => (
    "// Your language syntax here",
    RefactoringOperation::ExtractVariable { /* ... */ },
    ExpectedBehavior::Success,
),
```

### Step 3: Run Tests

```bash
cargo nextest run -p integration-tests --test e2e_refactoring_cross_language --no-capture
```

Your new language will automatically be tested in all scenarios!

## Expected Behaviors

### Success

```rust
ExpectedBehavior::Success
```

Test expects:
- Valid response with `result` field
- `status: "completed"` and `success: true`
- Operation completes without errors

### Not Supported

```rust
ExpectedBehavior::NotSupported
```

Test expects:
- Language doesn't have refactoring implementation yet
- Test skips this language gracefully
- No assertion failures

### Expected Error

```rust
ExpectedBehavior::ExpectedError {
    message_contains: Some("specific error text".to_string())
}
```

Test expects:
- Response contains `error` field
- Error message contains specified text (if provided)
- Useful for testing error handling

## Current Feature Matrix

| Operation          | Python | TypeScript | Rust | Go |
|--------------------|--------|------------|------|-----|
| Extract Variable   | ✅     | ✅         | ❌   | ❌  |
| Extract Function   | ✅     | ✅         | ❌   | ❌  |
| Inline Variable    | ✅     | ⚠️*        | ❌   | ❌  |

*TypeScript inline variable has coordinate detection issues in test harness (functionality works via LSP)

## Benefits

### For Developers

- **Write once, test everywhere**: Single test covers all languages
- **Consistency**: Same logic, same validation
- **Easy maintenance**: Update in one place
- **Clear requirements**: See exactly what each language should do

### For New Language Contributors

- **Clear template**: See exactly what fixtures to provide
- **Automatic coverage**: Your language gets tested in all scenarios
- **Feature parity**: Easy to see what operations to implement

### For Code Reviewers

- **Single source of truth**: One place to review test logic
- **Feature matrix**: Clear visibility of language support
- **Regression detection**: Changes affect all languages uniformly

## Best Practices

### 1. Keep Fixtures Logically Equivalent

Python:
```python
def calculate():
    result = 10 + 20
```

TypeScript:
```typescript
function calculate() {
    const result = 10 + 20;
}
```

Rust:
```rust
fn calculate() -> i32 {
    let result = 10 + 20;
}
```

Go:
```go
func calculate() int {
    result := 10 + 20
}
```

Same logic, different syntax!

### 2. Use Accurate Coordinates

Line and character positions are 0-indexed. Count carefully!

```python
def process():
    multiplier = 2
    #   ^^^^^^^^^ line:1, char:4 to char:14
```

### 3. Test One Thing

Each scenario should test ONE refactoring behavior:
- ✅ Good: "Extract simple arithmetic expression"
- ❌ Bad: "Extract expression and inline variable and rename"

### 4. Document Limitations

If a language/operation combo doesn't work, use `ExpectedError` or `NotSupported` and add a comment:

```rust
Language::TypeScript => (
    source,
    operation,
    // TypeScript inline variable has coordinate detection issues
    ExpectedBehavior::ExpectedError { message_contains: None },
),
```

## Troubleshooting

### Test Fails with "Could not find variable"

Check your line/character coordinates:
- Are they 0-indexed?
- Did you count spaces/tabs correctly?
- Try running the test with `--nocapture` to see error details

### Test Passes for Wrong Reason

Check if the language is marked as `NotSupported` - it might be skipping instead of actually testing.

### Adding New Operation Type

1. Add to `RefactoringOperation` enum
2. Implement `to_mcp_tool_name()`
3. Implement `to_json()`
4. Create scenario with all languages
5. Add parameterized test

## See Also

- [crates/cb-test-support/src/harness/refactoring_harness.rs](../../crates/cb-test-support/src/harness/refactoring_harness.rs) - Harness implementation
- [apps/codebuddy/tests/e2e_refactoring_cross_language.rs](../../apps/codebuddy/tests/e2e_refactoring_cross_language.rs) - Test examples
- [crates/languages/README.md](../../crates/languages/README.md) - Language plugin guide
