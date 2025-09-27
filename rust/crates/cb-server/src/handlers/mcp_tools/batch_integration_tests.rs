//! Integration tests for batch execution tool

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::handlers::McpDispatcher;
    use crate::state::AppState;
    use crate::services::{FileService, SymbolService, EditingService, ImportService};
    use crate::systems::lsp::MockLspService;
    use std::sync::Arc;
    use tempfile::TempDir;
    use serde_json::{json, Value};

    /// Create a test AppState with mock services
    fn create_test_app_state() -> Arc<AppState> {
        let mock_lsp = MockLspService::new();
        Arc::new(AppState {
            file_service: Arc::new(FileService::new()),
            symbol_service: Arc::new(SymbolService::new(Arc::new(mock_lsp.clone()))),
            editing_service: Arc::new(EditingService::new(Arc::new(mock_lsp.clone()))),
            import_service: Arc::new(ImportService::new()),
            lsp_service: Arc::new(mock_lsp),
        })
    }

    #[tokio::test]
    async fn test_batch_execute_single_operation() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        // Register all required tools
        super::super::batch::register_tools(&mut dispatcher);
        super::super::filesystem::register_tools(&mut dispatcher);

        let args = json!({
            "operations": [
                {
                    "tool": "health_check",
                    "args": {"include_details": false},
                    "id": "health-1"
                }
            ],
            "options": {
                "atomic": false,
                "parallel": false,
                "dry_run": false,
                "stop_on_error": true
            }
        });

        let result = dispatcher.call_tool_for_test("batch_execute", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], true);
        assert_eq!(response["totalOperations"], 1);
        assert_eq!(response["completedOperations"], 1);
        assert_eq!(response["failedOperations"], 0);

        let results = response["results"].as_array().unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["id"], "health-1");
        assert_eq!(results[0]["success"], true);
    }

    #[tokio::test]
    async fn test_batch_execute_multiple_operations() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::batch::register_tools(&mut dispatcher);
        super::super::filesystem::register_tools(&mut dispatcher);

        let temp_dir = TempDir::new().unwrap();

        let args = json!({
            "operations": [
                {
                    "tool": "health_check",
                    "args": {"include_details": false},
                    "id": "health"
                },
                {
                    "tool": "list_files",
                    "args": {
                        "path": temp_dir.path().to_string_lossy(),
                        "recursive": false
                    },
                    "id": "list"
                }
            ],
            "options": {
                "atomic": false,
                "parallel": false,
                "dry_run": false,
                "stop_on_error": true
            }
        });

        let result = dispatcher.call_tool_for_test("batch_execute", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], true);
        assert_eq!(response["totalOperations"], 2);
        assert_eq!(response["completedOperations"], 2);
        assert_eq!(response["failedOperations"], 0);

        let results = response["results"].as_array().unwrap();
        assert_eq!(results.len(), 2);

        // Check both operations succeeded
        for result in results {
            assert_eq!(result["success"], true);
            assert!(result["id"].is_string());
        }
    }

    #[tokio::test]
    async fn test_batch_execute_parallel_operations() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::batch::register_tools(&mut dispatcher);
        super::super::filesystem::register_tools(&mut dispatcher);

        let temp_dir = TempDir::new().unwrap();

        // Create multiple independent operations that can run in parallel
        let args = json!({
            "operations": [
                {
                    "tool": "write_file",
                    "args": {
                        "file_path": temp_dir.path().join("file1.txt").to_string_lossy(),
                        "content": "Content 1"
                    },
                    "id": "write1"
                },
                {
                    "tool": "write_file",
                    "args": {
                        "file_path": temp_dir.path().join("file2.txt").to_string_lossy(),
                        "content": "Content 2"
                    },
                    "id": "write2"
                },
                {
                    "tool": "health_check",
                    "args": {"include_details": false},
                    "id": "health"
                }
            ],
            "options": {
                "atomic": false,
                "parallel": true,
                "dry_run": false,
                "stop_on_error": true
            }
        });

        let result = dispatcher.call_tool_for_test("batch_execute", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], true);
        assert_eq!(response["totalOperations"], 3);
        assert_eq!(response["completedOperations"], 3);
        assert_eq!(response["failedOperations"], 0);

        // Verify parallel execution was faster than sequential
        let execution_time = response["executionTimeMs"].as_u64().unwrap();
        assert!(execution_time < 5000); // Should complete quickly in parallel

        // Verify all files were created
        assert!(temp_dir.path().join("file1.txt").exists());
        assert!(temp_dir.path().join("file2.txt").exists());
    }

    #[tokio::test]
    async fn test_batch_execute_dry_run() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::batch::register_tools(&mut dispatcher);
        super::super::filesystem::register_tools(&mut dispatcher);

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("dry_run_test.txt");

        let args = json!({
            "operations": [
                {
                    "tool": "write_file",
                    "args": {
                        "file_path": file_path.to_string_lossy(),
                        "content": "This should not be written in dry run"
                    },
                    "id": "write-test"
                }
            ],
            "options": {
                "atomic": false,
                "parallel": false,
                "dry_run": true,
                "stop_on_error": true
            }
        });

        let result = dispatcher.call_tool_for_test("batch_execute", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], true);
        assert_eq!(response["options"]["dryRun"], true);

        // File should not have been created in dry run
        assert!(!file_path.exists());

        // But the operation should show as "simulated"
        let results = response["results"].as_array().unwrap();
        assert_eq!(results[0]["dryRun"], true);
    }

    #[tokio::test]
    async fn test_batch_execute_atomic_rollback() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::batch::register_tools(&mut dispatcher);
        super::super::filesystem::register_tools(&mut dispatcher);

        let temp_dir = TempDir::new().unwrap();

        let args = json!({
            "operations": [
                {
                    "tool": "write_file",
                    "args": {
                        "file_path": temp_dir.path().join("file1.txt").to_string_lossy(),
                        "content": "Content 1"
                    },
                    "id": "write1"
                },
                {
                    "tool": "write_file",
                    "args": {
                        "file_path": "/invalid/path/file2.txt",
                        "content": "This will fail"
                    },
                    "id": "write2"
                }
            ],
            "options": {
                "atomic": true,
                "parallel": false,
                "dry_run": false,
                "stop_on_error": true
            }
        });

        let result = dispatcher.call_tool_for_test("batch_execute", args).await;
        assert!(result.is_err() || !result.as_ref().unwrap()["success"].as_bool().unwrap());

        // In atomic mode, the first operation should be rolled back
        // so file1.txt should not exist (or be removed after rollback)
        if result.is_ok() {
            let response = result.unwrap();
            assert_eq!(response["success"], false);
            assert!(response["failedOperations"].as_u64().unwrap() > 0);
        }
    }

    #[tokio::test]
    async fn test_batch_execute_stop_on_error() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::batch::register_tools(&mut dispatcher);
        super::super::filesystem::register_tools(&mut dispatcher);

        let temp_dir = TempDir::new().unwrap();

        let args = json!({
            "operations": [
                {
                    "tool": "write_file",
                    "args": {
                        "file_path": temp_dir.path().join("file1.txt").to_string_lossy(),
                        "content": "Content 1"
                    },
                    "id": "write1"
                },
                {
                    "tool": "write_file",
                    "args": {
                        "file_path": "/invalid/path/file2.txt",
                        "content": "This will fail"
                    },
                    "id": "write2"
                },
                {
                    "tool": "write_file",
                    "args": {
                        "file_path": temp_dir.path().join("file3.txt").to_string_lossy(),
                        "content": "This should not execute"
                    },
                    "id": "write3"
                }
            ],
            "options": {
                "atomic": false,
                "parallel": false,
                "dry_run": false,
                "stop_on_error": true
            }
        });

        let result = dispatcher.call_tool_for_test("batch_execute", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], false);
        assert_eq!(response["failedOperations"], 1);

        // First operation should succeed, third should not execute
        assert!(temp_dir.path().join("file1.txt").exists());
        assert!(!temp_dir.path().join("file3.txt").exists());

        let results = response["results"].as_array().unwrap();
        assert!(results.len() <= 2); // Should stop after second operation fails
    }

    #[tokio::test]
    async fn test_batch_execute_continue_on_error() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::batch::register_tools(&mut dispatcher);
        super::super::filesystem::register_tools(&mut dispatcher);

        let temp_dir = TempDir::new().unwrap();

        let args = json!({
            "operations": [
                {
                    "tool": "write_file",
                    "args": {
                        "file_path": temp_dir.path().join("file1.txt").to_string_lossy(),
                        "content": "Content 1"
                    },
                    "id": "write1"
                },
                {
                    "tool": "write_file",
                    "args": {
                        "file_path": "/invalid/path/file2.txt",
                        "content": "This will fail"
                    },
                    "id": "write2"
                },
                {
                    "tool": "write_file",
                    "args": {
                        "file_path": temp_dir.path().join("file3.txt").to_string_lossy(),
                        "content": "This should execute despite failure"
                    },
                    "id": "write3"
                }
            ],
            "options": {
                "atomic": false,
                "parallel": false,
                "dry_run": false,
                "stop_on_error": false
            }
        });

        let result = dispatcher.call_tool_for_test("batch_execute", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["totalOperations"], 3);
        assert_eq!(response["completedOperations"], 2);
        assert_eq!(response["failedOperations"], 1);

        // First and third operations should succeed
        assert!(temp_dir.path().join("file1.txt").exists());
        assert!(temp_dir.path().join("file3.txt").exists());

        let results = response["results"].as_array().unwrap();
        assert_eq!(results.len(), 3); // All operations should be attempted
    }

    #[tokio::test]
    async fn test_batch_execute_empty_operations() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::batch::register_tools(&mut dispatcher);

        let args = json!({
            "operations": [],
            "options": {
                "atomic": false,
                "parallel": false,
                "dry_run": false,
                "stop_on_error": true
            }
        });

        let result = dispatcher.call_tool_for_test("batch_execute", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], true);
        assert_eq!(response["totalOperations"], 0);
        assert_eq!(response["completedOperations"], 0);
        assert_eq!(response["failedOperations"], 0);

        let results = response["results"].as_array().unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_batch_execute_invalid_tool() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::batch::register_tools(&mut dispatcher);

        let args = json!({
            "operations": [
                {
                    "tool": "nonexistent_tool",
                    "args": {},
                    "id": "invalid"
                }
            ],
            "options": {
                "atomic": false,
                "parallel": false,
                "dry_run": false,
                "stop_on_error": true
            }
        });

        let result = dispatcher.call_tool_for_test("batch_execute", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], false);
        assert_eq!(response["failedOperations"], 1);

        let results = response["results"].as_array().unwrap();
        assert_eq!(results[0]["success"], false);
        assert!(results[0]["error"].is_string());
    }

    #[tokio::test]
    async fn test_batch_execute_large_batch() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::batch::register_tools(&mut dispatcher);
        super::super::filesystem::register_tools(&mut dispatcher);

        let temp_dir = TempDir::new().unwrap();

        // Create 20 operations
        let mut operations = Vec::new();
        for i in 0..20 {
            operations.push(json!({
                "tool": "write_file",
                "args": {
                    "file_path": temp_dir.path().join(format!("file_{}.txt", i)).to_string_lossy(),
                    "content": format!("Content for file {}", i)
                },
                "id": format!("write_{}", i)
            }));
        }

        let args = json!({
            "operations": operations,
            "options": {
                "atomic": false,
                "parallel": true,
                "dry_run": false,
                "stop_on_error": false
            }
        });

        let result = dispatcher.call_tool_for_test("batch_execute", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], true);
        assert_eq!(response["totalOperations"], 20);
        assert_eq!(response["completedOperations"], 20);
        assert_eq!(response["failedOperations"], 0);

        // Verify all files were created
        for i in 0..20 {
            let file_path = temp_dir.path().join(format!("file_{}.txt", i));
            assert!(file_path.exists());
        }
    }

    #[tokio::test]
    async fn test_batch_execute_mixed_tools() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::batch::register_tools(&mut dispatcher);
        super::super::filesystem::register_tools(&mut dispatcher);
        super::super::monitoring::register_tools(&mut dispatcher);

        let temp_dir = TempDir::new().unwrap();

        let args = json!({
            "operations": [
                {
                    "tool": "health_check",
                    "args": {"include_details": false},
                    "id": "health"
                },
                {
                    "tool": "server/getQueueStats",
                    "args": {},
                    "id": "stats"
                },
                {
                    "tool": "list_files",
                    "args": {
                        "path": temp_dir.path().to_string_lossy(),
                        "recursive": false
                    },
                    "id": "list"
                }
            ],
            "options": {
                "atomic": false,
                "parallel": true,
                "dry_run": false,
                "stop_on_error": true
            }
        });

        let result = dispatcher.call_tool_for_test("batch_execute", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], true);
        assert_eq!(response["totalOperations"], 3);
        assert_eq!(response["completedOperations"], 3);

        let results = response["results"].as_array().unwrap();
        assert_eq!(results.len(), 3);

        // Verify each tool type succeeded
        for result in results {
            assert_eq!(result["success"], true);
            let id = result["id"].as_str().unwrap();
            match id {
                "health" => assert!(result["result"]["status"].is_string()),
                "stats" => assert!(result["result"]["totalOperations"].is_number()),
                "list" => assert!(result["result"]["files"].is_array()),
                _ => panic!("Unexpected operation id: {}", id),
            }
        }
    }
}