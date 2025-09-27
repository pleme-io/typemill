//! Tests for MCP dispatcher transaction support

#[cfg(test)]
mod tests {
    use crate::handlers::{McpDispatcher, AppState};
    use crate::services::{LockManager, OperationQueue, FileService};
    use crate::systems::LspManager;
    use cb_core::config::LspConfig;
    use cb_core::model::mcp::ToolCall;
    use serde_json::json;
    use std::sync::Arc;
    use std::path::PathBuf;

    fn create_test_app_state() -> Arc<AppState> {
        let lsp_config = LspConfig::default();
        let lsp_manager = Arc::new(LspManager::new(lsp_config));
        let file_service = Arc::new(FileService::new(PathBuf::from("/tmp")));
        let project_root = PathBuf::from("/tmp");
        let lock_manager = Arc::new(LockManager::new());
        let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));

        Arc::new(AppState {
            lsp: lsp_manager,
            file_service,
            project_root,
            lock_manager,
            operation_queue,
        })
    }

    #[tokio::test]
    async fn test_refactoring_creates_transaction() {
        let app_state = create_test_app_state();
        let mut dispatcher = McpDispatcher::new(app_state.clone());

        // Register a mock rename_symbol handler that returns a WorkspaceEdit
        dispatcher.register_tool("rename_symbol".to_string(), |_app_state, args| async move {
            // Simulate a WorkspaceEdit that affects multiple files
            Ok(json!({
                "workspace_edit": {
                    "changes": {
                        "file:///src/main.rs": [
                            {
                                "range": {
                                    "start": {"line": 10, "character": 5},
                                    "end": {"line": 10, "character": 15}
                                },
                                "newText": "newSymbol"
                            }
                        ],
                        "file:///src/lib.rs": [
                            {
                                "range": {
                                    "start": {"line": 20, "character": 10},
                                    "end": {"line": 20, "character": 20}
                                },
                                "newText": "newSymbol"
                            }
                        ],
                        "file:///src/module.rs": [
                            {
                                "range": {
                                    "start": {"line": 5, "character": 0},
                                    "end": {"line": 5, "character": 10}
                                },
                                "newText": "newSymbol"
                            }
                        ]
                    }
                },
                "dry_run": false,
                "operation_type": "refactor",
                "original_args": args,
                "tool": "rename_symbol"
            }))
        });

        // Execute the rename operation
        let tool_call = ToolCall {
            name: "rename_symbol".to_string(),
            arguments: Some(json!({
                "file_path": "/src/main.rs",
                "line": 10,
                "character": 5,
                "new_name": "newSymbol"
            })),
        };

        let initial_stats = app_state.operation_queue.get_stats().await;
        assert_eq!(initial_stats.total_operations, 0);

        // Call the tool through the dispatcher
        let result = dispatcher.handle_tool_call(Some(json!(tool_call))).await;

        // Should succeed
        assert!(result.is_ok());

        // Check that operations were enqueued
        let final_stats = app_state.operation_queue.get_stats().await;
        assert_eq!(final_stats.total_operations, 3, "Expected 3 file operations for 3 affected files");

        // Verify all operations have high priority
        let pending = app_state.operation_queue.get_pending_operations().await;
        assert_eq!(pending.len(), 3);
        for (_, tool_name, _, _) in pending {
            assert!(tool_name.contains("rename_symbol_file_operation"));
        }
    }

    #[tokio::test]
    async fn test_dry_run_doesnt_create_transaction() {
        let app_state = create_test_app_state();
        let mut dispatcher = McpDispatcher::new(app_state.clone());

        // Register a mock rename_symbol handler
        dispatcher.register_tool("rename_symbol".to_string(), |_app_state, args| async move {
            Ok(json!({
                "workspace_edit": {
                    "changes": {
                        "file:///src/main.rs": [
                            {
                                "range": {
                                    "start": {"line": 10, "character": 5},
                                    "end": {"line": 10, "character": 15}
                                },
                                "newText": "newSymbol"
                            }
                        ]
                    }
                },
                "dry_run": true,  // This is a dry run
                "operation_type": "refactor",
                "original_args": args,
                "tool": "rename_symbol"
            }))
        });

        // Execute dry run
        let tool_call = ToolCall {
            name: "rename_symbol".to_string(),
            arguments: Some(json!({
                "file_path": "/src/main.rs",
                "line": 10,
                "character": 5,
                "new_name": "newSymbol",
                "dry_run": true
            })),
        };

        let initial_stats = app_state.operation_queue.get_stats().await;
        assert_eq!(initial_stats.total_operations, 0);

        // Call the tool
        let result = dispatcher.handle_tool_call(Some(json!(tool_call))).await;
        assert!(result.is_ok());

        // No operations should be enqueued for dry run
        let final_stats = app_state.operation_queue.get_stats().await;
        assert_eq!(final_stats.total_operations, 0, "Dry run should not enqueue operations");
    }
}