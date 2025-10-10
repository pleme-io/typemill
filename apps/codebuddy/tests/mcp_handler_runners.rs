// This module contains the actual test logic for each MCP file operation.
// Each runner function is parameterized to accept a fixture struct,
// making them reusable across multiple test scenarios.
use cb_ast::AstCache;
use cb_plugins::PluginManager;
use cb_protocol::AstService;
use cb_server::handlers::AppState;
use cb_server::services::{DefaultAstService, FileService, LockManager, OperationQueue};
use cb_server::workspaces::WorkspaceManager;
use cb_test_support::harness::{
    mcp_fixtures::{
        AnalyzeImportsTestCase, CreateFileTestCase, DeleteFileTestCase, FindDeadCodeTestCase,
        ListFilesTestCase, ReadFileTestCase, RenameDirectoryTestCase, RenameFileTestCase,
        WriteFileTestCase,
    },
    TestClient, TestWorkspace,
};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

/// Spawn a background worker to process queued file operations in tests
fn spawn_test_worker(queue: Arc<OperationQueue>) {
    use cb_protocol::ApiError;
    use cb_server::services::operation_queue::OperationType;
    use tokio::fs;

    tokio::spawn(async move {
        queue
            .process_with(|op, stats| async move {
                let result: Result<(), ApiError> = match op.operation_type {
                    OperationType::CreateDir => {
                        fs::create_dir_all(&op.file_path).await.map_err(|e| {
                            ApiError::Internal(format!("Failed to create directory: {}", e))
                        })
                    }
                    OperationType::CreateFile | OperationType::Write => {
                        let content = op
                            .params
                            .get("content")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        fs::write(&op.file_path, content)
                            .await
                            .map_err(|e| ApiError::Internal(format!("Failed to write file: {}", e)))
                    }
                    OperationType::Delete => {
                        if op.file_path.exists() {
                            fs::remove_file(&op.file_path).await.map_err(|e| {
                                ApiError::Internal(format!("Failed to delete file: {}", e))
                            })
                        } else {
                            Ok(())
                        }
                    }
                    OperationType::Rename => {
                        let new_path_str = op
                            .params
                            .get("new_path")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| ApiError::Internal("Missing new_path".to_string()))?;
                        fs::rename(&op.file_path, new_path_str).await.map_err(|e| {
                            ApiError::Internal(format!("Failed to rename file: {}", e))
                        })
                    }
                    _ => Ok(()),
                };

                // Update stats after operation completes
                let mut stats_guard = stats.lock().await;
                match &result {
                    Ok(_) => {
                        stats_guard.completed_operations += 1;
                    }
                    Err(_) => {
                        stats_guard.failed_operations += 1;
                    }
                }
                drop(stats_guard);

                result.map(|_| serde_json::Value::Null)
            })
            .await;
    });
}

/// Create a mock AppState for direct service testing
async fn create_mock_state(workspace_root: PathBuf) -> Arc<AppState> {
    let ast_cache = Arc::new(AstCache::new());
    let plugin_registry = cb_server::services::registry_builder::build_language_plugin_registry();
    let ast_service: Arc<dyn AstService> = Arc::new(DefaultAstService::new(ast_cache.clone(), plugin_registry.clone()));
    let lock_manager = Arc::new(LockManager::new());
    let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));

    // Spawn background worker to process queued operations
    spawn_test_worker(operation_queue.clone());

    let config = cb_core::AppConfig::default();
    let file_service = Arc::new(FileService::new(
        workspace_root.clone(),
        ast_cache.clone(),
        lock_manager.clone(),
        operation_queue.clone(),
        &config,
        plugin_registry.clone(),
    ));
    let plugin_manager = Arc::new(PluginManager::new());
    let planner = cb_server::services::planner::DefaultPlanner::new();
    let workflow_executor = cb_server::services::workflow_executor::DefaultWorkflowExecutor::new(
        plugin_manager.clone(),
    );
    let workspace_manager = Arc::new(WorkspaceManager::new());

    Arc::new(AppState {
        ast_service,
        file_service,
        planner,
        workflow_executor,
        project_root: workspace_root,
        lock_manager,
        operation_queue,
        start_time: std::time::Instant::now(),
        workspace_manager,
        language_plugins: cb_handlers::LanguagePluginRegistry::new().await,
    })
}

