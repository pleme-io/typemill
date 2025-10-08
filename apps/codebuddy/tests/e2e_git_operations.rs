//! Git integration tests
//!
//! These tests verify that file operations correctly use `git mv` and `git rm`
//! when the project is a git repository and git integration is enabled in config.

use serde_json::json;
use std::process::Command;
use tempfile::TempDir;
use test_support::harness::TestClient;

#[tokio::test]
async fn test_rename_file_uses_git_mv() {
    // Create a temporary directory for our test git repo
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Initialize a git repository
    let init_status = Command::new("git")
        .args(&["init"])
        .current_dir(project_path)
        .status()
        .expect("Failed to run git init");
    assert!(init_status.success(), "git init failed");

    // Configure git user for commits
    Command::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .current_dir(project_path)
        .status()
        .expect("Failed to configure git user.email");

    Command::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(project_path)
        .status()
        .expect("Failed to configure git user.name");

    // Create a codebuddy.toml with git enabled
    let config_content = r#"
[server]
host = "127.0.0.1"
port = 3000
timeoutMs = 30000

[lsp]
defaultTimeoutMs = 10000
enablePreload = false

[[lsp.servers]]
extensions = ["txt"]
command = ["cat"]

[logging]
level = "info"
format = "pretty"

[cache]
enabled = false

[git]
enabled = true
require = false
operations = ["mv", "rm"]

[validation]
enabled = false
"#;
    std::fs::write(project_path.join("codebuddy.toml"), config_content)
        .expect("Failed to write codebuddy.toml");

    // Create a test file
    let test_file = project_path.join("original.txt");
    std::fs::write(&test_file, "Hello, World!").expect("Failed to create test file");

    // Add and commit the file
    Command::new("git")
        .args(&["add", "original.txt"])
        .current_dir(project_path)
        .status()
        .expect("Failed to git add");

    Command::new("git")
        .args(&["commit", "-m", "Add original.txt"])
        .current_dir(project_path)
        .status()
        .expect("Failed to git commit");

    // Create a TestClient for the MCP server
    let mut client = TestClient::new(project_path);

    // Call the rename_file tool
    let rename_request = json!({
        "jsonrpc": "2.0",
        "id": "git-test-1",
        "method": "tools/call",
        "params": {
            "name": "rename_file",
            "arguments": {
                "old_path": project_path.join("original.txt").to_str().unwrap(),
                "new_path": project_path.join("renamed.txt").to_str().unwrap()
            }
        }
    });

    let response = client
        .send_request(rename_request)
        .expect("rename_file should work");

    assert_eq!(response["id"], "git-test-1");
    assert!(
        response["error"].is_null(),
        "rename_file should not error: {:?}",
        response["error"]
    );

    // Check git status to verify git mv was used
    let status_output = Command::new("git")
        .args(&["status", "--short"])
        .current_dir(project_path)
        .output()
        .expect("Failed to run git status");

    let status_str = String::from_utf8_lossy(&status_output.stdout);
    println!("Git status output: {}", status_str);

    // Git mv should show as "R  original.txt -> renamed.txt" or "R original.txt -> renamed.txt"
    assert!(
        status_str.contains("original.txt -> renamed.txt")
            || status_str.contains("R  original.txt -> renamed.txt"),
        "Git status should show file was renamed using git mv, but got: {}",
        status_str
    );

    // Verify the file actually exists at the new location
    assert!(
        project_path.join("renamed.txt").exists(),
        "renamed.txt should exist"
    );
    assert!(!test_file.exists(), "original.txt should no longer exist");

    println!("✅ Git mv integration test passed - file renamed using git");
}

