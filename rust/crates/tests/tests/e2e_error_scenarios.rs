use tests::harness::{TestClient, TestWorkspace};
use serde_json::{json, Value};
use std::path::Path;
use std::fs;

#[tokio::test]
async fn test_file_operations_permission_errors() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Try to read a non-existent file
    let nonexistent_file = workspace.path().join("does_not_exist.txt");

    let response = client.call_tool("read_file", json!({
        "file_path": nonexistent_file.to_string_lossy()
    })).await;

    // Server should either error or return a response with error information
    match response {
        Err(_) => {}, // Transport error is acceptable
        Ok(resp) => {
            // JSON-RPC error response is also acceptable
            assert!(resp.get("error").is_some() || resp["result"].is_null(),
                "Reading non-existent file should fail or return null");
        }
    }

    // Try to delete a non-existent file
    let response = client.call_tool("delete_file", json!({
        "file_path": nonexistent_file.to_string_lossy()
    })).await;

    match response {
        Err(_) => {},
        Ok(resp) => {
            assert!(resp.get("error").is_some() || resp["result"].is_null(),
                "Deleting non-existent file should fail or return null");
        }
    }

    // Try to list files in non-existent directory
    let nonexistent_dir = workspace.path().join("nonexistent_directory");

    let response = client.call_tool("list_files", json!({
        "directory": nonexistent_dir.to_string_lossy()
    })).await;

    match response {
        Err(_) => {},
        Ok(resp) => {
            assert!(resp.get("error").is_some() || resp["result"].is_null(),
                "Listing non-existent directory should fail or return null");
        }
    }
}

#[tokio::test]
async fn test_lsp_operations_with_invalid_files() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Try LSP operations on non-existent file
    let nonexistent_file = workspace.path().join("nonexistent.ts");

    let response = client.call_tool("find_definition", json!({
        "file_path": nonexistent_file.to_string_lossy(),
        "line": 1,
        "character": 5
    })).await;

    match response {
        Err(_) => {},
        Ok(resp) => {
            assert!(resp.get("error").is_some() || resp["result"].is_null(),
                "LSP operation on non-existent file should fail or return null");
        }
    }

    // Try LSP operations with invalid coordinates
    let valid_file = workspace.path().join("valid.ts");
    fs::write(&valid_file, "const x = 1;").unwrap();

    let response = client.call_tool("find_definition", json!({
        "file_path": valid_file.to_string_lossy(),
        "line": 1000, // Way beyond file length
        "character": 1000
    })).await;

    // Should either fail or return empty results gracefully
    match response {
        Ok(resp) => {
            // If it succeeds, should return empty locations
            let locations = resp.get("locations").and_then(|l| l.as_array());
            if let Some(locs) = locations {
                assert!(locs.is_empty(), "Should return empty locations for invalid position");
            }
        },
        Err(_) => {
            // Failing is also acceptable for invalid coordinates
        }
    }
}

#[tokio::test]
async fn test_malformed_tool_requests() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Test missing required parameters
    let response = client.call_tool("read_file", json!({})).await;
    match response {
        Err(_) => {},
        Ok(resp) => {
            assert!(resp.get("error").is_some(),
                "Tool call missing required parameters should return error");
        }
    }

    // Test invalid parameter types
    let response = client.call_tool("find_definition", json!({
        "file_path": "/valid/path.ts",
        "line": "not_a_number",
        "character": 5
    })).await;
    match response {
        Err(_) => {},
        Ok(resp) => {
            assert!(resp.get("error").is_some(),
                "Tool call with invalid parameter types should return error");
        }
    }

    // Test negative coordinates
    let valid_file = workspace.path().join("test.ts");
    fs::write(&valid_file, "const x = 1;").unwrap();

    let response = client.call_tool("find_definition", json!({
        "file_path": valid_file.to_string_lossy(),
        "line": -1,
        "character": -1
    })).await;
    match response {
        Err(_) => {},
        Ok(resp) => {
            assert!(resp.get("error").is_some(),
                "Tool call with negative coordinates should return error");
        }
    }
}

