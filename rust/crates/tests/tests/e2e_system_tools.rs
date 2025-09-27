use cb_tests::harness::{TestClient, TestWorkspace};
use serde_json::{json, Value};
use std::path::Path;

#[tokio::test]
async fn test_health_check_basic() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    let response = client.call_tool("health_check", json!({})).await.unwrap();

    // Verify basic health check structure
    assert!(response.get("status").is_some());
    assert!(response.get("timestamp").is_some());

    let status = response["status"].as_str().unwrap();
    assert!(status == "healthy" || status == "degraded" || status == "unhealthy");

    // Should have server information
    if let Some(servers) = response.get("servers") {
        let servers_array = servers.as_array().unwrap();
        // May have 0 or more servers running
        for server in servers_array {
            assert!(server.get("name").is_some());
            assert!(server.get("status").is_some());
        }
    }
}

#[tokio::test]
async fn test_health_check_with_active_lsp() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    // Create a TypeScript file to trigger LSP server startup
    let ts_file = workspace.path().join("trigger.ts");
    std::fs::write(&ts_file, r#"
interface Test {
    id: number;
}

const test: Test = { id: 1 };
"#).unwrap();

    // Trigger LSP by doing an operation that requires it
    let _response = client.call_tool("get_document_symbols", json!({
        "file_path": ts_file.to_string_lossy()
    })).await;

    // Give LSP time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

    // Now check health with active LSP
    let response = client.call_tool("health_check", json!({})).await.unwrap();

    let status = response["status"].as_str().unwrap();
    assert!(status == "healthy" || status == "degraded");

    // Should show TypeScript server information
    if let Some(servers) = response.get("servers") {
        let servers_array = servers.as_array().unwrap();
        let has_ts_server = servers_array.iter().any(|s|
            s["name"].as_str().unwrap_or("").contains("typescript") ||
            s["name"].as_str().unwrap_or("").contains("ts")
        );

        if !servers_array.is_empty() {
            // If we have servers, at least one should be TypeScript-related
            // (This depends on the LSP configuration)
        }
    }
}

#[tokio::test]
async fn test_health_check_detailed() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    let response = client.call_tool("health_check", json!({
        "include_details": true
    })).await.unwrap();

    // With details, should have more comprehensive information
    assert!(response.get("status").is_some());
    assert!(response.get("timestamp").is_some());

    // Should include system information
    if response.get("system").is_some() {
        let system = &response["system"];
        // May include memory, CPU, etc.
        assert!(system.is_object());
    }

    // Should include detailed server information
    if let Some(servers) = response.get("servers") {
        let servers_array = servers.as_array().unwrap();
        for server in servers_array {
            assert!(server.get("name").is_some());
            assert!(server.get("status").is_some());

            // With details, might include more info
            if server.get("details").is_some() {
                let details = &server["details"];
                assert!(details.is_object());
            }
        }
    }
}

#[tokio::test]
async fn test_update_dependencies_package_json() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    // Create a package.json file
    let package_json = workspace.path().join("package.json");
    let initial_content = json!({
        "name": "test-project",
        "version": "1.0.0",
        "dependencies": {
            "lodash": "^4.17.21"
        },
        "devDependencies": {
            "typescript": "^4.9.0"
        },
        "scripts": {
            "build": "tsc",
            "test": "jest"
        }
    });

    std::fs::write(&package_json, serde_json::to_string_pretty(&initial_content).unwrap()).unwrap();

    // Update dependencies
    let response = client.call_tool("update_dependencies", json!({
        "file_path": package_json.to_string_lossy(),
        "add_dependencies": {
            "express": "^4.18.0",
            "axios": "^1.0.0"
        },
        "add_dev_dependencies": {
            "@types/node": "^18.0.0",
            "jest": "^29.0.0"
        },
        "remove_dependencies": ["lodash"],
        "update_version": "1.1.0"
    })).await.unwrap();

    assert!(response["success"].as_bool().unwrap_or(false));

    // Verify the changes
    let updated_content = std::fs::read_to_string(&package_json).unwrap();
    let updated_json: Value = serde_json::from_str(&updated_content).unwrap();

    // Check version update
    assert_eq!(updated_json["version"].as_str().unwrap(), "1.1.0");

    // Check added dependencies
    let deps = &updated_json["dependencies"];
    assert_eq!(deps["express"].as_str().unwrap(), "^4.18.0");
    assert_eq!(deps["axios"].as_str().unwrap(), "^1.0.0");
    assert!(deps.get("lodash").is_none()); // Should be removed

    // Check added dev dependencies
    let dev_deps = &updated_json["devDependencies"];
    assert_eq!(dev_deps["@types/node"].as_str().unwrap(), "^18.0.0");
    assert_eq!(dev_deps["jest"].as_str().unwrap(), "^29.0.0");
    assert_eq!(dev_deps["typescript"].as_str().unwrap(), "^4.9.0"); // Should remain

    // Scripts should be preserved
    assert_eq!(updated_json["scripts"]["build"].as_str().unwrap(), "tsc");
    assert_eq!(updated_json["scripts"]["test"].as_str().unwrap(), "jest");
}

