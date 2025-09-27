use cb_tests::harness::{TestClient, TestWorkspace};
use serde_json::{json, Value};
use std::path::Path;
use std::fs;

#[tokio::test]
async fn test_file_operations_permission_errors() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    // Try to read a non-existent file
    let nonexistent_file = workspace.path().join("does_not_exist.txt");

    let response = client.call_tool("read_file", json!({
        "file_path": nonexistent_file.to_string_lossy()
    })).await;

    assert!(response.is_err(), "Reading non-existent file should fail");

    // Try to delete a non-existent file
    let response = client.call_tool("delete_file", json!({
        "file_path": nonexistent_file.to_string_lossy()
    })).await;

    assert!(response.is_err(), "Deleting non-existent file should fail");

    // Try to list files in non-existent directory
    let nonexistent_dir = workspace.path().join("nonexistent_directory");

    let response = client.call_tool("list_files", json!({
        "directory": nonexistent_dir.to_string_lossy()
    })).await;

    assert!(response.is_err(), "Listing non-existent directory should fail");
}

#[tokio::test]
async fn test_lsp_operations_with_invalid_files() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    // Try LSP operations on non-existent file
    let nonexistent_file = workspace.path().join("nonexistent.ts");

    let response = client.call_tool("find_definition", json!({
        "file_path": nonexistent_file.to_string_lossy(),
        "line": 1,
        "character": 5
    })).await;

    assert!(response.is_err(), "LSP operation on non-existent file should fail");

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
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    // Test missing required parameters
    let response = client.call_tool("read_file", json!({})).await;
    assert!(response.is_err(), "Tool call missing required parameters should fail");

    // Test invalid parameter types
    let response = client.call_tool("find_definition", json!({
        "file_path": "/valid/path.ts",
        "line": "not_a_number",
        "character": 5
    })).await;
    assert!(response.is_err(), "Tool call with invalid parameter types should fail");

    // Test negative coordinates
    let valid_file = workspace.path().join("test.ts");
    fs::write(&valid_file, "const x = 1;").unwrap();

    let response = client.call_tool("find_definition", json!({
        "file_path": valid_file.to_string_lossy(),
        "line": -1,
        "character": -1
    })).await;
    assert!(response.is_err(), "Tool call with negative coordinates should fail");
}

#[tokio::test]
async fn test_file_corruption_scenarios() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

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
            // If it succeeds, should handle encoding somehow
            assert!(resp.get("content").is_some());
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
            let content = resp["content"].as_str().unwrap();
            // Might be truncated or handled in chunks
            assert!(!content.is_empty());
        },
        Err(_) => {
            // May fail due to size limits
        }
    }
}

#[tokio::test]
async fn test_concurrent_file_access_errors() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    let file_path = workspace.path().join("concurrent_test.txt");
    fs::write(&file_path, "initial content").unwrap();

    // Simulate concurrent access by multiple rapid operations
    let mut handles = vec![];

    for i in 0..5 {
        let file_path_clone = file_path.clone();
        let client_clone = client.clone();

        let handle = tokio::spawn(async move {
            let content = format!("Content from task {}", i);

            // Try to write concurrently
            let write_result = client_clone.call_tool("write_file", json!({
                "file_path": file_path_clone.to_string_lossy(),
                "content": content
            })).await;

            // Try to read concurrently
            let read_result = client_clone.call_tool("read_file", json!({
                "file_path": file_path_clone.to_string_lossy()
            })).await;

            (write_result, read_result)
        });

        handles.push(handle);
    }

    // Wait for all concurrent operations
    let results = futures::future::join_all(handles).await;

    // At least some operations should succeed
    let successful_ops = results.iter().filter(|r| {
        if let Ok((write_res, read_res)) = r {
            write_res.is_ok() && read_res.is_ok()
        } else {
            false
        }
    }).count();

    assert!(successful_ops > 0, "At least some concurrent operations should succeed");
}

#[tokio::test]
async fn test_workspace_edit_rollback_on_failure() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

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
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

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
                let locs = locations.as_array().unwrap();
                assert!(locs.is_empty());
            }
        },
        Err(_) => {
            // Or fail gracefully
        }
    }
}

