//! A fluent builder for setting up LSP integration tests.

use crate::harness::{MockLspService, RealLspService, TestWorkspace};
use codebuddy_foundation::protocol::{ ApiError , LspService };
use std::sync::Arc;

/// Test mode for LSP services
pub enum LspTestMode {
    /// Use a mock LSP service with predictable responses
    Mock,
    /// Use a real LSP server process
    Real,
}

/// A builder for constructing LSP feature tests.
pub struct LspTestBuilder {
    workspace: TestWorkspace,
    mode: LspTestMode,
    language: String,
    initialization_options: Option<serde_json::Value>,
}

impl LspTestBuilder {
    /// Create a new test builder for the given language extension (e.g., "ts", "py", "rs")
    pub fn new(language: &str) -> Self {
        Self {
            workspace: TestWorkspace::new(),
            mode: LspTestMode::Mock, // Default to mock for speed
            language: language.to_string(),
            initialization_options: None,
        }
    }

    /// Set the test to run against a real LSP server.
    pub fn with_real_lsp(mut self) -> Self {
        self.mode = LspTestMode::Real;
        self
    }

    /// Set custom initialization options for the LSP server.
    pub fn with_initialization_options(mut self, options: serde_json::Value) -> Self {
        self.initialization_options = Some(options);
        self
    }

    /// Create a file in the test workspace.
    pub fn with_file(self, path: &str, content: &str) -> Self {
        self.workspace.create_file(path, content);
        self
    }

    /// Build the test context for mock mode, returning concrete MockLspService.
    /// This allows direct access to mock configuration methods.
    pub async fn build_mock(self) -> Result<(Arc<MockLspService>, TestWorkspace), ApiError> {
        match self.mode {
            LspTestMode::Mock => {
                let mock_service = Arc::new(MockLspService::new());
                Ok((mock_service, self.workspace))
            }
            LspTestMode::Real => Err(ApiError::lsp(
                "build_mock() called but mode is Real. Use build() for trait object.".to_string(),
            )),
        }
    }

    /// Build the test context, providing an LspService trait object and TestWorkspace.
    /// Returns either a MockLspService or RealLspService based on the configured mode.
    /// For mock tests that need to configure responses, use build_mock() instead.
    pub async fn build(self) -> Result<(Arc<dyn LspService>, TestWorkspace), ApiError> {
        let lsp_service: Arc<dyn LspService> = match self.mode {
            LspTestMode::Mock => Arc::new(MockLspService::new()),
            LspTestMode::Real => {
                let root_path = self.workspace.path();
                let real_service = RealLspService::new_with_options(
                    &self.language,
                    root_path,
                    self.initialization_options,
                )
                .await?;
                Arc::new(real_service)
            }
        };

        Ok((lsp_service, self.workspace))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_builder_creates_mock_service() {
        let result = LspTestBuilder::new("ts")
            .with_file("test.ts", "const x = 1;")
            .build()
            .await;

        assert!(result.is_ok());
        let (service, workspace) = result.unwrap();

        // Verify workspace has the file
        assert!(workspace.path().join("test.ts").exists());

        // Verify service is available
        assert!(service.is_available("ts").await);
    }

    #[tokio::test]
    async fn test_builder_creates_multiple_files() {
        let result = LspTestBuilder::new("ts")
            .with_file("src/index.ts", "export const foo = 'bar';")
            .with_file("src/types.ts", "export type Foo = string;")
            .build()
            .await;

        assert!(result.is_ok());
        let (_service, workspace) = result.unwrap();

        assert!(workspace.path().join("src/index.ts").exists());
        assert!(workspace.path().join("src/types.ts").exists());
    }
}