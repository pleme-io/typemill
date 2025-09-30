//! Data-Driven LSP Feature Tests
//!
//! This module provides comprehensive tests for LSP features across multiple languages.
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

use lsp_feature_runners::*;
use tests::harness::test_fixtures::*;

// =============================================================================
// Go To Definition Tests
// =============================================================================

#[tokio::test]
async fn test_go_to_definition_mock() {
    for (idx, case) in GO_TO_DEFINITION_TESTS.iter().enumerate() {
        println!(
            "Running mock go-to-definition test {}/{} for language: {}",
            idx + 1,
            GO_TO_DEFINITION_TESTS.len(),
            case.language_id
        );
        run_go_to_definition_test(case, false).await;
    }
}

#[tokio::test]
#[ignore] // Requires LSP servers to be installed
async fn test_go_to_definition_real() {
    for (idx, case) in GO_TO_DEFINITION_TESTS.iter().enumerate() {
        println!(
            "Running real go-to-definition test {}/{} for language: {}",
            idx + 1,
            GO_TO_DEFINITION_TESTS.len(),
            case.language_id
        );
        run_go_to_definition_test(case, true).await;
    }
}

// =============================================================================
// Find References Tests
// =============================================================================

#[tokio::test]
async fn test_find_references_mock() {
    for (idx, case) in FIND_REFERENCES_TESTS.iter().enumerate() {
        println!(
            "Running mock find-references test {}/{} for language: {}",
            idx + 1,
            FIND_REFERENCES_TESTS.len(),
            case.language_id
        );
        run_find_references_test(case, false).await;
    }
}

#[tokio::test]
#[ignore] // Requires LSP servers to be installed
async fn test_find_references_real() {
    for (idx, case) in FIND_REFERENCES_TESTS.iter().enumerate() {
        println!(
            "Running real find-references test {}/{} for language: {}",
            idx + 1,
            FIND_REFERENCES_TESTS.len(),
            case.language_id
        );
        run_find_references_test(case, true).await;
    }
}

// =============================================================================
// Hover Tests
// =============================================================================

#[tokio::test]
async fn test_hover_mock() {
    for (idx, case) in HOVER_TESTS.iter().enumerate() {
        println!(
            "Running mock hover test {}/{} for language: {}",
            idx + 1,
            HOVER_TESTS.len(),
            case.language_id
        );
        run_hover_test(case, false).await;
    }
}

#[tokio::test]
#[ignore] // Requires LSP servers to be installed
async fn test_hover_real() {
    for (idx, case) in HOVER_TESTS.iter().enumerate() {
        println!(
            "Running real hover test {}/{} for language: {}",
            idx + 1,
            HOVER_TESTS.len(),
            case.language_id
        );
        run_hover_test(case, true).await;
    }
}

// =============================================================================
// Document Symbols Tests
// =============================================================================

#[tokio::test]
async fn test_document_symbols_mock() {
    for (idx, case) in DOCUMENT_SYMBOLS_TESTS.iter().enumerate() {
        println!(
            "Running mock document-symbols test {}/{} for language: {}",
            idx + 1,
            DOCUMENT_SYMBOLS_TESTS.len(),
            case.language_id
        );
        run_document_symbols_test(case, false).await;
    }
}

#[tokio::test]
#[ignore] // Requires LSP servers to be installed
async fn test_document_symbols_real() {
    for (idx, case) in DOCUMENT_SYMBOLS_TESTS.iter().enumerate() {
        println!(
            "Running real document-symbols test {}/{} for language: {}",
            idx + 1,
            DOCUMENT_SYMBOLS_TESTS.len(),
            case.language_id
        );
        run_document_symbols_test(case, true).await;
    }
}

// =============================================================================
// Workspace Symbols Tests
// =============================================================================

#[tokio::test]
async fn test_workspace_symbols_mock() {
    for (idx, case) in WORKSPACE_SYMBOLS_TESTS.iter().enumerate() {
        println!(
            "Running mock workspace-symbols test {}/{} for language: {}",
            idx + 1,
            WORKSPACE_SYMBOLS_TESTS.len(),
            case.language_id
        );
        run_workspace_symbols_test(case, false).await;
    }
}

#[tokio::test]
#[ignore] // Requires LSP servers to be installed
async fn test_workspace_symbols_real() {
    for (idx, case) in WORKSPACE_SYMBOLS_TESTS.iter().enumerate() {
        println!(
            "Running real workspace-symbols test {}/{} for language: {}",
            idx + 1,
            WORKSPACE_SYMBOLS_TESTS.len(),
            case.language_id
        );
        run_workspace_symbols_test(case, true).await;
    }
}

// =============================================================================
// Completion Tests
// =============================================================================

#[tokio::test]
async fn test_completion_mock() {
    for (idx, case) in COMPLETION_TESTS.iter().enumerate() {
        println!(
            "Running mock completion test {}/{} for language: {}",
            idx + 1,
            COMPLETION_TESTS.len(),
            case.language_id
        );
        run_completion_test(case, false).await;
    }
}

#[tokio::test]
#[ignore] // Requires LSP servers to be installed
async fn test_completion_real() {
    for (idx, case) in COMPLETION_TESTS.iter().enumerate() {
        println!(
            "Running real completion test {}/{} for language: {}",
            idx + 1,
            COMPLETION_TESTS.len(),
            case.language_id
        );
        run_completion_test(case, true).await;
    }
}

// =============================================================================
// Rename Tests
// =============================================================================

#[tokio::test]
async fn test_rename_mock() {
    for (idx, case) in RENAME_TESTS.iter().enumerate() {
        println!(
            "Running mock rename test {}/{} for language: {}",
            idx + 1,
            RENAME_TESTS.len(),
            case.language_id
        );
        run_rename_test(case, false).await;
    }
}

#[tokio::test]
#[ignore] // Requires LSP servers to be installed
async fn test_rename_real() {
    for (idx, case) in RENAME_TESTS.iter().enumerate() {
        println!(
            "Running real rename test {}/{} for language: {}",
            idx + 1,
            RENAME_TESTS.len(),
            case.language_id
        );
        run_rename_test(case, true).await;
    }
}
