use mill_plugin_system::{
    Capabilities, LanguagePlugin, PluginManager, PluginMetadata, PluginRequest, PluginResponse,
    PluginResult,
};
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;
use async_trait::async_trait;

struct PerfTestPlugin {
    name: String,
}

#[async_trait]
impl LanguagePlugin for PerfTestPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new(&self.name, "1.0.0", "test")
    }

    fn supported_extensions(&self) -> Vec<String> {
        vec!["test".to_string()]
    }

    fn tool_definitions(&self) -> Vec<Value> {
        vec![]
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
}

#[tokio::test]
async fn benchmark_get_all_capabilities() {
    let manager = PluginManager::new();
    let num_plugins = 10000;

    println!("Registering {} plugins...", num_plugins);
    for i in 0..num_plugins {
        let name = format!("plugin-{}", i);
        let plugin = Arc::new(PerfTestPlugin { name: name.clone() });
        manager.register_plugin(name, plugin).await.unwrap();
    }

    println!("Starting benchmark...");
    let start = Instant::now();
    let capabilities = manager.get_all_capabilities().await;
    let duration = start.elapsed();

    println!("get_all_capabilities took: {:?}", duration);
    assert_eq!(capabilities.len(), num_plugins);
}
