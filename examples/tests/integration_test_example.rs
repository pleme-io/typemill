// Example: Integration Test Pattern
// Location: integration-tests/src/test_*.rs
// Purpose: Test tool handlers with mocked LSP servers

use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

#[tokio::test]
async fn test_rename_tool_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    workspace.create_file("test.ts", "function foo() {}");

    let response = client
        .call_tool(
            "rename.plan",
            json!({
                "symbol": "foo",
                "new_name": "bar",
                "file_path": workspace.absolute_path("test.ts")
            }),
        )
        .await
        .expect("rename.plan should succeed");

    assert!(response.get("result").is_some());
}
