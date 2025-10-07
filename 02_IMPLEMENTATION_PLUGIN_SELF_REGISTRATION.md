# 📋 IMPLEMENTATION GUIDE: Plugin Self-Registration Architecture

> **Companion to:** `01_PROPOSAL_AUTO_REGISTER_PLUGINS.md`
>
> This document provides **detailed code specifications** for implementing plugin self-registration.
> Read this if you're the coder implementing the changes.

---

## 🎯 Overview

**Goal:** Enable plugins to self-provide test fixtures, eliminating hard-coded language lists in tests.

**Files to modify:** 10 files (3 new, 7 edited)

**Estimated time:** 20-30 minutes

---

## 📁 PHASE 1: API Foundation (Agent 1: Morgan-API)

### File 1: CREATE `crates/cb-plugin-api/src/test_fixtures.rs`

**Purpose:** Define types that plugins can use to provide test fixtures.

**Full file contents:**

```rust
//! Test fixtures that language plugins can optionally provide
//!
//! This module enables plugins to self-provide test scenarios for
//! integration testing. When a new language plugin is added, it can
//! define its own test contracts without modifying the test framework.

use serde::{Deserialize, Serialize};

/// Collection of test fixtures a language plugin can provide
#[derive(Debug, Clone, Default)]
pub struct LanguageTestFixtures {
    /// Complexity analysis test scenarios
    pub complexity_scenarios: Vec<ComplexityFixture>,

    /// Refactoring operation test scenarios
    pub refactoring_scenarios: Vec<RefactoringFixture>,
}

/// A complexity analysis test scenario
#[derive(Debug, Clone)]
pub struct ComplexityFixture {
    /// Scenario identifier (e.g., "simple_function", "nested_complexity")
    pub scenario_name: &'static str,

    /// Source code for this scenario
    pub source_code: &'static str,

    /// Filename with correct extension (e.g., "simple.py")
    pub file_name: &'static str,

    /// Expected cyclomatic complexity range
    pub expected_cyclomatic_min: u32,
    pub expected_cyclomatic_max: u32,

    /// Expected cognitive complexity range
    pub expected_cognitive_min: u32,
    pub expected_cognitive_max: u32,

    /// Expected nesting depth
    pub expected_nesting_depth_min: u32,
}

/// A refactoring test scenario
#[derive(Debug, Clone)]
pub struct RefactoringFixture {
    /// Scenario identifier (e.g., "extract_simple_expression")
    pub scenario_name: &'static str,

    /// Source code for this scenario
    pub source_code: &'static str,

    /// Filename with correct extension
    pub file_name: &'static str,

    /// Refactoring operation to perform
    pub operation: RefactoringOperation,
}

/// Refactoring operation definition
#[derive(Debug, Clone)]
pub enum RefactoringOperation {
    ExtractFunction {
        new_name: String,
        start_line: u32,
        start_char: u32,
        end_line: u32,
        end_char: u32,
    },
    InlineVariable {
        line: u32,
        character: u32,
    },
    ExtractVariable {
        variable_name: String,
        start_line: u32,
        start_char: u32,
        end_line: u32,
        end_char: u32,
    },
}

impl RefactoringOperation {
    /// Convert to MCP tool name
    pub fn to_mcp_tool_name(&self) -> &'static str {
        match self {
            RefactoringOperation::ExtractFunction { .. } => "extract_function",
            RefactoringOperation::InlineVariable { .. } => "inline_variable",
            RefactoringOperation::ExtractVariable { .. } => "extract_variable",
        }
    }

    /// Convert to JSON parameters for MCP call
    pub fn to_json_params(&self, file_path: &str) -> serde_json::Value {
        match self {
            RefactoringOperation::ExtractFunction {
                new_name,
                start_line,
                start_char: _,
                end_line,
                end_char: _,
            } => serde_json::json!({
                "file_path": file_path,
                "start_line": start_line,
                "end_line": end_line,
                "function_name": new_name
            }),
            RefactoringOperation::InlineVariable { line, character } => serde_json::json!({
                "file_path": file_path,
                "line": line,
                "character": character
            }),
            RefactoringOperation::ExtractVariable {
                variable_name,
                start_line,
                start_char,
                end_line,
                end_char,
            } => serde_json::json!({
                "file_path": file_path,
                "start_line": start_line,
                "start_character": start_char,
                "end_line": end_line,
                "end_character": end_char,
                "variable_name": variable_name
            }),
        }
    }
}
```

