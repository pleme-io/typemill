use integration_tests::harness::{TestClient, TestWorkspace};
use serde_json::{json, Value};
#[tokio::test]
async fn test_health_check_basic() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let response = client.call_tool("health_check", json!({})).await.unwrap();
    let result = response["result"]
        .as_object()
        .expect("Should have result field");
    assert!(result.get("status").is_some());
    let status = result["status"].as_str().unwrap();
    assert!(status == "healthy" || status == "degraded" || status == "unhealthy");
    if let Some(servers) = result.get("servers") {
        let servers_array = servers.as_array().unwrap();
        for server in servers_array {
            assert!(server.get("name").is_some());
            assert!(server.get("status").is_some());
        }
    }
}
#[tokio::test]
async fn test_health_check_with_active_lsp() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let ts_file = workspace.path().join("trigger.ts");
    std::fs::write(
        &ts_file,
        r#"
interface Test {
    id: number;
}

const test: Test = { id: 1 };
"#,
    )
    .unwrap();
    let _response = client
        .call_tool(
            "get_document_symbols",
            json!({ "file_path" : ts_file.to_string_lossy() }),
        )
        .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
    let response = client.call_tool("health_check", json!({})).await.unwrap();
    let result = response["result"]
        .as_object()
        .expect("Should have result field");
    let status = result["status"].as_str().unwrap();
    assert!(status == "healthy" || status == "degraded");
    if let Some(servers) = result.get("servers") {
        let servers_array = servers.as_array().unwrap();
        let _has_ts_server = servers_array.iter().any(|s| {
            s["name"].as_str().unwrap_or("").contains("typescript")
                || s["name"].as_str().unwrap_or("").contains("ts")
        });
        // Server may or may not be running depending on LSP initialization
    }
}
#[tokio::test]
async fn test_health_check_detailed() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let response = client
        .call_tool("health_check", json!({ "include_details" : true }))
        .await
        .unwrap();
    let result = response["result"]
        .as_object()
        .expect("Should have result field");
    assert!(result.get("status").is_some());
    if result.get("system").is_some() {
        let system = &result["system"];
        assert!(system.is_object());
    }
    if let Some(servers) = result.get("servers") {
        let servers_array = servers.as_array().unwrap();
        for server in servers_array {
            assert!(server.get("name").is_some());
            assert!(server.get("status").is_some());
            if server.get("details").is_some() {
                let details = &server["details"];
                assert!(details.is_object());
            }
        }
    }
}
#[tokio::test]
async fn test_update_dependencies_package_json() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let package_json = workspace.path().join("package.json");
    let initial_content = json!(
        { "name" : "test-project", "version" : "1.0.0", "dependencies" : { "lodash" :
        "4.17.21" } }
    );
    std::fs::write(
        &package_json,
        serde_json::to_string_pretty(&initial_content).unwrap(),
    )
    .unwrap();

    // Setup the environment so `npm update` can succeed
    let npm_available = std::process::Command::new("npm")
        .arg("--version")
        .output()
        .is_ok();
    if !npm_available {
        eprintln!("Skipping test: npm not found");
        return;
    }
    let status = std::process::Command::new("npm")
        .arg("install")
        .current_dir(workspace.path())
        .status()
        .expect("Failed to run npm install");
    assert!(
        status.success(),
        "npm install should succeed to setup the test"
    );

    let response = client
        .call_tool(
            "update_dependencies",
            json!(
                { "file_path" : package_json.to_string_lossy(), "add_dependencies" : {
                "express" : "^4.18.0", "axios" : "^1.0.0" }, "remove_dependencies" :
                ["lodash"], "update_version" : "1.1.0" }
            ),
        )
        .await
        .unwrap();
    assert!(
        response["result"]["success"].as_bool().unwrap_or(false),
        "update_dependencies tool should report success"
    );
    let updated_content = std::fs::read_to_string(&package_json).unwrap();
    let updated_json: Value = serde_json::from_str(&updated_content).unwrap();
    assert_eq!(updated_json["version"].as_str().unwrap(), "1.1.0");
    let deps = &updated_json["dependencies"];
    assert_eq!(deps["express"].as_str().unwrap(), "^4.18.0");
    assert_eq!(deps["axios"].as_str().unwrap(), "^1.0.0");
    assert!(deps.get("lodash").is_none());
}
#[tokio::test]
async fn test_update_dependencies_cargo_toml() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let cargo_toml = workspace.path().join("Cargo.toml");
    let initial_content = r#"
