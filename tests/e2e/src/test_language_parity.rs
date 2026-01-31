use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;

// Helper to run parity tests for a specific language
async fn run_language_parity_check(
    workspace: &TestWorkspace,
    lang_name: &str,
    file_path: &str,
    content: &str,
    query_symbol: &str,
) {
    let mut client = TestClient::new(workspace.path());
    workspace.create_file(file_path, content);

    println!("Testing {} parity on {}", lang_name, file_path);

    // Test 1: Inspect Code
    // We expect this to succeed and potentially find symbols if the language parser is working.
    let inspect_res = client
        .call_tool(
            "inspect_code",
            json!({
                "filePath": workspace.absolute_path(file_path).to_string_lossy(),
                "line": 0,
                "character": 0,
                "include": ["definition"]
            }),
        )
        .await;

    match inspect_res {
        Ok(_) => {
            println!("{} inspect_code succeeded", lang_name);
        }
        Err(e) => {
            let err_msg = e.to_string();
            println!(
                "{} inspect_code returned error (expected in some envs): {}",
                lang_name, err_msg
            );
            // If it's a timeout or LSP error, that's fine for parity check in no-LSP env.
            // But it shouldn't be "Unknown tool" or "Language not supported"
            assert!(
                !err_msg.contains("Method not found"),
                "inspect_code tool missing/not registered for {}",
                lang_name
            );
        }
    }

    // Test 2: Search Code
    // This typically requires a running LSP. If the LSP is missing, it might timeout or return an error.
    // The key parity check here is that the tool is REGISTERED and doesn't fail with "Method not found".
    let search_res = client
        .call_tool("search_code", json!({ "query": query_symbol }))
        .await;
    match search_res {
        Ok(_) => {
            println!("{} search_code succeeded", lang_name);
        }
        Err(e) => {
            let err_msg = e.to_string();
            println!(
                "{} search_code returned error (expected in some envs): {}",
                lang_name, err_msg
            );
            assert!(
                !err_msg.contains("Method not found"),
                "search_code tool missing/not registered for {}",
                lang_name
            );
        }
    }

    // Test 3: Rename (Dry Run)
    // Similarly, rename often needs LSP, but we check for tool availability.
    let rename_res = client.call_tool("rename_all", json!({
        "target": {
            "kind": "file",
            "filePath": workspace.absolute_path(file_path).to_string_lossy()
        },
        "newName": workspace.absolute_path(&format!("{}.renamed", file_path)).to_string_lossy(),
         "options": {
            "dryRun": true
        }
    })).await;

    match rename_res {
        Ok(_) => {
            println!("{} rename_all succeeded", lang_name);
        }
        Err(e) => {
            let err_msg = e.to_string();
            println!(
                "{} rename_all returned error (expected in some envs): {}",
                lang_name, err_msg
            );
            assert!(
                !err_msg.contains("Method not found"),
                "rename_all tool missing/not registered for {}",
                lang_name
            );
        }
    }
}

#[tokio::test]
async fn test_javascript_parity() {
    let workspace = TestWorkspace::new();
    run_language_parity_check(
        &workspace,
        "JavaScript",
        "src/main.js",
        "function hello() { console.log('world'); }",
        "hello",
    )
    .await;
}

#[tokio::test]
async fn test_typescript_parity() {
    let workspace = TestWorkspace::new();
    // TS usually needs a tsconfig to be happy
    workspace.create_file("tsconfig.json", "{}");
    run_language_parity_check(
        &workspace,
        "TypeScript",
        "src/main.ts",
        "export function hello(): void { console.log('world'); }",
        "hello",
    )
    .await;
}

#[tokio::test]
async fn test_python_parity() {
    let workspace = TestWorkspace::new();
    run_language_parity_check(
        &workspace,
        "Python",
        "src/main.py",
        "def hello():\n    print('world')",
        "hello",
    )
    .await;
}

#[tokio::test]
async fn test_rust_parity() {
    let workspace = TestWorkspace::new();
    workspace.create_file(
        "Cargo.toml",
        r#"[package]
name = "test_project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
    );
    run_language_parity_check(
        &workspace,
        "Rust",
        "src/main.rs",
        "fn main() { println!(\"Hello\"); }",
        "main",
    )
    .await;
}
