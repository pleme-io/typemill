//! Advanced refactoring operations using AST analysis
use crate::error::AstResult;
use async_trait::async_trait;
use serde_json::Value;

// Re-export refactoring types from mill-lang-common for convenience
pub use mill_lang_common::{
    CodeRange, ExtractVariableAnalysis, ExtractableFunction, InlineVariableAnalysis, VariableUsage,
};

pub mod common;
pub mod extract_function;
pub mod extract_variable;
pub mod inline_variable;
pub mod move_symbol;

/// Trait for LSP refactoring service
///
/// This trait abstracts LSP code action requests to enable dependency injection
/// and testing without requiring a full LSP server.
#[async_trait]
pub trait LspRefactoringService: Send + Sync {
    /// Request code actions from LSP server
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file
    /// * `range` - Code range to refactor
    /// * `kinds` - Desired code action kinds (e.g., "refactor.extract.function")
    ///
    /// # Returns
    ///
    /// LSP CodeAction array or WorkspaceEdit
    async fn get_code_actions(
        &self,
        file_path: &str,
        range: &CodeRange,
        kinds: Option<Vec<String>>,
    ) -> AstResult<Value>;
}