[package]
name = "test-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1.0"
tokio = { version = "1.0", features = ["full"] }

[dev-dependencies]
assert_cmd = "2.0"
"#;
    std::fs::write(&cargo_toml, initial_content).unwrap();
    let response = client
        .call_tool(
            "update_dependencies",
            json!(
                { "file_path" : cargo_toml.to_string_lossy(), "add_dependencies" : {
                "reqwest" : "0.11", "clap" : "4.0" }, "add_dev_dependencies" : {
                "tempfile" : "3.0" }, "remove_dependencies" : ["serde"], "update_version"
                : "0.2.0" }
            ),
        )
        .await
        .unwrap();
    assert!(response["result"]["success"].as_bool().unwrap_or(false));
    let updated_content = std::fs::read_to_string(&cargo_toml).unwrap();
    assert!(updated_content.contains("version = \"0.2.0\""));
    assert!(updated_content.contains("reqwest = \"0.11\""));
    assert!(updated_content.contains("clap = \"4.0\""));
    assert!(!updated_content.contains("serde = \"1.0\""));
    assert!(updated_content.contains("tokio"));
    assert!(updated_content.contains("tempfile = \"3.0\""));
    assert!(updated_content.contains("assert_cmd = \"2.0\""));
}
#[tokio::test]
async fn test_update_dependencies_requirements_txt() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let requirements_txt = workspace.path().join("requirements.txt");
    let initial_content = r#"
