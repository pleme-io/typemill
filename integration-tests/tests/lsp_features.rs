//! Data-Driven LSP Feature Tests
//!
use integration_tests :: harness :: test_fixtures :: * ;
//! Tests are split into mock tests (fast, no dependencies) and real tests (marked with #[ignore]).
//!
//! ## Architecture
//!
//! The test suite is organized into three components:
//! 1. **Fixtures** (`src/harness/test_fixtures.rs`): Language-specific test data
//! 2. **Runners** (`tests/lsp_feature_runners.rs`): Generic test logic
//! 3. **Test Declarations** (this file): Generates test matrix from fixtures
//!
//! ## Adding a New Language
//!
//! To add test support for a new language (e.g., Go):
//! 1. Add test cases to the fixture arrays in `test_fixtures.rs`
//! 2. Tests will automatically run for the new language - no changes needed here!
//!
//! ## Adding a New Feature
//!
//! To add tests for a new LSP feature:
//! 1. Define a fixture struct in `test_fixtures.rs`
//! 2. Create a runner function in `lsp_feature_runners.rs`
//! 3. Add test declarations here following the pattern below

mod lsp_feature_runners;

use futures::future::join_all;
use integration_tests::harness::test_fixtures::*;
use lsp_feature_runners::*;

// =============================================================================
// Go To Definition Tests
// =============================================================================

#[tokio::test]
async fn test_go_to_definition_mock() {
    let futures = GO_TO_DEFINITION_TESTS
        .iter()
        .map(|case| run_go_to_definition_test(case, false));
    join_all(futures).await;
}

#[tokio::test]
#[ignore] // Requires LSP servers to be installed
#[cfg(feature = "lsp-tests")]
async fn test_go_to_definition_real() {
    let futures = GO_TO_DEFINITION_TESTS
        .iter()
        .map(|case| run_go_to_definition_test(case, true));
    join_all(futures).await;
}

// =============================================================================
// Find References Tests
// =============================================================================

#[tokio::test]
async fn test_find_references_mock() {
    let futures = FIND_REFERENCES_TESTS
        .iter()
        .map(|case| run_find_references_test(case, false));
    join_all(futures).await;
}

#[tokio::test]
#[ignore] // Requires LSP servers to be installed
#[cfg(feature = "lsp-tests")]
async fn test_find_references_real() {
    let futures = FIND_REFERENCES_TESTS
        .iter()
        .map(|case| run_find_references_test(case, true));
    join_all(futures).await;
}

// =============================================================================
// Hover Tests
// =============================================================================

#[tokio::test]
async fn test_hover_mock() {
    let futures = HOVER_TESTS.iter().map(|case| run_hover_test(case, false));
    join_all(futures).await;
}

#[tokio::test]
#[ignore] // Requires LSP servers to be installed
#[cfg(feature = "lsp-tests")]
async fn test_hover_real() {
    let futures = HOVER_TESTS.iter().map(|case| run_hover_test(case, true));
    join_all(futures).await;
}

// =============================================================================
// Document Symbols Tests
// =============================================================================

#[tokio::test]
async fn test_document_symbols_mock() {
    let futures = DOCUMENT_SYMBOLS_TESTS
        .iter()
        .map(|case| run_document_symbols_test(case, false));
    join_all(futures).await;
}

#[tokio::test]
#[ignore] // Requires LSP servers to be installed
#[cfg(feature = "lsp-tests")]
async fn test_document_symbols_real() {
    let futures = DOCUMENT_SYMBOLS_TESTS
        .iter()
        .map(|case| run_document_symbols_test(case, true));
    join_all(futures).await;
}

// =============================================================================
// Workspace Symbols Tests
// =============================================================================

#[tokio::test]
async fn test_workspace_symbols_mock() {
    let futures = WORKSPACE_SYMBOLS_TESTS
        .iter()
        .map(|case| run_workspace_symbols_test(case, false));
    join_all(futures).await;
}

#[tokio::test]
#[ignore] // Requires LSP servers to be installed
#[cfg(feature = "lsp-tests")]
async fn test_workspace_symbols_real() {
    let futures = WORKSPACE_SYMBOLS_TESTS
        .iter()
        .map(|case| run_workspace_symbols_test(case, true));
    join_all(futures).await;
}

// =============================================================================
// Completion Tests
// =============================================================================

#[tokio::test]
async fn test_completion_mock() {
    let futures = COMPLETION_TESTS
        .iter()
        .map(|case| run_completion_test(case, false));
    join_all(futures).await;
}

#[tokio::test]
#[ignore] // Requires LSP servers to be installed
#[cfg(feature = "lsp-tests")]
async fn test_completion_real() {
    let futures = COMPLETION_TESTS
        .iter()
        .map(|case| run_completion_test(case, true));
    join_all(futures).await;
}

// =============================================================================
// Rename Tests
// =============================================================================

#[tokio::test]
async fn test_rename_mock() {
    let futures = RENAME_TESTS.iter().map(|case| run_rename_test(case, false));
    join_all(futures).await;
}

#[tokio::test]
#[ignore] // Requires LSP servers to be installed
#[cfg(feature = "lsp-tests")]
async fn test_rename_real() {
    let futures = RENAME_TESTS.iter().map(|case| run_rename_test(case, true));
    join_all(futures).await;
}

// =============================================================================
// LSP Compliance Suite
// =============================================================================

#[tokio::test]
#[ignore] // Requires real LSP servers to be installed
#[cfg(feature = "lsp-tests")]
async fn test_lsp_compliance_suite() {
    let futures = LSP_COMPLIANCE_TESTS
        .iter()
        .map(|case| run_lsp_compliance_test(case));
    join_all(futures).await;
}