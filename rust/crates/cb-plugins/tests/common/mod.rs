//! Common test utilities for plugin testing
//!
//! This module provides helper structs and functions to simplify plugin integration testing.
//!
//! # PluginTestBuilder
//!
//! The `PluginTestBuilder` is a fluent API for setting up plugin tests with minimal boilerplate.
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use cb_plugins::*;
//! use common::PluginTestBuilder;
//!
//! #[tokio::test]
//! async fn test_my_plugin() {
//!     let result = PluginTestBuilder::new()
//!         .with_plugin("typescript", create_ts_plugin())
//!         .with_request("find_definition", "test.ts")
//!         .at_position(10, 5)
//!         .run()
//!         .await
//!         .unwrap();
//!
//!     // Use insta for snapshot testing
//!     insta::assert_yaml_snapshot!(result);
//! }
//! ```
//!
//! ## Snapshot Testing
//!
//! This test suite uses the `insta` crate for snapshot testing. Snapshots allow you to:
//! - Capture plugin responses and verify them against known-good outputs
//! - Detect unintended changes in plugin behavior
//! - Review changes via `cargo insta review` command
//!
//! When a test fails due to snapshot mismatch:
//! 1. Run `cargo insta review` to see the diff
//! 2. Accept changes if they're intentional: `cargo insta accept`
//! 3. Reject changes if they're bugs: fix the code and re-run tests

use async_trait::async_trait;
use cb_plugins::{
    Capabilities, LanguagePlugin, PluginError, PluginManager, PluginMetadata, PluginRequest,
    PluginResponse, PluginResult,
};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;

/// Builder for constructing plugin integration tests
///
/// This builder provides a fluent API for setting up plugin tests with minimal boilerplate.
/// It handles plugin registration, request construction, and execution.
///
/// # Example
///
/// ```rust,no_run
/// let result = PluginTestBuilder::new()
///     .with_plugin("typescript", my_plugin)
///     .with_request("find_definition", "test.ts")
///     .at_position(10, 5)
///     .with_params(json!({"symbol": "foo"}))
///     .run()
///     .await
///     .unwrap();
/// ```
pub struct PluginTestBuilder {
    manager: PluginManager,
    plugins: Vec<(String, Arc<dyn LanguagePlugin>)>,
    method: Option<String>,
    file_path: Option<PathBuf>,
    position: Option<(u32, u32)>,
    params: Option<Value>,
}

impl PluginTestBuilder {
    /// Create a new test builder with a fresh PluginManager
    pub fn new() -> Self {
        Self {
            manager: PluginManager::new(),
            plugins: Vec::new(),
            method: None,
            file_path: None,
            position: None,
            params: None,
        }
    }

    /// Register a plugin with the test manager
    ///
    /// # Arguments
    /// * `name` - Plugin name (e.g., "typescript", "python")
    /// * `plugin` - Plugin instance implementing LanguagePlugin
    pub fn with_plugin(
        mut self,
        name: impl Into<String>,
        plugin: Arc<dyn LanguagePlugin>,
    ) -> Self {
        self.plugins.push((name.into(), plugin));
        self
    }

    /// Set the method/capability to test
    ///
    /// # Arguments
    /// * `method` - Method name (e.g., "find_definition", "get_hover")
    /// * `file_path` - Path to the file being tested
    pub fn with_request(mut self, method: impl Into<String>, file_path: impl Into<PathBuf>) -> Self {
        self.method = Some(method.into());
        self.file_path = Some(file_path.into());
        self
    }

    /// Set the cursor position for the request
    ///
    /// # Arguments
    /// * `line` - Line number (0-indexed)
    /// * `character` - Character offset (0-indexed)
    pub fn at_position(mut self, line: u32, character: u32) -> Self {
        self.position = Some((line, character));
        self
    }

    /// Add custom parameters to the request
    pub fn with_params(mut self, params: Value) -> Self {
        self.params = Some(params);
        self
    }

