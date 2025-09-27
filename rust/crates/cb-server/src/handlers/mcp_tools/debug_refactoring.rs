//! Debug helper for refactoring tests

#[cfg(test)]
mod debug_tests {
    use crate::handlers::mcp_dispatcher::{AppState, McpDispatcher};
    use crate::handlers::mcp_tools::refactoring;
    use crate::services::{FileService, LockManager, OperationQueue};
    use cb_core::config::LspConfig;
    use crate::systems::LspManager;
    use serde_json::json;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Create a test app state for integration tests
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

    /// Create a temporary TypeScript file with given content
    fn create_temp_ts_file(content: &str) -> Result<NamedTempFile, Box<dyn std::error::Error>> {
        let mut file = NamedTempFile::with_suffix(".ts")?;
        file.write_all(content.as_bytes())?;
        file.flush()?;
        Ok(file)
    }

    #[tokio::test]
    async fn debug_extract_function_step_by_step() {
        println!("=== Starting debug_extract_function_step_by_step ===");

        println!("Step 1: Creating app state...");
        let app_state = create_test_app_state();
        println!("✓ App state created");

        println!("Step 2: Creating dispatcher...");
        let mut dispatcher = McpDispatcher::new(app_state);
        println!("✓ Dispatcher created");

        println!("Step 3: Registering tools...");
        refactoring::register_tools(&mut dispatcher);
        println!("✓ Tools registered");

        println!("Step 4: Creating temp file...");
        let content = r#"function test() {
    const x = 1;
    return x;
}"#;
        let temp_file = create_temp_ts_file(content).unwrap();
        let file_path = temp_file.path().to_str().unwrap();
        println!("✓ Temp file created: {}", file_path);

        println!("Step 5: Preparing test arguments...");
        let args = json!({
            "file_path": file_path,
            "start_line": 2,
            "start_col": 4,
            "end_line": 2,
            "end_col": 14,
            "new_function_name": "getOne",
            "preview": true
        });
        println!("✓ Args prepared: {}", args);

        println!("Step 6: Calling tool...");
        let result = dispatcher.call_tool_for_test("extract_function", args).await;
        println!("✓ Tool call completed");

        match result {
            Ok(response) => {
                println!("✓ SUCCESS! Response received:");
                println!("Response JSON: {:#}", response);

                // Test the specific assertions that are failing
                let success = response["success"].as_bool();
                let preview_mode = response["preview_mode"].as_bool();
                let analysis = response["analysis"].is_object();

                println!("success field: {:?}", success);
                println!("preview_mode field: {:?}", preview_mode);
                println!("analysis is object: {}", analysis);

                // Check actual structure
                println!("Response keys: {:?}", response.as_object().map(|o| o.keys().collect::<Vec<_>>()));
            },
            Err(e) => {
                println!("❌ ERROR: {:?}", e);
            }
        }

        println!("=== debug_extract_function_step_by_step completed ===");
    }

    #[tokio::test]
    async fn debug_inline_variable_step_by_step() {
        println!("=== Starting debug_inline_variable_step_by_step ===");

        println!("Step 1: Creating app state...");
        let app_state = create_test_app_state();
        println!("✓ App state created");

        println!("Step 2: Creating dispatcher...");
        let mut dispatcher = McpDispatcher::new(app_state);
        println!("✓ Dispatcher created");

        println!("Step 3: Registering tools...");
        refactoring::register_tools(&mut dispatcher);
        println!("✓ Tools registered");

        println!("Step 4: Creating temp file...");
        let content = r#"function test() {
    const x = 1;
    return x;
}"#;
        let temp_file = create_temp_ts_file(content).unwrap();
        let file_path = temp_file.path().to_str().unwrap();
        println!("✓ Temp file created: {}", file_path);

        println!("Step 5: Preparing test arguments...");
        let args = json!({
            "file_path": file_path,
            "line": 2,
            "col": 10,
            "preview": true
        });
        println!("✓ Args prepared: {}", args);

        println!("Step 6: Calling tool...");
        let result = dispatcher.call_tool_for_test("inline_variable", args).await;
        println!("✓ Tool call completed");

        match result {
            Ok(response) => {
                println!("✓ SUCCESS! Response received:");
                println!("Response JSON: {:#}", response);

                // Test the specific assertions that are failing
                let success = response["success"].as_bool();
                let preview_mode = response["preview_mode"].as_bool();
                let analysis = response["analysis"].is_object();

                println!("success field: {:?}", success);
                println!("preview_mode field: {:?}", preview_mode);
                println!("analysis is object: {}", analysis);

                // Check actual structure
                println!("Response keys: {:?}", response.as_object().map(|o| o.keys().collect::<Vec<_>>()));
            },
            Err(e) => {
                println!("❌ ERROR: {:?}", e);
            }
        }

        println!("=== debug_inline_variable_step_by_step completed ===");
    }

    #[tokio::test]
    async fn debug_inline_variable_execution_specific() {
        println!("=== Starting debug_inline_variable_execution_specific ===");

        println!("Step 1: Creating app state...");
        let app_state = create_test_app_state();
        println!("✓ App state created");

        println!("Step 2: Creating dispatcher...");
        let mut dispatcher = McpDispatcher::new(app_state);
        println!("✓ Dispatcher created");

        println!("Step 3: Registering tools...");
        refactoring::register_tools(&mut dispatcher);
        println!("✓ Tools registered");

        println!("Step 4: Creating temp file...");
        // Create test file with simple variable to inline
        let content = r#"function calculate(): number {
    const multiplier = 2;
    const result = 5 * multiplier;
    return result;
}"#;
        let temp_file = create_temp_ts_file(content).unwrap();
        let file_path = temp_file.path().to_str().unwrap();
        println!("✓ Temp file created: {}", file_path);
        println!("File content:\n{}", content);

        println!("Step 5: Preparing test arguments...");
        // Inline the multiplier variable
        let args = json!({
            "file_path": file_path,
            "line": 2,  // This is 1-based, so line 2 = "const multiplier = 2;"
            "col": 10,  // Should be on the "multiplier" part
            "preview": false
        });
        println!("✓ Args prepared: {}", args);

        println!("Step 6: Calling tool...");
        let result = dispatcher.call_tool_for_test("inline_variable", args).await;
        println!("✓ Tool call completed");

        match result {
            Ok(response) => {
                println!("✓ Response received:");
                println!("Response JSON: {:#}", response);

                // Check what we actually got
                let success = response["success"].as_bool().unwrap_or(false);
                let preview_mode = response["previewMode"].as_bool().unwrap_or(true);
                let modified_source = response["modifiedSource"].as_str();

                println!("success: {}", success);
                println!("previewMode: {}", preview_mode);

                if let Some(source) = modified_source {
                    println!("Modified source:\n{}", source);

                    // Check what the test is expecting
                    let contains_const_multiplier = source.contains("const multiplier");
                    let contains_5_times_2 = source.contains("5 * (2)");

                    println!("Contains 'const multiplier': {}", contains_const_multiplier);
                    println!("Contains '5 * (2)': {}", contains_5_times_2);
                    println!("Test would pass: {}", !contains_const_multiplier);

                } else {
                    println!("No modified source in response");
                }
            },
            Err(e) => {
                println!("❌ ERROR: {:?}", e);
            }
        }

        println!("=== debug_inline_variable_execution_specific completed ===");
    }
}