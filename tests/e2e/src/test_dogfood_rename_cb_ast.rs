use cb_test_support::{TestClient, TestWorkspace};
use serde_json::json;

#[tokio::test]
async fn test_rename_cb_ast_to_codebuddy_ast() {
    let workspace = TestWorkspace::at_path("/workspace");
    let mut client = TestClient::new(workspace.path());

    // Generate rename plan
    let plan_result = client
        .call_tool(
            "rename.plan",
            json!({
                "target": {
                    "kind": "directory",
                    "path": "/workspace/crates/cb-ast"
                },
                "new_name": "/workspace/crates/codebuddy-ast"
            }),
        )
        .await
        .expect("rename.plan should succeed");

    println!("=== RENAME PLAN ===");
    println!("{}", serde_json::to_string_pretty(&plan_result).unwrap());

    let plan = plan_result
        .get("result")
        .and_then(|r| r.get("content"))
        .expect("Plan should exist");

    // Save plan to file for review
    std::fs::write(
        "/tmp/cb_ast_rename_plan.json",
        serde_json::to_string_pretty(&plan).unwrap()
    ).unwrap();

    println!("\n✅ Plan saved to /tmp/cb_ast_rename_plan.json");
    println!("\nPlan summary:");
    if let Some(summary) = plan.get("summary") {
        println!("{}", serde_json::to_string_pretty(summary).unwrap());
    }
}
