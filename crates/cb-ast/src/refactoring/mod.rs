//! Advanced refactoring operations using AST analysis
use crate::error::AstResult;
use async_trait::async_trait;
use cb_protocol::EditLocation;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod common;
pub mod extract_function;
pub mod extract_variable;
pub mod inline_variable;

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

/// Range of selected code for extraction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

/// Variable usage information for refactoring analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VariableUsage {
    pub name: String,
    pub declaration_location: Option<CodeRange>,
    pub usages: Vec<CodeRange>,
    pub scope_depth: u32,
    pub is_parameter: bool,
    pub is_declared_in_selection: bool,
    pub is_used_after_selection: bool,
}

/// Information about a function that can be extracted
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractableFunction {
    pub selected_range: CodeRange,
    pub required_parameters: Vec<String>,
    pub return_variables: Vec<String>,
    pub suggested_name: String,
    pub insertion_point: CodeRange,
    pub contains_return_statements: bool,
    pub complexity_score: u32,
}

/// Analysis result for inline variable refactoring
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InlineVariableAnalysis {
    pub variable_name: String,
    pub declaration_range: CodeRange,
    pub initializer_expression: String,
    pub usage_locations: Vec<CodeRange>,
    pub is_safe_to_inline: bool,
    pub blocking_reasons: Vec<String>,
}

/// Analysis result for extract variable refactoring
#[derive(Debug, Clone)]
pub struct ExtractVariableAnalysis {
    pub expression: String,
    pub expression_range: CodeRange,
    pub can_extract: bool,
    pub suggested_name: String,
    pub insertion_point: CodeRange,
    pub blocking_reasons: Vec<String>,
    pub scope_type: String,
}

/// Convert CodeRange to EditLocation
impl From<CodeRange> for EditLocation {
    fn from(range: CodeRange) -> Self {
        EditLocation {
            start_line: range.start_line,
            start_column: range.start_col,
            end_line: range.end_line,
            end_column: range.end_col,
        }
    }
}