#[tokio::test]
async fn test_rename_file_falls_back_without_git() {
    // Create a temporary directory WITHOUT git
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Create a codebuddy.toml with git enabled (but no git repo exists)
    let config_content = r#"
[server]
host = "127.0.0.1"
port = 3000
timeoutMs = 30000

[lsp]
defaultTimeoutMs = 10000
enablePreload = false

[[lsp.servers]]
extensions = ["txt"]
command = ["cat"]

[logging]
level = "info"
format = "pretty"

[cache]
enabled = false

[git]
enabled = true
require = false
operations = ["mv", "rm"]
"#;
    std::fs::write(project_path.join("codebuddy.toml"), config_content)
        .expect("Failed to write codebuddy.toml");

    // Create a test file
    let test_file = project_path.join("original.txt");
    std::fs::write(&test_file, "Hello, World!").expect("Failed to create test file");

    // Create a TestClient for the MCP server
    let mut client = TestClient::new(project_path);

    // Call the rename_file tool
    let rename_request = json!({
        "jsonrpc": "2.0",
        "id": "fallback-test-1",
        "method": "tools/call",
        "params": {
            "name": "rename_file",
            "arguments": {
                "old_path": project_path.join("original.txt").to_str().unwrap(),
                "new_path": project_path.join("renamed.txt").to_str().unwrap()
            }
        }
    });

    let response = client
        .send_request(rename_request)
        .expect("rename_file should work");

    assert_eq!(response["id"], "fallback-test-1");
    assert!(
        response["error"].is_null(),
        "rename_file should not error even without git: {:?}",
        response["error"]
    );

    // Verify the file was renamed using filesystem operations
    assert!(
        project_path.join("renamed.txt").exists(),
        "renamed.txt should exist"
    );
    assert!(!test_file.exists(), "original.txt should no longer exist");

    println!("✅ Fallback test passed - file renamed without git");
}

#[tokio::test]
async fn test_git_disabled_in_config() {
    // Create a temporary directory with git
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_path = temp_dir.path();

    // Initialize git repo
    Command::new("git")
        .args(&["init"])
        .current_dir(project_path)
        .status()
        .expect("Failed to run git init");

    Command::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .current_dir(project_path)
        .status()
        .ok();

    Command::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(project_path)
        .status()
        .ok();

    // Create config with git DISABLED
    let config_content = r#"
[server]
host = "127.0.0.1"
port = 3000
timeoutMs = 30000

[lsp]
defaultTimeoutMs = 10000
enablePreload = false

[[lsp.servers]]
extensions = ["txt"]
command = ["cat"]

[logging]
level = "info"
format = "pretty"

[cache]
enabled = false

[git]
enabled = false
require = false
operations = ["mv", "rm"]
"#;
    std::fs::write(project_path.join("codebuddy.toml"), config_content)
        .expect("Failed to write codebuddy.toml");

    // Create and commit a test file
    let test_file = project_path.join("original.txt");
    std::fs::write(&test_file, "Hello, World!").expect("Failed to create test file");

    Command::new("git")
        .args(&["add", "original.txt"])
        .current_dir(project_path)
        .status()
        .ok();

    Command::new("git")
        .args(&["commit", "-m", "Add original.txt"])
        .current_dir(project_path)
        .status()
        .ok();

    // Create a TestClient
    let mut client = TestClient::new(project_path);

    // Rename the file
    let rename_request = json!({
        "jsonrpc": "2.0",
        "id": "disabled-test-1",
        "method": "tools/call",
        "params": {
            "name": "rename_file",
            "arguments": {
                "old_path": project_path.join("original.txt").to_str().unwrap(),
                "new_path": project_path.join("renamed.txt").to_str().unwrap()
            }
        }
    });

    let _response = client
        .send_request(rename_request)
        .expect("rename_file should work");

    // Check git status - should show as untracked + deleted (not renamed)
    let status_output = Command::new("git")
        .args(&["status", "--short"])
        .current_dir(project_path)
        .output()
        .expect("Failed to run git status");

    let status_str = String::from_utf8_lossy(&status_output.stdout);
    println!("Git status with disabled config: {}", status_str);

    // When git is disabled, the file should be shown as deleted and untracked
    // (not renamed with R), because we used filesystem operations instead of git mv
    assert!(
        status_str.contains("?? renamed.txt") || status_str.contains("D  original.txt"),
        "Git status should show filesystem operation (not git mv) when git is disabled, got: {}",
        status_str
    );

    println!("✅ Git disabled test passed - filesystem operations used instead of git");
}