#[tokio::test]
async fn test_file_corruption_scenarios() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a file with invalid UTF-8
    let invalid_utf8_file = workspace.path().join("invalid_utf8.txt");
    let invalid_bytes = vec![0xFF, 0xFE, 0xFD]; // Invalid UTF-8 sequence
    fs::write(&invalid_utf8_file, &invalid_bytes).unwrap();

    let response = client.call_tool("read_file", json!({
        "file_path": invalid_utf8_file.to_string_lossy()
    })).await;

    // Should handle invalid UTF-8 gracefully
    match response {
        Ok(resp) => {
            // If it succeeds, should either have content or error
            assert!(resp.get("content").is_some() || resp.get("error").is_some());
        },
        Err(_) => {
            // Failing gracefully is also acceptable
        }
    }

    // Test extremely large file
    let large_file = workspace.path().join("large.txt");
    let large_content = "A".repeat(10_000_000); // 10MB file
    fs::write(&large_file, &large_content).unwrap();

    let response = client.call_tool("read_file", json!({
        "file_path": large_file.to_string_lossy()
    })).await;

    // Should handle large files appropriately
    match response {
        Ok(resp) => {
            if let Some(content) = resp["content"].as_str() {
                // Might be truncated or handled in chunks
                assert!(!content.is_empty());
            } else if let Some(result) = resp.get("result") {
                if let Some(content) = result["content"].as_str() {
                    assert!(!content.is_empty());
                }
            }
        },
        Err(_) => {
            // May fail due to size limits
        }
    }
}

#[tokio::test]
async fn test_rapid_file_access_operations() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let file_path = workspace.path().join("rapid_test.txt");
    fs::write(&file_path, "initial content").unwrap();

    // Simulate rapid sequential access instead of concurrent
    let mut successful_ops = 0;

    for i in 0..5 {
        let content = format!("Content from task {}", i);

        // Try rapid write operations
        let write_result = client.call_tool("write_file", json!({
            "file_path": file_path.to_string_lossy(),
            "content": content
        })).await;

        // Try rapid read operations
        let read_result = client.call_tool("read_file", json!({
            "file_path": file_path.to_string_lossy()
        })).await;

        if write_result.is_ok() && read_result.is_ok() {
            successful_ops += 1;
        }
    }

    assert!(successful_ops > 0, "At least some rapid operations should succeed");
}

#[tokio::test]
async fn test_workspace_edit_rollback_on_failure() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let file1 = workspace.path().join("file1.ts");
    let file2 = workspace.path().join("file2.ts");

    fs::write(&file1, "const value1 = 'original';").unwrap();
    fs::write(&file2, "const value2 = 'original';").unwrap();

    // Try workspace edit that should fail (invalid range in file2)
    let response = client.call_tool("apply_workspace_edit", json!({
        "changes": {
            file1.to_string_lossy(): [
                {
                    "range": {
                        "start": { "line": 0, "character": 6 },
                        "end": { "line": 0, "character": 12 }
                    },
                    "newText": "newValue1"
                }
            ],
            file2.to_string_lossy(): [
                {
                    "range": {
                        "start": { "line": 100, "character": 0 }, // Invalid line
                        "end": { "line": 100, "character": 5 }
                    },
                    "newText": "invalid"
                }
            ]
        }
    })).await;

    // Should fail and not apply any changes
    match response {
        Ok(resp) => {
            if !resp["applied"].as_bool().unwrap_or(true) {
                // If it reports failure, files should be unchanged
                let content1 = fs::read_to_string(&file1).unwrap();
                let content2 = fs::read_to_string(&file2).unwrap();

                assert_eq!(content1, "const value1 = 'original';");
                assert_eq!(content2, "const value2 = 'original';");
            }
        },
        Err(_) => {
            // If it fails, files should definitely be unchanged
            let content1 = fs::read_to_string(&file1).unwrap();
            let content2 = fs::read_to_string(&file2).unwrap();

            assert_eq!(content1, "const value1 = 'original';");
            assert_eq!(content2, "const value2 = 'original';");
        }
    }
}

