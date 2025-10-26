#![allow(
    dead_code,
    unused_variables,
    clippy::mutable_key_type,
    clippy::needless_range_loop,
    clippy::ptr_arg,
    clippy::manual_clamp
)]

//! Inline operation handler - implements inline command with dryRun option
//!
//! Supports inlining variables, functions, and constants by replacing references
//! with their definitions. This handler reuses existing AST refactoring logic.

use crate::handlers::tools::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use lsp_types::{Position, Range, WorkspaceEdit};
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::protocol::{
    ApiError as ServerError, ApiResult as ServerResult, EditPlan, InlinePlan, PlanMetadata,
    PlanSummary, RefactorPlan,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, error, info};

pub struct InlineHandler;

impl InlineHandler {
    pub fn new() -> Self {
        Self
    }

    /// Handle inline() tool call
    async fn handle_inline_plan(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        let args = tool_call.arguments.clone().unwrap_or(json!({}));

        // Deserialize parameters
        let params: InlinePlanParams = serde_json::from_value(args).map_err(|e| {
            ServerError::InvalidRequest(format!("Invalid inline parameters: {}", e))
        })?;

        debug!(
            kind = %params.kind,
            file_path = %params.target.file_path,
            line = params.target.position.line,
            character = params.target.position.character,
            "Planning inline operation"
        );

        // Validate kind
        match params.kind.as_str() {
            "variable" | "function" | "constant" => {}
            other => {
                return Err(ServerError::InvalidRequest(format!(
                    "Unsupported inline kind: {}. Must be one of: variable, function, constant",
                    other
                )))
            }
        }

        // Generate the inline plan
        let plan = match params.kind.as_str() {
            "variable" => {
                self.plan_inline_variable(context, &params.target, &params.options)
                    .await?
            }
            "function" => {
                self.plan_inline_function(context, &params.target, &params.options)
                    .await?
            }
            "constant" => {
                self.plan_inline_constant(context, &params.target, &params.options)
                    .await?
            }
            _ => unreachable!("Already validated kind"),
        };

        // Wrap in RefactorPlan enum for discriminant
        let refactor_plan = RefactorPlan::InlinePlan(plan);

        // Check if we should execute or just return plan
        if params.options.dry_run {
            // Return plan only (preview mode)
            let plan_json = serde_json::to_value(&refactor_plan)
                .map_err(|e| ServerError::Internal(format!("Failed to serialize plan: {}", e)))?;

            info!(
                operation = "inline",
                dry_run = true,
                "Returning inline plan (preview mode)"
            );

            Ok(json!({"content": plan_json}))
        } else {
            // Execute the plan
            info!(
                operation = "inline",
                dry_run = false,
                "Executing inline plan"
            );

            use mill_services::services::{ExecutionOptions, PlanExecutor};

            let executor = PlanExecutor::new(context.app_state.file_service.clone());
            let result = executor
                .execute_plan(refactor_plan, ExecutionOptions::default())
                .await?;

            let result_json = serde_json::to_value(&result).map_err(|e| {
                ServerError::Internal(format!("Failed to serialize execution result: {}", e))
            })?;

            info!(
                operation = "inline",
                success = result.success,
                applied_files = result.applied_files.len(),
                "Inline execution completed"
            );

            Ok(json!({"content": result_json}))
        }
    }

    /// Plan inline variable operation
    async fn plan_inline_variable(
        &self,
        context: &ToolHandlerContext,
        target: &InlineTarget,
        options: &InlineOptions,
    ) -> ServerResult<InlinePlan> {
        // Read file content
        let file_path = Path::new(&target.file_path);
        let file_content = context
            .app_state
            .file_service
            .read_file(file_path)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to read file: {}", e)))?;

        // Call AST refactoring function directly without LSP service
        // Note: LSP integration removed as DirectLspAdapter doesn't implement LspRefactoringService
        let edit_plan = mill_ast::refactoring::inline_variable::plan_inline_variable(
            &file_content,
            target.position.line,
            target.position.character,
            &target.file_path,
            None, // No LSP service - use AST-only approach
            Some(&context.app_state.language_plugins.inner), // Pass plugin registry
        )
        .await
        .map_err(|e| ServerError::Internal(format!("Inline variable failed: {}", e)))?;

        // Convert EditPlan to InlinePlan
        self.convert_edit_plan_to_inline_plan(
            edit_plan,
            &target.file_path,
            "variable",
            context,
            options,
        )
        .await
    }

    /// Plan inline function operation
    async fn plan_inline_function(
        &self,
        context: &ToolHandlerContext,
        target: &InlineTarget,
        options: &InlineOptions,
    ) -> ServerResult<InlinePlan> {
        // Function inlining uses similar logic to variable inlining
        // Language plugins can provide more specialized implementations (AST-only approach)
        let file_path = Path::new(&target.file_path);
        let file_content = context
            .app_state
            .file_service
            .read_file(file_path)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to read file: {}", e)))?;

        // Try AST-based inline for functions
        let edit_plan = mill_ast::refactoring::inline_variable::plan_inline_variable(
            &file_content,
            target.position.line,
            target.position.character,
            &target.file_path,
            None, // No LSP service - use AST-only approach
            Some(&context.app_state.language_plugins.inner), // Pass plugin registry
        )
        .await
        .map_err(|e| ServerError::Internal(format!("Inline function failed: {}", e)))?;

        self.convert_edit_plan_to_inline_plan(
            edit_plan,
            &target.file_path,
            "function",
            context,
            options,
        )
        .await
    }

