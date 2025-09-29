//! Integration test for the plugin system

mod common;

use async_trait::async_trait;
use cb_plugins::{
    Capabilities, LspAdapterPlugin, LspService, NavigationCapabilities, PluginManager,
    PluginMetadata, PluginRequest,
};
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

#[tokio::test]
async fn test_complete_plugin_system_integration() {
    use common::{MockLspService, PluginTestBuilder};

    // Create mock LSP service
    let lsp_service = Arc::new(MockLspService::new("test-typescript-lsp"));

    // Create TypeScript plugin adapter
    let ts_plugin = Arc::new(LspAdapterPlugin::typescript(lsp_service));

    // Use builder to set up manager
    let manager = PluginTestBuilder::new()
        .with_plugin("typescript", ts_plugin)
        .build()
        .await
        .unwrap();

    // Verify plugin capabilities
    let capabilities = manager.get_plugin_capabilities("typescript").await;
    assert!(capabilities.is_some());

    let caps = capabilities.unwrap();
    assert!(caps.navigation.go_to_definition);
    assert!(caps.intelligence.hover);

    // Test method support checking
    let ts_file = PathBuf::from("test.ts");
    assert!(
        manager
            .is_method_supported(&ts_file, "find_definition")
            .await
    );
    assert!(manager.is_method_supported(&ts_file, "get_hover").await);
    assert!(
        !manager
            .is_method_supported(&ts_file, "unsupported_method")
            .await
    );

    // Test plugin request handling
    let request = PluginRequest::new("find_definition", ts_file.clone())
        .with_position(10, 20)
        .with_params(json!({"symbol": "testSymbol"}));

    let response = manager.handle_request(request).await;
    assert!(response.is_ok());

    let response = response.unwrap();
    assert!(response.success);
    assert!(response.data.is_some());

    // Test hover request
    let hover_request = PluginRequest::new("get_hover", ts_file).with_position(5, 10);

    let hover_response = manager.handle_request(hover_request).await;
    assert!(hover_response.is_ok());

    let hover_response = hover_response.unwrap();
    assert!(hover_response.success);

    // Check statistics
    let stats = manager.get_registry_statistics().await;
    assert_eq!(stats.total_plugins, 1);
    assert!(stats.supported_extensions > 0);
    assert!(stats.supported_methods > 0);

    // Test metrics
    let metrics = manager.get_metrics().await;
    assert!(metrics.total_requests >= 2);
    assert!(metrics.successful_requests >= 2);
    assert_eq!(metrics.failed_requests, 0);
}

#[tokio::test]
async fn test_multi_language_plugin_system() {
    use common::{MockLspService, PluginTestBuilder};

    // Register TypeScript plugin
    let ts_lsp = Arc::new(MockLspService::new("ts-lsp"));
    let ts_plugin = Arc::new(LspAdapterPlugin::typescript(ts_lsp));

    // Register Python plugin
    let py_lsp = Arc::new(MockLspService::new("py-lsp"));
    let py_plugin = Arc::new(LspAdapterPlugin::python(py_lsp));

    // Use builder to set up manager with multiple plugins
    let manager = PluginTestBuilder::new()
        .with_plugin("typescript", ts_plugin)
        .with_plugin("python", py_plugin)
        .build()
        .await
        .unwrap();

    // Test TypeScript file routing
    let ts_file = PathBuf::from("test.ts");
    assert!(
        manager
            .is_method_supported(&ts_file, "find_definition")
            .await
    );

    // Test Python file routing
    let py_file = PathBuf::from("test.py");
    assert!(
        manager
            .is_method_supported(&py_file, "find_definition")
            .await
    );

    // Test unsupported file
    let unknown_file = PathBuf::from("test.unknown");
    assert!(
        !manager
            .is_method_supported(&unknown_file, "find_definition")
            .await
    );

    // Verify statistics
    let stats = manager.get_registry_statistics().await;
    assert_eq!(stats.total_plugins, 2);

    let all_extensions = manager.get_supported_extensions().await;
    assert!(all_extensions.contains(&"ts".to_string()));
    assert!(all_extensions.contains(&"py".to_string()));
}

#[tokio::test]
async fn test_plugin_error_handling() {
    use common::PluginTestBuilder;

    // Set up manager with no plugins
    let manager = PluginTestBuilder::new().build().await.unwrap();

    // Test request to non-existent file type
    let unknown_file = PathBuf::from("test.unknown");
    let request = PluginRequest::new("find_definition", unknown_file);

    let result = manager.handle_request(request).await;
    assert!(result.is_err());

    // Verify metrics recorded the failure
    let metrics = manager.get_metrics().await;
    assert!(
        metrics.failed_requests >= 1,
        "Should have at least 1 failed request"
    );
}

