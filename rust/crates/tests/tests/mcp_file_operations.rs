//! Data-Driven MCP File Operation Tests
//!
//! This module provides comprehensive tests for MCP file operation handlers.
//! Tests are split into mock tests (fast, using FileService directly) and real tests
//! (marked with #[ignore], using TestClient and MCP protocol).
//!
//! ## Architecture
//!
//! The test suite is organized into three components:
//! 1. **Fixtures** (`src/harness/mcp_fixtures.rs`): Test case data
//! 2. **Runners** (`tests/mcp_handler_runners.rs`): Generic test logic
//! 3. **Test Declarations** (this file): Generates test matrix from fixtures
//!
//! ## Adding a New Test Case
//!
//! To add a new test scenario for an existing operation:
//! 1. Add a test case to the appropriate fixture array in `mcp_fixtures.rs`
//! 2. Tests will automatically run - no changes needed here!
//!
//! ## Adding a New Operation
//!
//! To add tests for a new MCP file operation:
//! 1. Define a fixture struct in `mcp_fixtures.rs`
//! 2. Create a runner function in `mcp_handler_runners.rs`
//! 3. Add test declarations here following the pattern below

mod mcp_handler_runners;

use mcp_handler_runners::*;
use tests::harness::mcp_fixtures::*;

// =============================================================================
// Create File Tests
// =============================================================================

#[tokio::test]
async fn test_create_file_mock() {
    for (idx, case) in CREATE_FILE_TESTS.iter().enumerate() {
        println!(
            "Running mock create_file test {}/{}: {}",
            idx + 1,
            CREATE_FILE_TESTS.len(),
            case.test_name
        );
        run_create_file_test(case, false).await;
    }
}

#[tokio::test]
#[ignore] // Requires MCP server to be running
async fn test_create_file_real() {
    for (idx, case) in CREATE_FILE_TESTS.iter().enumerate() {
        println!(
            "Running real create_file test {}/{}: {}",
            idx + 1,
            CREATE_FILE_TESTS.len(),
            case.test_name
        );
        run_create_file_test(case, true).await;
    }
}

// =============================================================================
// Read File Tests
// =============================================================================

#[tokio::test]
async fn test_read_file_mock() {
    for (idx, case) in READ_FILE_TESTS.iter().enumerate() {
        println!(
            "Running mock read_file test {}/{}: {}",
            idx + 1,
            READ_FILE_TESTS.len(),
            case.test_name
        );
        run_read_file_test(case, false).await;
    }
}

#[tokio::test]
#[ignore] // Requires MCP server to be running
async fn test_read_file_real() {
    for (idx, case) in READ_FILE_TESTS.iter().enumerate() {
        println!(
            "Running real read_file test {}/{}: {}",
            idx + 1,
            READ_FILE_TESTS.len(),
            case.test_name
        );
        run_read_file_test(case, true).await;
    }
}

// =============================================================================
// Write File Tests
// =============================================================================

#[tokio::test]
async fn test_write_file_mock() {
    for (idx, case) in WRITE_FILE_TESTS.iter().enumerate() {
        println!(
            "Running mock write_file test {}/{}: {}",
            idx + 1,
            WRITE_FILE_TESTS.len(),
            case.test_name
        );
        run_write_file_test(case, false).await;
    }
}

#[tokio::test]
#[ignore] // Requires MCP server to be running
async fn test_write_file_real() {
    for (idx, case) in WRITE_FILE_TESTS.iter().enumerate() {
        println!(
            "Running real write_file test {}/{}: {}",
            idx + 1,
            WRITE_FILE_TESTS.len(),
            case.test_name
        );
        run_write_file_test(case, true).await;
    }
}

// =============================================================================
// Delete File Tests
// =============================================================================

#[tokio::test]
async fn test_delete_file_mock() {
    for (idx, case) in DELETE_FILE_TESTS.iter().enumerate() {
        println!(
            "Running mock delete_file test {}/{}: {}",
            idx + 1,
            DELETE_FILE_TESTS.len(),
            case.test_name
        );
        run_delete_file_test(case, false).await;
    }
}

#[tokio::test]
#[ignore] // Requires MCP server to be running
async fn test_delete_file_real() {
    for (idx, case) in DELETE_FILE_TESTS.iter().enumerate() {
        println!(
            "Running real delete_file test {}/{}: {}",
            idx + 1,
            DELETE_FILE_TESTS.len(),
            case.test_name
        );
        run_delete_file_test(case, true).await;
    }
}

