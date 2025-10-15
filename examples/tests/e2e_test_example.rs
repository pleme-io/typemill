// Example: E2E Test Pattern
// Location: apps/codebuddy/tests/e2e_*.rs
// Purpose: Test complete workflows with real components

use cb_test_support::harness::{TestClient, TestWorkspace};
use serde_json::json;

#[tokio::test]
async fn test_refactoring_workflow_end_to_end() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Setup workspace with multiple files
    workspace.create_file("src/main.ts", "function foo() { bar(); }");
    workspace.create_file("src/helper.ts", "export function bar() {}");

    // Step 1: Plan the refactoring
    let plan = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "symbol",
                    "path": "src/main.ts",
                    "selector": {"position": {"line": 0, "character": 9}}
                },
                "new_name": "newFoo"
            }),
        )
        .await
        .unwrap();

    // Step 2: Apply the plan
    let result = client
        .call_tool(
            "workspace.apply_edit",
            json!({
                "plan": plan["result"],
                "options": {"dry_run": false}
            }),
        )
        .await
        .unwrap();

    // Step 3: Verify changes
    assert!(result.get("success").unwrap().as_bool().unwrap());

    // Step 4: Verify files were modified correctly
    let content = std::fs::read_to_string(workspace.path().join("src/main.ts")).unwrap();
    assert!(content.contains("newFoo"));
}