    /// Plan inline constant operation
    async fn plan_inline_constant(
        &self,
        context: &ToolHandlerContext,
        target: &InlineTarget,
        options: &InlineOptions,
    ) -> ServerResult<InlinePlan> {
        // Constants use similar logic to variables (AST-only approach)
        let file_path = Path::new(&target.file_path);
        let file_content = context
            .app_state
            .file_service
            .read_file(file_path)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to read file: {}", e)))?;

        // Use AST-only approach for inlining constants
        let edit_plan = mill_ast::refactoring::inline_variable::plan_inline_variable(
            &file_content,
            target.position.line,
            target.position.character,
            &target.file_path,
            None, // No LSP service - use AST-only approach
            Some(&context.app_state.language_plugins.inner), // Pass plugin registry
        )
        .await
        .map_err(|e| ServerError::Internal(format!("Inline constant failed: {}", e)))?;

        self.convert_edit_plan_to_inline_plan(
            edit_plan,
            &target.file_path,
            "constant",
            context,
            options,
        )
        .await
    }

    /// Convert EditPlan (from AST) to InlinePlan (protocol type)
    async fn convert_edit_plan_to_inline_plan(
        &self,
        edit_plan: EditPlan,
        file_path: &str,
        kind: &str,
        context: &ToolHandlerContext,
        _options: &InlineOptions,
    ) -> ServerResult<InlinePlan> {
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

        // Count created/deleted files (inline operations don't create or delete files)
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
        let language = self.detect_language(file_path);
        let estimated_impact = if affected_files.len() <= 1 {
            "low"
        } else if affected_files.len() <= 3 {
            "medium"
        } else {
            "high"
        };

        let metadata = PlanMetadata {
            plan_version: "1.0".to_string(),
            kind: "inline".to_string(),
            language: language.to_string(),
            estimated_impact: estimated_impact.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        // Generate file checksums
        let file_checksums = self
            .generate_file_checksums(context, &affected_files)
            .await?;

        Ok(InlinePlan {
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

        for edit in &edit_plan.edits {
            let file_path = edit.file_path.as_ref().unwrap_or(&edit_plan.source_file);

            // Convert file path to file:// URI
            let uri = url::Url::from_file_path(file_path)
                .map_err(|_| ServerError::Internal(format!("Invalid file path: {}", file_path)))?
                .to_string()
                .parse::<lsp_types::Uri>()
                .map_err(|e| ServerError::Internal(format!("Failed to parse URI: {}", e)))?;

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

    /// Detect language from file extension
    fn detect_language(&self, file_path: &str) -> &str {
        let path = Path::new(file_path);
        match path.extension().and_then(|e| e.to_str()) {
            Some("rs") => "rust",
            Some("ts") | Some("tsx") => "typescript",
            Some("js") | Some("jsx") => "javascript",
            Some("py") => "python",
            Some("go") => "go",
            Some("java") => "java",
            _ => "unknown",
        }
    }

    /// Generate SHA-256 checksums for all affected files
    async fn generate_file_checksums(
        &self,
        context: &ToolHandlerContext,
        file_paths: &std::collections::HashSet<String>,
    ) -> ServerResult<HashMap<String, String>> {
        use sha2::{Digest, Sha256};

        let mut checksums = HashMap::new();

        for file_path in file_paths {
            let path = Path::new(file_path);
            match context.app_state.file_service.read_file(path).await {
                Ok(content) => {
                    let mut hasher = Sha256::new();
                    hasher.update(content.as_bytes());
                    let hash = hasher.finalize();
                    // Use bare hex format (no prefix) to match rename_handler and workspace_apply_handler
                    let hash_str = format!("{:x}", hash);
                    checksums.insert(file_path.clone(), hash_str);
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

impl Default for InlineHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for InlineHandler {
    fn tool_names(&self) -> &[&str] {
        &["inline"]
    }

    fn is_internal(&self) -> bool {
        false // Public tool
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        match tool_call.name.as_str() {
            "inline" => self.handle_inline_plan(context, tool_call).await,
            _ => Err(ServerError::Unsupported(format!(
                "Unknown inline operation: {}",
                tool_call.name
            ))),
        }
    }
}

// Parameter structures

#[derive(Debug, Deserialize, Serialize)]
struct InlinePlanParams {
    kind: String,
    target: InlineTarget,
    #[serde(default)]
    options: InlineOptions,
}

#[derive(Debug, Deserialize, Serialize)]
struct InlineTarget {
    file_path: String,
    position: Position, // lsp_types::Position
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct InlineOptions {
    /// Preview mode - don't actually apply changes (default: true for safety)
    #[serde(default = "default_true")]
    dry_run: bool,
    #[serde(default)]
    inline_all: Option<bool>, // Default: false - inline all usages vs current only
}

impl Default for InlineOptions {
    fn default() -> Self {
        Self {
            dry_run: true, // Safe default: preview mode
            inline_all: None,
        }
    }
}

fn default_true() -> bool {
    true
}
