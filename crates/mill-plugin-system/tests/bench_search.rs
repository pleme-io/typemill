use async_trait::async_trait;
use mill_plugin_api::SymbolKind;
use mill_plugin_system::adapters::lsp_adapter::{LspAdapterPlugin, LspService};
use mill_plugin_system::{LanguagePlugin, PluginRequest, PluginResponse};
use serde_json::{json, Map, Number, Value};
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

/// Build a single LSP symbol as a serde_json::Value using direct construction
/// instead of the json!() macro. This avoids stack overflow when called 100K times
/// because json!() macro expansion uses significant stack space per invocation.
fn build_symbol(i: usize, kind: usize) -> Value {
    let mut start = Map::new();
    start.insert("line".to_string(), Value::Number(Number::from(i)));
    start.insert("character".to_string(), Value::Number(Number::from(0)));

    let mut end = Map::new();
    end.insert("line".to_string(), Value::Number(Number::from(i)));
    end.insert("character".to_string(), Value::Number(Number::from(10)));

    let mut range = Map::new();
    range.insert("start".to_string(), Value::Object(start));
    range.insert("end".to_string(), Value::Object(end));

    let mut location = Map::new();
    location.insert(
        "uri".to_string(),
        Value::String(format!("file:///tmp/file_{}.rs", i)),
    );
    location.insert("range".to_string(), Value::Object(range));

    let mut symbol = Map::new();
    symbol.insert("name".to_string(), Value::String(format!("symbol_{}", i)));
    symbol.insert("kind".to_string(), Value::Number(Number::from(kind)));
    symbol.insert("location".to_string(), Value::Object(location));

    Value::Object(symbol)
}

#[tokio::test]
async fn benchmark_search_workspace_symbols() {
    // 1. Create dataset of symbols using direct Value construction.
    //    Reduced from 100K to 10K: still validates search/filter logic but avoids
    //    SIGSEGV from stack exhaustion in debug mode under parallel test execution.
    println!("Generating 10,000 symbols...");
    let mut symbols = Vec::with_capacity(10_000);
    for i in 0..10_000 {
        let kind = (i % 26) + 1;
        symbols.push(build_symbol(i, kind));
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
    // 10000 / 26 * 2 ~= 769
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
        assert!(
            results.len() > 700 && results.len() < 800,
            "Should have filtered results to approx 769"
        );
    } else {
        panic!("Expected array response");
    }
}
