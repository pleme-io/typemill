// Test: Does LSP match partial symbol names?

use integration_tests::harness::{TestClient, TestWorkspace, LspSetupHelper};
use serde_json::json;

#[tokio::test]
async fn test_symbol_name_matching() {
    let workspace = TestWorkspace::new();
    LspSetupHelper::setup_lsp_config(&workspace);
    let mut client = TestClient::new(workspace.path());

    // Create Python file with clear symbol
    let py_file = workspace.path().join("validator.py");
    std::fs::write(&py_file, r#"
def validate_data():
    return True

def validate_config():
    return False

def other_function():
    pass
"#).unwrap();

    // Wait for indexing
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    println!("\n=== Testing different queries ===\n");

    for query in ["validate", "validate_", "validate_data", "data", "config"] {
        println!("Query: '{}'", query);
        match client.call_tool("search_workspace_symbols", json!({
            "query": query
        })).await {
            Ok(response) => {
                if let Some(symbols) = response.get("result")
                    .and_then(|r| r.get("content"))
                    .and_then(|c| c.as_array())
                {
                    println!("  Found {} symbols:", symbols.len());
                    for symbol in symbols {
                        if let Some(name) = symbol.get("name").and_then(|n| n.as_str()) {
                            println!("    - {}", name);
                        }
                    }
                } else {
                    println!("  No symbols found");
                }
            }
            Err(e) => println!("  ERROR: {}", e)
        }
        println!();
    }
}
