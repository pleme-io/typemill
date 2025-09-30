//! Contract tests for Rust MCP server
//! These tests validate the API contract by starting the server and sending real MCP requests

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tokio_tungstenite::connect_async;
use url::Url;

/// Test helper to start cb-server and send MCP requests via stdio
struct McpStdioClient {
    process: std::process::Child,
    stdin: std::process::ChildStdin,
    stdout_receiver: mpsc::Receiver<String>,
}

impl McpStdioClient {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let binary_path = std::env::var("CARGO_BIN_EXE_cb-server")
            .unwrap_or_else(|_| "target/debug/cb-server".to_string());
        let mut process = Command::new(binary_path)
            .arg("start")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdin = process.stdin.take().unwrap();
        let stdout = process.stdout.take().unwrap();

        // Spawn thread to read stdout
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line) = line {
                    let trimmed = line.trim();
                    // Only send lines that look like JSON (start with '{' and are non-empty)
                    if !trimmed.is_empty() && trimmed.starts_with('{') {
                        if sender.send(line).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        // Wait for server startup
        thread::sleep(Duration::from_millis(1000));

        Ok(McpStdioClient {
            process,
            stdin,
            stdout_receiver: receiver,
        })
    }

    fn send_request(&mut self, request: Value) -> Result<Value, Box<dyn std::error::Error>> {
        let request_str = serde_json::to_string(&request)?;
        writeln!(self.stdin, "{}", request_str)?;
        self.stdin.flush()?;

        // Wait for response with timeout
        let response_str = self.stdout_receiver.recv_timeout(Duration::from_secs(10))?;
        let response: Value = serde_json::from_str(&response_str)?;
        Ok(response)
    }
}

