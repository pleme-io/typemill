#![allow(
    dead_code,
    unused_variables,
    clippy::mutable_key_type,
    clippy::needless_range_loop,
    clippy::ptr_arg,
    clippy::manual_clamp
)]

//! Extract operation handler - implements extract command with dryRun option
//!
//! Supports extracting code elements into new functions, variables, constants, or modules.
//! This handler reuses existing AST refactoring logic from mill-ast and language plugins.

use crate::handlers::tools::ToolHandler;
use async_trait::async_trait;
use lsp_types::{Position, Range, WorkspaceEdit};
use mill_ast::refactoring::CodeRange;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_foundation::protocol::{EditPlan, ExtractPlan, PlanMetadata, PlanSummary, RefactorPlan};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, error, info};

pub struct ExtractHandler;

impl ExtractHandler {
    pub fn new() -> Self {
        Self
    }

    /// Handle extract() tool call
    async fn handle_extract_plan(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        let args = tool_call.arguments.clone().unwrap_or(json!({}));

        // Deserialize parameters
        let params: ExtractPlanParams = serde_json::from_value(args).map_err(|e| {
            ServerError::invalid_request(format!("Invalid extract parameters: {}", e))
        })?;

        debug!(
            kind = %params.kind,
            file_path = %params.source.file_path,
            "Planning extract operation"
        );

        // Validate kind
        match params.kind.as_str() {
            "function" | "variable" | "constant" | "module" => {}
            other => {
                return Err(ServerError::invalid_request(format!(
                    "Unsupported extract kind: {}. Must be one of: function, variable, constant, module",
                    other
                )))
            }
        }

        // Generate the extract plan
        let plan = match params.kind.as_str() {
            "function" => {
                self.plan_extract_function(context, &params.source, &params.options)
                    .await?
            }
            "variable" => {
                self.plan_extract_variable(context, &params.source, &params.options)
                    .await?
            }
            "constant" => {
                self.plan_extract_constant(context, &params.source, &params.options)
                    .await?
            }
            "module" => {
                self.plan_extract_module(context, &params.source, &params.options)
                    .await?
            }
            _ => unreachable!("Already validated kind"),
        };

        // Wrap in RefactorPlan enum for discriminant
        let refactor_plan = RefactorPlan::ExtractPlan(plan);

        // Check if we should execute or just return plan
        if params.options.dry_run {
            // Return plan only (existing behavior - preview mode)
            let plan_json = serde_json::to_value(&refactor_plan)
                .map_err(|e| ServerError::internal(format!("Failed to serialize plan: {}", e)))?;

            info!(
                operation = "extract",
                dry_run = true,
                "Returning extract plan (preview mode)"
            );

            Ok(json!({
                "content": plan_json
            }))
        } else {
            // NEW: Execute the plan
            info!(
                operation = "extract",
                dry_run = false,
                "Executing extract plan"
            );

            use crate::handlers::tools::extensions::get_concrete_app_state;
            use mill_services::services::{ExecutionOptions, PlanExecutor};

            // Get concrete AppState to access concrete FileService
            let concrete_state = get_concrete_app_state(&context.app_state)?;
            let executor = PlanExecutor::new(concrete_state.file_service.clone());
            let result = executor
                .execute_plan(refactor_plan, ExecutionOptions::default())
                .await?;

            let result_json = serde_json::to_value(&result).map_err(|e| {
                ServerError::internal(format!("Failed to serialize execution result: {}", e))
            })?;

            info!(
                operation = "extract",
                success = result.success,
                applied_files = result.applied_files.len(),
                "Extract execution completed"
            );

            Ok(json!({
                "content": result_json
            }))
        }
    }

    /// Plan extract function operation
    async fn plan_extract_function(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        source: &SourceRange,
        options: &ExtractOptions,
    ) -> ServerResult<ExtractPlan> {
        // Convert LSP Range to CodeRange
        let code_range = CodeRange {
            start_line: source.range.start.line,
            start_col: source.range.start.character,
            end_line: source.range.end.line,
            end_col: source.range.end.character,
        };

        // Read file content
        let file_path = Path::new(&source.file_path);
        let file_content = context
            .app_state
            .file_service
            .read_file(file_path)
            .await
            .map_err(|e| ServerError::internal(format!("Failed to read file: {}", e)))?;

        // Call AST refactoring function directly without LSP service
        // Note: LSP integration removed as DirectLspAdapter doesn't implement LspRefactoringService

        // Get PluginDiscovery from language_plugins by downcasting
        let plugin_discovery = context
            .app_state
            .language_plugins
            .inner()
            .downcast_ref::<mill_plugin_api::PluginDiscovery>()
            .ok_or_else(|| ServerError::internal("Failed to downcast to PluginDiscovery"))?;

        let edit_plan = mill_ast::refactoring::extract_function::plan_extract_function(
            &file_content,
            &code_range,
            &source.name,
            &source.file_path,
            None,                   // No LSP service - use AST-only approach
            Some(plugin_discovery), // Pass plugin registry
        )
        .await
        .map_err(|e| ServerError::internal(format!("Extract function failed: {}", e)))?;

        // Convert EditPlan to ExtractPlan
        self.convert_edit_plan_to_extract_plan(
            edit_plan,
            &source.file_path,
            "function",
            context,
            options,
        )
        .await
    }

