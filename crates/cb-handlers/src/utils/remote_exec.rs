//! Remote command execution utilities

use codebuddy_workspaces::WorkspaceManager;
use codebuddy_foundation::protocol::{ ApiError as ServerError , ApiResult as ServerResult };
use reqwest;
use serde_json::json;
use std::time::Duration;
use tracing::error;

/// Execute a command on a remote workspace via the workspace agent
pub async fn execute_in_workspace(
    workspace_manager: &WorkspaceManager,
    user_id: &str,
    workspace_id: &str,
    command: &str,
) -> ServerResult<String> {
    // Look up workspace for the specified user
    let workspace = workspace_manager
        .get(user_id, workspace_id)
        .ok_or_else(|| {
            ServerError::InvalidRequest(format!(
                "Workspace '{}' not found for user '{}'",
                workspace_id, user_id
            ))
        })?;

    // Build agent URL
    let agent_url = format!("{}/execute", workspace.agent_url);

    // Create HTTP client with timeout
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| {
            error!(error = %e, "Failed to create HTTP client");
            ServerError::Internal("HTTP client error".into())
        })?;

    // Execute command via agent
    let response = client
        .post(&agent_url)
        .json(&json!({ "command": command }))
        .send()
        .await
        .map_err(|e| {
            error!(
                workspace_id = %workspace_id,
                agent_url = %agent_url,
                error = %e,
                "Failed to send command to workspace agent"
            );
            ServerError::Internal(format!("Failed to reach workspace agent: {}", e))
        })?;

    // Check response status
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        error!(
            workspace_id = %workspace_id,
            status = %status,
            error = %error_text,
            "Workspace agent returned error"
        );
        return Err(ServerError::Internal(format!(
            "Workspace agent error ({}): {}",
            status, error_text
        )));
    }

    // Parse response
    let result: serde_json::Value = response.json().await.map_err(|e| {
        error!(error = %e, "Failed to parse agent response");
        ServerError::Internal("Failed to parse agent response".into())
    })?;

    // Extract stdout from response
    result
        .get("stdout")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            error!("Agent response missing stdout field");
            ServerError::Internal("Invalid agent response format".into())
        })
}