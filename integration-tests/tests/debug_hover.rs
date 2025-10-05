use integration_tests::harness::{TestClient, TestWorkspace, LspSetupHelper};
use serde_json::json;

#[tokio::test]
async fn debug_hover_positions() {
    println!("\n=== HOVER DEBUG TEST ===\n");

    let workspace = TestWorkspace::new();
    LspSetupHelper::setup_lsp_config(&workspace);
    let mut client = TestClient::new(workspace.path());

    let file_path = workspace.path().join("advanced_test.ts");
    let content = r#"
interface DataProcessor<T> {
    process(data: T): Promise<T>;
}

class StringProcessor implements DataProcessor<string> {
    async process(data: string): Promise<string> {
        return data.toUpperCase();
    }
}
"#;
    std::fs::write(&file_path, content).unwrap();

    println!("Created file: {}", file_path.display());
    println!("Content lines:");
    for (i, line) in content.lines().enumerate() {
        println!("  Line {}: {:?}", i, line);
    }

    // Wait for LSP using smart polling
    println!("\nPolling for LSP readiness...");
    let mut ready = false;
    for attempt in 1..=30 {
        if let Ok(response) = client.call_tool("get_document_symbols", json!({
            "file_path": file_path.to_string_lossy()
        })).await {
            if let Some(symbols) = response
                .get("result")
                .and_then(|r| r.get("content"))
                .and_then(|c| c.get("symbols"))
                .and_then(|s| s.as_array())
            {
                if !symbols.is_empty() {
                    println!("✅ LSP ready after {} attempts, found {} symbols", attempt, symbols.len());
                    ready = true;
                    break;
                }
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    if !ready {
        println!("⚠️ LSP never became ready!");
    }

    println!("\n=== Testing different hover positions ===\n");

    let test_positions = vec![
        (1, 0, "Start of 'interface'"),
        (1, 5, "Middle of 'interface'"),
        (1, 10, "Start of 'DataProcessor'"),
        (1, 15, "Middle of 'DataProcessor' (1)"),
        (1, 20, "Middle of 'DataProcessor' (2) - TEST POSITION"),
        (1, 22, "End of 'DataProcessor'"),
        (1, 23, "Generic bracket '<'"),
        (2, 4, "Method 'process'"),
    ];

    for (line, character, description) in test_positions {
        println!("\n📍 Testing position: line {}, char {} - {}", line, character, description);

        match client.call_tool("get_hover", json!({
            "file_path": file_path.to_string_lossy(),
            "line": line,
            "character": character
        })).await {
            Ok(response) => {
                // Try to extract hover content
                let hover_content = response
                    .get("result")
                    .and_then(|r| r.get("content"))
                    .and_then(|c| c.get("hover"))
                    .and_then(|h| h.get("contents"))
                    .or_else(|| {
                        response
                            .get("result")
                            .and_then(|r| r.get("content"))
                            .and_then(|c| c.get("contents"))
                    });

                // Handle LSP hover content which can be either:
                // 1. An object with {kind: "markdown", value: "text"}
                // 2. A plain string
                let hover_text = if let Some(content) = hover_content {
                    if let Some(obj) = content.as_object() {
                        obj.get("value")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                    } else {
                        content.as_str().unwrap_or("")
                    }
                } else {
                    ""
                };

                if hover_text.is_empty() {
                    println!("  ❌ EMPTY or no hover");
                    println!("  Full response: {}", serde_json::to_string_pretty(&response).unwrap());
                } else {
                    println!("  ✅ Got hover: {}", hover_text.lines().next().unwrap_or(hover_text));
                }
            }
            Err(e) => {
                println!("  ❌ Error: {}", e);
            }
        }
    }

    println!("\n=== CONCLUSIONS ===");
    println!("Check which positions returned hover content");
    println!("If ALL are empty: LSP issue or file not indexed");
    println!("If SOME work: Position calculation issue");
}
