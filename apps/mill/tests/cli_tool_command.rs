//! Integration tests for the `mill tool` CLI subcommand

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

/// Helper to create a test binary command with helpful error messaging
fn mill_cmd() -> Command {
    let binary_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target/debug/mill");

    if !binary_path.exists() {
        panic!(
            "\n\n❌ mill binary not found at: {}\n\
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

    Command::new(binary_path)
}

#[test]
fn test_tool_workspace_verify_project_success() {
    let mut cmd = mill_cmd();
    cmd.args(["tool", "workspace", r#"{"action":"verify_project"}"#]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("status"));
}

#[test]
fn test_tool_workspace_verify_project_pretty_format() {
    let mut cmd = mill_cmd();
    cmd.args(["tool", "workspace", r#"{"action":"verify_project"}"#, "--format", "pretty"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("{\n")) // Pretty format has newlines
        .stdout(predicate::str::contains("status"));
}

#[test]
fn test_tool_workspace_verify_project_compact_format() {
    let mut cmd = mill_cmd();
    cmd.args(["tool", "workspace", r#"{"action":"verify_project"}"#, "--format", "compact"]);

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Compact format should not have unnecessary whitespace
    assert!(!stdout.contains("{\n"));
    assert!(stdout.contains("status"));
}

#[test]
fn test_tool_invalid_json_arguments() {
    let mut cmd = mill_cmd();
    cmd.args(["tool", "workspace", "not-valid-json"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid JSON"));
}

#[test]
fn test_tool_unknown_tool_name() {
    let mut cmd = mill_cmd();
    cmd.args(["tool", "nonexistent_tool", "{}"]);

    cmd.assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Unknown tool"));
}

#[test]
fn test_tool_output_is_valid_json() {
    let mut cmd = mill_cmd();
    cmd.args(["tool", "workspace", r#"{"action":"verify_project"}"#, "--format", "compact"]);

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Verify output is valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(parsed.is_ok(), "Output should be valid JSON");
}
