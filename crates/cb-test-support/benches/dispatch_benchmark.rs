//! Performance benchmarks for the unified plugin dispatch system
//!
//! This benchmark suite measures:
//! - Plugin selection latency with different priority configurations
//! - Tool dispatch overhead for simple vs complex operations
//! - Concurrent request handling throughput
//! - Plugin registry initialization time

use async_trait::async_trait;
use cb_server::handlers::plugin_dispatcher::create_test_dispatcher;
use codebuddy_foundation::core::model::mcp::ToolCall;
use codebuddy_plugin_system::{
    Capabilities, LanguagePlugin, PluginMetadata, PluginRegistry, PluginRequest, PluginResponse,
    PluginResult,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Benchmark plugin that simulates simple operations
struct BenchmarkPlugin {
    name: String,
    extensions: Vec<String>,
    capabilities: Capabilities,
    priority: u32,
    response_delay_ms: u64,
}

impl BenchmarkPlugin {
    fn new(name: &str, extensions: Vec<String>, priority: u32, response_delay_ms: u64) -> Self {
        let mut capabilities = Capabilities::default();
        capabilities.navigation.go_to_definition = true;
        capabilities.navigation.find_references = true;
        capabilities.intelligence.hover = true;

        Self {
            name: name.to_string(),
            extensions,
            capabilities,
            priority,
            response_delay_ms,
        }
    }
}

#[async_trait]
impl LanguagePlugin for BenchmarkPlugin {
    fn metadata(&self) -> PluginMetadata {
        let mut meta = PluginMetadata::new(&self.name, "1.0.0-bench", "benchmark");
        meta.priority = self.priority;
        meta
    }

    fn supported_extensions(&self) -> Vec<String> {
        self.extensions.clone()
    }

    fn capabilities(&self) -> Capabilities {
        self.capabilities.clone()
    }

    async fn handle_request(&self, request: PluginRequest) -> PluginResult<PluginResponse> {
        if self.response_delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.response_delay_ms)).await;
        }

        Ok(PluginResponse {
            success: true,
            data: Some(json!({
                "plugin": self.name,
                "method": request.method,
                "handled": true
            })),
            error: None,
            request_id: request.request_id,
            metadata: codebuddy_plugin_system::protocol::ResponseMetadata {
                plugin_name: self.name.clone(),
                processing_time_ms: Some(self.response_delay_ms),
                cached: false,
                plugin_metadata: json!({}),
            },
        })
    }

    fn configure(&self, _config: Value) -> PluginResult<()> {
        Ok(())
    }

    fn tool_definitions(&self) -> Vec<Value> {
        vec![]
    }
}

/// Create a registry with multiple plugins of varying priorities
fn create_benchmark_registry(num_plugins: usize, use_priorities: bool) -> PluginRegistry {
    let mut registry = PluginRegistry::new();

    for i in 0..num_plugins {
        let priority = if use_priorities {
            50 + (i as u32 * 10)
        } else {
            50
        };

        let plugin = Arc::new(BenchmarkPlugin::new(
            &format!("bench-plugin-{}", i),
            vec!["ts".to_string(), "js".to_string()],
            priority,
            0, // No delay for selection benchmarks
        ));

        registry
            .register_plugin(format!("bench-plugin-{}", i), plugin)
            .unwrap();
    }

    registry
}

