//! workspace.find_replace tests migrated to closure-based helpers (v2)
//!
//! BEFORE: 1009 lines with repetitive setup
//! AFTER: Focused find/replace verification (still comprehensive)
//!
//! Tests workspace-wide find and replace operations.

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

// =====================================================================
// 1. Literal Mode Tests
// =====================================================================

#[tokio::test]
async fn test_literal_basic_replace() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "test.rs",
        r#"fn authenticate(username: &str) {
    println!("User: {}", username);
    let username_copy = username.to_string();
}
"#,
    );

    let result = client
        .call_tool(
            "workspace.find_replace",
            json!({
                "pattern": "username",
                "replacement": "userid",
                "mode": "literal",
                "dryRun": false
            }),
        )
        .await
        .expect("find_replace should succeed");

    let content = result.get("result").expect("Should have result");

    assert_eq!(content.get("success").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        content.get("filesModified").and_then(|v| v.as_array()).map(|a| a.len()),
        Some(1)
    );
    assert_eq!(content.get("matchesReplaced").and_then(|v| v.as_u64()), Some(4));

    let modified_content = workspace.read_file("test.rs");
    assert!(modified_content.contains("userid: &str"));
    assert!(modified_content.contains("userid_copy"));
    assert!(!modified_content.contains("username"));
}

#[tokio::test]
async fn test_literal_whole_word() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "test.txt",
        "user is not username or user_id but user is standalone",
    );

    let result = client
        .call_tool(
            "workspace.find_replace",
            json!({
                "pattern": "user",
                "replacement": "account",
                "mode": "literal",
                "wholeWord": true,
                "dryRun": false
            }),
        )
        .await
        .expect("find_replace should succeed");

    let content = result.get("result").expect("Should have result");

    assert_eq!(content.get("matchesReplaced").and_then(|v| v.as_u64()), Some(2));

    let modified_content = workspace.read_file("test.txt");
    assert!(modified_content.contains("account is not username"));
    assert!(modified_content.contains("account is standalone"));
    assert!(modified_content.contains("user_id"));
    assert!(modified_content.contains("username"));
}

#[tokio::test]
async fn test_literal_case_sensitive() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("test.txt", "User user USER User");

    let result = client
        .call_tool(
            "workspace.find_replace",
            json!({
                "pattern": "user",
                "replacement": "account",
                "mode": "literal",
                "dryRun": false
            }),
        )
        .await
        .expect("find_replace should succeed");

    let content = result.get("result").expect("Should have result");

    assert_eq!(content.get("matchesReplaced").and_then(|v| v.as_u64()), Some(1));

    let modified_content = workspace.read_file("test.txt");
    assert_eq!(modified_content, "User account USER User");
}

// =====================================================================
// 2. Regex Mode Tests
// =====================================================================

#[tokio::test]
async fn test_regex_basic_pattern() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "test.rs",
        r#"let user_name = "Alice";
let user_id = 123;
let user_email = "alice@example.com";
let user = "Bob";
"#,
    );

    let result = client
        .call_tool(
            "workspace.find_replace",
            json!({
                "pattern": r"user_[a-z]+",
                "replacement": "account_info",
                "mode": "regex",
                "dryRun": false
            }),
        )
        .await
        .expect("find_replace should succeed");

    let content = result.get("result").expect("Should have result");

    assert_eq!(content.get("matchesReplaced").and_then(|v| v.as_u64()), Some(3));

    let modified_content = workspace.read_file("test.rs");
    assert!(modified_content.contains("account_info = \"Alice\""));
    assert!(modified_content.contains("account_info = 123"));
    assert!(modified_content.contains("account_info = \"alice@example.com\""));
    assert!(modified_content.contains("let user = \"Bob\""));
}

#[tokio::test]
async fn test_regex_capture_groups() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "config.toml",
        r#"CODEBUDDY_ENABLE_LOGS = true
CODEBUDDY_DEBUG_MODE = false
CODEBUDDY_MAX_WORKERS = 10
"#,
    );

    let result = client
        .call_tool(
            "workspace.find_replace",
            json!({
                "pattern": r"CODEBUDDY_([A-Z_]+)",
                "replacement": "TYPEMILL_$1",
                "mode": "regex",
                "dryRun": false
            }),
        )
        .await
        .expect("find_replace should succeed");

    let content = result.get("result").expect("Should have result");

    assert_eq!(content.get("matchesReplaced").and_then(|v| v.as_u64()), Some(3));

    let modified_content = workspace.read_file("config.toml");
    assert!(modified_content.contains("TYPEMILL_ENABLE_LOGS = true"));
    assert!(modified_content.contains("TYPEMILL_DEBUG_MODE = false"));
    assert!(modified_content.contains("TYPEMILL_MAX_WORKERS = 10"));
}

