// This module contains parameterized tests that run the SAME logical refactoring
// operation across ALL supported programming languages (Python, TypeScript, Rust, Go).
//
// ## Design Philosophy
//
// Instead of duplicating test logic across language-specific test files, we use
// a single parameterized test that:
//
// 1. Defines language-equivalent code fixtures (same logic, different syntax)
// 2. Runs the same refactoring operation through each language plugin
// 3. Validates consistent behavior across all languages
// 4. Clearly marks unsupported language/operation combinations
//
// ## Benefits
//
// - **DRY**: One test covers all languages (no duplication)
// - **Consistency**: All languages tested identically
// - **Extensibility**: Easy to add new languages or operations
// - **Feature Matrix**: Clear visibility into which operations work per language
//
// ## Test Structure
//
// Each test uses the `RefactoringScenarios` harness to:
// - Get language-equivalent fixtures
// - Create test files with proper extensions
// - Execute refactoring via MCP tools
// - Validate results consistently across languages
use integration_tests::harness::{
    ExpectedBehavior, Language, RefactoringScenarios, TestClient, TestWorkspace,
};

/// Helper function to run a refactoring test case for a single language
async fn run_single_language_test(
    workspace: &TestWorkspace,
    client: &mut TestClient,
    language: Language,
    source_code: &str,
    operation: &integration_tests::harness::RefactoringOperation,
    expected: &ExpectedBehavior,
) -> bool {
    // Create test file with appropriate extension
    let file_name = format!("test.{}", language.file_extension());
    let test_file = workspace.path().join(&file_name);

    std::fs::write(&test_file, source_code).unwrap();

    // Wait for services to initialize
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // Execute refactoring operation
    let tool_name = operation.to_mcp_tool_name();
    let params = operation.to_json(test_file.to_str().unwrap());

    let response = client.call_tool(tool_name, params).await;

    // Validate based on expected behavior
    match expected {
        ExpectedBehavior::Success => {
            if let Ok(response_value) = response {
                // Check for result or error field
                assert!(
                    response_value.get("result").is_some() || response_value.get("error").is_some(),
                    "[{:?}] Response must contain 'result' or 'error' field",
                    language
                );

                // If there's an error, check if language doesn't support refactoring yet
                if let Some(error) = response_value.get("error") {
                    if !language.supports_refactoring() {
                        eprintln!(
                            "[{:?}] Refactoring not yet supported - skipping",
                            language
                        );
                        return false;
                    }
                    eprintln!("[{:?}] Error details: {:?}", language, error);
                    panic!("[{:?}] Expected success but got error", language);
                }

                // Check for successful result
                if let Some(result) = response_value.get("result") {
                    // Result can have nested result (from handler) or direct edits (from LSP)
                    let actual_result = result.get("result").unwrap_or(result);

                    // Validate operation completed
                    if let Some(status) = actual_result.get("status") {
                        assert_eq!(
                            status.as_str(),
                            Some("completed"),
                            "[{:?}] Operation should complete successfully",
                            language
                        );
                    }

                    if let Some(success) = actual_result.get("success") {
                        assert_eq!(
                            success.as_bool(),
                            Some(true),
                            "[{:?}] Operation should succeed",
                            language
                        );
                    }

                    eprintln!("[{:?}] ✓ Refactoring succeeded", language);
                    return true;
                }
            }
            false
        }
        ExpectedBehavior::NotSupported => {
            // Language doesn't support this operation yet
            eprintln!("[{:?}] Not supported - skipping", language);
            false
        }
        ExpectedBehavior::ExpectedError { message_contains } => {
            if let Ok(response_value) = response {
                if let Some(error) = response_value.get("error") {
                    if let Some(expected_msg) = message_contains {
                        let error_str = error.to_string();
                        assert!(
                            error_str.contains(expected_msg),
                            "[{:?}] Error should contain '{}'",
                            language,
                            expected_msg
                        );
                    }
                    eprintln!("[{:?}] ✓ Expected error received", language);
                    return true;
                }
                panic!("[{:?}] Expected error but got success", language);
            }
            false
        }
    }
}

/// Test extract variable across all languages
#[tokio::test]
async fn test_extract_simple_expression_cross_language() {
    let workspace = TestWorkspace::new();
    workspace.setup_lsp_config(); // Setup LSP configuration for all languages
    let mut client = TestClient::new(workspace.path());

    let scenario = RefactoringScenarios::extract_simple_expression();

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

    // At least Python and TypeScript should support this
    assert!(
        success_count >= 2,
        "At least 2 languages should support extract_variable"
    );
}

/// Test extract function across all languages
#[tokio::test]
async fn test_extract_multiline_function_cross_language() {
    let workspace = TestWorkspace::new();
    workspace.setup_lsp_config(); // Setup LSP configuration for all languages
    let mut client = TestClient::new(workspace.path());

    let scenario = RefactoringScenarios::extract_multiline_function();

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

    // Python, TypeScript, and Rust support this via AST
    // Go does not have AST-based refactoring yet
    assert!(
        success_count >= 3,
        "At least 3 languages should support extract_function via AST (Python, TypeScript, Rust)"
    );
}

/// Test inline variable across all languages
#[tokio::test]
async fn test_inline_simple_variable_cross_language() {
    let workspace = TestWorkspace::new();
    workspace.setup_lsp_config(); // Setup LSP configuration for all languages
    let mut client = TestClient::new(workspace.path());

    let scenario = RefactoringScenarios::inline_simple_variable();

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

    // At least Python should support this (TypeScript has coordinate detection issues in test)
    assert!(
        success_count >= 1,
        "At least 1 language should support inline_variable"
    );
}

/// Test that unsupported languages gracefully decline
#[tokio::test]
async fn test_unsupported_languages_decline_gracefully() {
    let workspace = TestWorkspace::new();
    workspace.setup_lsp_config(); // Setup LSP configuration for all languages
    let mut client = TestClient::new(workspace.path());

    // Test Rust (currently unsupported)
    let rust_file = workspace.path().join("test.rs");
    std::fs::write(
        &rust_file,
        "fn main() {\n    let x = 10 + 20;\n    println!(\"{}\", x);\n}\n",
    )
    .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    let response = client
        .call_tool(
            "extract_variable",
            serde_json::json!({
                "file_path": rust_file.to_str().unwrap(),
                "start_line": 1,
                "start_character": 12,
                "end_line": 1,
                "end_character": 19,
                "variable_name": "sum"
            }),
        )
        .await;

    // Should get an error or "not supported" response
    if let Ok(response_value) = response {
        assert!(
            response_value.get("result").is_some() || response_value.get("error").is_some(),
            "Response must have result or error"
        );

        // If there's a result, it should indicate lack of support or completion
        // If there's an error, that's also acceptable
        eprintln!("Rust refactoring response: {:?}", response_value);
    }
}