#[tokio::test]
async fn test_dependency_update_errors() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    // Test with completely invalid JSON
    let invalid_json = workspace.path().join("invalid.json");
    fs::write(&invalid_json, "{ this is not valid json }").unwrap();

    let response = client.call_tool("update_dependencies", json!({
        "file_path": invalid_json.to_string_lossy(),
        "add_dependencies": {
            "test": "1.0.0"
        }
    })).await;

    assert!(response.is_err(), "Updating invalid JSON should fail");

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
            // If it succeeds, it should have handled the structure gracefully
            assert!(resp.get("success").is_some());
        },
        Err(_) => {
            // Or it can fail gracefully
        }
    }
}

#[tokio::test]
async fn test_timeout_scenarios() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

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
"#, i, i, i, i, i, i, i, i, i, i, i, i));
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
            let symbols = resp["symbols"].as_array().unwrap();
            assert!(!symbols.is_empty());
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
            let symbols = resp["symbols"].as_array().unwrap();
            // Should find at least some interfaces
            assert!(!symbols.is_empty());
        },
        Err(_) => {
            // Timeout is acceptable for large workspace search
        }
    }
}

#[tokio::test]
async fn test_resource_exhaustion() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    // Try to create many files rapidly to test resource limits
    let mut handles = vec![];

    for i in 0..50 {
        let workspace_path = workspace.path().to_path_buf();
        let client_clone = client.clone();

        let handle = tokio::spawn(async move {
            let file_path = workspace_path.join(format!("file_{}.txt", i));
            let content = format!("Content for file {}", i);

            let create_result = client_clone.call_tool("create_file", json!({
                "file_path": file_path.to_string_lossy(),
                "content": content
            })).await;

            let read_result = client_clone.call_tool("read_file", json!({
                "file_path": file_path.to_string_lossy()
            })).await;

            (i, create_result, read_result)
        });

        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;

    // Count successful operations
    let successful_creates = results.iter().filter(|r| {
        if let Ok((_, create_res, _)) = r {
            create_res.is_ok()
        } else {
            false
        }
    }).count();

    let successful_reads = results.iter().filter(|r| {
        if let Ok((_, _, read_res)) = r {
            read_res.is_ok()
        } else {
            false
        }
    }).count();

    // Should handle at least some operations successfully
    assert!(successful_creates > 0, "Should successfully create some files");
    assert!(successful_reads > 0, "Should successfully read some files");

    // Cleanup - try to list all created files
    let list_response = client.call_tool("list_files", json!({
        "directory": workspace.path().to_string_lossy()
    })).await.unwrap();

    let files = list_response["files"].as_array().unwrap();
    assert!(!files.is_empty(), "Should list created files");
}

#[tokio::test]
async fn test_invalid_characters_in_paths() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

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
                    let read_response = client.call_tool("read_file", json!({
                        "file_path": file_path.to_string_lossy()
                    })).await.unwrap();

                    assert!(read_response.get("content").is_some());
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
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    // Step 1: Perform a successful operation
    let good_file = workspace.path().join("good.ts");
    fs::write(&good_file, "const good = true;").unwrap();

    let response = client.call_tool("read_file", json!({
        "file_path": good_file.to_string_lossy()
    })).await.unwrap();

    assert_eq!(response["content"].as_str().unwrap(), "const good = true;");

    // Step 2: Cause an error
    let bad_file = workspace.path().join("nonexistent.ts");
    let error_response = client.call_tool("read_file", json!({
        "file_path": bad_file.to_string_lossy()
    })).await;

    assert!(error_response.is_err());

    // Step 3: Verify system still works after error
    let another_good_file = workspace.path().join("another_good.ts");
    fs::write(&another_good_file, "const stillWorking = true;").unwrap();

    let recovery_response = client.call_tool("read_file", json!({
        "file_path": another_good_file.to_string_lossy()
    })).await.unwrap();

    assert_eq!(recovery_response["content"].as_str().unwrap(), "const stillWorking = true;");

    // Step 4: Check that health is still good
    let health_response = client.call_tool("health_check", json!({})).await.unwrap();
    let status = health_response["status"].as_str().unwrap();
    assert!(status == "healthy" || status == "degraded");
}