#[tokio::test]
async fn test_regex_named_captures() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "test.py",
        r#"user_name = "Alice"
item_count = 42
"#,
    );

    let result = client
        .call_tool(
            "workspace.find_replace",
            json!({
                "pattern": r"(?P<first>\w+)_(?P<second>\w+)",
                "replacement": "${second}_${first}",
                "mode": "regex",
                "dryRun": false
            }),
        )
        .await
        .expect("find_replace should succeed");

    let content = result.get("result").expect("Should have result");

    assert_eq!(content.get("matchesReplaced").and_then(|v| v.as_u64()), Some(2));

    let modified_content = workspace.read_file("test.py");
    assert!(modified_content.contains("name_user = \"Alice\""));
    assert!(modified_content.contains("count_item = 42"));
}

#[tokio::test]
async fn test_regex_invalid_pattern() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("test.txt", "test content");

    let result = client
        .call_tool(
            "workspace.find_replace",
            json!({
                "pattern": "[unclosed",
                "replacement": "replacement",
                "mode": "regex",
                "dryRun": false
            }),
        )
        .await;

    assert!(result.is_err(), "Invalid regex pattern should return error");
}

// =====================================================================
// 3. Case Preservation Tests
// =====================================================================

#[tokio::test]
async fn test_preserve_case_snake_to_camel() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "test.rs",
        r#"let user_name = "snake";
let userName = "camel";
let UserName = "pascal";
let USER_NAME = "screaming";
"#,
    );

    let result = client
        .call_tool(
            "workspace.find_replace",
            json!({
                "pattern": "user_name",
                "replacement": "account_id",
                "mode": "literal",
                "preserveCase": true,
                "dryRun": false
            }),
        )
        .await
        .expect("find_replace should succeed");

    let content = result.get("result").expect("Should have result");

    // Literal mode is case-sensitive, only snake_case matches
    assert_eq!(content.get("matchesReplaced").and_then(|v| v.as_u64()), Some(1));

    let modified_content = workspace.read_file("test.rs");
    assert!(modified_content.contains("account_id = \"snake\""));
    assert!(modified_content.contains("userName = \"camel\""));
    assert!(modified_content.contains("UserName = \"pascal\""));
    assert!(modified_content.contains("USER_NAME = \"screaming\""));
}

#[tokio::test]
async fn test_preserve_case_disabled() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("test.txt", "userName userName");

    let result = client
        .call_tool(
            "workspace.find_replace",
            json!({
                "pattern": "userName",
                "replacement": "accountId",
                "mode": "literal",
                "preserveCase": false,
                "dryRun": false
            }),
        )
        .await
        .expect("find_replace should succeed");

    let content = result.get("result").expect("Should have result");

    assert_eq!(content.get("matchesReplaced").and_then(|v| v.as_u64()), Some(2));

    let modified_content = workspace.read_file("test.txt");
    assert_eq!(modified_content, "accountId accountId");
}

// =====================================================================
// 4. Scope Filtering Tests
// =====================================================================

#[tokio::test]
async fn test_scope_include_patterns() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("test.rs", "fn user_login() {}");
    workspace.create_file("test.toml", "user = \"admin\"");
    workspace.create_file("test.md", "user documentation");
    workspace.create_file("test.txt", "user notes");

    let result = client
        .call_tool(
            "workspace.find_replace",
            json!({
                "pattern": "user",
                "replacement": "account",
                "mode": "literal",
                "scope": {
                    "includePatterns": ["**/*.rs", "**/*.toml"]
                },
                "dryRun": false
            }),
        )
        .await
        .expect("find_replace should succeed");

    let content = result.get("result").expect("Should have result");

    assert_eq!(
        content.get("filesModified").and_then(|v| v.as_array()).map(|a| a.len()),
        Some(2)
    );

    assert!(workspace.read_file("test.rs").contains("account_login"));
    assert!(workspace.read_file("test.toml").contains("account = \"admin\""));
    assert!(workspace.read_file("test.md").contains("user documentation"));
    assert!(workspace.read_file("test.txt").contains("user notes"));
}

