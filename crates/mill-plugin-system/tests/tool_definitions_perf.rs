use async_trait::async_trait;
use mill_plugin_system::{
    Capabilities, LanguagePlugin, PluginManager, PluginMetadata, PluginRequest, PluginResponse,
    PluginResult,
};
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;

struct BenchmarkPlugin {
    name: String,
}

#[async_trait]
impl LanguagePlugin for BenchmarkPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new(&self.name, "1.0.0", "test")
    }

    fn supported_extensions(&self) -> Vec<String> {
        vec!["test".to_string()]
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::default()
    }

    async fn handle_request(&self, _request: PluginRequest) -> PluginResult<PluginResponse> {
        Ok(PluginResponse::empty())
    }

    fn configure(&self, _config: Value) -> PluginResult<()> {
        Ok(())
    }

    fn tool_definitions(&self) -> Vec<Value> {
        // Return some dummy tools
        vec![
            serde_json::json!({
                "name": format!("{}_tool_1", self.name),
                "description": "A test tool"
            }),
            serde_json::json!({
                "name": format!("{}_tool_2", self.name),
                "description": "Another test tool"
            }),
        ]
    }
}

#[tokio::test]
async fn measure_get_all_tool_definitions_perf() {
    let manager = PluginManager::new();
    let num_plugins = 10_000;

    println!("Registering {} plugins...", num_plugins);
    for i in 0..num_plugins {
        let name = format!("plugin_{}", i);
        let plugin = Arc::new(BenchmarkPlugin { name: name.clone() });
        manager.register_plugin(&name, plugin).await.unwrap();
    }

    println!("Starting measurement...");
    let start = Instant::now();
    let tools = manager.get_all_tool_definitions().await;
    let duration = start.elapsed();

    println!("get_all_tool_definitions took: {:.2?}", duration);
    println!("Total tools retrieved: {}", tools.len());

    assert_eq!(tools.len(), num_plugins * 2);
}
