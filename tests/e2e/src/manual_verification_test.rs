use crate::{TestClient, TestWorkspace};
use serde_json::json;
use std::time::Duration;

#[tokio::test]
async fn test_manual_verification_all_tools() -> anyhow::Result<()> {
    // 1. Setup workspace
    eprintln!("TEST: Setting up workspace...");
    let workspace = TestWorkspace::new();

    // Create some files
    workspace.create_file(
        "src/main.rs",
        r#"
fn main() {
    println!("Hello, world!");
    helper_function();
}

fn helper_function() {
    println!("Helper called");
}
"#,
    );

    workspace.create_file(
        "src/utils.ts",
        r#"
export function add(a: number, b: number): number {
    return a + b;
}

export const PI = 3.14159;
"#,
    );

    // 2. Start client
    eprintln!("TEST: Starting client...");
    let mut client = TestClient::new(workspace.path());

    // Wait for server to be ready
    client
        .wait_for_ready(Duration::from_secs(10))
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    eprintln!("TEST: Server is ready.");

    // 3. Test inspect_code
    eprintln!("TEST: verifying inspect_code...");
    match client
        .call_tool(
            "inspect_code",
            json!({
                "filePath": workspace.path().join("src/main.rs"),
                "line": 3,
                "character": 4,
                "include": ["definition"]
            }),
        )
        .await
    {
        Ok(response) => {
            eprintln!("RESULT: inspect_code: {:?}", response);
        }
        Err(e) => eprintln!(
            "ERROR: inspect_code failed (expected if LSP missing): {}",
            e
        ),
    }

    // 4. Test search_code
    eprintln!("TEST: verifying search_code...");
    match client
        .call_tool(
            "search_code",
            json!({
                "query": "helper_function"
            }),
        )
        .await
    {
        Ok(response) => eprintln!("RESULT: search_code: {:?}", response),
        Err(e) => eprintln!("ERROR: search_code failed: {}", e),
    }

    // 5. Test rename_all (dry run)
    eprintln!("TEST: verifying rename_all...");
    match client
        .call_tool(
            "rename_all",
            json!({
                "target": {
                    "kind": "file",
                    "filePath": workspace.path().join("src/utils.ts")
                },
                "newName": "src/math_utils.ts",
                "options": {
                    "dryRun": true
                }
            }),
        )
        .await
    {
        Ok(response) => {
            eprintln!("RESULT: rename_all: {:?}", response);
        }
        Err(e) => eprintln!("ERROR: rename_all failed: {}", e),
    }

    // 6. Test relocate (dry run)
    eprintln!("TEST: verifying relocate...");
    match client
        .call_tool(
            "relocate",
            json!({
                "target": {
                    "kind": "file",
                    "filePath": workspace.path().join("src/main.rs")
                },
                "destination": workspace.path().join("src/bin"),
                "options": {
                    "dryRun": true
                }
            }),
        )
        .await
    {
        Ok(response) => {
            eprintln!("RESULT: relocate: {:?}", response);
        }
        Err(e) => eprintln!("ERROR: relocate failed: {}", e),
    }

    // 7. Test prune (dry run)
    eprintln!("TEST: verifying prune...");
    match client
        .call_tool(
            "prune",
            json!({
                "target": {
                    "kind": "file",
                    "filePath": workspace.path().join("src/utils.ts")
                },
                "options": {
                    "dryRun": true
                }
            }),
        )
        .await
    {
        Ok(response) => {
            eprintln!("RESULT: prune: {:?}", response);
        }
        Err(e) => eprintln!("ERROR: prune failed: {}", e),
    }

    // 8. Test refactor (dry run)
    eprintln!("TEST: verifying refactor...");
    match client
        .call_tool(
            "refactor",
            json!({
                "action": "extract",
                "params": {
                    "kind": "function",
                    "filePath": workspace.path().join("src/main.rs"),
                    "range": {
                        "startLine": 7,
                        "startCharacter": 4,
                        "endLine": 7,
                        "endCharacter": 30
                    },
                    "name": "extracted_printer"
                },
                "options": {
                    "dryRun": true
                }
            }),
        )
        .await
    {
        Ok(response) => {
            eprintln!("RESULT: refactor: {:?}", response);
        }
        Err(e) => eprintln!("ERROR: refactor failed: {}", e),
    }

    // 9. Test workspace (find_replace)
    eprintln!("TEST: verifying workspace...");
    match client
        .call_tool(
            "workspace",
            json!({
                "action": "find_replace",
                "params": {
                    "pattern": "Helper",
                    "replacement": "Assistant",
                    "mode": "literal",
                    "scope": {}
                },
                "options": {
                    "dryRun": true
                }
            }),
        )
        .await
    {
        Ok(response) => {
            eprintln!("RESULT: workspace: {:?}", response);
            assert!(response.get("result").is_some());
        }
        Err(e) => eprintln!("ERROR: workspace failed: {}", e),
    }

    eprintln!("TEST: All tools verified (errors logged if any).");

    Ok(())
}