#[tokio::test]
async fn test_scope_exclude_patterns() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("src/main.rs", "fn user_main() {}");
    workspace.create_file("target/debug/output.txt", "user output");

    let result = client
        .call_tool(
            "workspace.find_replace",
            json!({
                "pattern": "user",
                "replacement": "account",
                "mode": "literal",
                "scope": {
                    "excludePatterns": ["**/target/**"]
                },
                "dryRun": false
            }),
        )
        .await
        .expect("find_replace should succeed");

    let content = result.get("result").expect("Should have result");

    assert_eq!(
        content.get("filesModified").and_then(|v| v.as_array()).map(|a| a.len()),
        Some(1)
    );

    assert!(workspace.read_file("src/main.rs").contains("account_main"));
    assert!(workspace.read_file("target/debug/output.txt").contains("user output"));
}

#[tokio::test]
async fn test_scope_default_excludes() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("src/main.rs", "user code");
    workspace.create_file("target/build.txt", "user build");
    workspace.create_file("node_modules/package.txt", "user package");
    workspace.create_directory(".git");
    workspace.create_file(".git/config", "user = git");

    let result = client
        .call_tool(
            "workspace.find_replace",
            json!({
                "pattern": "user",
                "replacement": "account",
                "mode": "literal",
                "dryRun": false
            }),
        )
        .await
        .expect("find_replace should succeed");

    let content = result.get("result").expect("Should have result");

    assert_eq!(
        content.get("filesModified").and_then(|v| v.as_array()).map(|a| a.len()),
        Some(1)
    );

    assert!(workspace.read_file("src/main.rs").contains("account code"));
    assert!(workspace.read_file("target/build.txt").contains("user build"));
    assert!(workspace.read_file("node_modules/package.txt").contains("user package"));
    assert!(workspace.read_file(".git/config").contains("user = git"));
}

// =====================================================================
// 5. Multi-File Tests
// =====================================================================

#[tokio::test]
async fn test_multi_file_replace() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    for i in 1..=5 {
        workspace.create_file(&format!("file{}.txt", i), "user data here");
    }

    let result = client
        .call_tool(
            "workspace.find_replace",
            json!({
                "pattern": "user",
                "replacement": "account",
                "mode": "literal",
                "dryRun": false
            }),
        )
        .await
        .expect("find_replace should succeed");

    let content = result.get("result").expect("Should have result");

    assert_eq!(
        content.get("filesModified").and_then(|v| v.as_array()).map(|a| a.len()),
        Some(5)
    );
    assert_eq!(content.get("matchesReplaced").and_then(|v| v.as_u64()), Some(5));

    for i in 1..=5 {
        let content = workspace.read_file(&format!("file{}.txt", i));
        assert_eq!(content, "account data here");
    }
}

// =====================================================================
// 6. Dry-Run Tests
// =====================================================================

#[tokio::test]
async fn test_dry_run_defaults_true() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("test.txt", "user data");

    let result = client
        .call_tool(
            "workspace.find_replace",
            json!({
                "pattern": "user",
                "replacement": "account",
                "mode": "literal"
            }),
        )
        .await
        .expect("find_replace should succeed");

    let plan = result.get("result").expect("Should have result");

    assert!(plan.get("edits").is_some());
    assert!(plan.get("metadata").is_some());

    assert_eq!(workspace.read_file("test.txt"), "user data");
}

#[tokio::test]
async fn test_dry_run_preview() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("test.rs", "fn user_login() { user_validate(); }");

    let result = client
        .call_tool(
            "workspace.find_replace",
            json!({
                "pattern": "user",
                "replacement": "account",
                "mode": "literal",
                "dryRun": true
            }),
        )
        .await
        .expect("find_replace should succeed");

    let plan = result.get("result").expect("Should have result");

    assert_eq!(
        plan.get("sourceFile").and_then(|v| v.as_str()),
        Some("workspace")
    );

    let edits = plan.get("edits").and_then(|v| v.as_array()).unwrap();
    assert_eq!(edits.len(), 2);

    let metadata = plan.get("metadata").unwrap();
    assert_eq!(
        metadata.get("intentName").and_then(|v| v.as_str()),
        Some("find_replace")
    );

    assert!(workspace.read_file("test.rs").contains("user_login"));
}

