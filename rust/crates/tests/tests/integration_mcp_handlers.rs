//! Integration tests for refactored MCP handlers
//! Tests the actual handler implementations with minimal test LSP service

use cb_server::handlers::{McpDispatcher, AppState};
use cb_server::interfaces::{LspService, FileService, AstService};
use cb_tests::harness::test_lsp_service::TestLspService;
use cb_tests::mocks::{mock_ast_service, mock_lsp_service, MockAstService, MockLspService};
use cb_core::model::mcp::{McpMessage, McpRequest};
use serde_json::{json, Value};
use std::sync::Arc;
use tempfile::TempDir;

/// Integration test for navigation MCP handlers using the new util helper
#[tokio::test]
async fn test_navigation_find_definition_integration() {
    let test_lsp = Arc::new(TestLspService::new());
    test_lsp.setup_navigation_responses();

    let app_state = create_test_app_state(test_lsp.clone()).await;
    let mut dispatcher = McpDispatcher::new();

    // Register navigation tools (this is what we're testing)
    cb_server::handlers::mcp_tools::navigation::register_tools(&mut dispatcher);

    // Test find_definition tool
    let args = json!({
        "file_path": "/test/example.ts",
        "symbol_name": "testFunction",
        "symbol_kind": "function"
    });

    let result = dispatcher.call_tool(&app_state, "find_definition", args).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.is_array());

    // Verify the LSP request was made correctly
    let requests = test_lsp.get_requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "find_definition");

    let params = requests[0].params.as_ref().unwrap();
    assert_eq!(params["file_path"], "/test/example.ts");
    assert_eq!(params["symbol_name"], "testFunction");
}

/// Integration test for navigation find_references
#[tokio::test]
async fn test_navigation_find_references_integration() {
    let test_lsp = Arc::new(TestLspService::new());
    test_lsp.setup_navigation_responses();

    let app_state = create_test_app_state(test_lsp.clone()).await;
    let mut dispatcher = McpDispatcher::new();

    cb_server::handlers::mcp_tools::navigation::register_tools(&mut dispatcher);

    let args = json!({
        "file_path": "/test/example.ts",
        "symbol_name": "testVariable",
        "include_declaration": true
    });

    let result = dispatcher.call_tool(&app_state, "find_references", args).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.is_array());

    // Verify request parameters
    let requests = test_lsp.get_requests();
    let last_request = requests.last().unwrap();
    assert_eq!(last_request.method, "find_references");

    let params = last_request.params.as_ref().unwrap();
    assert_eq!(params["include_declaration"], true);
}

/// Integration test for workspace symbol search
#[tokio::test]
async fn test_navigation_workspace_symbols_integration() {
    let test_lsp = Arc::new(TestLspService::new());
    test_lsp.setup_navigation_responses();

    let app_state = create_test_app_state(test_lsp.clone()).await;
    let mut dispatcher = McpDispatcher::new();

    cb_server::handlers::mcp_tools::navigation::register_tools(&mut dispatcher);

    let args = json!({
        "query": "TestFunction",
        "workspace_path": "/test"
    });

    let result = dispatcher.call_tool(&app_state, "search_workspace_symbols", args).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.is_array());

    // Verify the search query was passed correctly
    let requests = test_lsp.get_requests();
    let last_request = requests.last().unwrap();
    assert_eq!(last_request.method, "search_workspace_symbols");

    let params = last_request.params.as_ref().unwrap();
    assert_eq!(params["query"], "TestFunction");
}

/// Integration test for editing rename_symbol
#[tokio::test]
async fn test_editing_rename_symbol_integration() {
    let test_lsp = Arc::new(TestLspService::new());
    test_lsp.setup_editing_responses();

    let app_state = create_test_app_state(test_lsp.clone()).await;
    let mut dispatcher = McpDispatcher::new();

    cb_server::handlers::mcp_tools::editing::register_tools(&mut dispatcher);

    let args = json!({
        "file_path": "/test/example.ts",
        "line": 10,
        "character": 15,
        "new_name": "renamedFunction"
    });

    let result = dispatcher.call_tool(&app_state, "rename_symbol", args).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.is_object());

    // Verify LSP request format for textDocument/rename
    let requests = test_lsp.get_requests();
    let last_request = requests.last().unwrap();
    assert_eq!(last_request.method, "textDocument/rename");

    let params = last_request.params.as_ref().unwrap();
    assert_eq!(params["textDocument"]["uri"], "file:///test/example.ts");
    assert_eq!(params["position"]["line"], 10);
    assert_eq!(params["position"]["character"], 15);
    assert_eq!(params["newName"], "renamedFunction");
}

/// Integration test for editing rename_symbol_strict
#[tokio::test]
async fn test_editing_rename_symbol_strict_integration() {
    let test_lsp = Arc::new(TestLspService::new());
    test_lsp.setup_editing_responses();

    let app_state = create_test_app_state(test_lsp.clone()).await;
    let mut dispatcher = McpDispatcher::new();

    cb_server::handlers::mcp_tools::editing::register_tools(&mut dispatcher);

    let args = json!({
        "file_path": "/test/example.ts",
        "line": 5,
        "character": 8,
        "new_name": "strictRename",
        "dry_run": false
    });

    let result = dispatcher.call_tool(&app_state, "rename_symbol_strict", args).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.is_object());

    // Verify strict rename metadata is added
    assert_eq!(response["renameType"], "strict");
    assert_eq!(response["position"]["line"], 5);
    assert_eq!(response["position"]["character"], 8);

    // Verify LSP request
    let requests = test_lsp.get_requests();
    let last_request = requests.last().unwrap();
    assert_eq!(last_request.method, "textDocument/rename");
}