    /// Plan extract variable operation
    async fn plan_extract_variable(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        source: &SourceRange,
        options: &ExtractOptions,
    ) -> ServerResult<ExtractPlan> {
        // For now, extract variable uses similar logic to extract function
        // Language plugins can provide more specialized implementations
        let code_range = CodeRange {
            start_line: source.range.start.line,
            start_col: source.range.start.character,
            end_line: source.range.end.line,
            end_col: source.range.end.character,
        };

        let file_path = Path::new(&source.file_path);
        let file_content = context
            .app_state
            .file_service
            .read_file(file_path)
            .await
            .map_err(|e| ServerError::internal(format!("Failed to read file: {}", e)))?;

        // Try to use extract_variable if available, otherwise fall back to extract_function
        // and adapt the result (AST-only approach, no LSP service)

        // Get PluginDiscovery from language_plugins by downcasting
        let plugin_discovery = context
            .app_state
            .language_plugins
            .inner()
            .downcast_ref::<mill_plugin_api::PluginDiscovery>()
            .ok_or_else(|| ServerError::internal("Failed to downcast to PluginDiscovery"))?;

        let edit_plan = mill_ast::refactoring::extract_function::plan_extract_function(
            &file_content,
            &code_range,
            &source.name,
            &source.file_path,
            None,                   // No LSP service - use AST-only approach
            Some(plugin_discovery), // Pass plugin registry
        )
        .await
        .map_err(|e| ServerError::internal(format!("Extract variable failed: {}", e)))?;

        self.convert_edit_plan_to_extract_plan(
            edit_plan,
            &source.file_path,
            "variable",
            context,
            options,
        )
        .await
    }

    /// Plan extract constant operation
    async fn plan_extract_constant(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        source: &SourceRange,
        options: &ExtractOptions,
    ) -> ServerResult<ExtractPlan> {
        let file_path = Path::new(&source.file_path);
        let file_content = context
            .app_state
            .file_service
            .read_file(file_path)
            .await
            .map_err(|e| ServerError::internal(format!("Failed to read file: {}", e)))?;

        // Get PluginDiscovery from language_plugins by downcasting
        let plugin_discovery = context
            .app_state
            .language_plugins
            .inner()
            .downcast_ref::<mill_plugin_api::PluginDiscovery>()
            .ok_or_else(|| ServerError::internal("Failed to downcast to PluginDiscovery"))?;

        // Get the refactoring provider for this file type
        let provider = plugin_discovery
            .refactoring_provider_for_file(&source.file_path)
            .ok_or_else(|| {
                ServerError::not_found(format!(
                    "No refactoring provider found for: {}",
                    source.file_path
                ))
            })?;

        // Check if extract_constant is supported
        if !provider.supports_extract_constant() {
            return Err(ServerError::not_supported(format!(
                "Extract constant not supported for: {}",
                source.file_path
            )));
        }

        // Call the language plugin's extract_constant method
        // For extract constant, we use the cursor position (start of the range)
        let edit_plan = provider
            .plan_extract_constant(
                &file_content,
                source.range.start.line,
                source.range.start.character,
                &source.name,
                &source.file_path,
            )
            .await
            .map_err(|e| ServerError::internal(format!("Extract constant failed: {}", e)))?;

        self.convert_edit_plan_to_extract_plan(
            edit_plan,
            &source.file_path,
            "constant",
            context,
            options,
        )
        .await
    }

    /// Plan extract module operation
    async fn plan_extract_module(
        &self,
        _context: &mill_handler_api::ToolHandlerContext,
        source: &SourceRange,
        _options: &ExtractOptions,
    ) -> ServerResult<ExtractPlan> {
        // Module extraction is more complex and typically requires language plugin support
        // For now, return a not implemented error
        Err(ServerError::not_supported(format!(
            "Extract module not yet implemented for: {}",
            source.file_path
        )))
    }

