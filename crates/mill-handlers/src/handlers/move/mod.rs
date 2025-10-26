//! Move handler for Unified Refactoring API
//!
//! Implements `move` command with dryRun option for:
//! - Symbol moving (via LSP if available, else AST fallback)
//! - File moving (via FileService)
//! - Directory moving (via FileService)
//! - Module moving (via AST)
//!
//! # Architecture
//!
//! The move handler uses a dispatcher pattern to route requests to specialized
//! handlers based on the `target.kind` field:
//!
//! - `symbol` → `symbol_move::plan_symbol_move`
//! - `file` → `file_move::plan_file_move`
//! - `directory` → `directory_move::plan_directory_move`
//! - `module` → `plan_module_move` (not yet implemented)
//!
//! Each specialized handler is responsible for:
//! 1. Validating input parameters
//! 2. Computing the move plan (edits, checksums, warnings)
//! 3. Returning a `MovePlan` with metadata
//!
//! # Modules
//!
//! - `file_move` - Single-file move operations
//! - `directory_move` - Directory move operations
//! - `symbol_move` - LSP-based symbol moves
//! - `validation` - Checksum calculation and conflict detection

mod converter;
mod directory_move;
mod file_move;
mod symbol_move;
mod validation;

use crate::handlers::tools::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::protocol::{ refactor_plan::MovePlan , ApiError as ServerError , ApiResult as ServerResult , RefactorPlan , };
use lsp_types::Position;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Handler for move operations
pub struct MoveHandler;

impl MoveHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MoveHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Reserved for future options support
struct MovePlanParams {
    target: MoveTarget,
    destination: String,
    #[serde(default)]
    options: MoveOptions,
}

#[derive(Debug, Deserialize)]
struct MoveTarget {
    kind: String, // "symbol" | "file" | "directory" | "module"
    path: String,
    #[serde(default)]
    selector: Option<SymbolSelector>,
}

#[derive(Debug, Deserialize)]
struct SymbolSelector {
    position: Position,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Reserved for future configuration
struct MoveOptions {
    /// Preview mode - don't actually apply changes (default: true for safety)
    #[serde(default = "default_true")]
    dry_run: bool,
    #[serde(default)]
    update_imports: Option<bool>,
    #[serde(default)]
    preserve_formatting: Option<bool>,
}

fn default_true() -> bool {
    true
}

#[async_trait]
impl ToolHandler for MoveHandler {
    fn tool_names(&self) -> &[&str] {
        &["move"]
    }

    fn is_internal(&self) -> bool {
        false // Public tool
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        // Generate unique operation ID for tracing this entire operation
        let operation_id = Uuid::new_v4().to_string();

        info!(
            operation_id = %operation_id,
            tool_name = %tool_call.name,
            "Starting move operation"
        );

        // Parse parameters
        let args = tool_call.arguments.clone().ok_or_else(|| {
            error!(
                operation_id = %operation_id,
                "Missing arguments for move"
            );
            ServerError::InvalidRequest("Missing arguments for move".into())
        })?;

        let params: MovePlanParams = serde_json::from_value(args.clone()).map_err(|e| {
            error!(
                operation_id = %operation_id,
                error = %e,
                arguments = ?args,
                "Failed to parse move parameters"
            );
            ServerError::InvalidRequest(format!("Invalid move parameters: {}", e))
        })?;

        info!(
            operation_id = %operation_id,
            kind = %params.target.kind,
            source_path = %params.target.path,
            destination_path = %params.destination,
            has_selector = params.target.selector.is_some(),
            "Parsed move parameters, dispatching to handler"
        );

        // Dispatch based on target kind
        let plan = self
            .dispatch_move_plan(&params, context, &operation_id)
            .await
            .map_err(|e| {
                error!(
                    operation_id = %operation_id,
                    error = %e,
                    kind = %params.target.kind,
                    source_path = %params.target.path,
                    destination_path = %params.destination,
                    function = "dispatch_move_plan",
                    "Failed to generate move plan"
                );
                e
            })?;

        info!(
            operation_id = %operation_id,
            affected_files = plan.summary.affected_files,
            created_files = plan.summary.created_files,
            deleted_files = plan.summary.deleted_files,
            warnings_count = plan.warnings.len(),
            "Move plan generated successfully"
        );

        // Wrap in RefactorPlan enum for discriminant
        let refactor_plan = RefactorPlan::MovePlan(plan);

        // Check if we should execute or just return plan
        if params.options.dry_run {
            // Return plan only (preview mode)
            let plan_json = serde_json::to_value(&refactor_plan).map_err(|e| {
                error!(
                    operation_id = %operation_id,
                    error = %e,
                    "Failed to serialize move plan to JSON"
                );
                ServerError::Internal(format!("Failed to serialize move plan: {}", e))
            })?;

            info!(
                operation_id = %operation_id,
                operation = "move",
                dry_run = true,
                "Returning move plan (preview mode)"
            );

            Ok(json!({"content": plan_json}))
        } else {
            // Execute the plan
            info!(
                operation_id = %operation_id,
                operation = "move",
                dry_run = false,
                "Executing move plan"
            );

            use mill_services::services::{ExecutionOptions, PlanExecutor};

            let executor = PlanExecutor::new(context.app_state.file_service.clone());
            let result = executor
                .execute_plan(refactor_plan, ExecutionOptions::default())
                .await?;

            let result_json = serde_json::to_value(&result).map_err(|e| {
                error!(
                    operation_id = %operation_id,
                    error = %e,
                    "Failed to serialize execution result"
                );
                ServerError::Internal(format!("Failed to serialize execution result: {}", e))
            })?;

            info!(
                operation_id = %operation_id,
                operation = "move",
                success = result.success,
                applied_files = result.applied_files.len(),
                "Move execution completed"
            );

            Ok(json!({"content": result_json}))
        }
    }
}

impl MoveHandler {
    /// Dispatch to appropriate move handler based on target kind
    async fn dispatch_move_plan(
        &self,
        params: &MovePlanParams,
        context: &ToolHandlerContext,
        operation_id: &str,
    ) -> ServerResult<MovePlan> {
        debug!(
            operation_id = %operation_id,
            kind = %params.target.kind,
            "Dispatching to specific move handler"
        );

        match params.target.kind.as_str() {
            "symbol" => {
                debug!(operation_id = %operation_id, "Routing to symbol_move handler");
                self.handle_symbol_move(params, context, operation_id).await
            }
            "file" => {
                debug!(operation_id = %operation_id, "Routing to file_move handler");
                self.handle_file_move(params, context, operation_id).await
            }
            "directory" => {
                debug!(operation_id = %operation_id, "Routing to directory_move handler");
                self.handle_directory_move(params, context, operation_id)
                    .await
            }
            "module" => {
                debug!(operation_id = %operation_id, "Routing to module_move handler");
                self.handle_module_move(params, context, operation_id).await
            }
            kind => {
                warn!(
                    operation_id = %operation_id,
                    unsupported_kind = %kind,
                    "Unsupported move kind requested"
                );
                Err(ServerError::InvalidRequest(format!(
                    "Unsupported move kind: {}. Must be one of: symbol, file, directory, module",
                    kind
                )))
            }
        }
    }