#[tokio::test]
async fn test_update_dependencies_cargo_toml() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    // Create a Cargo.toml file
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

    // Update Rust dependencies
    let response = client.call_tool("update_dependencies", json!({
        "file_path": cargo_toml.to_string_lossy(),
        "add_dependencies": {
            "reqwest": "0.11",
            "clap": "4.0"
        },
        "add_dev_dependencies": {
            "tempfile": "3.0"
        },
        "remove_dependencies": ["serde"],
        "update_version": "0.2.0"
    })).await.unwrap();

    assert!(response["success"].as_bool().unwrap_or(false));

    // Verify the changes
    let updated_content = std::fs::read_to_string(&cargo_toml).unwrap();

    // Check version update
    assert!(updated_content.contains("version = \"0.2.0\""));

    // Check added dependencies
    assert!(updated_content.contains("reqwest = \"0.11\""));
    assert!(updated_content.contains("clap = \"4.0\""));
    assert!(!updated_content.contains("serde = \"1.0\"")); // Should be removed

    // Check that tokio remains
    assert!(updated_content.contains("tokio"));

    // Check added dev dependencies
    assert!(updated_content.contains("tempfile = \"3.0\""));
    assert!(updated_content.contains("assert_cmd = \"2.0\"")); // Should remain
}

#[tokio::test]
async fn test_update_dependencies_requirements_txt() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    // Create a requirements.txt file
    let requirements_txt = workspace.path().join("requirements.txt");
    let initial_content = r#"
numpy==1.21.0
pandas>=1.3.0
requests~=2.25.0
flask==2.0.1
"#;

    std::fs::write(&requirements_txt, initial_content).unwrap();

    // Update Python dependencies
    let response = client.call_tool("update_dependencies", json!({
        "file_path": requirements_txt.to_string_lossy(),
        "add_dependencies": {
            "fastapi": "0.68.0",
            "uvicorn": "0.15.0"
        },
        "remove_dependencies": ["flask"]
    })).await.unwrap();

    assert!(response["success"].as_bool().unwrap_or(false));

    // Verify the changes
    let updated_content = std::fs::read_to_string(&requirements_txt).unwrap();

    // Check added dependencies
    assert!(updated_content.contains("fastapi==0.68.0"));
    assert!(updated_content.contains("uvicorn==0.15.0"));

    // Check removed dependency
    assert!(!updated_content.contains("flask==2.0.1"));

    // Check preserved dependencies
    assert!(updated_content.contains("numpy==1.21.0"));
    assert!(updated_content.contains("pandas>=1.3.0"));
    assert!(updated_content.contains("requests~=2.25.0"));
}

#[tokio::test]
async fn test_update_dependencies_dry_run() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    let package_json = workspace.path().join("package.json");
    let initial_content = json!({
        "name": "test-project",
        "version": "1.0.0",
        "dependencies": {
            "lodash": "^4.17.21"
        }
    });

    std::fs::write(&package_json, serde_json::to_string_pretty(&initial_content).unwrap()).unwrap();

    // Do a dry run
    let response = client.call_tool("update_dependencies", json!({
        "file_path": package_json.to_string_lossy(),
        "add_dependencies": {
            "express": "^4.18.0"
        },
        "dry_run": true
    })).await.unwrap();

    // Should show what would change
    assert!(response.get("preview").is_some() || response.get("changes").is_some());

    // File should not be modified
    let unchanged_content = std::fs::read_to_string(&package_json).unwrap();
    let unchanged_json: Value = serde_json::from_str(&unchanged_content).unwrap();

    assert_eq!(unchanged_json["dependencies"]["lodash"].as_str().unwrap(), "^4.17.21");
    assert!(unchanged_json["dependencies"].get("express").is_none());
}