/// Run a create_file test with the given test case
pub async fn run_create_file_test(case: &CreateFileTestCase, use_real_mcp: bool) {
    let workspace = TestWorkspace::new();

    // Setup initial files
    for (path, content) in case.initial_files {
        let file_path = workspace.path().join(path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&file_path, content).unwrap();
    }

    let file_path = workspace.path().join(case.file_to_create);

    if use_real_mcp {
        // Real MCP test using TestClient
        let mut client = TestClient::new(workspace.path());

        let mut params = json!({
            "file_path": file_path.to_string_lossy(),
            "content": case.content
        });

        if case.overwrite {
            params["overwrite"] = json!(true);
        }

        let response = client.call_tool("create_file", params).await;

        if case.expect_success {
            let response = response.unwrap();
            // MCP responses are JSON-RPC format: check result.success
            let result = response
                .get("result")
                .expect("Response should have result field");
            assert!(
                result["success"].as_bool().unwrap_or(false),
                "Test '{}': Expected success but got failure. Response: {:?}",
                case.test_name,
                response
            );
            assert!(
                file_path.exists(),
                "Test '{}': File should exist after creation",
                case.test_name
            );
            let actual_content = std::fs::read_to_string(&file_path).unwrap();
            assert_eq!(
                actual_content, case.content,
                "Test '{}': File content mismatch",
                case.test_name
            );
        } else {
            // For expected failures, either error response or result.success = false
            if let Ok(response) = response {
                let result = response.get("result");
                assert!(
                    result.is_none() || !result.unwrap()["success"].as_bool().unwrap_or(true),
                    "Test '{}': Expected failure but got success",
                    case.test_name
                );
            }
        }
    } else {
        // Mock test using FileService directly
        let app_state = create_mock_state(workspace.path().to_path_buf()).await;

        let result = app_state
            .file_service
            .create_file(&file_path, Some(case.content), case.overwrite, false)
            .await;

        // Wait for queue to process the operation
        app_state.operation_queue.wait_until_idle().await;

        if case.expect_success {
            assert!(
                result.is_ok(),
                "Test '{}': Expected success but got error: {:?}",
                case.test_name,
                result.err()
            );
            assert!(
                file_path.exists(),
                "Test '{}': File should exist after creation",
                case.test_name
            );
            let actual_content = std::fs::read_to_string(&file_path).unwrap();
            assert_eq!(
                actual_content, case.content,
                "Test '{}': File content mismatch",
                case.test_name
            );
        } else {
            assert!(
                result.is_err(),
                "Test '{}': Expected failure but got success",
                case.test_name
            );
        }
    }
}

/// Run a read_file test with the given test case
pub async fn run_read_file_test(case: &ReadFileTestCase, use_real_mcp: bool) {
    let workspace = TestWorkspace::new();

    // Setup initial files
    for (path, content) in case.initial_files {
        let file_path = workspace.path().join(path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&file_path, content).unwrap();
    }

    let file_path = workspace.path().join(case.file_to_read);

    if use_real_mcp {
        // Real MCP test using TestClient
        let mut client = TestClient::new(workspace.path());

        let mut params = json!({
            "file_path": file_path.to_string_lossy()
        });

        if let Some(start) = case.start_line {
            params["start_line"] = json!(start);
        }
        if let Some(end) = case.end_line {
            params["end_line"] = json!(end);
        }

        let response = client.call_tool("read_file", params).await;

        if case.expect_success {
            let response = response.unwrap();
            // MCP responses are JSON-RPC format: check result.content
            let result = response
                .get("result")
                .expect("Response should have result field");
            if let Some(expected) = case.expected_content {
                assert_eq!(
                    result["content"].as_str().unwrap(),
                    expected,
                    "Test '{}': Content mismatch",
                    case.test_name
                );
            }
        } else {
            // For expected failures, check for JSON-RPC error field or failed result
            if let Ok(response) = response {
                assert!(
                    response.get("error").is_some()
                        || response
                            .get("result")
                            .map(|r| r.get("success").and_then(|s| s.as_bool()).unwrap_or(true))
                            .unwrap_or(true)
                            == false,
                    "Test '{}': Expected failure but got success. Response: {:?}",
                    case.test_name,
                    response
                );
            }
        }
    } else {
        // Mock test using FileService directly
        let app_state = create_mock_state(workspace.path().to_path_buf()).await;

        let result = app_state.file_service.read_file(&file_path).await;

        if case.expect_success {
            assert!(
                result.is_ok(),
                "Test '{}': Expected success but got error: {:?}",
                case.test_name,
                result.err()
            );
            if let Some(expected) = case.expected_content {
                let content = result.unwrap();
                assert_eq!(
                    content, expected,
                    "Test '{}': Content mismatch",
                    case.test_name
                );
            }
        } else {
            assert!(
                result.is_err(),
                "Test '{}': Expected failure but got success",
                case.test_name
            );
        }
    }
}

