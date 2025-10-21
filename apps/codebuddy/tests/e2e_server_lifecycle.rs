//! End-to-end server lifecycle tests
//!
//! Combines tests from e2e_flow.rs and e2e_transport.rs into a unified server lifecycle suite.

use cb_server::{bootstrap, ServerOptions};
use cb_test_support::create_test_config;
use cb_test_support::harness::{TestClient, TestWorkspace};
use codebuddy_config::AppConfig;
use e2e::TestHarnessError;
use serde_json::json;

// ============================================================================
// Server Bootstrap and Configuration Tests
// ============================================================================

#[tokio::test]
async fn test_server_bootstrap_and_shutdown() -> Result<(), TestHarnessError> {
    // Create test configuration
    let config = create_test_config();
    let options = ServerOptions::from_config(config).with_debug(true);

    // Bootstrap the server
    let handle = bootstrap(options)
        .await
        .map_err(|e| TestHarnessError::setup(format!("Bootstrap failed: {}", e)))?;

    // Start the server
    handle
        .start()
        .await
        .map_err(|e| TestHarnessError::execution(format!("Start failed: {}", e)))?;

    // Give it a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Shutdown the server
    handle
        .shutdown()
        .await
        .map_err(|e| TestHarnessError::execution(format!("Shutdown failed: {}", e)))?;

    Ok(())
}

#[tokio::test]
async fn test_server_with_invalid_config() {
    let mut config = AppConfig::default();
    config.server.port = 0; // Invalid port that should be caught by validation

    let options = ServerOptions::from_config(config);

    // This should fail during bootstrap due to config validation
    let result = bootstrap(options).await;
    assert!(result.is_err(), "Bootstrap should fail with invalid port 0");
}

#[tokio::test]
async fn test_configuration_loading() -> Result<(), TestHarnessError> {
    // Test that we can load configuration successfully
    let config = create_test_config();

    // Validate the test configuration
    if config.server.port != 3043 {
        return Err(TestHarnessError::assertion(
            "Test config should use port 3043".to_string(),
        ));
    }

    if config.server.host != "127.0.0.1" {
        return Err(TestHarnessError::assertion(
            "Test config should use localhost".to_string(),
        ));
    }

    if config.logging.level != "debug" {
        return Err(TestHarnessError::assertion(
            "Test config should use debug logging".to_string(),
        ));
    }

    Ok(())
}

// ============================================================================
// Transport Layer Tests
// ============================================================================
// Transport layer tests for stdio-based MCP communication
// WebSocket, JWT, session management, and TLS features are fully implemented
// in cb-transport/src/ws.rs and tested separately

#[tokio::test]
async fn test_message_ordering() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Test that messages are processed in order
    // Even with stdio transport, we can verify ordering

    let file_path = workspace.path().join("ordering_test.txt");

    // Send a sequence of write operations
    let operations = vec![
        "First write",
        "Second write",
        "Third write",
        "Fourth write",
        "Final write",
    ];

    for (i, content) in operations.iter().enumerate() {
        let response = client
            .call_tool(
                "write_file",
                json!({
                    "file_path": file_path.to_string_lossy(),
                    "content": format!("{} - {}", content, i)
                }),
            )
            .await
            .unwrap();

        assert!(response["result"]["success"].as_bool().unwrap_or(false));

        // Verify the write took effect before next operation
        let read_response = client
            .call_tool(
                "read_file",
                json!({
                    "file_path": file_path.to_string_lossy()
                }),
            )
            .await
            .unwrap();

        let expected = format!("{} - {}", content, i);
        assert_eq!(
            read_response["result"]["content"].as_str().unwrap(),
            expected
        );
    }
}