---

### File 2: EDIT `crates/cb-plugin-api/src/lib.rs`

**Change 1: Add module declaration**

Find this section (around line 54):
```rust
pub mod metadata;
pub mod import_support;
pub mod workspace_support;
```

Add after it:
```rust
pub mod test_fixtures;
```

**Change 2: Add re-exports**

Find this section (around line 61):
```rust
pub use cb_core::language::ProjectLanguage;
pub use metadata::LanguageMetadata;
pub use import_support::ImportSupport;
pub use workspace_support::WorkspaceSupport;
```

Add after it:
```rust
pub use test_fixtures::{
    ComplexityFixture, LanguageTestFixtures, RefactoringFixture, RefactoringOperation,
};
```

**Change 3: Add trait method**

Find the `LanguagePlugin` trait (around line 335). After the `workspace_support()` method, add:

```rust
    /// Provide test fixtures for integration testing (optional)
    ///
    /// Language plugins can optionally provide test fixtures that define
    /// expected behavior for complexity analysis, refactoring operations, etc.
    /// This enables plugins to self-document their capabilities and participate
    /// in cross-language integration tests without modifying the test framework.
    ///
    /// When a plugin returns `Some(fixtures)`, those fixtures will be
    /// automatically discovered and tested by the integration test suite.
    ///
    /// # Returns
    ///
    /// - `Some(fixtures)` if the plugin provides test scenarios
    /// - `None` if the plugin does not participate in cross-language tests
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// fn test_fixtures(&self) -> Option<LanguageTestFixtures> {
    ///     Some(python_test_fixtures())
    /// }
    /// ```
    fn test_fixtures(&self) -> Option<LanguageTestFixtures> {
        None
    }
```

**Verification:**
```bash
cargo check --package cb-plugin-api
```
Should pass with no errors.

---

## 📁 PHASE 2: Python Plugin (Agent 2: Riley-Python)

### File 3: CREATE `crates/languages/cb-lang-python/src/test_fixtures.rs`

**Purpose:** Python's self-provided test scenarios.

**Full file contents:**

```rust
//! Python test fixtures for integration testing
//!
//! This module defines Python-equivalent code samples for cross-language
//! testing. Each scenario represents a specific test case (simple function,
//! complex nested logic, etc.) with expected complexity metrics.

use cb_plugin_api::test_fixtures::*;