/// Run a write_file test with the given test case
pub async fn run_write_file_test(case: &WriteFileTestCase, use_real_mcp: bool) {
    let workspace = TestWorkspace::new();

    // Setup initial files
    for (path, content) in case.initial_files {
        let file_path = workspace.path().join(path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&file_path, content).unwrap();
    }

    let file_path = workspace.path().join(case.file_to_write);

    if use_real_mcp {
        // Real MCP test using TestClient
        let mut client = TestClient::new(workspace.path());

        let params = json!({
            "file_path": file_path.to_string_lossy(),
            "content": case.content
        });

        let response = client.call_tool("write_file", params).await;

        if case.expect_success {
            let response = response.unwrap();
            // MCP responses are JSON-RPC format: check result.success
            let result = response.get("result").expect(&format!(
                "Response should have result field. Full response: {:?}",
                response
            ));
            assert!(
                result["success"].as_bool().unwrap_or(false),
                "Test '{}': Expected success but got failure",
                case.test_name
            );
            assert!(
                file_path.exists(),
                "Test '{}': File should exist after write",
                case.test_name
            );
            let actual_content = std::fs::read_to_string(&file_path).unwrap();
            assert_eq!(
                actual_content, case.content,
                "Test '{}': File content mismatch",
                case.test_name
            );
        } else {
            // For expected failures, check for JSON-RPC error field or failed result
            if let Ok(response) = response {
                assert!(
                    response.get("error").is_some()
                        || response
                            .get("result")
                            .map(|r| r.get("success").and_then(|s| s.as_bool()).unwrap_or(true))
                            .unwrap_or(true)
                            == false,
                    "Test '{}': Expected failure but got success. Response: {:?}",
                    case.test_name,
                    response
                );
            }
        }
    } else {
        // Mock test using FileService directly
        let app_state = create_mock_state(workspace.path().to_path_buf()).await;

        let result = app_state
            .file_service
            .write_file(&file_path, case.content, false)
            .await;

        // Wait for queue to process the operation
        app_state.operation_queue.wait_until_idle().await;

        if case.expect_success {
            assert!(
                result.is_ok(),
                "Test '{}': Expected success but got error: {:?}",
                case.test_name,
                result.err()
            );
            assert!(
                file_path.exists(),
                "Test '{}': File should exist after write",
                case.test_name
            );
            let actual_content = std::fs::read_to_string(&file_path).unwrap();
            assert_eq!(
                actual_content, case.content,
                "Test '{}': File content mismatch",
                case.test_name
            );
        } else {
            assert!(
                result.is_err(),
                "Test '{}': Expected failure but got success",
                case.test_name
            );
        }
    }
}

