use mill_plugin_system::adapters::lsp_adapter::{LspAdapterPlugin, LspService};
use mill_plugin_system::{LanguagePlugin, PluginRequest, PluginResponse};
use mill_plugin_api::SymbolKind;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;

struct MockLspService {
    symbols: Vec<Value>,
}

#[async_trait]
impl LspService for MockLspService {
    async fn request(&self, method: &str, _params: Value) -> Result<Value, String> {
        if method == "workspace/symbol" {
            Ok(Value::Array(self.symbols.clone()))
        } else {
            Ok(Value::Null)
        }
    }

    fn supports_extension(&self, _extension: &str) -> bool {
        true
    }

    fn service_name(&self) -> String {
        "MockLspService".to_string()
    }
}

#[tokio::test]
async fn benchmark_search_workspace_symbols() {
    // 1. Create large dataset of symbols
    println!("Generating 100,000 symbols...");
    let mut symbols = Vec::new();
    for i in 0..100000 {
        // Use LSP kind 12 (Function) for 1/26th of items
        let kind = (i % 26) + 1;
        symbols.push(json!({
            "name": format!("symbol_{}", i),
            "kind": kind,
            "location": {
                "uri": format!("file:///tmp/file_{}.rs", i),
                "range": {
                    "start": { "line": i, "character": 0 },
                    "end": { "line": i, "character": 10 }
                }
            }
        }));
    }

    // 2. Setup Mock Service and Plugin
    let lsp_service = Arc::new(MockLspService { symbols });
    let plugin = LspAdapterPlugin::new(
        "test-plugin".to_string(),
        vec!["rs".to_string()],
        lsp_service,
    );

    // 3. Measure Baseline (Requesting "Function" kind)
    // We expect "Function" to map to LSP kind 12 and 9 (Constructor).
    // 100000 / 26 * 2 ~= 7692
    let request = PluginRequest::new("search_workspace_symbols", std::path::PathBuf::from("."))
        .with_params(json!({
            "query": "test",
            "kind": SymbolKind::Function
        }));

    println!("Starting search benchmark...");
    let start = Instant::now();
    let response = plugin.handle_request(request).await.unwrap();
    let duration = start.elapsed();

    println!("Time taken: {:?}", duration);

    if let Some(Value::Array(results)) = response.data {
        println!("Received {} symbols", results.len());
        assert!(results.len() > 7000 && results.len() < 8000, "Should have filtered results to approx 7692");
    } else {
        panic!("Expected array response");
    }
}
