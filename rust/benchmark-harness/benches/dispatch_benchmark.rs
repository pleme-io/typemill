use cb_core::config::LspConfig;
use cb_core::model::mcp::{McpRequest, ToolCall};
use cb_server::handlers::{AppState, McpDispatcher};
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
    let dispatcher = McpDispatcher::new(app_state);
    let tool_call = create_simple_tool_call();

    c.bench_function("dispatch_simple_tool", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = dispatcher.call_tool(black_box(&tool_call)).await;
        });
    });
}

fn bench_dispatch_complex(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let app_state = create_test_app_state();
    let dispatcher = McpDispatcher::new(app_state);
    let tool_call = create_complex_tool_call();

    c.bench_function("dispatch_complex_tool", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = dispatcher.call_tool(black_box(&tool_call)).await;
        });
    });
}

fn bench_dispatch_parallel(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let app_state = create_test_app_state();
    let dispatcher = Arc::new(McpDispatcher::new(app_state));

    c.bench_function("dispatch_parallel_tools", |b| {
        b.to_async(&rt).iter(|| {
            let dispatcher = dispatcher.clone();
            async move {
                let mut handles = vec![];
                for _ in 0..10 {
                    let dispatcher = dispatcher.clone();
                    let tool_call = create_simple_tool_call();
                    let handle = tokio::spawn(async move {
                        let _ = dispatcher.call_tool(&tool_call).await;
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