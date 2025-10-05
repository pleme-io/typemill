// Test: Does Python LSP even support workspace symbols?

use integration_tests::harness::{TestClient, TestWorkspace, LspSetupHelper};
use serde_json::json;

#[tokio::test]
async fn test_python_lsp_capabilities() {
    let workspace = TestWorkspace::new();
    LspSetupHelper::setup_lsp_config(&workspace);
    let mut client = TestClient::new(workspace.path());

    // Create Python file
    let py_file = workspace.path().join("test.py");
    std::fs::write(&py_file, r#"
def my_function():
    return True

class MyClass:
    def my_method(self):
        pass
"#).unwrap();

    // Wait for indexing
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    println!("\n=== Testing Python LSP ===\n");

    // Test 1: Document symbols (should work)
    println!("1. Document symbols for Python file:");
    match client.call_tool("get_document_symbols", json!({
        "file_path": py_file.to_string_lossy()
    })).await {
        Ok(response) => {
            if let Some(symbols) = response.get("result")
                .and_then(|r| r.get("content"))
                .and_then(|c| c.get("symbols"))
                .and_then(|s| s.as_array())
            {
                println!("  ✅ Found {} document symbols", symbols.len());
                for symbol in symbols {
                    if let Some(name) = symbol.get("name").and_then(|n| n.as_str()) {
                        println!("    - {}", name);
                    }
                }
            } else {
                println!("  ❌ No symbols");
            }
        }
        Err(e) => println!("  ❌ ERROR: {}", e)
    }

    // Test 2: Workspace symbols with different queries
    println!("\n2. Workspace symbols - query 'my':");
    match client.call_tool("search_workspace_symbols", json!({
        "query": "my"
    })).await {
        Ok(response) => {
            println!("Full response: {}", serde_json::to_string_pretty(&response).unwrap());
        }
        Err(e) => println!("  ❌ ERROR: {}", e)
    }

    // Test 3: Empty query
    println!("\n3. Workspace symbols - empty query:");
    match client.call_tool("search_workspace_symbols", json!({
        "query": ""
    })).await {
        Ok(response) => {
            if let Some(symbols) = response.get("result")
                .and_then(|r| r.get("content"))
                .and_then(|c| c.as_array())
            {
                println!("  Found {} symbols with empty query", symbols.len());
            }
        }
        Err(e) => println!("  ❌ ERROR: {}", e)
    }
}
