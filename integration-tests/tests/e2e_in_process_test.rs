use cb_server::test_helpers::create_test_dispatcher_with_root;
use serde_json::json;
use std::time::Instant;
use tempfile::TempDir;

#[tokio::test]
async fn test_workspace_edit_in_process() {
    // Create temporary workspace
    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().to_path_buf();

    // Create in-process dispatcher
    let mut dispatcher = create_test_dispatcher_with_root(workspace_path.clone());

    // Create 50 test files
    let file_count = 50;
    let mut file_paths = Vec::new();

    for i in 0..file_count {
        let file_path = workspace_path.join(format!("edit_perf_{}.ts", i));
        let content = format!(
            r#"
export interface OldInterface{} {{
    id: number;
    oldProperty: string;
}}

export function oldFunction{}(param: OldInterface{}): string {{
    return param.oldProperty;
}}

const oldConstant{} = "old_value_{}";
"#,
            i, i, i, i, i
        );

        let request = json!({
            "jsonrpc": "2.0",
            "id": format!("create-{}", i),
            "method": "tools/call",
            "params": {
                "name": "create_file",
                "arguments": {
                    "file_path": file_path.to_string_lossy(),
                    "content": content
                }
            }
        });

        let request_str = serde_json::to_string(&request).unwrap();
        let response_str = dispatcher.handle_request(&request_str).await.unwrap();
        let response: serde_json::Value = serde_json::from_str(&response_str).unwrap();

        assert!(response.get("result").is_some());
        assert!(response["result"]["success"].as_bool().unwrap_or(false));
        file_paths.push(file_path);
    }

    eprintln!("DEBUG: Created {} files", file_count);

    // Verify all files have content
    for (i, file_path) in file_paths.iter().enumerate() {
        let content = tokio::fs::read_to_string(file_path).await.unwrap();
        assert!(!content.is_empty(), "File {} should have content", i);
    }
    eprintln!("DEBUG: All {} files verified to have content!", file_count);

    // Prepare large workspace edit
    let mut changes = json!({});
    for (index, file_path) in file_paths.iter().enumerate() {
        changes[file_path.to_string_lossy().to_string()] = json!([
            {
                "range": {
                    "start": { "line": 1, "character": 17 },
                    "end": { "line": 1, "character": 17 + format!("OldInterface{}", index).len() }
                },
                "newText": format!("NewInterface{}", index)
            },
            {
                "range": {
                    "start": { "line": 2, "character": 4 },
                    "end": { "line": 2, "character": 15 }
                },
                "newText": "newProperty"
            },
            {
                "range": {
                    "start": { "line": 5, "character": 16 },
                    "end": { "line": 5, "character": 16 + format!("oldFunction{}", index).len() }
                },
                "newText": format!("newFunction{}", index)
            }
        ]);
    }

    // Execute large workspace edit
    let start = Instant::now();
    let request = json!({
        "jsonrpc": "2.0",
        "id": "workspace-edit",
        "method": "tools/call",
        "params": {
            "name": "apply_workspace_edit",
            "arguments": {
                "changes": changes
            }
        }
    });

    let request_str = serde_json::to_string(&request).unwrap();
    let response_str = dispatcher.handle_request(&request_str).await.unwrap();
    let response: serde_json::Value = serde_json::from_str(&response_str).unwrap();
    let edit_duration = start.elapsed();

    eprintln!(
        "Workspace edit across {} files took: {:?}",
        file_count, edit_duration
    );
    eprintln!(
        "APPLY_EDITS RESPONSE: {}",
        serde_json::to_string_pretty(&response).unwrap()
    );

    // Check for errors
    if let Some(error) = response.get("error") {
        panic!("Workspace edit failed: {:?}", error);
    }

    let result = response
        .get("result")
        .expect("Response should have result field");
    assert!(
        result["applied"].as_bool().unwrap_or(false),
        "Workspace edit should be applied"
    );

    // Verify changes were applied correctly
    for (index, file_path) in file_paths.iter().enumerate().take(5) {
        let content = tokio::fs::read_to_string(file_path).await.unwrap();
        assert!(content.contains(&format!("NewInterface{}", index)));
        assert!(content.contains("newProperty"));
        assert!(content.contains(&format!("newFunction{}", index)));
        assert!(!content.contains(&format!("OldInterface{}", index)));
        assert!(!content.contains("oldProperty"));
    }

    eprintln!("✅ In-process workspace edit test PASSED!");
}
