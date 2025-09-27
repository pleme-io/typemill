use cb_tests::harness::{TestClient, TestWorkspace};
use serde_json::{json, Value};
use std::path::Path;
use tempfile::TempDir;

#[tokio::test]
async fn test_create_file_basic() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    let file_path = workspace.path().join("new_file.txt");
    let content = "Hello, World!";

    let response = client.call_tool("create_file", json!({
        "file_path": file_path.to_string_lossy(),
        "content": content
    })).await.unwrap();

    assert!(response["success"].as_bool().unwrap_or(false));
    assert!(file_path.exists());

    let actual_content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(actual_content, content);
}

#[tokio::test]
async fn test_create_file_with_directories() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    let file_path = workspace.path().join("nested/deep/new_file.js");
    let content = "export const greeting = 'Hello from nested file!';";

    let response = client.call_tool("create_file", json!({
        "file_path": file_path.to_string_lossy(),
        "content": content
    })).await.unwrap();

    assert!(response["success"].as_bool().unwrap_or(false));
    assert!(file_path.exists());
    assert!(file_path.parent().unwrap().exists());

    let actual_content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(actual_content, content);
}

#[tokio::test]
async fn test_create_file_overwrite_protection() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    let file_path = workspace.path().join("existing.txt");
    std::fs::write(&file_path, "original content").unwrap();

    let response = client.call_tool("create_file", json!({
        "file_path": file_path.to_string_lossy(),
        "content": "new content"
    })).await;

    // Should fail without overwrite flag
    assert!(response.is_err() || !response.unwrap()["success"].as_bool().unwrap_or(true));

    let actual_content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(actual_content, "original content");
}

#[tokio::test]
async fn test_create_file_with_overwrite() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    let file_path = workspace.path().join("existing.txt");
    std::fs::write(&file_path, "original content").unwrap();

    let response = client.call_tool("create_file", json!({
        "file_path": file_path.to_string_lossy(),
        "content": "new content",
        "overwrite": true
    })).await.unwrap();

    assert!(response["success"].as_bool().unwrap_or(false));

    let actual_content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(actual_content, "new content");
}

#[tokio::test]
async fn test_read_file_basic() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    let file_path = workspace.path().join("test_file.txt");
    let content = "This is test content\nwith multiple lines\nand unicode: 🚀";
    std::fs::write(&file_path, content).unwrap();

    let response = client.call_tool("read_file", json!({
        "file_path": file_path.to_string_lossy()
    })).await.unwrap();

    assert_eq!(response["content"].as_str().unwrap(), content);
}

#[tokio::test]
async fn test_read_file_nonexistent() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    let file_path = workspace.path().join("nonexistent.txt");

    let response = client.call_tool("read_file", json!({
        "file_path": file_path.to_string_lossy()
    })).await;

    assert!(response.is_err());
}

#[tokio::test]
async fn test_read_file_with_range() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    let file_path = workspace.path().join("large_file.txt");
    let lines: Vec<String> = (1..=100).map(|i| format!("Line {}", i)).collect();
    let content = lines.join("\n");
    std::fs::write(&file_path, &content).unwrap();

    let response = client.call_tool("read_file", json!({
        "file_path": file_path.to_string_lossy(),
        "start_line": 10,
        "end_line": 20
    })).await.unwrap();

    let expected_lines: Vec<String> = (10..=20).map(|i| format!("Line {}", i)).collect();
    let expected = expected_lines.join("\n");
    assert_eq!(response["content"].as_str().unwrap(), expected);
}

#[tokio::test]
async fn test_write_file_basic() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    let file_path = workspace.path().join("write_test.txt");
    let content = "Written content with special chars: @#$%^&*()";

    let response = client.call_tool("write_file", json!({
        "file_path": file_path.to_string_lossy(),
        "content": content
    })).await.unwrap();

    assert!(response["success"].as_bool().unwrap_or(false));
    assert!(file_path.exists());

    let actual_content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(actual_content, content);
}