/// Get all Python test fixtures
pub fn python_test_fixtures() -> LanguageTestFixtures {
    LanguageTestFixtures {
        complexity_scenarios: vec![
            // Scenario 1: Simple function (CC=1, Cognitive=0)
            ComplexityFixture {
                scenario_name: "simple_function",
                source_code: "def simple(x):\n    return x + 1\n",
                file_name: "simple.py",
                expected_cyclomatic_min: 1,
                expected_cyclomatic_max: 1,
                expected_cognitive_min: 0,
                expected_cognitive_max: 1,
                expected_nesting_depth_min: 0,
            },

            // Scenario 2: Moderate complexity (CC=3)
            ComplexityFixture {
                scenario_name: "moderate_complexity",
                source_code: "def moderate(x):\n    if x > 0:\n        return x * 2\n    elif x < 0:\n        return x * -1\n    else:\n        return 0\n",
                file_name: "moderate.py",
                expected_cyclomatic_min: 3,
                expected_cyclomatic_max: 4,
                expected_cognitive_min: 2,
                expected_cognitive_max: 5,
                expected_nesting_depth_min: 1,
            },

            // Scenario 3: High nested complexity (CC=7+)
            ComplexityFixture {
                scenario_name: "high_nested_complexity",
                source_code: "def complex_nested(a, b, c):\n    if a > 0:\n        if b > 0:\n            if c > 0:\n                return a + b + c\n            else:\n                return a + b\n        elif c > 0:\n            return a + c\n        else:\n            return a\n    elif b > 0:\n        if c > 0:\n            return b + c\n        else:\n            return b\n    else:\n        return c if c else 0\n",
                file_name: "complex.py",
                expected_cyclomatic_min: 7,
                expected_cyclomatic_max: 10,
                expected_cognitive_min: 10,
                expected_cognitive_max: 20,
                expected_nesting_depth_min: 3,
            },

            // Scenario 4: Flat with early returns
            ComplexityFixture {
                scenario_name: "flat_early_returns",
                source_code: "def flat_guards(a, b, c):\n    if not a:\n        return False\n    if not b:\n        return False\n    if not c:\n        return False\n    return True\n",
                file_name: "flat.py",
                expected_cyclomatic_min: 4,
                expected_cyclomatic_max: 5,
                expected_cognitive_min: 3,
                expected_cognitive_max: 8,
                expected_nesting_depth_min: 1,
            },
        ],

        refactoring_scenarios: vec![
            // Extract variable
            RefactoringFixture {
                scenario_name: "extract_simple_expression",
                source_code: "def calculate():\n    result = 10 + 20\n    return result\n",
                file_name: "extract_var.py",
                operation: RefactoringOperation::ExtractVariable {
                    variable_name: "sum".to_string(),
                    start_line: 1,
                    start_char: 13,
                    end_line: 1,
                    end_char: 19,
                },
            },

            // Extract function
            RefactoringFixture {
                scenario_name: "extract_multiline_function",
                source_code: "def main():\n    x = 1\n    y = 2\n    result = x + y\n    print(result)\n",
                file_name: "extract_func.py",
                operation: RefactoringOperation::ExtractFunction {
                    new_name: "add_numbers".to_string(),
                    start_line: 1,
                    start_char: 4,
                    end_line: 3,
                    end_char: 18,
                },
            },

            // Inline variable
            RefactoringFixture {
                scenario_name: "inline_simple_variable",
                source_code: "def process():\n    multiplier = 2\n    result = 10 * multiplier\n    return result\n",
                file_name: "inline_var.py",
                operation: RefactoringOperation::InlineVariable {
                    line: 1,
                    character: 4,
                },
            },
        ],
    }
}
```

---

### File 4: EDIT `crates/languages/cb-lang-python/src/lib.rs`

**Change 1: Add module declaration**

Find this section (around line 24):
```rust
pub mod import_support;
pub mod manifest;
pub mod parser;
pub mod refactoring;
pub mod workspace_support;
```

Add after `pub mod refactoring;`:
```rust
pub mod test_fixtures;
```

**Change 2: Add trait method implementation**

Find the `impl LanguagePlugin for PythonPlugin` block (around line 68).

After the `workspace_support()` method (around line 110), add:

```rust
    fn test_fixtures(&self) -> Option<cb_plugin_api::LanguageTestFixtures> {
        Some(test_fixtures::python_test_fixtures())
    }
```

**Verification:**
```bash
cargo check --package cb-lang-python
```
Should pass with no errors.

---

## 📁 PHASE 3: Test Framework (Agent 3: Taylor-Tests)

### File 5: CREATE `integration-tests/src/harness/plugin_discovery.rs`

**Purpose:** Helper functions for discovering plugins with test fixtures.

**Full file contents:**

```rust
//! Plugin discovery helpers for integration testing
//!
//! This module provides utilities for discovering language plugins
//! and their test fixtures at runtime. It enables truly dynamic
//! plugin testing where adding a new language plugin automatically
//! includes it in the test suite.

use cb_handlers::LanguagePluginRegistry;
use cb_plugin_api::{LanguagePlugin, LanguageTestFixtures};

/// Discover all installed language plugins that provide test fixtures
///
/// This function queries the plugin registry and returns all plugins
/// that have implemented the `test_fixtures()` method.
///
/// # Returns
///
/// A vector of tuples containing:
/// - Reference to the plugin
/// - The test fixtures it provides
///
/// # Example
///
/// ```rust,ignore
/// let plugins = discover_plugins_with_fixtures();
/// for (plugin, fixtures) in plugins {
///     println!("Found plugin: {}", plugin_language_name(plugin));
///     println!("  - {} complexity scenarios", fixtures.complexity_scenarios.len());
///     println!("  - {} refactoring scenarios", fixtures.refactoring_scenarios.len());
/// }
/// ```
pub fn discover_plugins_with_fixtures() -> Vec<(&'static dyn LanguagePlugin, LanguageTestFixtures)> {
    let registry = LanguagePluginRegistry::new();
    registry.plugins_with_fixtures()
}

/// Get the display name of a language plugin
///
/// Useful for logging and error messages.
pub fn plugin_language_name(plugin: &dyn LanguagePlugin) -> &str {
    plugin.metadata().name
}

