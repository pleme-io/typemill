//! Mock implementations for testing

use async_trait::async_trait;
use mill_foundation::protocol::{
    ApiError, AstService, CacheStats, ImportGraph, LspService, Message,
};
use mockall::mock;
use std::path::Path;

mock! {
    pub AstService {}

    #[async_trait]
    impl AstService for AstService {
        async fn build_import_graph(&self, file: &Path) -> Result<ImportGraph, ApiError>;
        async fn cache_stats(&self) -> CacheStats;
    }
}

mock! {
    pub LspService {}

    #[async_trait]
    impl LspService for LspService {
        async fn request(&self, message: Message) -> Result<Message, ApiError>;
        async fn is_available(&self, extension: &str) -> bool;
        async fn restart_servers(&self, extensions: Option<Vec<String>>) -> Result<(), ApiError>;
        async fn notify_file_opened(&self, file_path: &Path) -> Result<(), ApiError>;
    }
}

/// Create a mock AST service for testing
pub fn mock_ast_service() -> MockAstService {
    MockAstService::new()
}

/// Create a mock LSP service for testing
pub fn mock_lsp_service() -> MockLspService {
    MockLspService::new()
}
