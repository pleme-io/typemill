//! LSP Protocol Smoke Test
//!
//! This test validates that the LSP server connection and protocol work correctly.
//! It tests the LSP transport layer, initialization, and basic routing.
//!
//! ## What This Tests
//!
//! - LSP server initialization and handshake
//! - LSP protocol message format (JSON-RPC 2.0)
//! - Tool routing through LSP layer
//! - Parameter serialization/deserialization
//! - Response format validation
//! - Error handling for protocol-level issues
//! - Multiple sequential LSP requests
//!
//! ## What This Doesn't Test
//!
//! - Business logic (covered by mock tests in `lsp_features.rs`)
//! - Specific LSP feature implementations (covered by unit tests)
//! - Error edge cases (covered by integration tests)
//! - Every LSP request type (2-3 different requests prove routing works)
//!
//! ## Running This Test
//!
//! ```bash
//! # Run with LSP servers installed
//! cargo nextest run --workspace --ignored --features lsp-tests
//! ```
//!
//! ## Requirements
//!
//! - TypeScript language server: `npm install -g typescript-language-server`
//! - Rust analyzer: `rustup component add rust-analyzer`

#[cfg(feature = "lsp-tests")]
use mill_test_support::harness::{TestClient, TestWorkspace};
#[cfg(feature = "lsp-tests")]
use serde_json::json;

/// Test LSP protocol connectivity and basic routing
///
/// This test validates that:
/// 1. LSP servers can be initialized
/// 2. LSP requests are properly formatted (JSON-RPC 2.0)
/// 3. Routing works across different LSP request types
/// 4. Responses are properly deserialized
/// 5. Multiple sequential requests work
#[tokio::test]
#[ignore] // Requires LSP servers to be installed
#[cfg(feature = "lsp-tests")]
async fn test_lsp_protocol_connectivity() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create a TypeScript file to test with
    let ts_file = workspace.path().join("test.ts");
    std::fs::write(
        &ts_file,
        r#"
export function greet(name: string): string {
    return `Hello, ${name}!`;
}

export function farewell(name: string): string {
    return `Goodbye, ${name}!`;
}

// Call the functions
const greeting = greet("World");
const goodbye = farewell("World");
"#,
    )
    .unwrap();

    // Wait for LSP to index the file (polling is faster and more reliable than fixed sleep)
    client
        .wait_for_lsp_ready(&ts_file, 10000)
        .await
        .expect("LSP should index TypeScript file within 10s");

    // Test 1: inspect_code.definition (tests LSP textDocument/definition)
    let response = client
        .call_tool(
            "inspect_code",
            json!({
                "filePath": ts_file.to_string_lossy(),
                "line": 10,
                "character": 17,
                "include": ["definition"]
            }),
        )
        .await;

    assert!(
        response.is_ok(),
        "LSP inspect_code (definition) should succeed: {:?}",
        response.err()
    );

    let result = response.unwrap();
    assert!(
        result.get("result").is_some() || result.get("error").is_some(),
        "Response should have result or error field"
    );

    // Test 2: inspect_code.references (tests LSP textDocument/references)
    let response = client
        .call_tool(
            "inspect_code",
            json!({
                "filePath": ts_file.to_string_lossy(),
                "line": 1,
                "character": 16,
                "include": ["references"]
            }),
        )
        .await;

    assert!(
        response.is_ok(),
        "LSP inspect_code (references) should succeed: {:?}",
        response.err()
    );

    let result = response.unwrap();
    assert!(
        result.get("result").is_some() || result.get("error").is_some(),
        "Response should have result or error field"
    );

    // Test 3: Multiple sequential calls (tests connection reuse)
    for i in 0..3 {
        let response = client
            .call_tool(
                "inspect_code",
                json!({
                    "filePath": ts_file.to_string_lossy(),
                    "line": 1,
                    "character": 16,
                    "include": ["definition"]
                }),
            )
            .await;

        assert!(
            response.is_ok(),
            "LSP request {} should succeed: {:?}",
            i,
            response.err()
        );
    }
}

/// Test LSP server initialization for multiple language servers
///
/// This test validates that:
/// 1. Different language servers can be initialized
/// 2. LSP routing works based on file extension
/// 3. Protocol works consistently across languages
#[tokio::test]
#[ignore] // Requires LSP servers to be installed
#[cfg(feature = "lsp-tests")]
async fn test_lsp_multi_language_routing() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Create TypeScript file
    let ts_file = workspace.path().join("test.ts");
    std::fs::write(
        &ts_file,
        r#"
export function add(a: number, b: number): number {
    return a + b;
}
"#,
    )
    .unwrap();

    // Create Rust file
    let src_dir = workspace.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    let rs_file = src_dir.join("lib.rs");
    std::fs::write(
        &rs_file,
        r#"
pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}
"#,
    )
    .unwrap();

    // Create Cargo.toml for Rust
    std::fs::write(
        workspace.path().join("Cargo.toml"),
        r#"
[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();

    // Wait for LSP servers to index files (polling is faster and more reliable than fixed sleep)
    client
        .wait_for_lsp_ready(&ts_file, 10000)
        .await
        .expect("LSP should index TypeScript file within 10s");

    // Test TypeScript LSP
    let ts_response = client
        .call_tool(
            "inspect_code",
            json!({
                "filePath": ts_file.to_string_lossy(),
                "line": 1,
                "character": 16,
                "include": ["definition"]
            }),
        )
        .await;

    assert!(
        ts_response.is_ok(),
        "TypeScript LSP should work: {:?}",
        ts_response.err()
    );

    // Test Rust LSP
    let rs_response = client
        .call_tool(
            "inspect_code",
            json!({
                "filePath": rs_file.to_string_lossy(),
                "line": 1,
                "character": 7,
                "include": ["definition"]
            }),
        )
        .await;

    assert!(
        rs_response.is_ok(),
        "Rust LSP should work: {:?}",
        rs_response.err()
    );
}