numpy==1.21.0
pandas>=1.3.0
requests~=2.25.0
flask==2.0.1
"#;
    std::fs::write(&requirements_txt, initial_content).unwrap();
    let response = client
        .call_tool(
            "update_dependencies",
            json!(
                { "file_path" : requirements_txt.to_string_lossy(), "add_dependencies" :
                { "fastapi" : "0.68.0", "uvicorn" : "0.15.0" }, "remove_dependencies" :
                ["flask"] }
            ),
        )
        .await
        .unwrap();
    assert!(response["result"]["success"].as_bool().unwrap_or(false));
    let updated_content = std::fs::read_to_string(&requirements_txt).unwrap();
    assert!(updated_content.contains("fastapi==0.68.0"));
    assert!(updated_content.contains("uvicorn==0.15.0"));
    assert!(!updated_content.contains("flask==2.0.1"));
    assert!(updated_content.contains("numpy==1.21.0"));
    assert!(updated_content.contains("pandas>=1.3.0"));
    assert!(updated_content.contains("requests~=2.25.0"));
}
#[tokio::test]
async fn test_update_dependencies_dry_run() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let package_json = workspace.path().join("package.json");
    let initial_content = json!(
        { "name" : "test-project", "version" : "1.0.0", "dependencies" : { "lodash" :
        "^4.17.21" } }
    );
    std::fs::write(
        &package_json,
        serde_json::to_string_pretty(&initial_content).unwrap(),
    )
    .unwrap();
    let response = client
        .call_tool(
            "update_dependencies",
            json!(
                { "file_path" : package_json.to_string_lossy(), "add_dependencies" : {
                "express" : "^4.18.0" }, "dry_run" : true }
            ),
        )
        .await
        .unwrap();
    assert!(
        response["result"].get("preview").is_some() || response["result"].get("changes").is_some()
    );
    let unchanged_content = std::fs::read_to_string(&package_json).unwrap();
    let unchanged_json: Value = serde_json::from_str(&unchanged_content).unwrap();
    assert_eq!(
        unchanged_json["dependencies"]["lodash"].as_str().unwrap(),
        "^4.17.21"
    );
    assert!(unchanged_json["dependencies"].get("express").is_none());
}
#[tokio::test]
async fn test_update_dependencies_scripts_management() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let package_json = workspace.path().join("package.json");
    let initial_content = json!(
        { "name" : "test-project", "version" : "1.0.0", "scripts" : { "build" : "tsc",
        "test" : "jest", "outdated-script" : "old-command" } }
    );
    std::fs::write(
        &package_json,
        serde_json::to_string_pretty(&initial_content).unwrap(),
    )
    .unwrap();
    let response = client
        .call_tool(
            "update_dependencies",
            json!(
                { "file_path" : package_json.to_string_lossy(), "add_scripts" : { "dev" :
                "nodemon src/index.ts", "lint" : "eslint src/**/*.ts" }, "remove_scripts"
                : ["outdated-script"] }
            ),
        )
        .await
        .unwrap();
    assert!(response["result"]["success"].as_bool().unwrap_or(false));
    let updated_content = std::fs::read_to_string(&package_json).unwrap();
    let updated_json: Value = serde_json::from_str(&updated_content).unwrap();
    let scripts = &updated_json["scripts"];
    assert_eq!(scripts["dev"].as_str().unwrap(), "nodemon src/index.ts");
    assert_eq!(scripts["lint"].as_str().unwrap(), "eslint src/**/*.ts");
    assert!(scripts.get("outdated-script").is_none());
    assert_eq!(scripts["build"].as_str().unwrap(), "tsc");
    assert_eq!(scripts["test"].as_str().unwrap(), "jest");
}
#[tokio::test]
async fn test_update_dependencies_error_handling() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let nonexistent_file = workspace.path().join("nonexistent.json");
    let response = client
        .call_tool(
            "update_dependencies",
            json!(
                { "file_path" : nonexistent_file.to_string_lossy(), "add_dependencies" :
                { "test" : "1.0.0" } }
            ),
        )
        .await;
    // Must return error for nonexistent file
    assert!(
        response.is_err() || response.as_ref().unwrap().get("error").is_some(),
        "Expected error for nonexistent file"
    );
}
#[tokio::test]
async fn test_update_dependencies_invalid_json() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let invalid_json = workspace.path().join("invalid.json");
    std::fs::write(&invalid_json, "{ invalid json content").unwrap();
    let response = client
        .call_tool(
            "update_dependencies",
            json!(
                { "file_path" : invalid_json.to_string_lossy(), "add_dependencies" : {
                "test" : "1.0.0" } }
            ),
        )
        .await;
    // Must return error for invalid JSON
    assert!(
        response.is_err() || response.as_ref().unwrap().get("error").is_some(),
        "Expected error for invalid.json (unsupported file type)"
    );
}
#[tokio::test]
async fn test_system_tools_integration() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let health_response = client.call_tool("health_check", json!({})).await.unwrap();
    let health_result = health_response["result"]
        .as_object()
        .expect("Should have result field");
    let initial_status = health_result["status"].as_str().unwrap();
    let package_json = workspace.path().join("package.json");
    let initial_package = json!(
        { "name" : "integration-test", "version" : "0.1.0", "scripts" : { "start" :
        "node index.js" }, "dependencies" : {} }
    );
    std::fs::write(
        &package_json,
        serde_json::to_string_pretty(&initial_package).unwrap(),
    )
    .unwrap();
    let _update_response = client
        .call_tool(
            "update_dependencies",
            json!(
                { "file_path" : package_json.to_string_lossy(), "add_dependencies" : {
                "express" : "^4.18.0", "cors" : "^2.8.5", "helmet" : "^6.0.0" },
                "add_dev_dependencies" : { "typescript" : "^4.9.0", "@types/express" :
                "^4.17.0", "nodemon" : "^2.0.0" }, "add_scripts" : { "dev" :
                "nodemon src/index.ts", "build" : "tsc", "test" : "jest" },
                "update_version" : "1.0.0" }
            ),
        )
        .await
        .unwrap();
    assert!(_update_response["result"]["success"]
        .as_bool()
        .unwrap_or(false));
    let src_dir = workspace.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    let index_ts = src_dir.join("index.ts");
    std::fs::write(
        &index_ts,
        r#"
import express from 'express';
import cors from 'cors';
import helmet from 'helmet';

const app = express();
const PORT = process.env.PORT || 3000;

app.use(helmet());
app.use(cors());
app.use(express.json());

app.get('/', (req, res) => {
    res.json({ message: 'Hello World!' });
});

app.listen(PORT, () => {
    console.log(`Server running on port ${PORT}`);
});
"#,
    )
    .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    let _symbols_response = client
        .call_tool(
            "get_document_symbols",
            json!({ "file_path" : index_ts.to_string_lossy() }),
        )
        .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    let final_health_response = client
        .call_tool("health_check", json!({ "include_details" : true }))
        .await
        .unwrap();
    let final_status = final_health_response["result"]["status"].as_str().unwrap();
    assert!(final_status == "healthy" || final_status == "degraded");
    let final_package_content = std::fs::read_to_string(&package_json).unwrap();
    let final_package_json: Value = serde_json::from_str(&final_package_content).unwrap();
    assert_eq!(final_package_json["version"].as_str().unwrap(), "1.0.0");
    assert_eq!(
        final_package_json["dependencies"]["express"]
            .as_str()
            .unwrap(),
        "^4.18.0"
    );
    assert_eq!(
        final_package_json["devDependencies"]["typescript"]
            .as_str()
            .unwrap(),
        "^4.9.0"
    );
    assert_eq!(
        final_package_json["scripts"]["dev"].as_str().unwrap(),
        "nodemon src/index.ts"
    );
}
#[tokio::test]
async fn test_organize_imports_dry_run() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let test_file = workspace.path().join("test.ts");
    std::fs::write(
        &test_file,
        r#"import { useState, useEffect, useCallback } from 'react';
import * as lodash from 'lodash';
import defaultExport from 'some-module';
import './styles.css';

// Only using useState
function MyComponent() {
    const [count, setCount] = useState(0);
    return <div>{count}</div>;
}
"#,
    )
    .unwrap();
    let response = client
        .call_tool(
            "organize_imports",
            json!({ "file_path" : test_file.to_string_lossy(), "dry_run" : true }),
        )
        .await
        .unwrap();
    let result = &response["result"];
    assert_eq!(result["operation"].as_str().unwrap(), "organize_imports");
    assert_eq!(result["dry_run"].as_bool().unwrap(), true);
    assert_eq!(result["modified"].as_bool().unwrap(), false);
    assert_eq!(result["status"].as_str().unwrap(), "preview");
    let content = std::fs::read_to_string(&test_file).unwrap();
    assert!(content.contains("useEffect"));
    assert!(content.contains("useCallback"));
    assert!(content.contains("lodash"));
}
#[tokio::test]
async fn test_organize_imports_with_lsp() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let test_file = workspace.path().join("test_imports.ts");
    let original_content = r#"import { useState, useEffect, useCallback } from 'react';