#[tokio::test]
async fn test_write_file_overwrites_existing() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    let file_path = workspace.path().join("overwrite_test.txt");
    std::fs::write(&file_path, "original").unwrap();

    let new_content = "completely new content";
    let response = client.call_tool("write_file", json!({
        "file_path": file_path.to_string_lossy(),
        "content": new_content
    })).await.unwrap();

    assert!(response["success"].as_bool().unwrap_or(false));

    let actual_content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(actual_content, new_content);
}

#[tokio::test]
async fn test_delete_file_basic() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    let file_path = workspace.path().join("to_delete.txt");
    std::fs::write(&file_path, "content to be deleted").unwrap();
    assert!(file_path.exists());

    let response = client.call_tool("delete_file", json!({
        "file_path": file_path.to_string_lossy()
    })).await.unwrap();

    assert!(response["success"].as_bool().unwrap_or(false));
    assert!(!file_path.exists());
}

#[tokio::test]
async fn test_delete_file_nonexistent() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    let file_path = workspace.path().join("nonexistent.txt");

    let response = client.call_tool("delete_file", json!({
        "file_path": file_path.to_string_lossy()
    })).await;

    assert!(response.is_err());
}

#[tokio::test]
async fn test_list_files_basic() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    // Create test files
    let files = vec!["file1.txt", "file2.js", "file3.py"];
    for file in &files {
        std::fs::write(workspace.path().join(file), "content").unwrap();
    }

    // Create subdirectory with files
    let subdir = workspace.path().join("subdir");
    std::fs::create_dir(&subdir).unwrap();
    std::fs::write(subdir.join("nested.txt"), "nested content").unwrap();

    let response = client.call_tool("list_files", json!({
        "directory": workspace.path().to_string_lossy()
    })).await.unwrap();

    let file_list = response["files"].as_array().unwrap();
    assert!(file_list.len() >= 4); // 3 files + 1 directory

    let file_names: Vec<&str> = file_list.iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();

    for file in &files {
        assert!(file_names.contains(file));
    }
    assert!(file_names.contains(&"subdir"));
}

#[tokio::test]
async fn test_list_files_recursive() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    // Create nested structure
    std::fs::write(workspace.path().join("root.txt"), "root").unwrap();

    let level1 = workspace.path().join("level1");
    std::fs::create_dir(&level1).unwrap();
    std::fs::write(level1.join("file1.txt"), "level1").unwrap();

    let level2 = level1.join("level2");
    std::fs::create_dir(&level2).unwrap();
    std::fs::write(level2.join("deep.txt"), "deep").unwrap();

    let response = client.call_tool("list_files", json!({
        "directory": workspace.path().to_string_lossy(),
        "recursive": true
    })).await.unwrap();

    let file_list = response["files"].as_array().unwrap();
    let paths: Vec<String> = file_list.iter()
        .map(|f| f["path"].as_str().unwrap().to_string())
        .collect();

    assert!(paths.iter().any(|p| p.ends_with("root.txt")));
    assert!(paths.iter().any(|p| p.ends_with("level1/file1.txt")));
    assert!(paths.iter().any(|p| p.ends_with("level1/level2/deep.txt")));
}

#[tokio::test]
async fn test_list_files_with_pattern() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    // Create files with different extensions
    let files = vec![
        ("test.js", "javascript"),
        ("test.ts", "typescript"),
        ("test.py", "python"),
        ("test.txt", "text"),
        ("README.md", "markdown"),
    ];

    for (file, content) in &files {
        std::fs::write(workspace.path().join(file), content).unwrap();
    }

    let response = client.call_tool("list_files", json!({
        "directory": workspace.path().to_string_lossy(),
        "pattern": "*.js"
    })).await.unwrap();

    let file_list = response["files"].as_array().unwrap();
    assert_eq!(file_list.len(), 1);
    assert_eq!(file_list[0]["name"].as_str().unwrap(), "test.js");
}

