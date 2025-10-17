//! Rename handler for Unified Refactoring API
//!
//! Implements `rename.plan` command for:
//! - Symbol renaming (via LSP)
//! - File renaming (via FileService)
//! - Directory renaming (via FileService)

mod directory_rename;
mod file_rename;
mod symbol_rename;
mod utils;

use crate::handlers::tools::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::{
    refactor_plan::{PlanSummary, PlanWarning},
    ApiError as ServerError, ApiResult as ServerResult, RefactorPlan,
};
use lsp_types::{Position, WorkspaceEdit};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use tracing::{debug, info};

/// Handler for rename.plan operations
pub struct RenameHandler;

impl RenameHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RenameHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Reserved for future options support
pub(crate) struct RenamePlanParams {
    target: RenameTarget,
    new_name: String,
    #[serde(default)]
    options: RenameOptions,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RenameTarget {
    kind: String, // "symbol" | "file" | "directory"
    path: String,
    #[serde(default)]
    selector: Option<SymbolSelector>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SymbolSelector {
    position: Position,
}

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)] // Reserved for future configuration
pub(crate) struct RenameOptions {
    #[serde(default)]
    strict: Option<bool>,
    #[serde(default)]
    validate_scope: Option<bool>,
    #[serde(default)]
    update_imports: Option<bool>,

    /// Scope configuration for what to update
    #[serde(default)]
    pub scope: Option<String>, // "code-only" | "all" | "custom"

    /// Custom scope configuration (when scope="custom")
    #[serde(default)]
    pub custom_scope: Option<cb_core::rename_scope::RenameScope>,
}

impl RenameOptions {
    /// Build RenameScope from options
    pub fn to_rename_scope(&self) -> Option<cb_core::rename_scope::RenameScope> {
        match self.scope.as_deref() {
            Some("code-only") => Some(cb_core::rename_scope::RenameScope::code_only()),
            Some("all") | None => Some(cb_core::rename_scope::RenameScope::all()),
            Some("custom") => self.custom_scope.clone(),
            _ => None,
        }
    }
}

#[async_trait]
impl ToolHandler for RenameHandler {
    fn tool_names(&self) -> &[&str] {
        &["rename.plan"]
    }

    fn is_internal(&self) -> bool {
        false // Public tool
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        info!(tool_name = %tool_call.name, "Handling rename.plan");

        // Parse parameters
        let args = tool_call.arguments.clone().ok_or_else(|| {
            ServerError::InvalidRequest("Missing arguments for rename.plan".into())
        })?;

        let params: RenamePlanParams = serde_json::from_value(args).map_err(|e| {
            ServerError::InvalidRequest(format!("Invalid rename.plan parameters: {}", e))
        })?;

        debug!(
            kind = %params.target.kind,
            path = %params.target.path,
            new_name = %params.new_name,
            "Generating rename plan"
        );

        // Dispatch based on target kind
        let plan = match params.target.kind.as_str() {
            "symbol" => self.plan_symbol_rename(&params, context).await?,
            "file" => self.plan_file_rename(&params, context).await?,
            "directory" => self.plan_directory_rename(&params, context).await?,
            kind => {
                return Err(ServerError::InvalidRequest(format!(
                    "Unsupported rename kind: {}. Must be one of: symbol, file, directory",
                    kind
                )));
            }
        };

        // Wrap in RefactorPlan enum for discriminant, then serialize for MCP protocol
        let refactor_plan = RefactorPlan::RenamePlan(plan);
        let plan_json = serde_json::to_value(&refactor_plan).map_err(|e| {
            ServerError::Internal(format!("Failed to serialize rename plan: {}", e))
        })?;

        Ok(json!({
            "content": plan_json
        }))
    }
}