/// Integration test for dry run rename
#[tokio::test]
async fn test_editing_rename_symbol_strict_dry_run() {
    let test_lsp = Arc::new(TestLspService::new());

    let app_state = create_test_app_state(test_lsp.clone()).await;
    let mut dispatcher = McpDispatcher::new();

    cb_server::handlers::mcp_tools::editing::register_tools(&mut dispatcher);

    let args = json!({
        "file_path": "/test/example.ts",
        "line": 5,
        "character": 8,
        "new_name": "dryRunRename",
        "dry_run": true
    });

    let result = dispatcher.call_tool(&app_state, "rename_symbol_strict", args).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.is_object());

    // Verify dry run response
    assert_eq!(response["dryRun"], true);
    assert_eq!(response["newName"], "dryRunRename");
    assert!(response["preview"].is_array());

    // Verify no LSP request was made for dry run
    let requests = test_lsp.get_requests();
    assert_eq!(requests.len(), 0);
}

/// Integration test for intelligence get_hover
#[tokio::test]
async fn test_intelligence_hover_integration() {
    let test_lsp = Arc::new(TestLspService::new());
    test_lsp.setup_intelligence_responses();

    let app_state = create_test_app_state(test_lsp.clone()).await;
    let mut dispatcher = McpDispatcher::new();

    cb_server::handlers::mcp_tools::intelligence::register_tools(&mut dispatcher);

    let args = json!({
        "file_path": "/test/example.ts",
        "line": 10,
        "character": 8
    });

    let result = dispatcher.call_tool(&app_state, "get_hover", args).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.is_object());

    // Verify LSP request format for textDocument/hover
    let requests = test_lsp.get_requests();
    let last_request = requests.last().unwrap();
    assert_eq!(last_request.method, "textDocument/hover");

    let params = last_request.params.as_ref().unwrap();
    assert_eq!(params["textDocument"]["uri"], "file:///test/example.ts");
    assert_eq!(params["position"]["line"], 10);
    assert_eq!(params["position"]["character"], 8);
}

/// Test error handling in navigation handlers
#[tokio::test]
async fn test_navigation_error_handling() {
    let test_lsp = Arc::new(TestLspService::new());
    test_lsp.set_error("find_definition", "Language server not available");

    let app_state = create_test_app_state(test_lsp.clone()).await;
    let mut dispatcher = McpDispatcher::new();

    cb_server::handlers::mcp_tools::navigation::register_tools(&mut dispatcher);

    let args = json!({
        "file_path": "/test/example.ts",
        "symbol_name": "testFunction"
    });

    let result = dispatcher.call_tool(&app_state, "find_definition", args).await;
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert!(error.to_string().contains("Language server not available"));
}

/// Test that unique request IDs are generated for concurrent requests
#[tokio::test]
async fn test_concurrent_request_ids() {
    let test_lsp = Arc::new(TestLspService::new());
    test_lsp.setup_navigation_responses();

    let app_state = create_test_app_state(test_lsp.clone()).await;
    let mut dispatcher = McpDispatcher::new();

    cb_server::handlers::mcp_tools::navigation::register_tools(&mut dispatcher);

    // Make multiple concurrent requests
    let mut handles = vec![];
    for i in 0..10 {
        let app_state_clone = app_state.clone();
        let dispatcher_clone = dispatcher.clone();

        let handle = tokio::spawn(async move {
            let args = json!({
                "file_path": format!("/test/example{}.ts", i),
                "symbol_name": format!("symbol{}", i)
            });

            dispatcher_clone.call_tool(&app_state_clone, "find_definition", args).await
        });

        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    // Verify all requests had unique IDs
    let requests = test_lsp.get_requests();
    assert_eq!(requests.len(), 10);

    let mut ids = std::collections::HashSet::new();
    for request in requests {
        if let Some(id) = request.id {
            assert!(ids.insert(id), "Duplicate request ID found");
        }
    }
}

/// Helper function to create test app state
async fn create_test_app_state(lsp_service: Arc<TestLspService>) -> AppState {
    // Create mock file and AST services
    let mut mock_file_service = cb_tests::mocks::mock_file_service();
    let mut mock_ast_service = mock_ast_service();

    // Configure mock file service for basic operations
    mock_file_service
        .expect_read_file()
        .returning(|_| Ok("test file content".to_string()));

    AppState {
        lsp: lsp_service,
        file_service: Arc::new(mock_file_service),
        ast: Arc::new(mock_ast_service),
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_request_ids_are_unique_in_batch(
            num_requests in 1..50usize
        ) {
            tokio_test::block_on(async {
                let test_lsp = Arc::new(TestLspService::new());
                test_lsp.setup_navigation_responses();

                let app_state = create_test_app_state(test_lsp.clone()).await;
                let mut dispatcher = McpDispatcher::new();
                cb_server::handlers::mcp_tools::navigation::register_tools(&mut dispatcher);

                // Make multiple requests
                for i in 0..num_requests {
                    let args = json!({
                        "file_path": format!("/test/file{}.ts", i),
                        "symbol_name": format!("symbol{}", i)
                    });

                    let _ = dispatcher.call_tool(&app_state, "find_definition", args).await;
                }

                // Verify all IDs are unique
                let requests = test_lsp.get_requests();
                let mut ids = std::collections::HashSet::new();
                for request in requests {
                    if let Some(id) = request.id {
                        prop_assert!(ids.insert(id.clone()), "Found duplicate ID: {:?}", id);
                    }
                }
            });
        }
    }
}