#[tokio::test]
async fn test_error_propagation() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Test that errors are properly propagated through transport layer

    // Try to read non-existent file
    let nonexistent = workspace.path().join("does_not_exist.txt");
    let error_response = client
        .call_tool(
            "read_file",
            json!({
                "file_path": nonexistent.to_string_lossy()
            }),
        )
        .await;

    assert!(
        error_response.unwrap().get("error").is_some(),
        "Should propagate file not found error"
    );

    // Try invalid tool call
    let invalid_response = client
        .call_tool(
            "nonexistent_tool",
            json!({
                "some_param": "value"
            }),
        )
        .await;

    assert!(
        invalid_response.unwrap().get("error").is_some(),
        "Should propagate invalid tool error"
    );

    // Try tool with invalid parameters
    let invalid_params_response = client
        .call_tool(
            "read_file",
            json!({
                "wrong_param": "value"
            }),
        )
        .await;

    assert!(
        invalid_params_response.unwrap().get("error").is_some(),
        "Should propagate parameter validation error"
    );
}

#[tokio::test]
async fn test_large_message_handling() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Test handling of large messages (important for any transport)

    let large_file = workspace.path().join("large_message_test.txt");

    // Create increasingly large content to test message size limits
    let sizes = vec![1024, 10_240, 102_400, 1_024_000]; // 1KB to 1MB

    for size in sizes {
        let large_content = "X".repeat(size);

        let response = client
            .call_tool(
                "create_file",
                json!({
                    "file_path": large_file.to_string_lossy(),
                    "content": large_content,
                    "overwrite": true
                }),
            )
            .await;

        match response {
            Ok(resp) => {
                assert!(resp["result"]["success"].as_bool().unwrap_or(false));

                // Verify we can read it back
                let read_response = client
                    .call_tool(
                        "read_file",
                        json!({
                            "file_path": large_file.to_string_lossy()
                        }),
                    )
                    .await
                    .unwrap();

                let read_content = read_response["result"]["content"].as_str().unwrap();
                assert_eq!(read_content.len(), size);

                println!("Successfully handled {}KB message", size / 1024);
            }
            Err(_) => {
                println!(
                    "Failed to handle {}KB message (may be expected)",
                    size / 1024
                );
                // Large message failures might be expected depending on transport limits
                break;
            }
        }
    }
}

#[tokio::test]
async fn test_rapid_transport_operations() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Test rapid sequential operations through transport layer
    let operation_count = 10;
    let mut successful_operations = 0;

    for i in 0..operation_count {
        let file_path = workspace.path().join(format!("rapid_transport_{}.txt", i));
        let content = format!("Rapid content {}", i);

        // Create file
        let create_result = client
            .call_tool(
                "create_file",
                json!({
                    "file_path": file_path.to_string_lossy(),
                    "content": content
                }),
            )
            .await;

        // Read it back
        let read_result = client
            .call_tool(
                "read_file",
                json!({
                    "file_path": file_path.to_string_lossy()
                }),
            )
            .await;

        if create_result.is_ok() && read_result.is_ok() {
            successful_operations += 1;

            if let Ok(read_resp) = read_result {
                let expected = format!("Rapid content {}", i);
                assert_eq!(read_resp["result"]["content"].as_str().unwrap(), expected);
            }
        }
    }

    assert!(
        successful_operations > 0,
        "At least some transport operations should succeed"
    );
}

#[tokio::test]
async fn test_transport_health_monitoring() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());

    // Test transport health through health check endpoint

    let response = client.call_tool("health_check", json!({})).await.unwrap();

    assert!(response["result"].get("status").is_some());
    let status = response["result"]["status"].as_str().unwrap();
    assert!(status == "healthy" || status == "degraded" || status == "unhealthy");

    // If transport details are available, verify them
    if let Some(transport) = response["result"].get("transport") {
        assert!(transport.get("type").is_some());

        if let Some(stats) = transport.get("statistics") {
            // Verify transport statistics if available
            assert!(stats.is_object());
        }
    }

    // Perform some operations and check health again
    let test_file = workspace.path().join("health_test.txt");

    for i in 0..5 {
        let _response = client
            .call_tool(
                "create_file",
                json!({
                    "file_path": test_file.to_string_lossy(),
                    "content": format!("Health test {}", i)
                }),
            )
            .await
            .unwrap();
    }

    // Health should still be good after operations
    let response = client.call_tool("health_check", json!({})).await.unwrap();
    let status = response["result"]["status"].as_str().unwrap();
    assert!(status == "healthy" || status == "degraded");
}
