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
    use std::io::Write;
    use serde_json::{json, Value};
    use std::path::PathBuf;

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

    /// Create a temporary file with content
    fn create_temp_file(content: &str) -> Result<NamedTempFile, Box<dyn std::error::Error>> {
        let mut file = NamedTempFile::new()?;
        file.write_all(content.as_bytes())?;
        file.flush()?;
        Ok(file)
    }

    #[tokio::test]
    async fn test_read_file_success() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::filesystem::register_tools(&mut dispatcher);

        let temp_file = create_temp_file("Hello, World!\nThis is a test file.").unwrap();
        let file_path = temp_file.path().to_string_lossy();

        let args = json!({
            "file_path": file_path
        });

        let result = dispatcher.call_tool_for_test("read_file", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], true);
        assert_eq!(response["file_path"], file_path);
        assert_eq!(response["content"], "Hello, World!\nThis is a test file.");
        assert!(response["size"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_read_file_nonexistent() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::filesystem::register_tools(&mut dispatcher);

        let args = json!({
            "file_path": "/nonexistent/file.txt"
        });

        let result = dispatcher.call_tool_for_test("read_file", args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No such file"));
    }

    #[tokio::test]
    async fn test_read_file_empty() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::filesystem::register_tools(&mut dispatcher);

        let temp_file = create_temp_file("").unwrap();
        let file_path = temp_file.path().to_string_lossy();

        let args = json!({
            "file_path": file_path
        });

        let result = dispatcher.call_tool_for_test("read_file", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], true);
        assert_eq!(response["content"], "");
        assert_eq!(response["size"], 0);
    }

    #[tokio::test]
    async fn test_write_file_success() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::filesystem::register_tools(&mut dispatcher);

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_write.txt");

        let args = json!({
            "file_path": file_path.to_string_lossy(),
            "content": "New file content\nWith multiple lines."
        });

        let result = dispatcher.call_tool_for_test("write_file", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], true);
        assert_eq!(response["file_path"], file_path.to_string_lossy());
        assert!(response["bytes_written"].as_u64().unwrap() > 0);

        // Verify the file was actually written
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "New file content\nWith multiple lines.");
    }

    #[tokio::test]
    async fn test_write_file_overwrite() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::filesystem::register_tools(&mut dispatcher);

        let temp_file = create_temp_file("Original content").unwrap();
        let file_path = temp_file.path().to_string_lossy();

        let args = json!({
            "file_path": file_path,
            "content": "Overwritten content"
        });

        let result = dispatcher.call_tool_for_test("write_file", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], true);

        // Verify the file was overwritten
        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        assert_eq!(content, "Overwritten content");
    }

    #[tokio::test]
    async fn test_write_file_invalid_path() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::filesystem::register_tools(&mut dispatcher);

        let args = json!({
            "file_path": "/nonexistent/directory/file.txt",
            "content": "Content"
        });

        let result = dispatcher.call_tool_for_test("write_file", args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No such file or directory"));
    }

    #[tokio::test]
    async fn test_list_files_directory() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::filesystem::register_tools(&mut dispatcher);

        let temp_dir = TempDir::new().unwrap();

        // Create some test files
        std::fs::write(temp_dir.path().join("file1.txt"), "content1").unwrap();
        std::fs::write(temp_dir.path().join("file2.js"), "content2").unwrap();
        std::fs::create_dir(temp_dir.path().join("subdir")).unwrap();

        let args = json!({
            "path": temp_dir.path().to_string_lossy(),
            "recursive": false
        });

        let result = dispatcher.call_tool_for_test("list_files", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], true);
        assert_eq!(response["path"], temp_dir.path().to_string_lossy());

        let files = response["files"].as_array().unwrap();
        assert_eq!(files.len(), 3); // 2 files + 1 directory

        // Check that our files are listed
        let file_names: Vec<String> = files.iter()
            .map(|f| f["name"].as_str().unwrap().to_string())
            .collect();
        assert!(file_names.contains(&"file1.txt".to_string()));
        assert!(file_names.contains(&"file2.js".to_string()));
        assert!(file_names.contains(&"subdir".to_string()));
    }

    #[tokio::test]
    async fn test_list_files_recursive() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::filesystem::register_tools(&mut dispatcher);

        let temp_dir = TempDir::new().unwrap();

        // Create nested structure
        std::fs::write(temp_dir.path().join("root.txt"), "root").unwrap();
        std::fs::create_dir(temp_dir.path().join("subdir")).unwrap();
        std::fs::write(temp_dir.path().join("subdir").join("nested.txt"), "nested").unwrap();

        let args = json!({
            "path": temp_dir.path().to_string_lossy(),
            "recursive": true
        });

        let result = dispatcher.call_tool_for_test("list_files", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], true);

        let files = response["files"].as_array().unwrap();
        assert!(files.len() >= 3); // root.txt, subdir, nested.txt

        // Check for nested file
        let file_paths: Vec<String> = files.iter()
            .map(|f| f["path"].as_str().unwrap().to_string())
            .collect();
        assert!(file_paths.iter().any(|p| p.contains("nested.txt")));
    }

    #[tokio::test]
    async fn test_list_files_nonexistent_directory() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::filesystem::register_tools(&mut dispatcher);

        let args = json!({
            "path": "/nonexistent/directory",
            "recursive": false
        });

        let result = dispatcher.call_tool_for_test("list_files", args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No such file or directory"));
    }

    #[tokio::test]
    async fn test_health_check_success() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::filesystem::register_tools(&mut dispatcher);

        let args = json!({
            "include_details": true
        });

        let result = dispatcher.call_tool_for_test("health_check", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["status"], "healthy");
        assert!(response["timestamp"].is_string());
        assert!(response["uptime"].is_string());
        assert!(response["version"].is_string());
        assert!(response["details"].is_object());

        let details = &response["details"];
        assert!(details["memory_usage"].is_object());
        assert!(details["lsp_servers"].is_array());
        assert!(details["file_system"].is_object());
    }

    #[tokio::test]
    async fn test_health_check_minimal() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::filesystem::register_tools(&mut dispatcher);

        let args = json!({
            "include_details": false
        });

        let result = dispatcher.call_tool_for_test("health_check", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["status"], "healthy");
        assert!(response["timestamp"].is_string());
        assert!(response["uptime"].is_string());
        assert!(response["version"].is_string());
        // Details should be minimal or absent
        if response["details"].is_object() {
            let details = &response["details"];
            assert!(details.as_object().unwrap().len() < 5); // Minimal details
        }
    }

    #[tokio::test]
    async fn test_update_dependencies_npm() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::filesystem::register_tools(&mut dispatcher);

        let temp_dir = TempDir::new().unwrap();

        // Create a mock package.json
        let package_json = json!({
            "name": "test-project",
            "version": "1.0.0",
            "dependencies": {
                "lodash": "^4.17.21"
            }
        });
        std::fs::write(
            temp_dir.path().join("package.json"),
            serde_json::to_string_pretty(&package_json).unwrap()
        ).unwrap();

        let args = json!({
            "language": "javascript",
            "workspace_path": temp_dir.path().to_string_lossy(),
            "dependencies": ["express@^4.18.0"],
            "dev_dependencies": ["jest@^29.0.0"]
        });

        let result = dispatcher.call_tool_for_test("update_dependencies", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], true);
        assert_eq!(response["language"], "javascript");
        assert!(response["added_dependencies"].as_array().unwrap().len() > 0);
    }

    #[tokio::test]
    async fn test_update_dependencies_python() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::filesystem::register_tools(&mut dispatcher);

        let temp_dir = TempDir::new().unwrap();

        // Create a mock requirements.txt
        std::fs::write(temp_dir.path().join("requirements.txt"), "requests==2.28.0\n").unwrap();

        let args = json!({
            "language": "python",
            "workspace_path": temp_dir.path().to_string_lossy(),
            "dependencies": ["flask>=2.0.0"],
            "dev_dependencies": ["pytest>=7.0.0"]
        });

        let result = dispatcher.call_tool_for_test("update_dependencies", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], true);
        assert_eq!(response["language"], "python");
        assert!(response["added_dependencies"].as_array().unwrap().len() > 0);
    }

    #[tokio::test]
    async fn test_update_dependencies_unsupported_language() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::filesystem::register_tools(&mut dispatcher);

        let args = json!({
            "language": "fortran",
            "workspace_path": "/tmp",
            "dependencies": ["some-lib"]
        });

        let result = dispatcher.call_tool_for_test("update_dependencies", args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported language"));
    }

    #[tokio::test]
    async fn test_concurrent_file_operations() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::filesystem::register_tools(&mut dispatcher);

        let temp_dir = TempDir::new().unwrap();

        // Create multiple files concurrently
        let tasks = (0..5).map(|i| {
            let file_path = temp_dir.path().join(format!("concurrent_file_{}.txt", i));
            dispatcher.call_tool_for_test("write_file", json!({
                "file_path": file_path.to_string_lossy(),
                "content": format!("Content for file {}", i)
            }))
        }).collect::<Vec<_>>();

        let results = futures::future::join_all(tasks).await;

        // All writes should succeed
        for result in results {
            assert!(result.is_ok());
            let response = result.unwrap();
            assert_eq!(response["success"], true);
        }

        // Verify all files were created
        for i in 0..5 {
            let file_path = temp_dir.path().join(format!("concurrent_file_{}.txt", i));
            assert!(file_path.exists());
            let content = std::fs::read_to_string(&file_path).unwrap();
            assert_eq!(content, format!("Content for file {}", i));
        }
    }

    #[tokio::test]
    async fn test_large_file_operations() {
        let mut dispatcher = McpDispatcher::new();
        let app_state = create_test_app_state();

        super::super::filesystem::register_tools(&mut dispatcher);

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("large_file.txt");

        // Create a large content string (1MB)
        let large_content = "x".repeat(1024 * 1024);

        let args = json!({
            "file_path": file_path.to_string_lossy(),
            "content": large_content
        });

        let result = dispatcher.call_tool_for_test("write_file", args).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["success"], true);
        assert_eq!(response["bytes_written"], 1024 * 1024);

        // Now read it back
        let read_args = json!({
            "file_path": file_path.to_string_lossy()
        });

        let read_result = dispatcher.call_tool_for_test("read_file", read_args).await;
        assert!(read_result.is_ok());

        let read_response = read_result.unwrap();
        assert_eq!(read_response["success"], true);
        assert_eq!(read_response["size"], 1024 * 1024);
        assert_eq!(read_response["content"].as_str().unwrap().len(), 1024 * 1024);
    }
}