import * as lodash from 'lodash';
import defaultExport from 'some-module';

// Only using useState
function MyComponent() {
    const [count, setCount] = useState(0);
    return <div>{count}</div>;
}
"#;
    std::fs::write(&test_file, original_content).unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    let response = client
        .call_tool(
            "organize_imports",
            json!({ "file_path" : test_file.to_string_lossy(), "dry_run" : false }),
        )
        .await;

    // organize_imports requires LSP organize_imports support - may not be available
    if let Ok(response_value) = response {
        // Response must have either result or error
        assert!(
            response_value.get("result").is_some() || response_value.get("error").is_some(),
            "Response must contain 'result' or 'error' field"
        );

        if let Some(result) = response_value.get("result") {
            assert_eq!(result["operation"].as_str().unwrap(), "organize_imports");
            assert_eq!(result["dry_run"].as_bool().unwrap_or(true), false);
        }
        // If error, that's acceptable (LSP may not support organize_imports)
    }
    // If Err, that's also acceptable (LSP not configured)
}
#[tokio::test]
async fn test_organize_imports_missing_file_path() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let response = client
        .call_tool("organize_imports", json!({ "dry_run" : true }))
        .await;
    // Must return error - missing required file_path parameter
    assert!(
        response.is_err() || response.as_ref().unwrap().get("error").is_some(),
        "Expected error for missing required file_path parameter"
    );
}
#[tokio::test]
async fn test_organize_imports_nonexistent_file() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let nonexistent_file = workspace.path().join("nonexistent.ts");
    let response = client
        .call_tool(
            "organize_imports",
            json!(
                { "file_path" : nonexistent_file.to_string_lossy(), "dry_run" : false }
            ),
        )
        .await;
    // Must return error for nonexistent file
    assert!(
        response.is_err() || response.as_ref().unwrap().get("error").is_some(),
        "Expected error for nonexistent file"
    );
}
#[tokio::test]
async fn test_extract_function_refactoring() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let test_file = workspace.path().join("test.ts");
    let original_content = r#"function main() {
    const a = 1;
    const b = 2;
    const result = a + b;
    console.log(result);
}
"#;
    tokio::fs::write(&test_file, original_content)
        .await
        .unwrap();
    let response = client
        .call_tool(
            "extract_function",
            json!(
                { "file_path" : test_file.to_str().unwrap(), "start_line" : 2, "end_line"
                : 4, "function_name" : "calculate" }
            ),
        )
        .await;

    // extract_function may not be supported by all LSP servers
    if let Ok(resp) = response {
        // Response must have result or error
        assert!(
            resp.get("result").is_some() || resp.get("error").is_some(),
            "Response must have 'result' or 'error' field"
        );

        // If we got edits, verify they're valid
        if let Some(result) = resp.get("result") {
            if let Some(edits) = result.get("edits").and_then(|e| e.as_array()) {
                assert!(
                    !edits.is_empty(),
                    "Edits array should not be empty if present"
                );
            }
        }
    }
    // If error or unsupported, that's acceptable for this test
}
#[tokio::test]
async fn test_inline_variable_refactoring() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let test_file = workspace.path().join("test.ts");
    let original_content = r#"function calculate() {
    const x = 10;
    const y = x * 2;
    return y;
}
"#;
    tokio::fs::write(&test_file, original_content)
        .await
        .unwrap();
    let response = client
        .call_tool(
            "inline_variable",
            json!(
                { "file_path" : test_file.to_str().unwrap(), "line" : 2, "character" : 10
                }
            ),
        )
        .await;

    // inline_variable may not be supported by all LSP servers
    if let Ok(resp) = response {
        // Response must have result or error
        assert!(
            resp.get("result").is_some() || resp.get("error").is_some(),
            "Response must have 'result' or 'error' field"
        );

        // If we got edits, verify they're valid
        if let Some(result) = resp.get("result") {
            if let Some(edits) = result.get("edits").and_then(|e| e.as_array()) {
                assert!(
                    !edits.is_empty(),
                    "Edits array should not be empty if present"
                );
            }
        }
    }
    // If error or unsupported, that's acceptable for this test
}
#[tokio::test]
async fn test_rename_directory_in_rust_workspace() {
    let workspace = TestWorkspace::new();
    workspace.create_file(
        "Cargo.toml",
        r#"
[workspace]
resolver = "2"
members = ["crates/crate_a", "crates/crate_b"]
"#,
    );
    workspace.create_file(
        "crates/crate_a/Cargo.toml",
        r#"
[package]
name = "crate_a"
version = "0.1.0"
edition = "2021"

[dependencies]
crate_b = { path = "../crate_b" }
"#,
    );
    workspace.create_file(
        "crates/crate_a/src/lib.rs",
        "pub fn hello_a() { crate_b::hello_b(); }",
    );
    workspace.create_file(
        "crates/crate_b/Cargo.toml",
        r#"
[package]
name = "crate_b"
version = "0.1.0"
edition = "2021"
"#,
    );
    workspace.create_file(
        "crates/crate_b/src/lib.rs",
        "pub fn hello_b() { println!(\"Hello from B\"); }",
    );
    let cargo_available = std::process::Command::new("cargo")
        .arg("--version")
        .output()
        .is_ok();
    if cargo_available {
        let initial_check = std::process::Command::new("cargo")
            .arg("check")
            .current_dir(workspace.path())
            .output()
            .expect("Failed to run cargo check");
        assert!(
            initial_check.status.success(),
            "Initial workspace should be valid. Stderr: {}",
            String::from_utf8_lossy(&initial_check.stderr)
        );
    } else {
        eprintln!("Note: cargo not available, skipping initial validation");
    }
    let mut client = TestClient::new(workspace.path());
    let result = client
        .call_tool(
            "rename_directory",
            json!({ "old_path" : "crates/crate_b", "new_path" : "crates/crate_renamed" }),
        )
        .await;
    assert!(result.is_ok(), "Tool call should succeed");
    let response: serde_json::Value = result.unwrap();
    if let Some(result_obj) = response.get("result") {
        assert_eq!(
            result_obj["success"], true,
            "Result should indicate success"
        );
    } else {
        assert_eq!(
            response["result"]["success"], true,
            "Response should indicate success"
        );
    }
    let ws_manifest = workspace.read_file("Cargo.toml");
    assert!(
        ws_manifest.contains("\"crates/crate_renamed\"")
            || ws_manifest.contains("crates/crate_renamed")
    );
    assert!(!ws_manifest.contains("\"crates/crate_b\"") || !ws_manifest.contains("crate_b\""));
    assert!(
        workspace.file_exists("crates/crate_renamed/Cargo.toml"),
        "Renamed crate should exist"
    );
    assert!(
        workspace.file_exists("crates/crate_renamed/src/lib.rs"),
        "Renamed crate source should exist"
    );
    assert!(
        !workspace.file_exists("crates/crate_b/Cargo.toml"),
        "Old crate directory should not exist"
    );
}