#[tokio::test]
async fn test_file_lifecycle_hooks_integration() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    // Create a mock plugin that counts hook invocations
    struct HookCountingLspService {
        name: String,
        open_count: Arc<AtomicUsize>,
        save_count: Arc<AtomicUsize>,
        close_count: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl LspService for HookCountingLspService {
        async fn request(&self, method: &str, _params: Value) -> Result<Value, String> {
            match method {
                "textDocument/definition" => Ok(json!([{
                    "uri": "file:///test.ts",
                    "range": {
                        "start": { "line": 0, "character": 0 },
                        "end": { "line": 0, "character": 10 }
                    }
                }])),
                _ => Ok(json!(null)),
            }
        }

        fn supports_extension(&self, extension: &str) -> bool {
            ["ts", "tsx"].contains(&extension)
        }

        fn service_name(&self) -> String {
            self.name.clone()
        }
    }

    // Custom plugin wrapper that tracks lifecycle hooks
    struct HookTrackingPlugin {
        inner: Arc<LspAdapterPlugin>,
        open_count: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl cb_plugins::LanguagePlugin for HookTrackingPlugin {
        fn metadata(&self) -> PluginMetadata {
            self.inner.metadata()
        }

        fn supported_extensions(&self) -> Vec<String> {
            self.inner.supported_extensions()
        }

        fn capabilities(&self) -> Capabilities {
            self.inner.capabilities()
        }

        async fn handle_request(
            &self,
            request: cb_plugins::PluginRequest,
        ) -> cb_plugins::PluginResult<cb_plugins::PluginResponse> {
            self.inner.handle_request(request).await
        }

        fn configure(&self, config: Value) -> cb_plugins::PluginResult<()> {
            self.inner.configure(config)
        }

        fn on_file_open(&self, path: &std::path::Path) -> cb_plugins::PluginResult<()> {
            self.open_count.fetch_add(1, Ordering::SeqCst);
            self.inner.on_file_open(path)
        }

        fn tool_definitions(&self) -> Vec<Value> {
            self.inner.tool_definitions()
        }
    }

    let manager = PluginManager::new();

    let open_count = Arc::new(AtomicUsize::new(0));
    let save_count = Arc::new(AtomicUsize::new(0));
    let close_count = Arc::new(AtomicUsize::new(0));

    let lsp_service = Arc::new(HookCountingLspService {
        name: "hook-test-lsp".to_string(),
        open_count: open_count.clone(),
        save_count: save_count.clone(),
        close_count: close_count.clone(),
    });

    let inner_plugin = Arc::new(LspAdapterPlugin::typescript(lsp_service));
    let tracking_plugin = Arc::new(HookTrackingPlugin {
        inner: inner_plugin,
        open_count: open_count.clone(),
    });

    manager
        .register_plugin("typescript", tracking_plugin)
        .await
        .unwrap();

    // Test file open hook
    let ts_file = std::path::PathBuf::from("test.ts");
    manager.trigger_file_open_hooks(&ts_file).await.unwrap();

    assert_eq!(
        open_count.load(Ordering::SeqCst),
        1,
        "on_file_open should be called once"
    );

    // Test multiple hook invocations
    manager.trigger_file_open_hooks(&ts_file).await.unwrap();
    manager.trigger_file_open_hooks(&ts_file).await.unwrap();

    assert_eq!(
        open_count.load(Ordering::SeqCst),
        3,
        "on_file_open should be called three times total"
    );

    // Test that hooks are NOT called for non-matching files
    let py_file = std::path::PathBuf::from("test.py");
    manager.trigger_file_open_hooks(&py_file).await.unwrap();

    assert_eq!(
        open_count.load(Ordering::SeqCst),
        3,
        "on_file_open should NOT be called for .py file"
    );
}

#[tokio::test]
async fn test_hooks_with_multiple_plugins() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let ts_hook_count = Arc::new(AtomicUsize::new(0));
    let js_hook_count = Arc::new(AtomicUsize::new(0));

    struct CountingLspService {
        name: String,
        hook_count: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl LspService for CountingLspService {
        async fn request(&self, _method: &str, _params: Value) -> Result<Value, String> {
            Ok(json!(null))
        }

        fn supports_extension(&self, _extension: &str) -> bool {
            true
        }

        fn service_name(&self) -> String {
            self.name.clone()
        }
    }

    struct CountingPlugin {
        inner: Arc<LspAdapterPlugin>,
        hook_count: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl cb_plugins::LanguagePlugin for CountingPlugin {
        fn metadata(&self) -> PluginMetadata {
            self.inner.metadata()
        }

        fn supported_extensions(&self) -> Vec<String> {
            self.inner.supported_extensions()
        }

        fn capabilities(&self) -> Capabilities {
            self.inner.capabilities()
        }

        async fn handle_request(
            &self,
            request: cb_plugins::PluginRequest,
        ) -> cb_plugins::PluginResult<cb_plugins::PluginResponse> {
            self.inner.handle_request(request).await
        }

        fn configure(&self, config: Value) -> cb_plugins::PluginResult<()> {
            self.inner.configure(config)
        }

        fn on_file_open(&self, path: &std::path::Path) -> cb_plugins::PluginResult<()> {
            self.hook_count.fetch_add(1, Ordering::SeqCst);
            self.inner.on_file_open(path)
        }

        fn tool_definitions(&self) -> Vec<Value> {
            self.inner.tool_definitions()
        }
    }

    let manager = PluginManager::new();

    // Register TypeScript plugin
    let ts_service = Arc::new(CountingLspService {
        name: "ts-lsp".to_string(),
        hook_count: ts_hook_count.clone(),
    });
    let ts_inner = Arc::new(LspAdapterPlugin::new(
        "typescript",
        vec!["ts".to_string(), "tsx".to_string()],
        ts_service,
    ));
    let ts_plugin = Arc::new(CountingPlugin {
        inner: ts_inner,
        hook_count: ts_hook_count.clone(),
    });
    manager.register_plugin("typescript", ts_plugin).await.unwrap();

    // Register JavaScript plugin (also handles .js files)
    let js_service = Arc::new(CountingLspService {
        name: "js-lsp".to_string(),
        hook_count: js_hook_count.clone(),
    });
    let js_inner = Arc::new(LspAdapterPlugin::new(
        "javascript",
        vec!["js".to_string()],
        js_service,
    ));
    let js_plugin = Arc::new(CountingPlugin {
        inner: js_inner,
        hook_count: js_hook_count.clone(),
    });
    manager.register_plugin("javascript", js_plugin).await.unwrap();

    // Test TypeScript file - only TS plugin should receive hook
    manager
        .trigger_file_open_hooks(&std::path::PathBuf::from("test.ts"))
        .await
        .unwrap();

    assert_eq!(ts_hook_count.load(Ordering::SeqCst), 1);
    assert_eq!(js_hook_count.load(Ordering::SeqCst), 0);

    // Test JavaScript file - only JS plugin should receive hook
    manager
        .trigger_file_open_hooks(&std::path::PathBuf::from("test.js"))
        .await
        .unwrap();

    assert_eq!(ts_hook_count.load(Ordering::SeqCst), 1);
    assert_eq!(js_hook_count.load(Ordering::SeqCst), 1);
}

/// Test using the new PluginTestBuilder with snapshot testing
#[tokio::test]
async fn test_plugin_with_builder_and_snapshots() {
    use common::{MockLspService, PluginTestBuilder};

    // Create a mock LSP service with custom responses
    let lsp_service = Arc::new(
        MockLspService::new("test-ts-lsp").with_response(
            "textDocument/definition",
            json!([
                {
                    "uri": "file:///src/module.ts",
                    "range": {
                        "start": { "line": 42, "character": 10 },
                        "end": { "line": 42, "character": 20 }
                    }
                }
            ]),
        ),
    );

    let plugin = Arc::new(LspAdapterPlugin::typescript(lsp_service));

    // Use the builder to set up and run the test
    let response = PluginTestBuilder::new()
        .with_plugin("typescript", plugin)
        .with_request("find_definition", "src/app.ts")
        .at_position(10, 5)
        .with_params(json!({"symbol": "myFunction"}))
        .run()
        .await
        .unwrap();

    // Verify basic properties
    assert!(response.success, "Response should be successful");
    assert_eq!(response.metadata.plugin_name, "typescript");

    // Use snapshot testing for the full response
    insta::assert_yaml_snapshot!(response.data, @r###"
    ---
    locations:
      - range:
          end:
            character: 20
            line: 42
          start:
            character: 10
            line: 42
        uri: "file:///src/module.ts"
    "###);
}

/// Test TypeScript plugin capabilities with snapshot
#[tokio::test]
async fn test_typescript_plugin_capabilities_snapshot() {
    use common::{MockLspService, PluginTestBuilder};

    let lsp_service = Arc::new(MockLspService::new("ts-lsp"));
    let plugin = Arc::new(LspAdapterPlugin::typescript(lsp_service));

    let manager = PluginTestBuilder::new()
        .with_plugin("typescript", plugin)
        .build()
        .await
        .unwrap();

    let capabilities = manager.get_plugin_capabilities("typescript").await;

    assert!(capabilities.is_some());

    // Snapshot the capabilities structure
    // Note: Actual snapshot saved to file by insta
    insta::assert_yaml_snapshot!(capabilities.unwrap());
}

/// Test hover request with builder pattern
#[tokio::test]
async fn test_hover_with_builder() {
    use common::{MockLspService, PluginTestBuilder};

    let lsp_service = Arc::new(MockLspService::new("hover-test").with_response(
        "textDocument/hover",
        json!({
            "contents": {
                "kind": "markdown",
                "value": "```typescript\nfunction myFunction(): void\n```\n\nA helpful function"
            }
        }),
    ));

    let plugin = Arc::new(LspAdapterPlugin::typescript(lsp_service));

    let response = PluginTestBuilder::new()
        .with_plugin("typescript", plugin)
        .with_request("get_hover", "src/utils.ts")
        .at_position(15, 8)
        .run()
        .await
        .unwrap();

    assert!(response.success);

    // Snapshot the hover response
    insta::assert_yaml_snapshot!(response.data, @r###"
    ---
    hover:
      contents:
        kind: markdown
        value: "```typescript\nfunction myFunction(): void\n```\n\nA helpful function"
    "###);
}

/// Test Python plugin with hover snapshot
#[tokio::test]
async fn test_python_plugin_hover_snapshot() {
    use common::{MockLspService, PluginTestBuilder};

    let lsp_service = Arc::new(MockLspService::new("python-lsp").with_response(
        "textDocument/hover",
        json!({
            "contents": {
                "kind": "markdown",
                "value": "```python\ndef my_function(x: int, y: int) -> int\n```\n\nCalculates the sum of two numbers"
            }
        }),
    ));

    let plugin = Arc::new(LspAdapterPlugin::python(lsp_service));

    let response = PluginTestBuilder::new()
        .with_plugin("python", plugin)
        .with_request("get_hover", "src/utils.py")
        .at_position(42, 12)
        .run()
        .await
        .unwrap();

    assert!(response.success);
    assert_eq!(response.metadata.plugin_name, "python");

    // Snapshot the Python hover response
    insta::assert_yaml_snapshot!(response.data);
}

/// Test Go plugin with definition snapshot
#[tokio::test]
async fn test_go_plugin_definition_snapshot() {
    use common::{MockLspService, PluginTestBuilder};

    let lsp_service = Arc::new(MockLspService::new("go-lsp").with_response(
        "textDocument/definition",
        json!([
            {
                "uri": "file:///src/main.go",
                "range": {
                    "start": { "line": 15, "character": 5 },
                    "end": { "line": 15, "character": 20 }
                }
            }
        ]),
    ));

    let plugin = Arc::new(LspAdapterPlugin::go(lsp_service));

    let response = PluginTestBuilder::new()
        .with_plugin("go", plugin)
        .with_request("find_definition", "cmd/server/main.go")
        .at_position(30, 8)
        .with_params(json!({"symbol": "HandleRequest"}))
        .run()
        .await
        .unwrap();

    assert!(response.success);
    assert_eq!(response.metadata.plugin_name, "go");

    // Snapshot the Go definition response
    insta::assert_yaml_snapshot!(response.data);
}

/// Test Rust plugin with references snapshot
#[tokio::test]
async fn test_rust_plugin_references_snapshot() {
    use common::{MockLspService, PluginTestBuilder};

    let lsp_service = Arc::new(MockLspService::new("rust-analyzer").with_response(
        "textDocument/references",
        json!([
            {
                "uri": "file:///src/lib.rs",
                "range": {
                    "start": { "line": 10, "character": 4 },
                    "end": { "line": 10, "character": 16 }
                }
            },
            {
                "uri": "file:///src/main.rs",
                "range": {
                    "start": { "line": 25, "character": 8 },
                    "end": { "line": 25, "character": 20 }
                }
            }
        ]),
    ));

    let plugin = Arc::new(LspAdapterPlugin::rust(lsp_service));

    let response = PluginTestBuilder::new()
        .with_plugin("rust", plugin)
        .with_request("find_references", "src/plugin.rs")
        .at_position(50, 10)
        .with_params(json!({"includeDeclaration": true}))
        .run()
        .await
        .unwrap();

    assert!(response.success);
    assert_eq!(response.metadata.plugin_name, "rust");

    // Snapshot the Rust references response
    insta::assert_yaml_snapshot!(response.data);
}