/// Get the file extension for a language plugin
///
/// Returns the first registered extension (e.g., "py", "ts", "rs").
pub fn plugin_file_extension(plugin: &dyn LanguagePlugin) -> &str {
    plugin.metadata().extensions[0]
}
```

---

### File 6: EDIT `integration-tests/src/harness/mod.rs`

**Change 1: Add module declaration**

Find the module declarations (around line 1). Add:
```rust
pub mod plugin_discovery;
```

**Change 2: Add re-export**

Find the re-exports section (around line 14). Add:
```rust
pub use plugin_discovery::*;
```

---

### File 7: EDIT `crates/cb-handlers/src/language_plugin_registry.rs`

**Add new method**

Find the `impl LanguagePluginRegistry` block (around line 25).

After the `supports_extension()` method (around line 84), add:

```rust
    /// Get all plugins that provide test fixtures
    ///
    /// This method filters the registered plugins to return only those
    /// that have implemented the `test_fixtures()` method and returned
    /// `Some(fixtures)`.
    ///
    /// Used by integration tests to discover available test scenarios.
    ///
    /// # Returns
    ///
    /// A vector of tuples containing:
    /// - Reference to the plugin
    /// - The test fixtures it provides
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let registry = LanguagePluginRegistry::new();
    /// for (plugin, fixtures) in registry.plugins_with_fixtures() {
    ///     println!("Testing {}", plugin.metadata().name);
    ///     for scenario in &fixtures.complexity_scenarios {
    ///         // Run test with scenario
    ///     }
    /// }
    /// ```
    pub fn plugins_with_fixtures(&self) -> Vec<(&dyn LanguagePlugin, cb_plugin_api::LanguageTestFixtures)> {
        self.inner
            .all()
            .iter()
            .filter_map(|plugin| {
                let fixtures = plugin.test_fixtures()?;
                Some((plugin.as_ref(), fixtures))
            })
            .collect()
    }