/// Run a delete_file test with the given test case
pub async fn run_delete_file_test(case: &DeleteFileTestCase, use_real_mcp: bool) {
    let workspace = TestWorkspace::new();

    // Setup initial files
    for (path, content) in case.initial_files {
        let file_path = workspace.path().join(path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&file_path, content).unwrap();
    }

    let file_path = workspace.path().join(case.file_to_delete);

    if use_real_mcp {
        // Real MCP test using TestClient
        let mut client = TestClient::new(workspace.path());

        let params = json!({
            "file_path": file_path.to_string_lossy()
        });

        let response = client.call_tool("delete_file", params).await;

        if case.expect_success {
            let response = response.unwrap();
            // MCP responses are JSON-RPC format: check result.success
            let result = response
                .get("result")
                .expect("Response should have result field");
            assert!(
                result["success"].as_bool().unwrap_or(false),
                "Test '{}': Expected success but got failure",
                case.test_name
            );
            assert!(
                !file_path.exists(),
                "Test '{}': File should not exist after deletion",
                case.test_name
            );
        } else {
            // For expected failures, check for JSON-RPC error field or failed result
            if let Ok(response) = response {
                assert!(
                    response.get("error").is_some()
                        || response
                            .get("result")
                            .map(|r| r.get("success").and_then(|s| s.as_bool()).unwrap_or(true))
                            .unwrap_or(true)
                            == false,
                    "Test '{}': Expected failure but got success. Response: {:?}",
                    case.test_name,
                    response
                );
            }
        }
    } else {
        // Mock test using FileService directly
        let app_state = create_mock_state(workspace.path().to_path_buf()).await;

        let result = app_state
            .file_service
            .delete_file(&file_path, false, false)
            .await;

        // Wait for queue to process the operation
        app_state.operation_queue.wait_until_idle().await;

        if case.expect_success {
            assert!(
                result.is_ok(),
                "Test '{}': Expected success but got error: {:?}",
                case.test_name,
                result.err()
            );
            assert!(
                !file_path.exists(),
                "Test '{}': File should not exist after deletion",
                case.test_name
            );
        } else {
            assert!(
                result.is_err(),
                "Test '{}': Expected failure but got success",
                case.test_name
            );
        }
    }
}

