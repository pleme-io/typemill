//! End-to-end flow tests

use cb_core::AppConfig;
use cb_server::{bootstrap, ServerOptions, AstService, LspService};
use tests::{create_test_config, TestHarnessError};

#[tokio::test]
async fn test_server_bootstrap_and_shutdown() -> Result<(), TestHarnessError> {
    // Create test configuration
    let config = create_test_config();
    let options = ServerOptions::from_config(config).with_debug(true);

    // Bootstrap the server
    let handle = bootstrap(options).await
        .map_err(|e| TestHarnessError::setup(format!("Bootstrap failed: {}", e)))?;

    // Start the server
    handle.start().await
        .map_err(|e| TestHarnessError::execution(format!("Start failed: {}", e)))?;

    // Give it a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Shutdown the server
    handle.shutdown().await
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
            "Test config should use port 3043".to_string()
        ));
    }

    if config.server.host != "127.0.0.1" {
        return Err(TestHarnessError::assertion(
            "Test config should use localhost".to_string()
        ));
    }

    if config.logging.level != "debug" {
        return Err(TestHarnessError::assertion(
            "Test config should use debug logging".to_string()
        ));
    }

    Ok(())
}

#[tokio::test]
async fn test_mock_services_integration() -> Result<(), TestHarnessError> {
    use tests::{MockAstService, MockLspService};
    use std::path::Path;

    // Create mock services
    let mut ast_service = MockAstService::new();
    let mut lsp_service = MockLspService::new();

    // Set up expectations
    ast_service
        .expect_build_import_graph()
        .times(1)
        .returning(|_| Ok(tests::create_test_import_graph("test.ts")));

    lsp_service
        .expect_is_available()
        .times(1)
        .with(mockall::predicate::eq("ts"))
        .returning(|_| true);

    // Use the mock services
    let import_graph = ast_service.build_import_graph(Path::new("test.ts")).await
        .map_err(|e| TestHarnessError::execution(format!("AST service failed: {}", e)))?;

    let is_available = lsp_service.is_available("ts").await;

    // Verify results
    if import_graph.source_file != "test.ts" {
        return Err(TestHarnessError::assertion(
            "Import graph should have correct source file".to_string()
        ));
    }

    if !is_available {
        return Err(TestHarnessError::assertion(
            "LSP service should be available for TypeScript".to_string()
        ));
    }

    Ok(())
}