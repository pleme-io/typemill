use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mill_foundation::core::model::mcp::{McpError, McpRequest, McpResponse};
use serde_json::json;

fn create_simple_request() -> McpRequest {
    McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!("bench-123")),
        method: "tools/call".to_string(),
        params: Some(json!({
            "tool": "get_project_info",
            "arguments": {}
        })),
    }
}

fn create_complex_request() -> McpRequest {
    McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!("bench-456")),
        method: "tools/call".to_string(),
        params: Some(json!({
            "tool": "find_references",
            "arguments": {
                "filePath": "/tmp/very/long/path/to/some/deeply/nested/source/file.rs",
                "symbol_name": "VeryLongSymbolNameThatRepresentsARealWorldScenario",
                "include_declaration": true,
                "additional_options": {
                    "max_results": 1000,
                    "include_tests": true,
                    "include_docs": false,
                    "search_depth": 10
                }
            }
        })),
    }
}

fn create_simple_response() -> McpResponse {
    McpResponse {
        id: Some(json!("bench-123")),
        result: Some(json!({
            "project": "mill",
            "version": "0.1.0"
        })),
        error: None,
    }
}

fn create_complex_response() -> McpResponse {
    let mut locations = Vec::new();
    for i in 0..50 {
        locations.push(json!({
            "uri": format!("file:///tmp/src/module{}/file{}.rs", i / 10, i),
            "range": {
                "start": { "line": i * 10, "character": 0 },
                "end": { "line": i * 10 + 5, "character": 80 }
            }
        }));
    }

    McpResponse {
        id: Some(json!("bench-456")),
        result: Some(json!({
            "references": locations,
            "total_count": 50,
            "search_time_ms": 125
        })),
        error: None,
    }
}

fn create_error_response() -> McpResponse {
    McpResponse {
        id: Some(json!("bench-789")),
        result: None,
        error: Some(McpError {
            code: -32603,
            message: "Internal server error: Failed to connect to language server after 3 attempts"
                .to_string(),
            data: Some(json!({
                "attempts": 3,
                "last_error": "Connection refused",
                "server": "typescript-language-server",
                "file_type": "typescript"
            })),
        }),
    }
}

fn bench_serialize_requests(c: &mut Criterion) {
    let simple_request = create_simple_request();
    let complex_request = create_complex_request();

    let mut group = c.benchmark_group("serialize_requests");

    group.bench_function("simple", |b| {
        b.iter(|| {
            let _ = serde_json::to_string(black_box(&simple_request));
        });
    });

    group.bench_function("complex", |b| {
        b.iter(|| {
            let _ = serde_json::to_string(black_box(&complex_request));
        });
    });

    group.bench_function("simple_pretty", |b| {
        b.iter(|| {
            let _ = serde_json::to_string_pretty(black_box(&simple_request));
        });
    });

    group.finish();
}

fn bench_serialize_responses(c: &mut Criterion) {
    let simple_response = create_simple_response();
    let complex_response = create_complex_response();
    let error_response = create_error_response();

    let mut group = c.benchmark_group("serialize_responses");

    group.bench_function("simple", |b| {
        b.iter(|| {
            let _ = serde_json::to_string(black_box(&simple_response));
        });
    });

    group.bench_function("complex", |b| {
        b.iter(|| {
            let _ = serde_json::to_string(black_box(&complex_response));
        });
    });

    group.bench_function("error", |b| {
        b.iter(|| {
            let _ = serde_json::to_string(black_box(&error_response));
        });
    });

    group.finish();
}

fn bench_deserialize_requests(c: &mut Criterion) {
    let simple_json = serde_json::to_string(&create_simple_request()).unwrap();
    let complex_json = serde_json::to_string(&create_complex_request()).unwrap();

    let mut group = c.benchmark_group("deserialize_requests");

    group.bench_function("simple", |b| {
        b.iter(|| {
            let _: McpRequest = serde_json::from_str(black_box(&simple_json)).unwrap();
        });
    });

    group.bench_function("complex", |b| {
        b.iter(|| {
            let _: McpRequest = serde_json::from_str(black_box(&complex_json)).unwrap();
        });
    });

    group.finish();
}

fn bench_deserialize_responses(c: &mut Criterion) {
    let simple_json = serde_json::to_string(&create_simple_response()).unwrap();
    let complex_json = serde_json::to_string(&create_complex_response()).unwrap();
    let error_json = serde_json::to_string(&create_error_response()).unwrap();

    let mut group = c.benchmark_group("deserialize_responses");

    group.bench_function("simple", |b| {
        b.iter(|| {
            let _: McpResponse = serde_json::from_str(black_box(&simple_json)).unwrap();
        });
    });

    group.bench_function("complex", |b| {
        b.iter(|| {
            let _: McpResponse = serde_json::from_str(black_box(&complex_json)).unwrap();
        });
    });

    group.bench_function("error", |b| {
        b.iter(|| {
            let _: McpResponse = serde_json::from_str(black_box(&error_json)).unwrap();
        });
    });

    group.finish();
}

fn bench_value_manipulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("value_manipulation");

    group.bench_function("create_nested_json", |b| {
        b.iter(|| {
            let _ = json!({
                "level1": {
                    "level2": {
                        "level3": {
                            "data": vec![1, 2, 3, 4, 5],
                            "text": "Some text data",
                            "flag": true
                        }
                    }
                }
            });
        });
    });

    group.bench_function("access_nested_field", |b| {
        let data = json!({
            "level1": {
                "level2": {
                    "level3": {
                        "target": "found"
                    }
                }
            }
        });
        b.iter(|| {
            let _ = black_box(&data["level1"]["level2"]["level3"]["target"]);
        });
    });

    group.bench_function("modify_json_value", |b| {
        b.iter(|| {
            let mut data = json!({"count": 0});
            for i in 0..10 {
                data["count"] = json!(i);
            }
            black_box(data);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_serialize_requests,
    bench_serialize_responses,
    bench_deserialize_requests,
    bench_deserialize_responses,
    bench_value_manipulation
);
criterion_main!(benches);