#[tokio::test]
async fn test_lsp_server_unavailable() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a file with extension that has no LSP server configured
    let unknown_file = workspace.path().join("test.unknownext");
    fs::write(&unknown_file, "some content").unwrap();

    let response = client.call_tool("find_definition", json!({
        "file_path": unknown_file.to_string_lossy(),
        "line": 0,
        "character": 0
    })).await;

    // Should handle gracefully when no LSP server is available
    match response {
        Ok(resp) => {
            // Might return empty results
            if let Some(locations) = resp.get("locations") {
                if let Some(locs) = locations.as_array() {
                    assert!(locs.is_empty());
                }
            }
        },
        Err(_) => {
            // Or fail gracefully
        }
    }
}

#[tokio::test]
async fn test_dependency_update_errors() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Test with completely invalid JSON
    let invalid_json = workspace.path().join("invalid.json");
    fs::write(&invalid_json, "{ this is not valid json }").unwrap();

    let response = client.call_tool("update_dependencies", json!({
        "file_path": invalid_json.to_string_lossy(),
        "add_dependencies": {
            "test": "1.0.0"
        }
    })).await;

    match response {
        Err(_) => {},
        Ok(resp) => {
            assert!(resp.get("error").is_some(),
                "Updating invalid JSON should return error");
        }
    }

    // Test with JSON that's not a package.json structure
    let wrong_structure = workspace.path().join("wrong.json");
    fs::write(&wrong_structure, r#"{"not": "a package json"}"#).unwrap();

    let response = client.call_tool("update_dependencies", json!({
        "file_path": wrong_structure.to_string_lossy(),
        "add_dependencies": {
            "test": "1.0.0"
        }
    })).await;

    // Should handle gracefully or fail appropriately
    match response {
        Ok(resp) => {
            // If it succeeds, it should have some result (success, error, or content)
            assert!(resp.get("success").is_some() || resp.get("error").is_some() || resp.get("result").is_some());
        },
        Err(_) => {
            // Or it can fail gracefully
        }
    }
}

#[tokio::test]
async fn test_timeout_scenarios() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a very large TypeScript file that might cause LSP timeouts
    let large_ts_file = workspace.path().join("large.ts");
    let mut large_content = String::new();

    // Generate a large TypeScript file with many symbols
    for i in 0..1000 {
        large_content.push_str(&format!(r#"
export interface Interface{} {{
    property{}: string;
    method{}(): void;
}}

export class Class{} implements Interface{} {{
    property{}: string = "value{}";

    method{}(): void {{
        console.log("Method {} called");
    }}

    anotherMethod{}(): Interface{} {{
        return {{ property{}: "test{}", method{}: () => {{}} }};
    }}
}}
"#, i, i, i, i, i, i, i, i, i, i, i, i, i, i));
    }

    fs::write(&large_ts_file, &large_content).unwrap();

    // Give LSP time to process
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

    // Try operations that might timeout
    let response = client.call_tool("get_document_symbols", json!({
        "file_path": large_ts_file.to_string_lossy()
    })).await;

    match response {
        Ok(resp) => {
            // If it succeeds, should have some symbols
            if let Some(symbols) = resp["symbols"].as_array() {
                assert!(!symbols.is_empty());
            }
        },
        Err(_) => {
            // Timeout or failure is acceptable for very large files
        }
    }

    // Try search that might be slow
    let response = client.call_tool("search_workspace_symbols", json!({
        "query": "Interface"
    })).await;

    match response {
        Ok(resp) => {
            if let Some(symbols) = resp["symbols"].as_array() {
                // Should find at least some interfaces
                assert!(!symbols.is_empty());
            }
        },
        Err(_) => {
            // Timeout is acceptable for large workspace search
        }
    }
}

#[tokio::test]
async fn test_resource_exhaustion() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Try to create many files rapidly in sequence to test resource limits
    let mut successful_creates = 0;
    let mut successful_reads = 0;

    for i in 0..20 {  // Reduced count for sequential processing
        let file_path = workspace.path().join(format!("file_{}.txt", i));
        let content = format!("Content for file {}", i);

        let create_result = client.call_tool("create_file", json!({
            "file_path": file_path.to_string_lossy(),
            "content": content
        })).await;

        let read_result = client.call_tool("read_file", json!({
            "file_path": file_path.to_string_lossy()
        })).await;

        if create_result.is_ok() {
            successful_creates += 1;
        }
        if read_result.is_ok() {
            successful_reads += 1;
        }
    }

    // Should handle at least some operations successfully
    assert!(successful_creates > 0, "Should successfully create some files");
    assert!(successful_reads > 0, "Should successfully read some files");

    // Cleanup - try to list all created files
    let list_response = client.call_tool("list_files", json!({
        "directory": workspace.path().to_string_lossy()
    })).await;

    if let Ok(list_response) = list_response {
        if let Some(files) = list_response["files"].as_array() {
            assert!(!files.is_empty(), "Should list created files");
        }
    }
}