    /// Execute the test and return the plugin response
    ///
    /// This method:
    /// 1. Registers all plugins with the manager
    /// 2. Constructs a PluginRequest from the builder state
    /// 3. Executes the request via the PluginManager
    /// 4. Returns the PluginResponse
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No request method/file was specified
    /// - Plugin registration fails
    /// - Request execution fails
    pub async fn run(self) -> PluginResult<PluginResponse> {
        // Validate required fields
        let method = self
            .method
            .ok_or_else(|| PluginError::request_failed("test", "No method specified"))?;
        let file_path = self
            .file_path
            .ok_or_else(|| PluginError::request_failed("test", "No file path specified"))?;

        // Register all plugins
        for (name, plugin) in self.plugins {
            self.manager.register_plugin(&name, plugin).await?;
        }

        // Build request
        let mut request = PluginRequest::new(&method, file_path);

        if let Some((line, character)) = self.position {
            request = request.with_position(line, character);
        }

        if let Some(params) = self.params {
            request = request.with_params(params);
        }

        // Execute request
        self.manager.handle_request(request).await
    }

    /// Execute the test and return the plugin manager for further inspection
    ///
    /// Use this when you need to perform multiple operations or inspect manager state.
    pub async fn build(self) -> PluginResult<PluginManager> {
        for (name, plugin) in self.plugins {
            self.manager.register_plugin(&name, plugin).await?;
        }
        Ok(self.manager)
    }
}

impl Default for PluginTestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock LSP service for testing
///
/// This service provides canned responses for common LSP methods,
/// making it easy to test plugin behavior without a real language server.
pub struct MockLspService {
    pub name: String,
    pub responses: std::collections::HashMap<String, Value>,
}

impl MockLspService {
    /// Create a new mock LSP service with default responses
    pub fn new(name: impl Into<String>) -> Self {
        let mut responses = std::collections::HashMap::new();

        // Default definition response
        responses.insert(
            "textDocument/definition".to_string(),
            serde_json::json!([{
                "uri": "file:///test.ts",
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 0, "character": 10 }
                }
            }]),
        );

        // Default hover response
        responses.insert(
            "textDocument/hover".to_string(),
            serde_json::json!({
                "contents": "Mock hover content"
            }),
        );

        Self {
            name: name.into(),
            responses,
        }
    }

    /// Add a custom response for a specific method
    pub fn with_response(mut self, method: impl Into<String>, response: Value) -> Self {
        self.responses.insert(method.into(), response);
        self
    }
}

#[async_trait]
impl cb_plugins::LspService for MockLspService {
    async fn request(&self, method: &str, _params: Value) -> Result<Value, String> {
        self.responses
            .get(method)
            .cloned()
            .ok_or_else(|| format!("No mock response for method: {}", method))
    }

    fn supports_extension(&self, extension: &str) -> bool {
        ["ts", "tsx", "js", "jsx", "py", "rs", "go"].contains(&extension)
    }

    fn service_name(&self) -> String {
        self.name.clone()
    }
}

/// Create a simple test plugin for testing purposes
///
/// This is a minimal plugin that can be used in tests without complex setup.
pub fn create_test_plugin(
    name: impl Into<String>,
    extensions: Vec<String>,
    capabilities: Capabilities,
) -> Arc<dyn LanguagePlugin> {
    struct SimpleTestPlugin {
        name: String,
        extensions: Vec<String>,
        capabilities: Capabilities,
    }

    #[async_trait]
    impl LanguagePlugin for SimpleTestPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata::new(&self.name, "1.0.0", "test")
        }

        fn supported_extensions(&self) -> Vec<String> {
            self.extensions.clone()
        }

        fn capabilities(&self) -> Capabilities {
            self.capabilities.clone()
        }

        async fn handle_request(&self, _request: PluginRequest) -> PluginResult<PluginResponse> {
            Ok(PluginResponse::success(
                serde_json::json!({"test": "response"}),
                &self.name,
            ))
        }

        fn configure(&self, _config: Value) -> PluginResult<()> {
            Ok(())
        }

        fn tool_definitions(&self) -> Vec<Value> {
            vec![]
        }
    }

    Arc::new(SimpleTestPlugin {
        name: name.into(),
        extensions,
        capabilities,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cb_plugins::LspService;

    #[tokio::test]
    async fn test_builder_basic_usage() {
        let mut caps = Capabilities::default();
        caps.navigation.go_to_definition = true;

        let plugin = create_test_plugin("test", vec!["test".to_string()], caps);

        let result = PluginTestBuilder::new()
            .with_plugin("test", plugin)
            .with_request("find_definition", "file.test")
            .at_position(10, 5)
            .run()
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.success);
    }

    #[tokio::test]
    async fn test_mock_lsp_service() {
        let service = MockLspService::new("test-lsp");

        let result = service
            .request("textDocument/definition", serde_json::json!({}))
            .await;

        assert!(result.is_ok());
    }
}