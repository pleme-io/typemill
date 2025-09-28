#!/usr/bin/env cargo
//! Simple standalone test for the plugin system

use cb_plugins::{
    PluginManager, LspAdapterPlugin, LspService, PluginRequest,
    Capabilities, NavigationCapabilities, PluginMetadata
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

/// Mock LSP service for testing
struct MockLspService {
    name: String,
}

#[async_trait]
impl LspService for MockLspService {
    async fn request(&self, method: &str, _params: Value) -> Result<Value, String> {
        match method {
            "textDocument/definition" => Ok(json!([{
                "uri": "file:///test.ts",
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 0, "character": 10 }
                }
            }])),
            "textDocument/hover" => Ok(json!({
                "contents": "Mock hover content for testing"
            })),
            _ => Ok(json!(null)),
        }
    }

    fn supports_extension(&self, extension: &str) -> bool {
        ["ts", "tsx", "js", "jsx"].contains(&extension)
    }

    fn service_name(&self) -> String {
        self.name.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Plugin System Integration");

    // 1. Create plugin manager
    let manager = PluginManager::new();
    println!("✅ Plugin manager created");

    // 2. Create mock LSP service
    let lsp_service = Arc::new(MockLspService {
        name: "test-typescript-lsp".to_string(),
    });

    // 3. Create TypeScript plugin adapter
    let ts_plugin = Arc::new(LspAdapterPlugin::typescript(lsp_service));
    println!("✅ TypeScript plugin adapter created");

    // 4. Register plugin
    manager.register_plugin("typescript", ts_plugin).await?;
    println!("✅ Plugin registered successfully");

    // 5. Verify plugin capabilities
    let capabilities = manager.get_plugin_capabilities("typescript").await;
    assert!(capabilities.is_some(), "Plugin capabilities should be available");

    let caps = capabilities.unwrap();
    assert!(caps.navigation.go_to_definition, "Should support go to definition");
    assert!(caps.intelligence.hover, "Should support hover");
    println!("✅ Plugin capabilities verified");

    // 6. Test method support checking
    let ts_file = PathBuf::from("test.ts");
    assert!(manager.is_method_supported(&ts_file, "find_definition").await);
    assert!(manager.is_method_supported(&ts_file, "get_hover").await);
    assert!(!manager.is_method_supported(&ts_file, "unsupported_method").await);
    println!("✅ Method support checking works");

    // 7. Test plugin request handling
    let request = PluginRequest::new("find_definition", ts_file.clone())
        .with_position(10, 20)
        .with_params(json!({"symbol": "testSymbol"}));

    let response = manager.handle_request(request).await?;
    assert!(response.success, "Request should succeed");
    assert!(response.data.is_some(), "Response should have data");
    println!("✅ Plugin request handling works");

    // 8. Test hover request
    let hover_request = PluginRequest::new("get_hover", ts_file)
        .with_position(5, 10);

    let hover_response = manager.handle_request(hover_request).await?;
    assert!(hover_response.success, "Hover request should succeed");

    if let Some(data) = hover_response.data {
        if let Some(hover) = data.get("hover") {
            assert!(!hover.is_null(), "Hover data should not be null");
        }
    }
    println!("✅ Hover request works");

    // 9. Check statistics
    let stats = manager.get_registry_statistics().await;
    assert_eq!(stats.total_plugins, 1, "Should have 1 plugin registered");
    assert!(stats.supported_extensions > 0, "Should support some extensions");
    assert!(stats.supported_methods > 0, "Should support some methods");
    println!("✅ Registry statistics work");

    // 10. Test metrics
    let metrics = manager.get_metrics().await;
    assert!(metrics.total_requests >= 2, "Should have processed at least 2 requests");
    assert!(metrics.successful_requests >= 2, "Should have 2+ successful requests");
    assert_eq!(metrics.failed_requests, 0, "Should have no failed requests");
    println!("✅ Performance metrics work");

    println!("\n🎉 ALL PLUGIN SYSTEM TESTS PASSED!");
    println!("   - Plugin registration ✅");
    println!("   - Capability checking ✅");
    println!("   - Method routing ✅");
    println!("   - Request handling ✅");
    println!("   - LSP adapter translation ✅");
    println!("   - Performance tracking ✅");

    println!("\n📊 Final Statistics:");
    println!("   - Plugins: {}", stats.total_plugins);
    println!("   - Extensions: {}", stats.supported_extensions);
    println!("   - Methods: {}", stats.supported_methods);
    println!("   - Requests processed: {}", metrics.total_requests);
    println!("   - Success rate: 100%");

    Ok(())
}