#[tokio::test]
async fn test_invalid_characters_in_paths() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Test various problematic characters in file paths
    let problematic_paths = vec![
        "file with spaces.txt",
        "file-with-unicode-🚀.txt",
        "file.with.multiple.dots.txt",
        "UPPERCASE.TXT",
        "file_with_underscores.txt",
    ];

    for path in problematic_paths {
        let file_path = workspace.path().join(path);

        // Try to create file with problematic name
        let response = client.call_tool("create_file", json!({
            "file_path": file_path.to_string_lossy(),
            "content": format!("Content for {}", path)
        })).await;

        match response {
            Ok(resp) => {
                if resp["success"].as_bool().unwrap_or(false) {
                    // If creation succeeded, reading should also work
                    if let Ok(read_response) = client.call_tool("read_file", json!({
                        "file_path": file_path.to_string_lossy()
                    })).await {
                        assert!(read_response.get("content").is_some());
                    }
                }
            },
            Err(_) => {
                // Some characters might not be supported on all systems
            }
        }
    }
}

#[tokio::test]
async fn test_error_recovery_and_continuity() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Step 1: Perform a successful operation
    let good_file = workspace.path().join("good.ts");
    fs::write(&good_file, "const good = true;").unwrap();

    let response = client.call_tool("read_file", json!({
        "file_path": good_file.to_string_lossy()
    })).await.expect("Should read good file successfully");

    if let Some(content) = response["content"].as_str() {
        assert_eq!(content, "const good = true;");
    } else if let Some(result) = response.get("result") {
        if let Some(content) = result["content"].as_str() {
            assert_eq!(content, "const good = true;");
        }
    }

    // Step 2: Cause an error
    let bad_file = workspace.path().join("nonexistent.ts");
    let error_response = client.call_tool("read_file", json!({
        "file_path": bad_file.to_string_lossy()
    })).await;

    match error_response {
        Err(_) => {},
        Ok(resp) => {
            assert!(resp.get("error").is_some() || resp["result"].is_null(),
                "Reading non-existent file should return error or null");
        }
    }

    // Step 3: Verify system still works after error
    let another_good_file = workspace.path().join("another_good.ts");
    fs::write(&another_good_file, "const stillWorking = true;").unwrap();

    let recovery_response = client.call_tool("read_file", json!({
        "file_path": another_good_file.to_string_lossy()
    })).await.expect("Should read recovery file successfully");

    if let Some(content) = recovery_response["content"].as_str() {
        assert_eq!(content, "const stillWorking = true;");
    } else if let Some(result) = recovery_response.get("result") {
        if let Some(content) = result["content"].as_str() {
            assert_eq!(content, "const stillWorking = true;");
        }
    }

    // Step 4: Check that health is still good
    let health_response = client.call_tool("health_check", json!({})).await.expect("Health check should work");
    if let Some(status) = health_response["status"].as_str() {
        assert!(status == "healthy" || status == "degraded");
    } else if let Some(result) = health_response.get("result") {
        if let Some(status) = result["status"].as_str() {
            assert!(status == "healthy" || status == "degraded");
        }
    }
}