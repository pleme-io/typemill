// Direct test: Call workspace symbols and see what happens
// Compile and run: cargo test --package integration-tests --test direct_test -- --nocapture

use integration_tests::harness::{TestClient, TestWorkspace, LspSetupHelper};
use serde_json::json;

#[tokio::test]
async fn test_workspace_symbols_direct() {
    println!("\n=== DIRECT WORKSPACE SYMBOLS TEST ===\n");

    let workspace = TestWorkspace::new();
    LspSetupHelper::setup_lsp_config(&workspace);
    let mut client = TestClient::new(workspace.path());

    // Create TypeScript file
    let ts_file = workspace.path().join("app.ts");
    std::fs::write(&ts_file, r#"
function validateConfig() {
    return true;
}
"#).unwrap();

    // Create Python file
    let py_file = workspace.path().join("validator.py");
    std::fs::write(&py_file, r#"
def validate_data():
    return True
"#).unwrap();

    println!("Created files:");
    println!("  - {}", ts_file.display());
    println!("  - {}", py_file.display());

    // Wait for LSP to index files
    println!("\nWaiting for LSP to index files...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Test TypeScript file symbols
    println!("\n--- Testing TypeScript symbols ---");
    match client.call_tool("get_document_symbols", json!({
        "file_path": ts_file.to_string_lossy()
    })).await {
        Ok(response) => {
            println!("TypeScript symbols response: {}", serde_json::to_string_pretty(&response).unwrap());
        }
        Err(e) => println!("ERROR: {}", e)
    }

    // Test Python file symbols
    println!("\n--- Testing Python symbols ---");
    match client.call_tool("get_document_symbols", json!({
        "file_path": py_file.to_string_lossy()
    })).await {
        Ok(response) => {
            println!("Python symbols response: {}", serde_json::to_string_pretty(&response).unwrap());
        }
        Err(e) => println!("ERROR: {}", e)
    }

    // THE KEY TEST: Workspace symbols
    println!("\n--- Testing WORKSPACE symbols ---");
    match client.call_tool("search_workspace_symbols", json!({
        "query": "validate"
    })).await {
        Ok(response) => {
            let response_str = serde_json::to_string_pretty(&response).unwrap();
            println!("Workspace symbols response:\n{}", response_str);

            // Check for multi-plugin
            if response_str.contains("multi-plugin") {
                println!("\n✅ SUCCESS: Multi-plugin code is running!");
            } else {
                println!("\n❌ FAIL: Still using single plugin");
            }

            // Check plugin field
            if let Some(plugin) = response.get("result")
                .and_then(|r| r.get("plugin"))
                .and_then(|p| p.as_str())
            {
                println!("Plugin field value: '{}'", plugin);
            }

            // Count symbols
            if let Some(symbols) = response.get("result")
                .and_then(|r| r.get("content"))
                .and_then(|c| c.as_array())
            {
                println!("Found {} symbols total", symbols.len());
                for symbol in symbols {
                    if let Some(name) = symbol.get("name").and_then(|n| n.as_str()) {
                        if let Some(uri) = symbol.get("location")
                            .and_then(|l| l.get("uri"))
                            .and_then(|u| u.as_str())
                        {
                            println!("  - {} from {}", name, uri);
                        }
                    }
                }
            }
        }
        Err(e) => println!("ERROR: {}", e)
    }

    println!("\n=== END TEST ===\n");
}
