use crate::handlers::tools::ToolHandlerContext;
use cb_core::model::mcp::ToolCall;
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};
use serde_json::{json, Value};
use std::path::Path;
use tracing::{debug, info};

pub async fn handle_analyze_complexity(
    context: &ToolHandlerContext,
    tool_call: &ToolCall,
) -> ServerResult<Value> {
    let args = tool_call.arguments.clone().unwrap_or(json!({}));

    let file_path_str = args
        .get("file_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ServerError::InvalidRequest("Missing file_path parameter".into()))?;

    debug!(
        file_path = %file_path_str,
        "Analyzing cyclomatic complexity"
    );

    let file_path = Path::new(file_path_str);

    // Get file extension
    let extension = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| {
            ServerError::InvalidRequest(format!("File has no extension: {}", file_path_str))
        })?;

    // Read file content
    let content = context
        .app_state
        .file_service
        .read_file(file_path)
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to read file: {}", e)))?;

    // Find language plugin
    let plugin = context
        .app_state
        .language_plugins
        .get_plugin(extension)
        .ok_or_else(|| {
            ServerError::Unsupported(format!(
                "No language plugin found for extension: {}",
                extension
            ))
        })?;

    // Parse file to get symbols
    let parsed = plugin
        .parse(&content)
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to parse file: {}", e)))?;

    // Get language name for complexity patterns
    let language = plugin.metadata().name;

    info!(
        file_path = %file_path_str,
        language = %language,
        symbols_count = parsed.symbols.len(),
        "Analyzing complexity for file"
    );

    // Analyze complexity using cb-ast module
    let report = cb_ast::complexity::analyze_file_complexity(
        file_path_str,
        &content,
        &parsed.symbols,
        language,
    );

    info!(
        file_path = %file_path_str,
        total_functions = report.total_functions,
        average_complexity = report.average_complexity,
        max_complexity = report.max_complexity,
        "Complexity analysis complete"
    );

    serde_json::to_value(report)
        .map_err(|e| ServerError::Internal(format!("Failed to serialize report: {}", e)))
}