#[tokio::test]
async fn test_file_operations_integration() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    // Create a TypeScript file
    let ts_file = workspace.path().join("integration.ts");
    let initial_content = r#"
export interface User {
    id: number;
    name: string;
}

export function createUser(name: string): User {
    return { id: Math.random(), name };
}
"#;

    // Test create_file
    let response = client.call_tool("create_file", json!({
        "file_path": ts_file.to_string_lossy(),
        "content": initial_content
    })).await.unwrap();
    assert!(response["success"].as_bool().unwrap_or(false));

    // Test read_file
    let response = client.call_tool("read_file", json!({
        "file_path": ts_file.to_string_lossy()
    })).await.unwrap();
    assert_eq!(response["content"].as_str().unwrap().trim(), initial_content.trim());

    // Test write_file with modified content
    let modified_content = r#"
export interface User {
    id: number;
    name: string;
    email?: string;
}

export function createUser(name: string, email?: string): User {
    return { id: Math.random(), name, email };
}
"#;

    let response = client.call_tool("write_file", json!({
        "file_path": ts_file.to_string_lossy(),
        "content": modified_content
    })).await.unwrap();
    assert!(response["success"].as_bool().unwrap_or(false));

    // Verify the modification
    let response = client.call_tool("read_file", json!({
        "file_path": ts_file.to_string_lossy()
    })).await.unwrap();
    assert_eq!(response["content"].as_str().unwrap().trim(), modified_content.trim());

    // Test list_files to ensure our file is there
    let response = client.call_tool("list_files", json!({
        "directory": workspace.path().to_string_lossy()
    })).await.unwrap();

    let file_list = response["files"].as_array().unwrap();
    let has_our_file = file_list.iter().any(|f|
        f["name"].as_str().unwrap() == "integration.ts"
    );
    assert!(has_our_file);

    // Test delete_file
    let response = client.call_tool("delete_file", json!({
        "file_path": ts_file.to_string_lossy()
    })).await.unwrap();
    assert!(response["success"].as_bool().unwrap_or(false));
    assert!(!ts_file.exists());
}

#[tokio::test]
async fn test_large_file_handling() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    let file_path = workspace.path().join("large_file.txt");

    // Create a large file (1MB of content)
    let line = "This is a test line with some content to make it reasonably long.\n";
    let large_content = line.repeat(1024 * 16); // ~1MB

    let response = client.call_tool("create_file", json!({
        "file_path": file_path.to_string_lossy(),
        "content": large_content
    })).await.unwrap();

    assert!(response["success"].as_bool().unwrap_or(false));

    // Read back and verify size
    let response = client.call_tool("read_file", json!({
        "file_path": file_path.to_string_lossy()
    })).await.unwrap();

    let read_content = response["content"].as_str().unwrap();
    assert_eq!(read_content.len(), large_content.len());
    assert_eq!(read_content, large_content);
}

#[tokio::test]
async fn test_binary_file_handling() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    let file_path = workspace.path().join("binary_file.dat");

    // Create binary content
    let binary_data: Vec<u8> = (0..=255).cycle().take(1024).collect();
    std::fs::write(&file_path, &binary_data).unwrap();

    // Test reading binary file (should handle gracefully)
    let response = client.call_tool("read_file", json!({
        "file_path": file_path.to_string_lossy()
    })).await;

    // Binary files might be handled differently, but should not crash
    // The exact behavior depends on implementation
    match response {
        Ok(resp) => {
            // If it succeeds, content should be present
            assert!(resp.get("content").is_some());
        },
        Err(_) => {
            // If it fails, that's acceptable for binary files
            // but it should be a graceful error
        }
    }

    // List files should still work and show the binary file
    let response = client.call_tool("list_files", json!({
        "directory": workspace.path().to_string_lossy()
    })).await.unwrap();

    let file_list = response["files"].as_array().unwrap();
    let has_binary_file = file_list.iter().any(|f|
        f["name"].as_str().unwrap() == "binary_file.dat"
    );
    assert!(has_binary_file);
}