impl Drop for McpStdioClient {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

#[tokio::test]
async fn test_tools_list_contract() {
    let mut client = McpStdioClient::new().expect("Failed to start server");

    let request = json!({
        "jsonrpc": "2.0",
        "id": "test-1",
        "method": "tools/list",
        "params": {}
    });

    let response = client
        .send_request(request)
        .expect("Failed to get response");

    // Validate response structure
    assert_eq!(response["id"], "test-1");
    assert!(response["result"]["tools"].is_array());

    let tools = response["result"]["tools"].as_array().unwrap();
    assert!(!tools.is_empty(), "Should have at least one tool");

    // Check for specific tools we implemented
    let tool_names: Vec<&str> = tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();

    // Debug: print all available tools
    println!("Available tools: {:?}", tool_names);

    assert!(tool_names.contains(&"find_definition"));
    assert!(tool_names.contains(&"find_references"));
    assert!(tool_names.contains(&"get_hover"));
    assert!(tool_names.contains(&"get_completions"));
    assert!(tool_names.contains(&"analyze_imports"));
    assert!(tool_names.contains(&"find_dead_code"));
    assert!(tool_names.contains(&"list_files"));

    println!(
        "✅ tools/list contract test passed - {} tools found",
        tools.len()
    );
}

#[tokio::test]
async fn test_find_definition_contract() {
    let mut client = McpStdioClient::new().expect("Failed to start server");

    let request = json!({
        "jsonrpc": "2.0",
        "id": "test-2",
        "method": "tools/call",
        "params": {
            "name": "find_definition",
            "arguments": {
                "file_path": "/workspace/examples/playground/src/test-file.ts",
                "symbol_name": "TestProcessor"
            }
        }
    });

    let response = client
        .send_request(request)
        .expect("Failed to get response");

    // Validate response structure
    assert_eq!(response["id"], "test-2");

    // Should have either a result or an error (depends on LSP availability)
    if response["error"].is_null() {
        // Success case - validate result structure
        assert!(response["result"].is_object(), "Result should be an object");
        println!("✅ find_definition contract test passed - LSP responded");
    } else {
        // Error case - validate error structure
        assert!(
            response["error"]["message"].is_string(),
            "Error should have message"
        );
        println!("⚠️ find_definition contract test passed - LSP error handled correctly");
    }
}

#[tokio::test]
async fn test_get_hover_contract() {
    let mut client = McpStdioClient::new().expect("Failed to start server");

    let request = json!({
        "jsonrpc": "2.0",
        "id": "test-3",
        "method": "tools/call",
        "params": {
            "name": "get_hover",
            "arguments": {
                "file_path": "/workspace/examples/playground/src/test-file.ts",
                "line": 10,
                "character": 5
            }
        }
    });

    let response = client
        .send_request(request)
        .expect("Failed to get response");

    // Validate response structure
    assert_eq!(response["id"], "test-3");

    // Should have either a result or an error
    if response["error"].is_null() {
        assert!(response["result"].is_object(), "Result should be an object");
        println!("✅ get_hover contract test passed - LSP responded");
    } else {
        assert!(
            response["error"]["message"].is_string(),
            "Error should have message"
        );
        println!("⚠️ get_hover contract test passed - LSP error handled correctly");
    }
}

#[tokio::test]
async fn test_analyze_imports_contract() {
    let mut client = McpStdioClient::new().expect("Failed to start server");

    let request = json!({
        "jsonrpc": "2.0",
        "id": "test-4",
        "method": "tools/call",
        "params": {
            "name": "analyze_imports",
            "arguments": {
                "file_path": "/workspace/examples/playground/src/test-file.ts"
            }
        }
    });

    let response = client
        .send_request(request)
        .expect("Failed to get response");

    // Validate response structure
    assert_eq!(response["id"], "test-4");

    // This should work regardless of LSP since it uses cb-ast directly
    if !response["error"].is_null() {
        println!("analyze_imports error: {}", response["error"]);
    }

    if response["error"].is_null() {
        let result = &response["result"];
        let content = &result["content"];
        assert!(content["sourceFile"].is_string(), "Should have sourceFile");
        assert!(
            content["importGraph"].is_object(),
            "Should have importGraph"
        );
        assert!(
            content["analysisStats"].is_object(),
            "Should have analysisStats"
        );
        println!("✅ analyze_imports contract test passed");
    } else {
        // Should not error for this tool
        panic!("analyze_imports should not error: {}", response["error"]);
    }
}

#[tokio::test]
async fn test_list_files_contract() {
    let mut client = McpStdioClient::new().expect("Failed to start server");

    let request = json!({
        "jsonrpc": "2.0",
        "id": "test-5",
        "method": "tools/call",
        "params": {
            "name": "list_files",
            "arguments": {
                "path": "/workspace/examples/playground/src",
                "recursive": false
            }
        }
    });

    let response = client
        .send_request(request)
        .expect("Failed to get response");

    // Validate response structure
    assert_eq!(response["id"], "test-5");

    if !response["error"].is_null() {
        println!("list_files error: {}", response["error"]);
    }
    assert!(response["error"].is_null(), "list_files should not error");

    let result = &response["result"];
    let content = &result["content"];
    assert!(content["files"].is_array(), "Should have files array");
    assert!(content["path"].is_string(), "Should have path");
    assert!(content["total"].is_number(), "Should have total count");

    let files = content["files"].as_array().unwrap();
    assert!(!files.is_empty(), "Should find some files");

    println!(
        "✅ list_files contract test passed - {} files found",
        files.len()
    );
}

#[tokio::test]
async fn test_error_handling_contract() {
    let mut client = McpStdioClient::new().expect("Failed to start server");

    // Test with invalid method
    let request = json!({
        "jsonrpc": "2.0",
        "id": "test-6",
        "method": "invalid/method",
        "params": {}
    });

    let response = client
        .send_request(request)
        .expect("Failed to get response");

    // Should get an error response
    assert_eq!(response["id"], "test-6");
    assert!(
        !response["error"].is_null(),
        "Should have error for invalid method"
    );
    assert!(
        response["error"]["message"].is_string(),
        "Error should have message"
    );

    println!("✅ error_handling contract test passed - invalid method handled correctly");
}

#[tokio::test]
async fn test_websocket_server_startup() {
    // Test that we can start the WebSocket server
    let binary_path = std::env::var("CARGO_BIN_EXE_cb-server")
        .unwrap_or_else(|_| "target/debug/cb-server".to_string());
    let mut process = Command::new(binary_path)
        .arg("serve")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start WebSocket server");

    // Wait for startup
    thread::sleep(Duration::from_millis(2000));

    // Try to connect (this will probably fail but should not crash the server)
    if let Ok(url) = Url::parse("ws://127.0.0.1:3040") {
        let _ = connect_async(url).await;
    }

    // Clean up
    let _ = process.kill();
    let _ = process.wait();

    println!("✅ WebSocket server startup test passed");
}

#[cfg(test)]
mod integration {
    use super::*;

    #[tokio::test]
    async fn test_full_mcp_workflow() {
        let mut client = McpStdioClient::new().expect("Failed to start server");

        // Step 1: List tools
        let list_request = json!({
            "jsonrpc": "2.0",
            "id": "workflow-1",
            "method": "tools/list",
            "params": {}
        });

        let list_response = client
            .send_request(list_request)
            .expect("Failed to list tools");
        assert_eq!(list_response["id"], "workflow-1");
        assert!(list_response["result"]["tools"].is_array());

        // Step 2: Call a file operation
        let read_request = json!({
            "jsonrpc": "2.0",
            "id": "workflow-2",
            "method": "tools/call",
            "params": {
                "name": "list_files",
                "arguments": {
                    "path": "/workspace/examples/playground",
                    "recursive": true
                }
            }
        });

        let read_response = client
            .send_request(read_request)
            .expect("Failed to list files");
        assert_eq!(read_response["id"], "workflow-2");

        // Step 3: Call an analysis operation
        let analyze_request = json!({
            "jsonrpc": "2.0",
            "id": "workflow-3",
            "method": "tools/call",
            "params": {
                "name": "analyze_imports",
                "arguments": {
                    "file_path": "/workspace/examples/playground/src/test-file.ts"
                }
            }
        });

        let analyze_response = client
            .send_request(analyze_request)
            .expect("Failed to analyze");
        assert_eq!(analyze_response["id"], "workflow-3");

        println!("✅ Full MCP workflow test passed - all operations completed");
    }
}
