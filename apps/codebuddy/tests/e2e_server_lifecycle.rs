//! End-to-end server lifecycle tests
//!
//! Tests server bootstrap, configuration loading, and shutdown behavior.

use mill_server::{ bootstrap , ServerOptions };
use mill_test_support::create_test_config;
use mill_config::AppConfig;
use e2e::TestHarnessError;

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

