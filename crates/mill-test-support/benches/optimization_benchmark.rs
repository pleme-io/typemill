use mill_server::utils::{ create_paginated_response , SimdJsonParser };
use codebuddy_foundation::core::model::mcp::{McpRequest, McpResponse};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde_json::{json, Value};

/// Benchmark comparing serde_json vs simd-json for complex deserialization
fn bench_simd_vs_serde_complex_deserialization(c: &mut Criterion) {
    // Create a complex JSON response with 50 reference locations (typical find_references)
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

    let complex_response = McpResponse {
        id: Some(json!("bench-456")),
        result: Some(json!({
            "references": locations,
            "total_count": 50,
            "search_time_ms": 125
        })),
        error: None,
    };

    let json_string = serde_json::to_string(&complex_response).unwrap();
    let json_bytes = json_string.as_bytes().to_vec();

    let mut group = c.benchmark_group("complex_deserialization");

    // Benchmark serde_json (baseline)
    group.bench_function("serde_json", |b| {
        b.iter(|| {
            let _: McpResponse = serde_json::from_str(black_box(&json_string)).unwrap();
        });
    });

    // Benchmark simd-json (optimized)
    group.bench_function("simd_json", |b| {
        b.iter(|| {
            let bytes = black_box(json_bytes.clone());
            let _: McpResponse = SimdJsonParser::from_slice(bytes).unwrap_or_else(|_| {
                // Fallback for benchmark consistency
                serde_json::from_str(&json_string).unwrap()
            });
        });
    });

    group.finish();
}

/// Benchmark pagination effectiveness for large result sets
fn bench_pagination_vs_full_response(c: &mut Criterion) {
    // Create a large array of references (200 items)
    let mut all_references = Vec::new();
    for i in 0..200 {
        all_references.push(json!({
            "uri": format!("file:///tmp/large_project/module{}/file{}.rs", i / 50, i),
            "range": {
                "start": { "line": i * 5, "character": 0 },
                "end": { "line": i * 5 + 2, "character": 40 }
            },
            "context": format!("Reference context for symbol_{}", i)
        }));
    }

    let mut group = c.benchmark_group("pagination_vs_full");

    // Benchmark full response serialization (baseline)
    group.bench_function("full_response_serialize", |b| {
        b.iter(|| {
            let response = json!({
                "references": all_references,
                "total_count": 200
            });
            let _ = serde_json::to_string(black_box(&response)).unwrap();
        });
    });

    // Benchmark paginated response serialization (optimized)
    group.bench_function("paginated_response_serialize", |b| {
        b.iter(|| {
            let response = create_paginated_response(
                black_box(all_references.clone()),
                50,  // page_size
                0,   // page
                200, // total_count
            );
            let _ = serde_json::to_string(&response).unwrap();
        });
    });

    // Benchmark paginated response deserialization
    let paginated_json = serde_json::to_string(&create_paginated_response(
        all_references.clone(),
        50,
        0,
        200,
    ))
    .unwrap();

    group.bench_function("paginated_response_deserialize", |b| {
        b.iter(|| {
            let _: Value = serde_json::from_str(black_box(&paginated_json)).unwrap();
        });
    });

    group.finish();
}

/// Benchmark the complete MCP request-response cycle with optimizations
fn bench_complete_mcp_cycle_optimized(c: &mut Criterion) {
    // Complex find_references request
    let request = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!("cycle-test")),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "find_references",
            "arguments": {
                "file_path": "/tmp/large_project/main.rs",
                "symbol_name": "process_data",
                "include_declaration": true,
                "page": 0,
                "page_size": 25
            }
        })),
    };

    // Mock complex response
    let mut references = Vec::new();
    for i in 0..100 {
        references.push(json!({
            "uri": format!("file:///tmp/large_project/module{}.rs", i),
            "range": {
                "start": { "line": i * 3, "character": 4 },
                "end": { "line": i * 3, "character": 20 }
            }
        }));
    }

    let response = McpResponse {
        id: Some(json!("cycle-test")),
        result: Some(create_paginated_response(references, 25, 0, 100)),
        error: None,
    };

    c.bench_function("complete_mcp_cycle_optimized", |b| {
        b.iter(|| {
            // Serialize request
            let request_json = serde_json::to_string(black_box(&request)).unwrap();

            // Deserialize request (using SIMD optimization)
            let req_bytes = request_json.clone().into_bytes();
            let _parsed_req: McpRequest = SimdJsonParser::from_slice(req_bytes)
                .unwrap_or_else(|_| serde_json::from_str(&request_json).unwrap());

            // Serialize paginated response
            let response_json = serde_json::to_string(black_box(&response)).unwrap();

            // Deserialize response
            let resp_bytes = response_json.clone().into_bytes();
            let _parsed_resp: McpResponse = SimdJsonParser::from_slice(resp_bytes)
                .unwrap_or_else(|_| serde_json::from_str(&response_json).unwrap());
        });
    });
}

/// Benchmark JSON array processing with different sizes to identify scaling
fn bench_json_array_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_array_scaling");

    for size in [10, 50, 100, 500, 1000].iter() {
        let mut items = Vec::new();
        for i in 0..*size {
            items.push(json!({
                "id": i,
                "path": format!("/project/file_{}.rs", i),
                "metadata": {
                    "size": i * 100,
                    "modified": "2024-01-01T00:00:00Z"
                }
            }));
        }

        let json_string = serde_json::to_string(&items).unwrap();
        let json_bytes = json_string.as_bytes().to_vec();

        group.bench_with_input(
            criterion::BenchmarkId::new("serde_json", size),
            size,
            |b, _| {
                b.iter(|| {
                    let _: Vec<Value> = serde_json::from_str(black_box(&json_string)).unwrap();
                });
            },
        );

        group.bench_with_input(
            criterion::BenchmarkId::new("simd_json", size),
            size,
            |b, _| {
                b.iter(|| {
                    let bytes = black_box(json_bytes.clone());
                    let _: Vec<Value> = SimdJsonParser::from_slice(bytes)
                        .unwrap_or_else(|_| serde_json::from_str(&json_string).unwrap());
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_simd_vs_serde_complex_deserialization,
    bench_pagination_vs_full_response,
    bench_complete_mcp_cycle_optimized,
    bench_json_array_scaling
);
criterion_main!(benches);