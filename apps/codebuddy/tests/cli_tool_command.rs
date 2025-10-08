//! Integration tests for the `codebuddy tool` CLI subcommand

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test binary command with helpful error messaging
fn codebuddy_cmd() -> Command {
    let binary_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("target/debug/codebuddy");

    if !binary_path.exists() {
        panic!(
            "\n\n❌ codebuddy binary not found at: {}\n\
            \n\
            ℹ️  To run CLI integration tests, build the binary first:\n\
            \n\
                cargo build\n\
            \n\
            Then re-run the tests:\n\
            \n\
                cargo test --test cli_tool_command\n\n",
            binary_path.display()
        );
    }

    Command::cargo_bin("codebuddy").expect("codebuddy binary should exist after check above")
}

#[test]
fn test_tool_health_check_success() {
    let mut cmd = codebuddy_cmd();
    cmd.args(["tool", "health_check", "{}"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("healthy"));
}

#[test]
fn test_tool_health_check_pretty_format() {
    let mut cmd = codebuddy_cmd();
    cmd.args(["tool", "health_check", "{}", "--format", "pretty"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("{\n")) // Pretty format has newlines
        .stdout(predicate::str::contains("status"));
}

#[test]
fn test_tool_health_check_compact_format() {
    let mut cmd = codebuddy_cmd();
    cmd.args(["tool", "health_check", "{}", "--format", "compact"]);

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Compact format should not have unnecessary whitespace
    assert!(!stdout.contains("{\n"));
    assert!(stdout.contains("status"));
}

#[test]
fn test_tool_invalid_json_arguments() {
    let mut cmd = codebuddy_cmd();
    cmd.args(["tool", "health_check", "not-valid-json"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid JSON"));
}

#[test]
fn test_tool_invalid_file_path() {
    let mut cmd = codebuddy_cmd();
    cmd.args([
        "tool",
        "read_file",
        r#"{"file_path": "/nonexistent/file/path.txt"}"#,
    ]);

    cmd.assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("File does not exist"));
}

#[test]
fn test_tool_list_files_success() {
    // Create a temporary directory with some files
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").unwrap();

    let mut cmd = codebuddy_cmd();
    cmd.current_dir(temp_dir.path());
    cmd.args(["tool", "list_files", r#"{"path": "."}"#]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("test.txt"));
}

#[test]
fn test_tool_read_file_success() {
    // Create a temporary file
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "Hello, World!").unwrap();

    let mut cmd = codebuddy_cmd();
    cmd.current_dir(temp_dir.path());
    cmd.args(["tool", "read_file", r#"{"file_path": "test.txt"}"#]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Hello, World!"));
}

#[test]
fn test_tool_create_file_dry_run() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = codebuddy_cmd();
    cmd.current_dir(temp_dir.path());
    cmd.args([
        "tool",
        "create_file",
        r#"{"file_path": "new_file.txt", "content": "test", "dry_run": true}"#,
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("preview"));

    // File should not actually be created
    assert!(!temp_dir.path().join("new_file.txt").exists());
}

#[test]
fn test_tool_create_and_read_file() {
    let temp_dir = TempDir::new().unwrap();

    // Create file
    let mut cmd = codebuddy_cmd();
    cmd.current_dir(temp_dir.path());
    cmd.args([
        "tool",
        "create_file",
        r#"{"file_path": "created.txt", "content": "Created content"}"#,
    ]);

    cmd.assert().success();

    // Verify file was created
    assert!(temp_dir.path().join("created.txt").exists());

    // Read file back
    let mut cmd = codebuddy_cmd();
    cmd.current_dir(temp_dir.path());
    cmd.args(["tool", "read_file", r#"{"file_path": "created.txt"}"#]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Created content"));
}

#[test]
fn test_tool_unknown_tool_name() {
    let mut cmd = codebuddy_cmd();
    cmd.args(["tool", "nonexistent_tool", "{}"]);

    cmd.assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("No handler for tool"));
}

#[test]
fn test_tool_missing_required_arguments() {
    let mut cmd = codebuddy_cmd();
    cmd.args([
        "tool",
        "read_file",
        "{}", // Missing required file_path argument
    ]);

    cmd.assert().failure().code(1);
}

#[test]
fn test_tool_output_is_valid_json() {
    let mut cmd = codebuddy_cmd();
    cmd.args(["tool", "health_check", "{}", "--format", "compact"]);

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Verify output is valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(parsed.is_ok(), "Output should be valid JSON");
}

#[test]
fn test_tool_error_output_is_valid_json() {
    let mut cmd = codebuddy_cmd();
    cmd.args(["tool", "read_file", r#"{"file_path": "/nonexistent.txt"}"#]);

    let output = cmd.assert().failure();
    let stderr = String::from_utf8_lossy(&output.get_output().stderr);

    // Verify error output is valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stderr);
    assert!(parsed.is_ok(), "Error output should be valid JSON");
}
