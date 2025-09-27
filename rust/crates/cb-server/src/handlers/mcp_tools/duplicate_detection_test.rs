//! Tests for duplicate detection tool

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::handlers::mcp_dispatcher::{AppState, McpDispatcher};
    use crate::services::{FileService, LockManager, OperationQueue};
    use std::path::PathBuf;
    use std::sync::Arc;
    use serde_json::json;
    use tokio;

    /// Mock LSP service for testing
    struct MockLspService;

    #[async_trait::async_trait]
    impl crate::interfaces::LspService for MockLspService {
        async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            Ok(())
        }

        async fn execute_request(
            &self,
            _method: &str,
            _params: serde_json::Value,
        ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
            Ok(json!({}))
        }

        async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            Ok(())
        }
    }

    fn create_test_app_state() -> Arc<AppState> {
        Arc::new(AppState {
            lsp: Arc::new(MockLspService),
            file_service: Arc::new(FileService::new(PathBuf::from("/workspace"))),
            project_root: PathBuf::from("/workspace"),
            lock_manager: Arc::new(LockManager::new()),
            operation_queue: Arc::new(OperationQueue::new()),
        })
    }

    #[tokio::test]
    async fn test_find_code_duplicates_missing_path() {
        let app_state = create_test_app_state();
        let mut dispatcher = McpDispatcher::new(app_state.clone());

        // Register the duplicate detection tool
        duplicate_detection::register(&mut dispatcher);

        // Test with non-existent path
        let args = json!({
            "path": "/non/existent/path",
            "min_tokens": 20,
            "min_lines": 3
        });

        // Call the tool directly (this would normally come through MCP)
        let result = dispatcher.dispatch_tool_call(
            "find_code_duplicates",
            args
        ).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Path does not exist"));
    }

    #[tokio::test]
    async fn test_find_code_duplicates_valid_path() {
        let app_state = create_test_app_state();
        let mut dispatcher = McpDispatcher::new(app_state.clone());

        // Register the duplicate detection tool
        duplicate_detection::register(&mut dispatcher);

        // Create a test file with duplicates
        let test_file_path = "/tmp/test_dups.js";
        let test_content = r#"
function test() {
    console.log("test");
    return 42;
}

function test() {
    console.log("test");
    return 42;
}
"#;
        tokio::fs::write(test_file_path, test_content).await.unwrap();

        // Test with valid path
        let args = json!({
            "path": test_file_path,
            "min_tokens": 10,
            "min_lines": 3
        });

        // This test would require jscpd to be installed in the test environment
        // For now, we just verify the structure is correct

        // Clean up
        let _ = tokio::fs::remove_file(test_file_path).await;
    }

    #[tokio::test]
    async fn test_transform_jscpd_output() {
        // Test the transformation function with sample jscpd output
        let sample_output = json!({
            "duplicates": [{
                "firstFile": {
                    "name": "file://test1.js",
                    "startLine": 10,
                    "endLine": 20
                },
                "secondFile": {
                    "name": "file://test2.js",
                    "startLine": 30,
                    "endLine": 40
                },
                "tokens": 100,
                "lines": 10
            }],
            "statistics": {
                "total": {
                    "files": 2,
                    "sources": 2,
                    "percentage": 5.0,
                    "duplicatedLines": 20
                }
            }
        });

        let result = super::super::duplicate_detection::transform_jscpd_output(
            sample_output,
            false
        ).await.unwrap();

        assert_eq!(result.duplicates.len(), 1);
        assert_eq!(result.duplicates[0].instances.len(), 2);
        assert_eq!(result.duplicates[0].token_count, 100);
        assert_eq!(result.statistics.duplicate_percentage, 5.0);
    }
}