use cb_core::model::lsp::{
    CompletionItem, CompletionList, Diagnostic, DiagnosticSeverity, DocumentSymbol, Hover,
    Location, Position, Range, SymbolKind, TextEdit, WorkspaceEdit,
};
use cb_server::helpers::lsp::forward_lsp_request;
use cb_tests::harness::lsp::TestLspService;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn create_test_lsp_service() -> Arc<TestLspService> {
    let mut service = TestLspService::new();

    // Pre-configure some mock responses for common LSP methods
    service.add_response(
        "textDocument/hover",
        json!({
            "contents": {
                "kind": "markdown",
                "value": "```rust\nfn main()\n```\n\nThe main entry point"
            },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 4 }
            }
        }),
    );

    service.add_response(
        "textDocument/completion",
        json!({
            "isIncomplete": false,
            "items": [
                {
                    "label": "println!",
                    "kind": 3,
                    "detail": "macro",
                    "documentation": "Prints to stdout"
                },
                {
                    "label": "vec!",
                    "kind": 3,
                    "detail": "macro",
                    "documentation": "Creates a Vec"
                }
            ]
        }),
    );

    service.add_response(
        "textDocument/definition",
        json!([{
            "uri": "file:///tmp/src/main.rs",
            "range": {
                "start": { "line": 10, "character": 0 },
                "end": { "line": 15, "character": 0 }
            }
        }]),
    );

    Arc::new(service)
}

fn bench_forward_simple_request(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let lsp_service = create_test_lsp_service();

    c.bench_function("forward_simple_hover", |b| {
        b.to_async(&rt).iter(|| {
            let service = lsp_service.clone();
            async move {
                let params = json!({
                    "textDocument": { "uri": "file:///tmp/test.rs" },
                    "position": { "line": 5, "character": 10 }
                });
                let _ = forward_lsp_request::<Value, Value>(
                    black_box(service.as_ref()),
                    black_box("textDocument/hover"),
                    black_box(params),
                )
                .await;
            }
        });
    });
}

fn bench_forward_complex_request(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let lsp_service = create_test_lsp_service();

    // Add a complex response with many symbols
    let mut symbols = Vec::new();
    for i in 0..100 {
        symbols.push(json!({
            "name": format!("symbol_{}", i),
            "kind": 12,  // Function
            "range": {
                "start": { "line": i * 10, "character": 0 },
                "end": { "line": i * 10 + 5, "character": 0 }
            },
            "selectionRange": {
                "start": { "line": i * 10, "character": 4 },
                "end": { "line": i * 10, "character": 20 }
            }
        }));
    }

    lsp_service.add_response("textDocument/documentSymbol", json!(symbols));

    c.bench_function("forward_complex_symbols", |b| {
        b.to_async(&rt).iter(|| {
            let service = lsp_service.clone();
            async move {
                let params = json!({
                    "textDocument": { "uri": "file:///tmp/large_file.rs" }
                });
                let _ = forward_lsp_request::<Value, Value>(
                    black_box(service.as_ref()),
                    black_box("textDocument/documentSymbol"),
                    black_box(params),
                )
                .await;
            }
        });
    });
}

fn bench_forward_workspace_edit(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let lsp_service = create_test_lsp_service();

    // Create a workspace edit with multiple file changes
    let mut changes = HashMap::new();
    for i in 0..10 {
        let file_uri = format!("file:///tmp/file_{}.rs", i);
        let mut edits = Vec::new();
        for j in 0..5 {
            edits.push(json!({
                "range": {
                    "start": { "line": j * 10, "character": 0 },
                    "end": { "line": j * 10 + 1, "character": 0 }
                },
                "newText": format!("// Modified line {}\n", j)
            }));
        }
        changes.insert(file_uri, edits);
    }

    lsp_service.add_response(
        "workspace/executeCommand",
        json!({
            "changes": changes
        }),
    );

    c.bench_function("forward_workspace_edit", |b| {
        b.to_async(&rt).iter(|| {
            let service = lsp_service.clone();
            async move {
                let params = json!({
                    "command": "refactor.rename",
                    "arguments": ["oldName", "newName"]
                });
                let _ = forward_lsp_request::<Value, Value>(
                    black_box(service.as_ref()),
                    black_box("workspace/executeCommand"),
                    black_box(params),
                )
                .await;
            }
        });
    });
}

fn bench_forward_parallel_requests(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let lsp_service = create_test_lsp_service();

    c.bench_function("forward_parallel_10", |b| {
        b.to_async(&rt).iter(|| {
            let service = lsp_service.clone();
            async move {
                let mut handles = vec![];
                for i in 0..10 {
                    let service = service.clone();
                    let params = json!({
                        "textDocument": { "uri": format!("file:///tmp/test{}.rs", i) },
                        "position": { "line": i, "character": 0 }
                    });
                    let handle = tokio::spawn(async move {
                        let _ = forward_lsp_request::<Value, Value>(
                            service.as_ref(),
                            "textDocument/hover",
                            params,
                        )
                        .await;
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

fn bench_forward_with_error(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let lsp_service = create_test_lsp_service();

    // Configure an error response
    lsp_service.add_error_response(
        "textDocument/formatting",
        -32603,
        "Formatter not available",
    );

    c.bench_function("forward_error_response", |b| {
        b.to_async(&rt).iter(|| {
            let service = lsp_service.clone();
            async move {
                let params = json!({
                    "textDocument": { "uri": "file:///tmp/unformattable.rs" },
                    "options": { "tabSize": 4, "insertSpaces": true }
                });
                let _ = forward_lsp_request::<Value, Value>(
                    black_box(service.as_ref()),
                    black_box("textDocument/formatting"),
                    black_box(params),
                )
                .await;
            }
        });
    });
}

fn bench_forward_various_sizes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let lsp_service = create_test_lsp_service();

    let mut group = c.benchmark_group("forward_response_sizes");

    for size in [10, 100, 1000, 10000].iter() {
        // Create a response with the specified number of items
        let mut items = Vec::new();
        for i in 0..*size {
            items.push(json!({
                "uri": format!("file:///tmp/result_{}.rs", i),
                "score": 1.0 / (i as f64 + 1.0)
            }));
        }

        lsp_service.add_response("workspace/symbol", json!(items));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.to_async(&rt).iter(|| {
                let service = lsp_service.clone();
                async move {
                    let params = json!({
                        "query": "search_term"
                    });
                    let _ = forward_lsp_request::<Value, Value>(
                        service.as_ref(),
                        "workspace/symbol",
                        params,
                    )
                    .await;
                }
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_forward_simple_request,
    bench_forward_complex_request,
    bench_forward_workspace_edit,
    bench_forward_parallel_requests,
    bench_forward_with_error,
    bench_forward_various_sizes
);
criterion_main!(benches);