    /// Handle symbol move operation
    async fn handle_symbol_move(
        &self,
        params: &MovePlanParams,
        context: &ToolHandlerContext,
        operation_id: &str,
    ) -> ServerResult<MovePlan> {
        // Extract position from selector
        let position = params
            .target
            .selector
            .as_ref()
            .ok_or_else(|| {
                error!(
                    operation_id = %operation_id,
                    path = %params.target.path,
                    "Symbol move requires selector.position but none was provided"
                );
                ServerError::InvalidRequest("Symbol move requires selector.position".into())
            })?
            .position;

        debug!(
            operation_id = %operation_id,
            path = %params.target.path,
            destination = %params.destination,
            line = position.line,
            character = position.character,
            "Delegating to symbol_move::plan_symbol_move"
        );

        symbol_move::plan_symbol_move(
            &params.target.path,
            &params.destination,
            position,
            context,
            operation_id,
        )
        .await
    }

    /// Handle file move operation
    async fn handle_file_move(
        &self,
        params: &MovePlanParams,
        context: &ToolHandlerContext,
        operation_id: &str,
    ) -> ServerResult<MovePlan> {
        let old_path = Path::new(&params.target.path);
        let new_path = Path::new(&params.destination);

        debug!(
            operation_id = %operation_id,
            old_path = %old_path.display(),
            new_path = %new_path.display(),
            "Delegating to file_move::plan_file_move"
        );

        file_move::plan_file_move(old_path, new_path, context, operation_id).await
    }

    /// Handle directory move operation
    async fn handle_directory_move(
        &self,
        params: &MovePlanParams,
        context: &ToolHandlerContext,
        operation_id: &str,
    ) -> ServerResult<MovePlan> {
        let old_path = Path::new(&params.target.path);
        let new_path = Path::new(&params.destination);

        debug!(
            operation_id = %operation_id,
            old_path = %old_path.display(),
            new_path = %new_path.display(),
            "Delegating to directory_move::plan_directory_move"
        );

        directory_move::plan_directory_move(old_path, new_path, context, operation_id).await
    }

    /// Handle module move operation (not yet implemented)
    async fn handle_module_move(
        &self,
        _params: &MovePlanParams,
        _context: &ToolHandlerContext,
        operation_id: &str,
    ) -> ServerResult<MovePlan> {
        warn!(
            operation_id = %operation_id,
            "Module move requested but not yet implemented"
        );
        // Module move is complex and requires language-specific support
        // Would use extract_module_to_package or similar AST functions
        Err(ServerError::Unsupported(
            "Module move not yet implemented. Requires language plugin support.".into(),
        ))
    }
}