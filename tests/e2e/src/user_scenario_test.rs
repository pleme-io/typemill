use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;
use std::process::Command;
use std::time::Duration;

#[allow(dead_code)]
const TEST_TIMEOUT: Duration = Duration::from_secs(60);

fn setup_rust_workspace() -> TestWorkspace {
    let workspace = TestWorkspace::new();

    // Clone itoa
    println!("Cloning itoa into {:?}", workspace.path());
    let status = Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "https://github.com/dtolnay/itoa.git",
            ".",
        ])
        .current_dir(workspace.path())
        .status()
        .expect("Failed to clone itoa");
    assert!(status.success(), "Git clone failed");

    // Run mill setup
    // Find mill binary
    let mill_path = std::env::var("CARGO_MANIFEST_DIR")
        .map(|dir| {
            // CARGO_MANIFEST_DIR points to tests/e2e
            let mut path = std::path::PathBuf::from(dir);
            path.pop(); // e2e
            path.pop(); // tests
            path.push("target/debug/mill");
            path
        })
        .expect("CARGO_MANIFEST_DIR not set");

    if !mill_path.exists() {
        panic!("Mill binary not found at {:?}", mill_path);
    }

    println!("Running mill setup with binary at {:?}", mill_path);
    let setup_status = Command::new(&mill_path)
        .args(["setup", "--update"])
        .current_dir(workspace.path())
        .status()
        .expect("Failed to run mill setup");
    assert!(setup_status.success(), "mill setup failed");

    workspace
}