    /// Convert EditPlan (from AST) to ExtractPlan (protocol type)
    async fn convert_edit_plan_to_extract_plan(
        &self,
        edit_plan: EditPlan,
        file_path: &str,
        kind: &str,
        context: &mill_handler_api::ToolHandlerContext,
        _options: &ExtractOptions,
    ) -> ServerResult<ExtractPlan> {
        // Convert EditPlan edits to LSP WorkspaceEdit
        let workspace_edit = self.convert_to_workspace_edit(&edit_plan)?;

        // Collect all affected files
        let mut affected_files = std::collections::HashSet::new();
        affected_files.insert(file_path.to_string());

        // Add any additional files from the edits
        for edit in &edit_plan.edits {
            if let Some(ref path) = edit.file_path {
                affected_files.insert(path.clone());
            }
        }

        // Count created/deleted files (extract operations typically don't create new files)
        let created_files = 0;
        let deleted_files = 0;

        let summary = PlanSummary {
            affected_files: affected_files.len(),
            created_files,
            deleted_files,
        };

        // Generate warnings if needed
        let warnings = Vec::new();

        // Generate metadata
        let language = crate::handlers::common::detect_language(file_path);
        let estimated_impact = if affected_files.len() <= 1 {
            "low"
        } else if affected_files.len() <= 3 {
            "medium"
        } else {
            "high"
        };

        let metadata = PlanMetadata {
            plan_version: "1.0".to_string(),
            kind: "extract".to_string(),
            language: language.to_string(),
            estimated_impact: estimated_impact.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        // Generate file checksums
        let file_checksums = self
            .generate_file_checksums(context, &affected_files)
            .await?;

        Ok(ExtractPlan {
            edits: workspace_edit,
            summary,
            warnings,
            metadata,
            file_checksums,
        })
    }

    /// Convert EditPlan edits to LSP WorkspaceEdit
    fn convert_to_workspace_edit(&self, edit_plan: &EditPlan) -> ServerResult<WorkspaceEdit> {
        let mut changes: HashMap<lsp_types::Uri, Vec<lsp_types::TextEdit>> = HashMap::new();

        // Sort edits by priority (highest first) to preserve execution order
        // LSP WorkspaceEdit doesn't have priority, so we must sort before conversion
        let mut sorted_edits = edit_plan.edits.clone();
        sorted_edits.sort_by(|a, b| b.priority.cmp(&a.priority));

        for edit in &sorted_edits {
            let file_path = edit.file_path.as_ref().unwrap_or(&edit_plan.source_file);

            // Convert file path to file:// URI
            let uri = url::Url::from_file_path(file_path)
                .map_err(|_| ServerError::internal(format!("Invalid file path: {}", file_path)))?
                .to_string()
                .parse::<lsp_types::Uri>()
                .map_err(|e| ServerError::internal(format!("Failed to parse URI: {}", e)))?;

            let lsp_edit = lsp_types::TextEdit {
                range: Range {
                    start: Position {
                        line: edit.location.start_line,
                        character: edit.location.start_column,
                    },
                    end: Position {
                        line: edit.location.end_line,
                        character: edit.location.end_column,
                    },
                },
                new_text: edit.new_text.clone(),
            };

            changes.entry(uri).or_default().push(lsp_edit);
        }

        Ok(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        })
    }

    /// Generate SHA-256 checksums for all affected files
    async fn generate_file_checksums(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        file_paths: &std::collections::HashSet<String>,
    ) -> ServerResult<HashMap<String, String>> {
        use crate::handlers::common::calculate_checksum;

        let mut checksums = HashMap::new();

        for file_path in file_paths {
            let path = Path::new(file_path);
            match context.app_state.file_service.read_file(path).await {
                Ok(content) => {
                    checksums.insert(file_path.clone(), calculate_checksum(&content));
                }
                Err(e) => {
                    error!(
                        file_path = %file_path,
                        error = %e,
                        "Failed to read file for checksum"
                    );
                    // Continue with other files, don't fail the entire operation
                }
            }
        }

        Ok(checksums)
    }
}

impl Default for ExtractHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for ExtractHandler {
    fn tool_names(&self) -> &[&str] {
        &["extract"]
    }

    fn is_internal(&self) -> bool {
        false // Public tool
    }

    async fn handle_tool_call(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        match tool_call.name.as_str() {
            "extract" => self.handle_extract_plan(context, tool_call).await,
            _ => Err(ServerError::not_supported(format!(
                "Unknown extract operation: {}",
                tool_call.name
            ))),
        }
    }
}

// Parameter structures

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExtractPlanParams {
    kind: String,
    source: SourceRange,
    #[serde(default)]
    options: ExtractOptions,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceRange {
    file_path: String,
    range: Range, // lsp_types::Range
    name: String,
    #[serde(default)]
    destination: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ExtractOptions {
    /// Preview mode - don't actually apply changes (default: true for safety)
    #[serde(default = "crate::default_true")]
    dry_run: bool,
    #[serde(default)]
    visibility: Option<String>, // "public" | "private"
    #[serde(default)]
    destination_path: Option<String>,
}
