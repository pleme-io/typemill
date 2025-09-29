//! Concrete implementation of the AstService trait

use async_trait::async_trait;
use std::path::Path;

use cb_ast::{EditPlan, ImportGraph};
use cb_core::CoreError;
use cb_core::model::IntentSpec;

use crate::interfaces::AstService;

/// Default implementation of the AST service
pub struct DefaultAstService;

impl DefaultAstService {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AstService for DefaultAstService {
    async fn build_import_graph(&self, file: &Path) -> Result<ImportGraph, CoreError> {
        // Read the file content
        let content = tokio::fs::read_to_string(file).await?;

        // Directly use the advanced parser from the cb-ast crate
        cb_ast::parser::build_import_graph(&content, file)
            .map_err(|e| CoreError::internal(format!("AST parsing failed: {}", e)))
    }

    async fn plan_refactor(&self, intent: &IntentSpec, file: &Path) -> Result<EditPlan, CoreError> {
        match intent.name() {
            "rename_symbol_with_imports" => {
                // Extract parameters from intent
                let old_name = intent.arguments().get("oldName")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CoreError::invalid_data("Missing 'oldName' parameter in intent"))?;

                let new_name = intent.arguments().get("newName")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CoreError::invalid_data("Missing 'newName' parameter in intent"))?;

                // Call the plan_rename_refactor function from cb-ast
                cb_ast::refactoring::plan_rename_refactor(old_name, new_name, file)
                    .map_err(|e| CoreError::internal(format!("Refactoring planning failed: {}", e)))
            }
            _ => Err(CoreError::not_supported(format!("Intent '{}' is not supported for refactoring", intent.name())))
        }
    }
}