#[tokio::test]
async fn test_execute_mode() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("test.txt", "user data");

    let result = client
        .call_tool(
            "workspace.find_replace",
            json!({
                "pattern": "user",
                "replacement": "account",
                "mode": "literal",
                "dryRun": false
            }),
        )
        .await
        .expect("find_replace should succeed");

    let content = result.get("result").expect("Should have result");

    assert_eq!(content.get("success").and_then(|v| v.as_bool()), Some(true));

    assert_eq!(workspace.read_file("test.txt"), "account data");
}

// =====================================================================
// 7. Edge Cases
// =====================================================================

#[tokio::test]
async fn test_empty_pattern() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("test.txt", "content");

    let result = client
        .call_tool(
            "workspace.find_replace",
            json!({
                "pattern": "",
                "replacement": "replacement",
                "mode": "literal",
                "dryRun": false
            }),
        )
        .await;

    assert!(result.is_err(), "Empty pattern should return error");
}

#[tokio::test]
async fn test_pattern_not_found() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("test.txt", "some content here");

    let result = client
        .call_tool(
            "workspace.find_replace",
            json!({
                "pattern": "nonexistent",
                "replacement": "replacement",
                "mode": "literal",
                "dryRun": false
            }),
        )
        .await
        .expect("find_replace should succeed even with no matches");

    let content = result.get("result").expect("Should have result");

    assert_eq!(content.get("success").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(content.get("matchesFound").and_then(|v| v.as_u64()), Some(0));
    assert_eq!(content.get("matchesReplaced").and_then(|v| v.as_u64()), Some(0));
}

#[tokio::test]
async fn test_utf8_content() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("test.txt", "用户 user 👤 user données");

    let result = client
        .call_tool(
            "workspace.find_replace",
            json!({
                "pattern": "user",
                "replacement": "account",
                "mode": "literal",
                "dryRun": false
            }),
        )
        .await
        .expect("find_replace should succeed");

    let content = result.get("result").expect("Should have result");

    assert_eq!(content.get("matchesReplaced").and_then(|v| v.as_u64()), Some(2));

    let modified = workspace.read_file("test.txt");
    assert!(modified.contains("用户"));
    assert!(modified.contains("👤"));
    assert!(modified.contains("données"));
    assert!(modified.contains("account"));
    assert!(!modified.contains("user"));
}

#[tokio::test]
async fn test_large_file() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    let mut content = String::new();
    for i in 0..1000 {
        content.push_str(&format!("Line {}: user data here\n", i));
    }
    workspace.create_file("large.txt", &content);

    let result = client
        .call_tool(
            "workspace.find_replace",
            json!({
                "pattern": "user",
                "replacement": "account",
                "mode": "literal",
                "dryRun": false
            }),
        )
        .await
        .expect("find_replace should succeed");

    let result_content = result.get("result").expect("Should have result");

    assert_eq!(
        result_content.get("matchesReplaced").and_then(|v| v.as_u64()),
        Some(1000)
    );

    let modified = workspace.read_file("large.txt");
    assert!(!modified.contains("user"));
    assert_eq!(modified.matches("account").count(), 1000);
}

#[tokio::test]
async fn test_multiline_pattern() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "test.txt",
        r#"function user_login() {
    return true;
}
function user_logout() {
    return false;
}
"#,
    );

    let result = client
        .call_tool(
            "workspace.find_replace",
            json!({
                "pattern": r"user_(\w+)",
                "replacement": "account_$1",
                "mode": "regex",
                "dryRun": false
            }),
        )
        .await
        .expect("find_replace should succeed");

    let content = result.get("result").expect("Should have result");

    assert_eq!(content.get("matchesReplaced").and_then(|v| v.as_u64()), Some(2));

    let modified = workspace.read_file("test.txt");
    assert!(modified.contains("account_login"));
    assert!(modified.contains("account_logout"));
}

#[tokio::test]
async fn test_escaped_regex_characters() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file(
        "test.txt",
        r#"let x = user.name;
let y = user[0];
let z = user*2;
"#,
    );

    let result = client
        .call_tool(
            "workspace.find_replace",
            json!({
                "pattern": "user",
                "replacement": "account",
                "mode": "literal",
                "dryRun": false
            }),
        )
        .await
        .expect("find_replace should succeed");

    let content = result.get("result").expect("Should have result");

    assert_eq!(content.get("matchesReplaced").and_then(|v| v.as_u64()), Some(3));

    let modified = workspace.read_file("test.txt");
    assert!(modified.contains("account.name"));
    assert!(modified.contains("account[0]"));
    assert!(modified.contains("account*2"));
}