/// Benchmark: Plugin selection with varying numbers of plugins
fn bench_plugin_selection(c: &mut Criterion) {
    let mut group = c.benchmark_group("plugin_selection");

    for num_plugins in [1, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("same_priority", num_plugins),
            num_plugins,
            |b, &num| {
                let registry = create_benchmark_registry(num, false);
                let file_path = PathBuf::from("test.ts");

                b.iter(|| {
                    let result = registry
                        .find_best_plugin(black_box(&file_path), black_box("find_definition"));
                    black_box(result)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("different_priorities", num_plugins),
            num_plugins,
            |b, &num| {
                let registry = create_benchmark_registry(num, true);
                let file_path = PathBuf::from("test.ts");

                b.iter(|| {
                    let result = registry
                        .find_best_plugin(black_box(&file_path), black_box("find_definition"));
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Priority override performance
fn bench_priority_override(c: &mut Criterion) {
    let mut registry = create_benchmark_registry(10, false);

    c.bench_function("priority_override_lookup", |b| {
        let mut overrides = HashMap::new();
        overrides.insert("bench-plugin-5".to_string(), 100);
        registry.set_priority_overrides(overrides);

        let file_path = PathBuf::from("test.ts");

        b.iter(|| {
            let result =
                registry.find_best_plugin(black_box(&file_path), black_box("find_definition"));
            black_box(result)
        });
    });
}

/// Benchmark: Dispatch latency for simple tools
fn bench_dispatch_simple_tool(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let dispatcher = create_test_dispatcher();

    c.bench_function("dispatch_simple_tool", |b| {
        b.to_async(&rt).iter(|| async {
            let tool_call = ToolCall {
                name: "list_files".to_string(),
                arguments: Some(json!({
                    "path": ".",
                    "recursive": false
                })),
            };

            let result = dispatcher.benchmark_tool_call(Some(json!(tool_call))).await;
            black_box(result)
        });
    });
}

/// Benchmark: Dispatch latency for complex tools
fn bench_dispatch_complex_tool(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let dispatcher = create_test_dispatcher();

    c.bench_function("dispatch_complex_tool", |b| {
        b.to_async(&rt).iter(|| async {
            let tool_call = ToolCall {
                name: "find_definition".to_string(),
                arguments: Some(json!({
                    "file_path": "test.ts",
                    "line": 10,
                    "character": 5
                })),
            };

            let result = dispatcher.benchmark_tool_call(Some(json!(tool_call))).await;
            black_box(result)
        });
    });
}

/// Benchmark: Concurrent dispatch throughput
fn bench_dispatch_concurrency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("dispatch_concurrency");

    for num_concurrent in [1, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_concurrent),
            num_concurrent,
            |b, &num| {
                let dispatcher = Arc::new(create_test_dispatcher());

                b.to_async(&rt).iter(|| {
                    let dispatcher = dispatcher.clone();
                    async move {
                        let mut handles = vec![];

                        for i in 0..num {
                            let dispatcher = dispatcher.clone();
                            let handle = tokio::spawn(async move {
                                let tool_call = ToolCall {
                                    name: "list_files".to_string(),
                                    arguments: Some(json!({
                                        "path": format!("./dir_{}", i),
                                        "recursive": false
                                    })),
                                };

                                dispatcher.benchmark_tool_call(Some(json!(tool_call))).await
                            });
                            handles.push(handle);
                        }

                        for handle in handles {
                            let _ = handle.await;
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Registry initialization overhead
fn bench_registry_initialization(c: &mut Criterion) {
    c.bench_function("registry_initialization", |b| {
        b.iter(|| {
            let registry = create_benchmark_registry(black_box(10), black_box(true));
            black_box(registry)
        });
    });
}

/// Benchmark: Tool scope detection
fn bench_tool_scope_detection(c: &mut Criterion) {
    let capabilities = Capabilities::default();

    c.bench_function("tool_scope_detection_file", |b| {
        b.iter(|| {
            let scope = capabilities.get_tool_scope(black_box("find_definition"));
            black_box(scope)
        });
    });

    c.bench_function("tool_scope_detection_workspace", |b| {
        b.iter(|| {
            let scope = capabilities.get_tool_scope(black_box("search_workspace_symbols"));
            black_box(scope)
        });
    });
}

criterion_group!(
    benches,
    bench_plugin_selection,
    bench_priority_override,
    bench_dispatch_simple_tool,
    bench_dispatch_complex_tool,
    bench_dispatch_concurrency,
    bench_registry_initialization,
    bench_tool_scope_detection
);
criterion_main!(benches);