```

---

### File 8: EDIT `integration-tests/tests/e2e_analysis_features.rs`

**Change 1: Update imports**

Find the imports at the top of the file. Replace:
```rust
use integration_tests::harness::{ComplexityScenarios, TestClient, TestWorkspace};
```

With:
```rust
use integration_tests::harness::{
    discover_plugins_with_fixtures,
    plugin_language_name,
    TestClient,
    TestWorkspace,
};
```

**Change 2: Refactor test_analyze_project_complexity_cross_language**

Find the function `test_analyze_project_complexity_cross_language` (around line 409).

Replace the entire function body with:

```rust
/// Test analyze_project_complexity across all installed language plugins
#[tokio::test]
async fn test_analyze_project_complexity_cross_language() {
    let plugins_with_fixtures = discover_plugins_with_fixtures();

    if plugins_with_fixtures.is_empty() {
        eprintln!("⚠️  No plugins with test fixtures found - skipping test");
        return;
    }

    for (plugin, fixtures) in plugins_with_fixtures {
        let lang_name = plugin_language_name(plugin);

        for scenario in &fixtures.complexity_scenarios {
            let workspace = TestWorkspace::new();
            let mut client = TestClient::new(workspace.path());

            // Create language-specific file
            let test_file = workspace.path().join(scenario.file_name);
            std::fs::write(&test_file, scenario.source_code).unwrap();

            // Wait for analysis to initialize
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Call analyze_project_complexity
            let response = client
                .call_tool(
                    "analyze_project_complexity",
                    json!({
                        "directory_path": workspace.path().to_str().unwrap()
                    }),
                )
                .await;

            // Validate response
            assert!(
                response.is_ok(),
                "[{}] {} - analyze_project_complexity should succeed",
                lang_name,
                scenario.scenario_name
            );

            let response_value = response.unwrap();

            // Verify response structure (language-agnostic)
            assert!(
                response_value.get("result").is_some(),
                "[{}] {} - Response should have result field",
                lang_name,
                scenario.scenario_name
            );

            let result = &response_value["result"];

            // Verify required fields exist
            assert!(
                result.get("files").is_some(),
                "[{}] {} - Result should have files array",
                lang_name,
                scenario.scenario_name
            );
            assert!(
                result.get("total_files").is_some(),
                "[{}] {} - Result should have total_files",
                lang_name,
                scenario.scenario_name
            );
            assert!(
                result.get("total_functions").is_some(),
                "[{}] {} - Result should have total_functions",
                lang_name,
                scenario.scenario_name
            );

            // Validate files structure
            let files = result["files"].as_array().unwrap();
            if !files.is_empty() {
                for file in files {
                    assert!(
                        file.get("file_path").is_some(),
                        "[{}] {} - File should have file_path",
                        lang_name,
                        scenario.scenario_name
                    );
                    assert!(
                        file.get("function_count").is_some(),
                        "[{}] {} - File should have function_count",
                        lang_name,
                        scenario.scenario_name
                    );
                }
            }

            eprintln!(
                "✅ [{}] {} - Test passed",
                lang_name, scenario.scenario_name
            );
        }
    }
}
```

**Change 3: Refactor test_find_complexity_hotspots_cross_language**

Find the function `test_find_complexity_hotspots_cross_language` (around line 507).

Replace the entire function body with:

```rust
/// Test find_complexity_hotspots across all installed language plugins
#[tokio::test]
async fn test_find_complexity_hotspots_cross_language() {
    let plugins_with_fixtures = discover_plugins_with_fixtures();

    if plugins_with_fixtures.is_empty() {
        eprintln!("⚠️  No plugins with test fixtures found - skipping test");
        return;
    }

    for (plugin, fixtures) in plugins_with_fixtures {
        let lang_name = plugin_language_name(plugin);
        let file_ext = plugin.metadata().extensions[0];

        // Find simple and complex scenarios
        let simple_scenario = fixtures
            .complexity_scenarios
            .iter()
            .find(|s| s.scenario_name == "simple_function");
        let complex_scenario = fixtures
            .complexity_scenarios
            .iter()
            .find(|s| s.scenario_name == "high_nested_complexity");

        if simple_scenario.is_none() || complex_scenario.is_none() {
            eprintln!("[{}] Missing required scenarios - skipping", lang_name);
            continue;
        }

        let simple = simple_scenario.unwrap();
        let complex = complex_scenario.unwrap();

        let workspace = TestWorkspace::new();
        let mut client = TestClient::new(workspace.path());

        // Create both files
        let simple_file = workspace.path().join(format!("simple.{}", file_ext));
        let complex_file = workspace.path().join(format!("complex.{}", file_ext));

        std::fs::write(&simple_file, simple.source_code).unwrap();
        std::fs::write(&complex_file, complex.source_code).unwrap();

        // Wait for analysis
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Call find_complexity_hotspots
        let response = client
            .call_tool(
                "find_complexity_hotspots",
                json!({
                    "directory_path": workspace.path().to_str().unwrap(),
                    "limit": 5
                }),
            )
            .await;

        assert!(
            response.is_ok(),
            "[{}] find_complexity_hotspots should succeed",
            lang_name
        );

        let response_value = response.unwrap();
        assert!(
            response_value.get("result").is_some(),
            "[{}] Response should have result field",
            lang_name
        );

        let result = &response_value["result"];

        // Verify structure
        assert!(
            result.get("top_functions").is_some(),
            "[{}] Result should have top_functions array",
            lang_name
        );
        assert!(
            result.get("summary").is_some(),
            "[{}] Result should have summary",
            lang_name
        );

        eprintln!("✅ [{}] Hotspots test passed", lang_name);
    }
}
```

**Verification:**
```bash
cargo test --test e2e_analysis_features
```
Should pass: 7 tests, 7 passed

---

## ✅ FINAL VERIFICATION

After all changes, run:

```bash
# Check everything compiles
cargo check --all-features

# Run analysis tests
cargo test --test e2e_analysis_features

# Expected result: 7 tests, 7 passed (same as baseline)
```

---

## 🎯 SUCCESS CRITERIA

- ✅ `cargo check --package cb-plugin-api` passes
- ✅ `cargo check --package cb-lang-python` passes
- ✅ `cargo test --test e2e_analysis_features` passes (7/7)
- ✅ Python test fixtures discovered automatically
- ✅ No hard-coded `Language` enum usage in tests

---

## 🚀 WHAT HAPPENS NEXT

After this implementation:

1. **Adding TypeScript fixtures:** Create `cb-lang-typescript/src/test_fixtures.rs` → Auto-discovered
2. **Adding Java plugin:** Create `cb-lang-java/` with test fixtures → Auto-discovered
3. **Community plugins:** Work in separate repos → Auto-discovered when installed

**Zero test framework changes needed for any of these!**