// =============================================================================
// List Files Tests
// =============================================================================

#[tokio::test]
async fn test_list_files_mock() {
    for (idx, case) in LIST_FILES_TESTS.iter().enumerate() {
        println!(
            "Running mock list_files test {}/{}: {}",
            idx + 1,
            LIST_FILES_TESTS.len(),
            case.test_name
        );
        run_list_files_test(case, false).await;
    }
}

#[tokio::test]
#[ignore] // Requires MCP server to be running
async fn test_list_files_real() {
    for (idx, case) in LIST_FILES_TESTS.iter().enumerate() {
        println!(
            "Running real list_files test {}/{}: {}",
            idx + 1,
            LIST_FILES_TESTS.len(),
            case.test_name
        );
        run_list_files_test(case, true).await;
    }
}


// =============================================================================
// Analyze Imports Tests
// =============================================================================

#[tokio::test]
async fn test_analyze_imports_mock() {
    for (idx, case) in ANALYZE_IMPORTS_TESTS.iter().enumerate() {
        println!(
            "Running mock analyze_imports test {}/{}: {}",
            idx + 1,
            ANALYZE_IMPORTS_TESTS.len(),
            case.test_name
        );
        run_analyze_imports_test(case, false).await;
    }
}

#[tokio::test]
#[ignore] // Requires MCP server to be running
async fn test_analyze_imports_real() {
    for (idx, case) in ANALYZE_IMPORTS_TESTS.iter().enumerate() {
        println!(
            "Running real analyze_imports test {}/{}: {}",
            idx + 1,
            ANALYZE_IMPORTS_TESTS.len(),
            case.test_name
        );
        run_analyze_imports_test(case, true).await;
    }
}

// =============================================================================
// Find Dead Code Tests
// =============================================================================

#[tokio::test]
async fn test_find_dead_code_mock() {
    for (idx, case) in FIND_DEAD_CODE_TESTS.iter().enumerate() {
        println!(
            "Running mock find_dead_code test {}/{}: {}",
            idx + 1,
            FIND_DEAD_CODE_TESTS.len(),
            case.test_name
        );
        run_find_dead_code_test(case, false).await;
    }
}

#[tokio::test]
#[ignore] // Requires MCP server to be running
async fn test_find_dead_code_real() {
    for (idx, case) in FIND_DEAD_CODE_TESTS.iter().enumerate() {
        println!(
            "Running real find_dead_code test {}/{}: {}",
            idx + 1,
            FIND_DEAD_CODE_TESTS.len(),
            case.test_name
        );
        run_find_dead_code_test(case, true).await;
    }
}

// =============================================================================
// Rename Directory Tests
// =============================================================================

#[tokio::test]
async fn test_rename_directory_mock() {
    for (idx, case) in RENAME_DIRECTORY_TESTS.iter().enumerate() {
        println!(
            "Running mock rename_directory test {}/{}: {}",
            idx + 1,
            RENAME_DIRECTORY_TESTS.len(),
            case.test_name
        );
        run_rename_directory_test(case, false).await;
    }
}

#[tokio::test]
#[ignore] // Requires MCP server to be running
async fn test_rename_directory_real() {
    for (idx, case) in RENAME_DIRECTORY_TESTS.iter().enumerate() {
        println!(
            "Running real rename_directory test {}/{}: {}",
            idx + 1,
            RENAME_DIRECTORY_TESTS.len(),
            case.test_name
        );
        run_rename_directory_test(case, true).await;
    }
}
// =============================================================================
// Rename File Tests
// =============================================================================

#[tokio::test]
async fn test_rename_file_mock() {
    for (idx, case) in RENAME_FILE_TESTS.iter().enumerate() {
        println!(
            "Running mock rename_file test {}/{}: {}",
            idx + 1,
            RENAME_FILE_TESTS.len(),
            case.test_name
        );
        run_rename_file_test(case, false).await;
    }
}

#[tokio::test]
#[ignore] // Requires MCP server to be running
async fn test_rename_file_real() {
    for (idx, case) in RENAME_FILE_TESTS.iter().enumerate() {
        println!(
            "Running real rename_file test {}/{}: {}",
            idx + 1,
            RENAME_FILE_TESTS.len(),
            case.test_name
        );
        run_rename_file_test(case, true).await;
    }
}
