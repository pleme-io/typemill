//! Workspace Java dependency tool handler
//!
//! Handles: workspace.add_java_dependency
//!
//! This tool adds a Maven dependency to a pom.xml file

use super::ToolHandler;
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use tracing::{debug, info};

/// Handler for workspace Java dependency operations
pub struct WorkspaceAddJavaDependencyHandler;

impl WorkspaceAddJavaDependencyHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ToolHandler for WorkspaceAddJavaDependencyHandler {
    fn tool_names(&self) -> &[&str] {
        &["workspace.add_java_dependency"]
    }

    async fn handle_tool_call(
        &self,
        context: &mill_handler_api::ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        match tool_call.name.as_str() {
            "workspace.add_java_dependency" => {
                handle_add_java_dependency(context, tool_call).await
            }
            _ => Err(ServerError::invalid_request(format!(
                "Unknown workspace Java tool: {}",
                tool_call.name
            ))),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AddJavaDependencyParams {
    manifest_path: PathBuf,
    group_id: String,
    artifact_id: String,
    version: String,
    #[serde(default)]
    dry_run: Option<bool>,
}

#[cfg(feature = "lang-java")]
async fn handle_add_java_dependency(
    _context: &mill_handler_api::ToolHandlerContext,
    tool_call: &ToolCall,
) -> ServerResult<Value> {
    debug!("Handling workspace.add_java_dependency");

    // Parse parameters
    let params: AddJavaDependencyParams = serde_json::from_value(
        tool_call
            .arguments
            .clone()
            .unwrap_or(serde_json::json!({})),
    )
    .map_err(|e| {
        ServerError::invalid_request(format!("Invalid parameters for add_java_dependency: {}", e))
    })?;

    let dry_run = params.dry_run.unwrap_or(true);

    info!(
        manifest_path = %params.manifest_path.display(),
        group_id = %params.group_id,
        artifact_id = %params.artifact_id,
        version = %params.version,
        dry_run = dry_run,
        "Adding Java dependency"
    );

    // Validate that the manifest path exists
    if !params.manifest_path.exists() {
        return Err(ServerError::NotFound {
            resource: params.manifest_path.display().to_string(),
            resource_type: Some("pom.xml file".to_string()),
        });
    }

    // Validate that it's a pom.xml file
    if !params
        .manifest_path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n == "pom.xml")
        .unwrap_or(false)
    {
        return Err(ServerError::InvalidRequest {
            message: format!(
                "File must be named 'pom.xml', got: {}",
                params.manifest_path.display()
            ),
            parameter: Some("manifest_path".to_string()),
        });
    }

    // Read current pom.xml content
    let content = tokio::fs::read_to_string(&params.manifest_path)
        .await
        .map_err(|e| ServerError::io(format!("Failed to read file '{}': {}", params.manifest_path.display(), e)))?;

    debug!("Read pom.xml ({} bytes)", content.len());

    // Add dependency using Java plugin function
    let updated_content = mill_lang_java::manifest_updater::add_dependency_to_pom(
        &content,
        &params.group_id,
        &params.artifact_id,
        &params.version,
    )
    .map_err(|e| ServerError::InvalidRequest {
        message: format!("Failed to add dependency: {}", e),
        parameter: None,
    })?;

    debug!(
        "Generated updated pom.xml ({} bytes)",
        updated_content.len()
    );

    // Apply changes if not dry-run
    if !dry_run {
        tokio::fs::write(&params.manifest_path, &updated_content)
            .await
            .map_err(|e| ServerError::io(format!("Failed to write file '{}': {}", params.manifest_path.display(), e)))?;

        info!(
            "Successfully added dependency {}:{}:{} to {}",
            params.group_id,
            params.artifact_id,
            params.version,
            params.manifest_path.display()
        );
    } else {
        debug!("Dry-run mode: changes not applied");
    }

    Ok(json!({
        "success": true,
        "manifest_path": params.manifest_path.display().to_string(),
        "dependency": {
            "groupId": params.group_id,
            "artifactId": params.artifact_id,
            "version": params.version
        },
        "dry_run": dry_run,
        "message": if dry_run {
            "Dependency addition previewed (dry-run mode)"
        } else {
            "Dependency successfully added"
        },
        "preview": if dry_run {
            Some(updated_content)
        } else {
            None
        }
    }))
}

#[cfg(not(feature = "lang-java"))]
async fn handle_add_java_dependency(
    _context: &mill_handler_api::ToolHandlerContext,
    _tool_call: &ToolCall,
) -> ServerResult<Value> {
    Err(ServerError::not_supported(
        "Java dependency management requires the 'lang-java' feature to be enabled. \
         Please rebuild with --features lang-java or enable it in default features.".to_string()
    ))
}