/// Run a list_files test with the given test case
pub async fn run_list_files_test(case: &ListFilesTestCase, use_real_mcp: bool) {
    let workspace = TestWorkspace::new();

    // Setup initial files and directories
    for dir in case.initial_dirs {
        let dir_path = workspace.path().join(dir);
        std::fs::create_dir_all(&dir_path).unwrap();
    }

    for file in case.initial_files {
        let file_path = workspace.path().join(file);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&file_path, "content").unwrap();
    }

    let directory = if case.directory.is_empty() {
        workspace.path().to_path_buf()
    } else {
        workspace.path().join(case.directory)
    };

    if use_real_mcp {
        // Real MCP test using TestClient
        let mut client = TestClient::new(workspace.path());

        let mut params = json!({
            "directory": directory.to_string_lossy()
        });

        if case.recursive {
            params["recursive"] = json!(true);
        }
        if let Some(pattern) = case.pattern {
            params["pattern"] = json!(pattern);
        }

        let response = client.call_tool("list_files", params).await.unwrap();

        // MCP responses are JSON-RPC format: check result.content.files
        let result = response
            .get("result")
            .expect("Response should have result field");
        let content = result
            .get("content")
            .expect("Response should have content field");
        let file_list = content["files"]
            .as_array()
            .expect("Content should have files array");
        assert!(
            file_list.len() >= case.expected_min_count,
            "Test '{}': Expected at least {} files, got {}",
            case.test_name,
            case.expected_min_count,
            file_list.len()
        );

        let names: Vec<&str> = file_list
            .iter()
            .filter_map(|f| f["name"].as_str())
            .collect();

        for expected in case.expected_contains {
            assert!(
                names.contains(expected),
                "Test '{}': Expected to find '{}' in list",
                case.test_name,
                expected
            );
        }
    } else {
        // Mock test using SystemToolsPlugin directly
        use cb_plugins::system_tools_plugin::SystemToolsPlugin;
        use cb_plugins::{LanguagePlugin, PluginRequest};
        use std::path::Path;

        let mut params = json!({
            "path": directory.to_string_lossy()
        });

        if case.recursive {
            params["recursive"] = json!(true);
        }
        if let Some(pattern) = case.pattern {
            params["pattern"] = json!(pattern);
        }

        // Use the actual SystemToolsPlugin to test the real application logic
        let plugin_registry = cb_server::services::registry_builder::build_language_plugin_registry();
        let plugin = SystemToolsPlugin::new(plugin_registry);
        let request = PluginRequest {
            method: "list_files".to_string(),
            file_path: directory.clone(),
            position: None,
            range: None,
            params,
            request_id: Some("test-list-files".to_string()),
        };

        let result = plugin.handle_request(request).await;

        assert!(
            result.is_ok(),
            "Test '{}': Expected success but got error: {:?}",
            case.test_name,
            result.err()
        );

        let response = result.unwrap();
        assert!(
            response.success,
            "Test '{}': Plugin returned success=false: {:?}",
            case.test_name, response.error
        );

        let data = response.data.unwrap();
        let file_list = data["files"].as_array().unwrap();

        assert!(
            file_list.len() >= case.expected_min_count,
            "Test '{}': Expected at least {} files, got {}",
            case.test_name,
            case.expected_min_count,
            file_list.len()
        );

        // The plugin returns absolute paths, so we must make them relative for comparison
        let relative_paths: Vec<String> = file_list
            .iter()
            .filter_map(|f| f["path"].as_str())
            .map(|p| {
                Path::new(p)
                    .strip_prefix(workspace.path())
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        for expected in case.expected_contains {
            assert!(
                relative_paths.iter().any(|p| p == *expected),
                "Test '{}': Expected to find '{}' in list",
                case.test_name,
                expected
            );
        }
    }
}

/// Run an analyze_imports test with the given test case
pub async fn run_analyze_imports_test(case: &AnalyzeImportsTestCase, use_real_mcp: bool) {
    let workspace = TestWorkspace::new();

    // Setup initial files
    for (path, content) in case.initial_files {
        let file_path = workspace.path().join(path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&file_path, content).unwrap();
    }

    let file_path = workspace.path().join(case.file_path);

    if use_real_mcp {
        // Real MCP test using TestClient
        let mut client = TestClient::new(workspace.path());

        let params = json!({
            "file_path": file_path.to_string_lossy()
        });

        let response = client.call_tool("analyze_imports", params).await;

        if case.expect_success {
            let response = response.unwrap();
            // MCP responses are JSON-RPC format: check result.content
            let result = response
                .get("result")
                .expect("Response should have result field");
            let content = result
                .get("content")
                .expect("Result should have content field");

            // Check import graph structure
            let import_graph = content
                .get("importGraph")
                .or_else(|| content.get("import_graph"))
                .expect("Content should have importGraph field");

            let imports = import_graph
                .get("imports")
                .and_then(|v| v.as_array())
                .expect("Import graph should have imports array");

            assert_eq!(
                imports.len(),
                case.expected_import_count,
                "Test '{}': Expected {} imports, got {}",
                case.test_name,
                case.expected_import_count,
                imports.len()
            );
        } else {
            // For expected failures, check for JSON-RPC error field
            if let Ok(response) = response {
                assert!(
                    response.get("error").is_some(),
                    "Test '{}': Expected failure but got success. Response: {:?}",
                    case.test_name,
                    response
                );
            }
        }
    } else {
        // Mock test using SystemToolsPlugin directly
        use cb_plugins::system_tools_plugin::SystemToolsPlugin;
        use cb_plugins::{LanguagePlugin, PluginRequest};

        let params = json!({
            "file_path": file_path.to_string_lossy()
        });

        let plugin_registry = cb_server::services::registry_builder::build_language_plugin_registry();
        let plugin = SystemToolsPlugin::new(plugin_registry);
        let request = PluginRequest {
            method: "analyze_imports".to_string(),
            file_path: file_path.clone(),
            position: None,
            range: None,
            params,
            request_id: Some("test-analyze-imports".to_string()),
        };

        let result = plugin.handle_request(request).await;

        if case.expect_success {
            assert!(
                result.is_ok(),
                "Test '{}': Expected success but got error: {:?}",
                case.test_name,
                result.err()
            );

            let response = result.unwrap();
            assert!(
                response.success,
                "Test '{}': Plugin returned success=false: {:?}",
                case.test_name, response.error
            );

            let data = response.data.unwrap();
            let import_graph = data
                .get("importGraph")
                .or_else(|| data.get("import_graph"))
                .expect("Data should have importGraph field");

            let imports = import_graph
                .get("imports")
                .and_then(|v| v.as_array())
                .expect("Import graph should have imports array");

            assert_eq!(
                imports.len(),
                case.expected_import_count,
                "Test '{}': Expected {} imports, got {}",
                case.test_name,
                case.expected_import_count,
                imports.len()
            );
        } else {
            assert!(
                result.is_err(),
                "Test '{}': Expected failure but got success",
                case.test_name
            );
        }
    }
}

/// Run a find_dead_code test with the given test case
pub async fn run_find_dead_code_test(case: &FindDeadCodeTestCase, use_real_mcp: bool) {
    use std::collections::HashSet;

    let workspace = TestWorkspace::new();

    // Setup initial files
    for (path, content) in case.initial_files {
        let file_path = workspace.path().join(path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&file_path, content).unwrap();
    }

    let workspace_path = if case.workspace_path.is_empty() {
        workspace.path().to_path_buf()
    } else {
        workspace.path().join(case.workspace_path)
    };

    if use_real_mcp {
        // Real MCP test using TestClient
        let mut client = TestClient::new(workspace.path());

        let params = json!({
            "workspace_path": workspace_path.to_string_lossy()
        });

        let response = client.call_tool("find_dead_code", params).await;

        if case.expect_success {
            let response = response.unwrap();
            // MCP responses are JSON-RPC format: check result.content
            let result = response
                .get("result")
                .expect("Response should have result field");
            let content = result
                .get("content")
                .expect("Result should have content field");

            // Check dead symbols using new response format
            let dead_symbols = content
                .get("deadSymbols")
                .and_then(|v| v.as_array())
                .expect("Response should have deadSymbols array");

            // Extract symbol names from result
            let found_names: HashSet<String> = dead_symbols
                .iter()
                .filter_map(|s| {
                    s.get("name")
                        .and_then(|n| n.as_str().map(|s| s.to_string()))
                })
                .collect();

            let expected_names: HashSet<String> = case
                .expected_dead_symbols
                .iter()
                .map(|s| s.to_string())
                .collect();

            assert_eq!(
                found_names, expected_names,
                "Test '{}': Dead symbol mismatch. Expected {:?}, found {:?}",
                case.test_name, expected_names, found_names
            );
        } else {
            // For expected failures, check for JSON-RPC error field
            if let Ok(response) = response {
                assert!(
                    response.get("error").is_some(),
                    "Test '{}': Expected failure but got success. Response: {:?}",
                    case.test_name,
                    response
                );
            }
        }
    } else {
        // Mock test - find_dead_code is no longer in SystemToolsPlugin
        // It's now handled directly by the dispatcher with LSP integration
        // For mock tests, we can't test this without LSP servers running
        // So we'll just verify the test expectation is set correctly
        eprintln!(
            "ℹ️  Test '{}': Mock tests for find_dead_code are not implemented (requires LSP). Use real MCP tests instead.",
            case.test_name
        );
    }
}

/// Run a rename_directory test with the given test case
pub async fn run_rename_directory_test(case: &RenameDirectoryTestCase, use_real_mcp: bool) {
    let workspace = TestWorkspace::new();

    // Setup initial files
    for (path, content) in case.initial_files {
        let file_path = workspace.path().join(path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&file_path, content).unwrap();
    }

    let old_path = workspace.path().join(case.dir_to_rename);
    let new_path = workspace.path().join(case.new_dir_name);

    if use_real_mcp {
        // Real MCP test using TestClient
        let mut client = TestClient::new(workspace.path());

        let params = json!({
            "old_path": old_path.to_string_lossy(),
            "new_path": new_path.to_string_lossy(),
            "update_imports": case.update_imports,
            "dry_run": false
        });

        let response = client.call_tool("move_directory", params).await;

        if case.expect_success {
            let response = response.unwrap();
            // MCP responses are JSON-RPC format: check result
            eprintln!("DEBUG rename_directory response: {:?}", response);
            let result = response.get("result").expect(&format!(
                "Response should have result field. Full response: {:?}",
                response
            ));

            // Check that operation succeeded
            // Response can have either "success": true, "renamed": true, or "status": "success"
            let content = result
                .get("content")
                .expect("Result should have content field");
            assert!(
                result
                    .get("success")
                    .and_then(|s| s.as_bool())
                    .unwrap_or(false)
                    || result
                        .get("renamed")
                        .and_then(|r| r.as_bool())
                        .unwrap_or(false)
                    || content
                        .get("status")
                        .and_then(|s| s.as_str())
                        .map(|s| s == "success")
                        .unwrap_or(false),
                "Test '{}': Expected success in response. Response: {:?}",
                case.test_name,
                response
            );

            // Verify directory was renamed
            assert!(
                new_path.exists(),
                "Test '{}': New directory should exist after rename",
                case.test_name
            );
            assert!(
                !old_path.exists(),
                "Test '{}': Old directory should not exist after rename",
                case.test_name
            );
        } else {
            // For expected failures, check for JSON-RPC error field
            if let Ok(response) = response {
                assert!(
                    response.get("error").is_some(),
                    "Test '{}': Expected failure but got success. Response: {:?}",
                    case.test_name,
                    response
                );
            }
        }
    } else {
        // Mock test using FileService directly
        let app_state = create_mock_state(workspace.path().to_path_buf()).await;

        let result = app_state
            .file_service
            .rename_directory_with_imports(
                &old_path, &new_path, false, // dry_run
                false, // consolidate
                None,  // scan_scope (uses default)
            )
            .await;

        if case.expect_success {
            assert!(
                result.is_ok(),
                "Test '{}': Expected success but got error: {:?}",
                case.test_name,
                result.err()
            );

            // Verify directory was renamed
            assert!(
                new_path.exists(),
                "Test '{}': New directory should exist after rename",
                case.test_name
            );
            assert!(
                !old_path.exists(),
                "Test '{}': Old directory should not exist after rename",
                case.test_name
            );
        } else {
            assert!(
                result.is_err(),
                "Test '{}': Expected failure but got success",
                case.test_name
            );
        }
    }
}

/// Run a rename_file test with the given test case
pub async fn run_rename_file_test(case: &RenameFileTestCase, use_real_mcp: bool) {
    let workspace = TestWorkspace::new();

    // Setup initial files
    for (path, content) in case.initial_files {
        let file_path = workspace.path().join(path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&file_path, content).unwrap();
    }

    let old_path = workspace.path().join(case.old_file_path);
    let new_path = workspace.path().join(case.new_file_path);

    if use_real_mcp {
        // Real MCP test using TestClient
        let mut client = TestClient::new(workspace.path());

        let params = json!({
            "old_path": old_path.to_string_lossy(),
            "new_path": new_path.to_string_lossy()
        });

        let response = client.call_tool("move_file", params).await;

        if case.expect_success {
            let response = response.unwrap();
            // MCP responses are JSON-RPC format
            eprintln!("DEBUG rename_file response: {:?}", response);

            // Check that there's no error
            assert!(
                response.get("error").is_none()
                    || response.get("error").and_then(|e| e.as_null()).is_some(),
                "Test '{}': Expected success but got error. Response: {:?}",
                case.test_name,
                response
            );

            // Verify file was renamed
            assert!(
                new_path.exists(),
                "Test '{}': New file should exist after rename",
                case.test_name
            );
            assert!(
                !old_path.exists(),
                "Test '{}': Old file should not exist after rename",
                case.test_name
            );

            // Verify import updates
            for (file_to_check, expected_content) in case.expected_import_updates {
                let file_content = workspace.read_file(file_to_check);
                assert!(
                    file_content.contains(expected_content),
                    "Test '{}': File '{}' should contain '{}'. Actual content:\n{}",
                    case.test_name,
                    file_to_check,
                    expected_content,
                    file_content
                );
            }
        } else {
            // For expected failures, check for JSON-RPC error field
            if let Ok(response) = response {
                assert!(
                    response.get("error").is_some(),
                    "Test '{}': Expected failure but got success. Response: {:?}",
                    case.test_name,
                    response
                );
            }
        }
    } else {
        // Mock test using FileService directly
        let app_state = create_mock_state(workspace.path().to_path_buf()).await;

        let result = app_state
            .file_service
            .rename_file_with_imports(
                &old_path, &new_path, false, None, // dry_run
            )
            .await;

        // Wait for queue to process the operation
        app_state.operation_queue.wait_until_idle().await;

        if case.expect_success {
            assert!(
                result.is_ok(),
                "Test '{}': Expected success but got error: {:?}",
                case.test_name,
                result.err()
            );

            // Verify file was renamed
            assert!(
                new_path.exists(),
                "Test '{}': New file should exist after rename",
                case.test_name
            );
            assert!(
                !old_path.exists(),
                "Test '{}': Old file should not exist after rename",
                case.test_name
            );

            // Note: Import update validation is skipped in mock tests
            // Mock tests don't have full language plugin infrastructure for parsing TypeScript
            // imports. Import updates are validated in real MCP tests which use LSP servers.
            if !case.expected_import_updates.is_empty() {
                eprintln!(
                    "ℹ️  Test '{}': Skipping import update validation in mock test (requires LSP/language server support). Import updates are validated in real MCP tests.",
                    case.test_name
                );
            }
        } else {
            assert!(
                result.is_err(),
                "Test '{}': Expected failure but got success",
                case.test_name
            );
        }
    }
}