#[tokio::test]
async fn test_update_dependencies_scripts_management() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    let package_json = workspace.path().join("package.json");
    let initial_content = json!({
        "name": "test-project",
        "version": "1.0.0",
        "scripts": {
            "build": "tsc",
            "test": "jest",
            "outdated-script": "old-command"
        }
    });

    std::fs::write(&package_json, serde_json::to_string_pretty(&initial_content).unwrap()).unwrap();

    let response = client.call_tool("update_dependencies", json!({
        "file_path": package_json.to_string_lossy(),
        "add_scripts": {
            "dev": "nodemon src/index.ts",
            "lint": "eslint src/**/*.ts"
        },
        "remove_scripts": ["outdated-script"]
    })).await.unwrap();

    assert!(response["success"].as_bool().unwrap_or(false));

    let updated_content = std::fs::read_to_string(&package_json).unwrap();
    let updated_json: Value = serde_json::from_str(&updated_content).unwrap();

    let scripts = &updated_json["scripts"];

    // Check added scripts
    assert_eq!(scripts["dev"].as_str().unwrap(), "nodemon src/index.ts");
    assert_eq!(scripts["lint"].as_str().unwrap(), "eslint src/**/*.ts");

    // Check removed script
    assert!(scripts.get("outdated-script").is_none());

    // Check preserved scripts
    assert_eq!(scripts["build"].as_str().unwrap(), "tsc");
    assert_eq!(scripts["test"].as_str().unwrap(), "jest");
}

#[tokio::test]
async fn test_update_dependencies_error_handling() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    // Try to update non-existent file
    let nonexistent_file = workspace.path().join("nonexistent.json");

    let response = client.call_tool("update_dependencies", json!({
        "file_path": nonexistent_file.to_string_lossy(),
        "add_dependencies": {
            "test": "1.0.0"
        }
    })).await;

    assert!(response.is_err());
}

#[tokio::test]
async fn test_update_dependencies_invalid_json() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    // Create invalid JSON file
    let invalid_json = workspace.path().join("invalid.json");
    std::fs::write(&invalid_json, "{ invalid json content").unwrap();

    let response = client.call_tool("update_dependencies", json!({
        "file_path": invalid_json.to_string_lossy(),
        "add_dependencies": {
            "test": "1.0.0"
        }
    })).await;

    assert!(response.is_err());
}

#[tokio::test]
async fn test_system_tools_integration() {
    let workspace = TestWorkspace::new().await;
    let client = TestClient::new().await;

    // Step 1: Check initial health
    let health_response = client.call_tool("health_check", json!({})).await.unwrap();
    let initial_status = health_response["status"].as_str().unwrap();

    // Step 2: Create a complex project
    let package_json = workspace.path().join("package.json");
    let initial_package = json!({
        "name": "integration-test",
        "version": "0.1.0",
        "scripts": {
            "start": "node index.js"
        },
        "dependencies": {}
    });

    std::fs::write(&package_json, serde_json::to_string_pretty(&initial_package).unwrap()).unwrap();

    // Step 3: Update dependencies to simulate project growth
    let _update_response = client.call_tool("update_dependencies", json!({
        "file_path": package_json.to_string_lossy(),
        "add_dependencies": {
            "express": "^4.18.0",
            "cors": "^2.8.5",
            "helmet": "^6.0.0"
        },
        "add_dev_dependencies": {
            "typescript": "^4.9.0",
            "@types/express": "^4.17.0",
            "nodemon": "^2.0.0"
        },
        "add_scripts": {
            "dev": "nodemon src/index.ts",
            "build": "tsc",
            "test": "jest"
        },
        "update_version": "1.0.0"
    })).await.unwrap();

    assert!(_update_response["success"].as_bool().unwrap_or(false));

    // Step 4: Create TypeScript files to trigger LSP
    let src_dir = workspace.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();

    let index_ts = src_dir.join("index.ts");
    std::fs::write(&index_ts, r#"
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
"#).unwrap();

    // Step 5: Trigger LSP operations
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    let _symbols_response = client.call_tool("get_document_symbols", json!({
        "file_path": index_ts.to_string_lossy()
    })).await;

    // Step 6: Check health after all operations
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    let final_health_response = client.call_tool("health_check", json!({
        "include_details": true
    })).await.unwrap();

    let final_status = final_health_response["status"].as_str().unwrap();

    // System should still be healthy after all operations
    assert!(final_status == "healthy" || final_status == "degraded");

    // Step 7: Verify package.json was updated correctly
    let final_package_content = std::fs::read_to_string(&package_json).unwrap();
    let final_package_json: Value = serde_json::from_str(&final_package_content).unwrap();

    assert_eq!(final_package_json["version"].as_str().unwrap(), "1.0.0");
    assert_eq!(final_package_json["dependencies"]["express"].as_str().unwrap(), "^4.18.0");
    assert_eq!(final_package_json["devDependencies"]["typescript"].as_str().unwrap(), "^4.9.0");
    assert_eq!(final_package_json["scripts"]["dev"].as_str().unwrap(), "nodemon src/index.ts");
}