impl RenameHandler {
    /// Analyze WorkspaceEdit to calculate checksums and summary
    pub(crate) async fn analyze_workspace_edit(
        &self,
        edit: &WorkspaceEdit,
        context: &ToolHandlerContext,
    ) -> ServerResult<(HashMap<String, String>, PlanSummary, Vec<PlanWarning>)> {
        let mut file_checksums = HashMap::new();
        let mut affected_files: HashSet<std::path::PathBuf> = HashSet::new();

        // Extract file paths from WorkspaceEdit
        if let Some(ref changes) = edit.changes {
            for (uri, _edits) in changes {
                let path = std::path::PathBuf::from(
                    urlencoding::decode(uri.path().as_str())
                        .map_err(|_| ServerError::Internal("Invalid URI path".to_string()))?
                        .into_owned(),
                );
                affected_files.insert(path);
            }
        }

        if let Some(ref document_changes) = edit.document_changes {
            match document_changes {
                lsp_types::DocumentChanges::Edits(edits) => {
                    for edit in edits {
                        let path = std::path::PathBuf::from(
                            urlencoding::decode(edit.text_document.uri.path().as_str())
                                .map_err(|_| {
                                    ServerError::Internal("Invalid URI path".to_string())
                                })?
                                .into_owned(),
                        );
                        affected_files.insert(path);
                    }
                }
                lsp_types::DocumentChanges::Operations(ops) => {
                    for op in ops {
                        match op {
                            lsp_types::DocumentChangeOperation::Edit(edit) => {
                                let path = std::path::PathBuf::from(
                                    urlencoding::decode(edit.text_document.uri.path().as_str())
                                        .map_err(|_| {
                                            ServerError::Internal("Invalid URI path".to_string())
                                        })?
                                        .into_owned(),
                                );
                                affected_files.insert(path);
                            }
                            lsp_types::DocumentChangeOperation::Op(resource_op) => {
                                match resource_op {
                                    lsp_types::ResourceOp::Create(create) => {
                                        let path = std::path::PathBuf::from(
                                            urlencoding::decode(create.uri.path().as_str())
                                                .map_err(|_| {
                                                    ServerError::Internal(
                                                        "Invalid URI path".to_string(),
                                                    )
                                                })?
                                                .into_owned(),
                                        );
                                        affected_files.insert(path);
                                    }
                                    lsp_types::ResourceOp::Rename(rename) => {
                                        let path = std::path::PathBuf::from(
                                            urlencoding::decode(rename.old_uri.path().as_str())
                                                .map_err(|_| {
                                                    ServerError::Internal(
                                                        "Invalid URI path".to_string(),
                                                    )
                                                })?
                                                .into_owned(),
                                        );
                                        affected_files.insert(path);
                                        let path = std::path::PathBuf::from(
                                            urlencoding::decode(rename.new_uri.path().as_str())
                                                .map_err(|_| {
                                                    ServerError::Internal(
                                                        "Invalid URI path".to_string(),
                                                    )
                                                })?
                                                .into_owned(),
                                        );
                                        affected_files.insert(path);
                                    }
                                    lsp_types::ResourceOp::Delete(delete) => {
                                        let path = std::path::PathBuf::from(
                                            urlencoding::decode(delete.uri.path().as_str())
                                                .map_err(|_| {
                                                    ServerError::Internal(
                                                        "Invalid URI path".to_string(),
                                                    )
                                                })?
                                                .into_owned(),
                                        );
                                        affected_files.insert(path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Calculate checksums for all affected files
        for file_path in &affected_files {
            if file_path.exists() {
                if let Ok(content) = context.app_state.file_service.read_file(file_path).await {
                    file_checksums.insert(
                        file_path.to_string_lossy().to_string(),
                        utils::calculate_checksum(&content),
                    );
                }
            }
        }

        let summary = PlanSummary {
            affected_files: affected_files.len(),
            created_files: 0,
            deleted_files: 0,
        };

        let warnings = Vec::new();

        Ok((file_checksums, summary, warnings))
    }
}