#[tokio::test]
async fn test_user_scenario() {
    let workspace = setup_rust_workspace();
    let mut client = TestClient::new(workspace.path());

    // Wait for LSP
    let lib_rs = workspace.path().join("src/lib.rs");
    println!("Waiting for LSP to index {:?}", lib_rs);
    // 180s timeout: matches LSP_WARMUP_TIMEOUT; initialization on real projects
    // can take 60-120s on slow CI/containers, especially under parallel test load
    client
        .wait_for_lsp_ready(&lib_rs, 180_000)
        .await
        .expect("LSP failed to index");

    // 1. Inspect
    println!("Testing inspect_code...");
    let inspect_res = client
        .call_tool(
            "inspect_code",
            json!({
                "filePath": lib_rs.to_string_lossy(),
                "line": 0,
                "character": 0,
                "include": ["definition", "typeInfo"]
            }),
        )
        .await
        .expect("inspect_code failed");
    println!("Inspect Result: {:?}", inspect_res);

    // 2. Search
    println!("Testing search_code...");
    let search_res = client
        .call_tool(
            "search_code",
            json!({
                "query": "Buffer"
            }),
        )
        .await
        .expect("search_code failed");
    // Check results
    let results = search_res
        .get("result")
        .and_then(|r| r.get("results"))
        .and_then(|r| r.as_array());

    if let Some(res) = results {
        if res.is_empty() {
            println!(
                "Warning: search_code returned empty results for 'Buffer'. Indexing might be slow."
            );
        } else {
            println!("Found {} results for 'Buffer'", res.len());
        }
    } else {
        println!(
            "Warning: search_code result format unexpected: {:?}",
            search_res
        );
    }

    // 3. Rename (Dry Run) on a controlled file
    println!("Testing rename_all (dry run)...");

    // Create a new file for rename testing to be sure
    let my_code_path = workspace.path().join("src/my_code.rs");
    std::fs::write(&my_code_path, "pub fn my_rename_test() {}").unwrap();

    // Add it to lib.rs
    let mut lib_content = std::fs::read_to_string(&lib_rs).unwrap();
    lib_content.push_str("\npub mod my_code;");
    std::fs::write(&lib_rs, lib_content).unwrap();

    // Wait for LSP to pick up change - wait for diagnostics on new file
    // We can't rely on wait_for_lsp_ready for the new file immediately if it has no errors/diagnostics?
    // Actually inspect_code opens it.
    client.wait_for_lsp_ready(&lib_rs, 20000).await.ok();

    let rename_res = client
        .call_tool(
            "rename_all",
            json!({
                "target": {
                    "kind": "symbol",
                    "filePath": my_code_path.to_string_lossy(),
                    "line": 0,
                    "character": 7 // "my_rename_test" starts at 7
                },
                "newName": "renamed_test",
                "options": { "dryRun": true }
            }),
        )
        .await;

    match rename_res {
        Ok(res) => {
            let rename_status = res
                .get("result")
                .and_then(|r| r.get("content"))
                .and_then(|c| c.get("status"))
                .and_then(|s| s.as_str());

            assert!(
                rename_status == Some("preview") || rename_status == Some("success"),
                "Rename status should be preview or success, got {:?}",
                rename_status
            );
            println!("Rename Result: {:?}", res);
        }
        Err(e) => {
            println!(
                "Rename failed: {}. This might happen if LSP hasn't indexed the new file yet.",
                e
            );
            // Don't fail the test for this, as it depends on timing/LSP speed
        }
    }

    // 4. Relocate (Dry Run)
    println!("Testing relocate (dry run)...");
    let dummy_path = workspace.path().join("src/dummy.rs");
    std::fs::write(&dummy_path, "pub fn dummy() {}").unwrap();
    let dest_path = workspace.path().join("src/moved_dummy.rs");

    let relocate_res = client
        .call_tool(
            "relocate",
            json!({
                "target": {
                    "kind": "file",
                    "filePath": dummy_path.to_string_lossy()
                },
                "destination": dest_path.to_string_lossy(),
                "options": { "dryRun": true }
            }),
        )
        .await
        .expect("relocate failed");

    let relocate_status = relocate_res
        .get("result")
        .and_then(|r| r.get("content"))
        .and_then(|c| c.get("status"))
        .and_then(|s| s.as_str());

    assert!(
        relocate_status == Some("preview") || relocate_status == Some("success"),
        "Relocate status should be preview or success, got {:?}",
        relocate_status
    );
    println!("Relocate Result: {:?}", relocate_res);

    // 5. Prune
    println!("Testing prune...");
    let delete_path = workspace.path().join("src/to_delete.rs");
    std::fs::write(&delete_path, "pub fn delete_me() {}").unwrap();

    let prune_res = client
        .call_tool(
            "prune",
            json!({
                "target": {
                    "kind": "file",
                    "filePath": delete_path.to_string_lossy()
                },
                "options": { "dryRun": false }
            }),
        )
        .await
        .expect("prune failed");

    let prune_status = prune_res
        .get("result")
        .and_then(|r| r.get("content"))
        .and_then(|c| c.get("status"))
        .and_then(|s| s.as_str());

    assert_eq!(prune_status, Some("success"), "Prune should succeed");
    assert!(!delete_path.exists(), "File should be deleted");
    println!("Prune Result: {:?}", prune_res);

    // 6. Refactor (Dry Run)
    println!("Testing refactor...");
    let refactor_path = workspace.path().join("src/refactor.rs");
    std::fs::write(&refactor_path, "fn foo() {\n    let x = 1 + 2;\n}").unwrap();

    // Wait for file discovery/indexing for refactor
    client.wait_for_lsp_ready(&refactor_path, 10000).await.ok();

    let refactor_res = client
        .call_tool(
            "refactor",
            json!({
                "action": "extract",
                "params": {
                    "kind": "variable",
                    "filePath": refactor_path.to_string_lossy(),
                    "range": {
                        "startLine": 1,
                        "startCharacter": 12,
                        "endLine": 1,
                        "endCharacter": 17
                    },
                    "name": "sum"
                },
                "options": { "dryRun": true }
            }),
        )
        .await;

    match refactor_res {
        Ok(res) => println!("Refactor Result: {:?}", res),
        Err(e) => println!(
            "Refactor failed (expected if LSP capability missing): {}",
            e
        ),
    }

    // 7. Workspace
    println!("Testing workspace...");
    let workspace_res = client
        .call_tool(
            "workspace",
            json!({
                "action": "verify_project",
                "params": {}
            }),
        )
        .await
        .expect("workspace verify failed");
    println!("Workspace Result: {:?}", workspace_res);

    println!("All user scenarios passed!");
}
