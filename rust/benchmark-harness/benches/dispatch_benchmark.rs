// BENCHMARK DISABLED - Old McpDispatcher has been replaced by plugin system
// TODO: Rewrite benchmarks for new PluginDispatcher architecture
#![cfg(skip_benchmarks)]

use cb_core::config::LspConfig;
use cb_core::model::mcp::{McpMessage, McpRequest, ToolCall};
// NOTE: McpDispatcher no longer exists - replaced by PluginDispatcher
// use cb_server::handlers::{AppState, McpDispatcher};
use cb_server::handlers::{AppState, PluginDispatcher};
use cb_server::services::{FileService, LockManager, OperationQueue};
use cb_server::systems::LspManager;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn create_test_app_state() -> Arc<AppState> {
    let lsp_config = LspConfig::default();
    let lsp_manager = Arc::new(LspManager::new(lsp_config));
    let file_service = Arc::new(FileService::new(PathBuf::from("/tmp")));
    let project_root = PathBuf::from("/tmp");
    let lock_manager = Arc::new(LockManager::new());
    let operation_queue = Arc::new(OperationQueue::new(lock_manager.clone()));

    Arc::new(AppState {
        lsp: lsp_manager,
        file_service,
        project_root,
        lock_manager,
        operation_queue,
    })
}

fn create_simple_tool_call() -> ToolCall {
    ToolCall {
        name: "get_project_info".to_string(),
        arguments: Some(json!({})),
    }
}

fn create_complex_tool_call() -> ToolCall {
    ToolCall {
        name: "find_references".to_string(),
        arguments: Some(json!({
            "file_path": "/tmp/test.rs",
            "symbol_name": "main",
            "include_declaration": true
        })),
    }
}

fn bench_dispatch_simple(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let app_state = create_test_app_state();
    let mut dispatcher = McpDispatcher::new(app_state);

    // Register a simple mock tool
    dispatcher.register_tool("get_project_info".to_string(), |_app_state, _args| async move {
        Ok(json!({"status": "success"}))
    });

    c.bench_function("dispatch_simple_tool", |b| {
        b.to_async(&rt).iter(|| async {
            let request = McpRequest {
                id: Some(json!("bench-123")),
                method: "tools/call".to_string(),
                params: Some(json!({
                    "name": "get_project_info",
                    "arguments": {}
                })),
            };
            let message = McpMessage::Request(request);
            let _ = dispatcher.dispatch(black_box(message)).await;
        });
    });
}

fn bench_dispatch_complex(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let app_state = create_test_app_state();
    let mut dispatcher = McpDispatcher::new(app_state);

    // Register a complex mock tool
    dispatcher.register_tool("find_references".to_string(), |_app_state, _args| async move {
        let mut references = Vec::new();
        for i in 0..50 {
            references.push(json!({
                "uri": format!("file:///tmp/file_{}.rs", i),
                "range": {
                    "start": { "line": i, "character": 0 },
                    "end": { "line": i, "character": 10 }
                }
            }));
        }
        Ok(json!({ "references": references }))
    });

    c.bench_function("dispatch_complex_tool", |b| {
        b.to_async(&rt).iter(|| async {
            let request = McpRequest {
                id: Some(json!("bench-456")),
                method: "tools/call".to_string(),
                params: Some(json!({
                    "name": "find_references",
                    "arguments": {
                        "file_path": "/tmp/test.rs",
                        "symbol_name": "main",
                        "include_declaration": true
                    }
                })),
            };
            let message = McpMessage::Request(request);
            let _ = dispatcher.dispatch(black_box(message)).await;
        });
    });
}

fn bench_dispatch_parallel(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let app_state = create_test_app_state();
    let mut dispatcher = McpDispatcher::new(app_state);

    // Register a simple tool for parallel testing
    dispatcher.register_tool("parallel_test".to_string(), |_app_state, _args| async move {
        // Small delay to make concurrency visible
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        Ok(json!({"status": "parallel_success"}))
    });

    let dispatcher = Arc::new(dispatcher);

    c.bench_function("dispatch_parallel_tools", |b| {
        b.to_async(&rt).iter(|| {
            let dispatcher = dispatcher.clone();
            async move {
                let mut handles = vec![];
                for _ in 0..10 {
                    let dispatcher = dispatcher.clone();
                    let handle = tokio::spawn(async move {
                        let request = McpRequest {
                            id: Some(json!("bench-parallel")),
                            method: "tools/call".to_string(),
                            params: Some(json!({
                                "name": "parallel_test",
                                "arguments": {}
                            })),
                        };
                        let message = McpMessage::Request(request);
                        let _ = dispatcher.dispatch(message).await;
                    });
                    handles.push(handle);
                }
                for handle in handles {
                    let _ = handle.await;
                }
            }
        });
    });
}

fn bench_create_dispatcher(c: &mut Criterion) {
    let app_state = create_test_app_state();

    c.bench_function("create_dispatcher", |b| {
        b.iter(|| {
            let _ = McpDispatcher::new(black_box(app_state.clone()));
        });
    });
}

criterion_group!(
    benches,
    bench_dispatch_simple,
    bench_dispatch_complex,
    bench_dispatch_parallel,
    bench_create_dispatcher
);
criterion_main!(benches);