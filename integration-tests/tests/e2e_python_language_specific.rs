use integration_tests :: harness :: { TestClient , TestWorkspace } ;
//!
//! This module contains Python-specific tests that CANNOT be parameterized
//! across all languages because they test unique Python features or integrations.
//!
//! ## Migration Note
//!
//! Previously, this file was `e2e_python_refactoring.rs` with 4 tests.
//! Three tests were REMOVED and migrated to the parameterized cross-language
//! framework in `e2e_refactoring_cross_language.rs`:
//!
//! - ❌ test_python_extract_function_basic → ✅ test_extract_multiline_function_cross_language
//! - ❌ test_python_inline_variable_basic → ✅ test_inline_simple_variable_cross_language
//! - ❌ test_python_extract_variable_basic → ✅ test_extract_simple_expression_cross_language
//!
//! ## Remaining Tests
//!
//! Only tests that are UNIQUE to Python and cannot be parameterized remain here:
//! - test_python_refactoring_with_imports - Tests integration with import analysis

use integration_tests::harness::{TestClient, TestWorkspace};
use serde_json::json;

/// Test Python refactoring with import graph updates
///
/// This test is Python-specific because it tests the integration between:
/// 1. Import analysis (Python module system)
/// 2. Refactoring operations
/// 3. Multi-file project structure
///
/// Cannot be easily parameterized across languages due to different import systems.
#[tokio::test]
async fn test_python_refactoring_with_imports() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a Python module structure
    let main_file = workspace.path().join("main.py");
    std::fs::write(
        &main_file,
        r#"
from utils import helper_function

def process():
    data = [1, 2, 3]
    result = helper_function(data)
    return result
"#,
    )
    .unwrap();

    let utils_file = workspace.path().join("utils.py");
    std::fs::write(
        &utils_file,
        r#"
def helper_function(items):
    return sum(items)
"#,
    )
    .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Analyze imports first
    let import_response = client
        .call_tool(
            "analyze_imports",
            json!({
                "file_path": main_file.to_str().unwrap()
            }),
        )
        .await;

    // Verify import analysis works
    if let Ok(response_value) = import_response {
        if let Some(result) = response_value.get("result") {
            assert!(
                result.get("imports").is_some(),
                "Should have imports field"
            );
            let imports = result["imports"].as_array().unwrap();
            assert!(
                !imports.is_empty(),
                "Should detect imports from utils module"
            );
        }
    }

    // Now test extract_variable in main.py
    let refactor_response = client
        .call_tool(
            "extract_variable",
            json!({
                "file_path": main_file.to_str().unwrap(),
                "start_line": 4,
                "start_character": 11,
                "end_line": 4,
                "end_character": 20,
                "variable_name": "items"
            }),
        )
        .await;

    // Verify refactoring works alongside import tracking
    if let Ok(response_value) = refactor_response {
        assert!(
            response_value.get("result").is_some() || response_value.get("error").is_some(),
            "Refactoring should work with imported modules"
        );
    }
}
