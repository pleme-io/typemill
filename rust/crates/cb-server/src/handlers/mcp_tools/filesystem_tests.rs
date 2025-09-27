//! Integration tests for filesystem tools

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::handlers::McpDispatcher;
    use crate::state::AppState;
    use crate::services::{FileService, SymbolService, EditingService, ImportService};
    use crate::systems::lsp::MockLspService;
    use std::sync::Arc;
    use tempfile::{NamedTempFile, TempDir};
    use std::fs;
    use std::io::Write;
    use serde_json::json;
    use std::path::Path;

    /// Create a test AppState with real FileService
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
    async fn test_delete_file_success() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::register_tools(&mut dispatcher);

        // Create a temporary file
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test content").unwrap();
        let file_path = temp_file.path().to_str().unwrap().to_string();

        // Verify file exists
        assert!(Path::new(&file_path).exists());

        let args = json!({
            "file_path": file_path.clone()
        });

        let result = dispatcher.call_tool_for_test("delete_file", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], true);
        assert!(response["file_path"].as_str().unwrap().ends_with(&file_path));

        // Verify file was deleted
        assert!(!Path::new(&file_path).exists());
    }

    #[tokio::test]
    async fn test_delete_file_force() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::register_tools(&mut dispatcher);

        // Create a temporary file with content
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"important data").unwrap();
        let file_path = temp_file.path().to_str().unwrap().to_string();

        let args = json!({
            "file_path": file_path.clone(),
            "force": true
        });

        let result = dispatcher.call_tool_for_test("delete_file", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], true);

        // Verify file was deleted even with force flag
        assert!(!Path::new(&file_path).exists());
    }

    #[tokio::test]
    async fn test_delete_file_nonexistent() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::register_tools(&mut dispatcher);

        let nonexistent_path = "/tmp/nonexistent_file_12345.txt";

        let args = json!({
            "file_path": nonexistent_path
        });

        let result = dispatcher.call_tool_for_test("delete_file", args).await;
        assert!(result.is_ok()); // Returns Ok with success=false for error reporting

        let response = result.unwrap();
        assert_eq!(response["success"], false);
        assert!(response["message"].is_string());
        assert!(response["message"].as_str().unwrap().contains("not found") ||
                response["message"].as_str().unwrap().contains("No such file"));
    }

    #[tokio::test]
    async fn test_delete_file_directory_error() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::register_tools(&mut dispatcher);

        // Create a temporary directory
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().to_str().unwrap().to_string();

        let args = json!({
            "file_path": dir_path.clone()
        });

        let result = dispatcher.call_tool_for_test("delete_file", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], false);
        assert!(response["message"].is_string());
        // Error message should indicate it's a directory
        let msg = response["message"].as_str().unwrap();
        assert!(msg.contains("directory") || msg.contains("Directory") || msg.contains("Is a directory"));

        // Verify directory still exists
        assert!(Path::new(&dir_path).exists());
    }

    #[tokio::test]
    async fn test_delete_file_permission_denied() {
        // This test requires proper setup of a read-only file
        // Skip on systems where we can't test permissions properly
        if cfg!(windows) {
            return; // Windows permission model is different
        }

        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::register_tools(&mut dispatcher);

        // Create a temporary directory and file
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("readonly.txt");
        fs::write(&file_path, "test content").unwrap();

        // Make the directory read-only (prevents file deletion on Unix)
        let dir_permissions = fs::metadata(temp_dir.path()).unwrap().permissions();
        let mut readonly_perms = dir_permissions.clone();
        readonly_perms.set_readonly(true);

        // Try to set read-only permissions
        if fs::set_permissions(temp_dir.path(), readonly_perms).is_ok() {
            let args = json!({
                "file_path": file_path.to_str().unwrap()
            });

            let result = dispatcher.call_tool_for_test("delete_file", args).await;
            assert!(result.is_ok());

            let response = result.unwrap();
            assert_eq!(response["success"], false);
            assert!(response["message"].is_string());

            // Restore permissions for cleanup
            fs::set_permissions(temp_dir.path(), dir_permissions).ok();
        }
    }

    #[tokio::test]
    async fn test_delete_file_invalid_args() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::register_tools(&mut dispatcher);

        // Test with missing file_path
        let args = json!({});
        let result = dispatcher.call_tool_for_test("delete_file", args).await;
        assert!(result.is_err());

        // Test with null file_path
        let args = json!({
            "file_path": null
        });
        let result = dispatcher.call_tool_for_test("delete_file", args).await;
        assert!(result.is_err());

        // Test with invalid type for force flag
        let args = json!({
            "file_path": "/test.txt",
            "force": "yes"  // Should be boolean
        });
        let result = dispatcher.call_tool_for_test("delete_file", args).await;
        // This might work due to type coercion, but let's verify behavior
        if result.is_ok() {
            let response = result.unwrap();
            // Should still handle the request appropriately
            assert!(response.is_object());
        }
    }

    #[tokio::test]
    async fn test_delete_file_with_lsp_notification() {
        let mut dispatcher = McpDispatcher::new();

        let mut mock_lsp = MockLspService::new();

        // Expect LSP notification about file deletion
        mock_lsp.expect_did_delete_files()
            .times(1)
            .returning(|_| Ok(()));

        let app_state = Arc::new(AppState {
            file_service: Arc::new(FileService::new()),
            symbol_service: Arc::new(SymbolService::new(Arc::new(mock_lsp.clone()))),
            editing_service: Arc::new(EditingService::new(Arc::new(mock_lsp.clone()))),
            import_service: Arc::new(ImportService::new()),
            lsp_service: Arc::new(mock_lsp),
        });

        super::register_tools(&mut dispatcher);

        // Create a temporary file
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test content").unwrap();
        let file_path = temp_file.path().to_str().unwrap().to_string();

        let args = json!({
            "file_path": file_path.clone()
        });

        let result = dispatcher.call_tool_for_test("delete_file", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], true);
    }

    #[tokio::test]
    async fn test_create_and_delete_file_workflow() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::register_tools(&mut dispatcher);

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt").to_str().unwrap().to_string();

        // Create file
        let create_args = json!({
            "file_path": file_path.clone(),
            "content": "Hello, World!"
        });

        let create_result = dispatcher.call_tool_for_test("create_file", create_args).await;
        assert!(create_result.is_ok());

        let create_response = create_result.unwrap();
        assert_eq!(create_response["success"], true);
        assert!(Path::new(&file_path).exists());

        // Delete file
        let delete_args = json!({
            "file_path": file_path.clone()
        });

        let delete_result = dispatcher.call_tool_for_test("delete_file", delete_args).await;
        assert!(delete_result.is_ok());

        let delete_response = delete_result.unwrap();
        assert_eq!(delete_response["success"], true);
        assert!(!Path::new(&file_path).exists());
    }

    #[tokio::test]
    async fn test_delete_file_relative_path() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::register_tools(&mut dispatcher);

        // Create a file in current directory
        let mut temp_file = NamedTempFile::new_in(".").unwrap();
        temp_file.write_all(b"test content").unwrap();
        let file_name = temp_file.path().file_name().unwrap().to_str().unwrap().to_string();

        let args = json!({
            "file_path": file_name.clone()
        });

        let result = dispatcher.call_tool_for_test("delete_file", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        // Should work with relative paths
        assert_eq!(response["success"], true);
        assert!(!Path::new(&file_name).exists());
    }
}