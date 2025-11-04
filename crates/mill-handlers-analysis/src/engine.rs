//! Shared analysis engine for all analysis handlers
//!
//! This module provides a reusable workflow engine that eliminates boilerplate
//! from analysis handlers by orchestrating the common steps:
//! 1. Parse and validate arguments
//! 2. Read file and get language plugin
//! 3. Parse file with language plugin
//! 4. Run complexity analysis
//! 5. Execute custom analysis function
//! 6. Build and return AnalysisResult

use crate::ToolHandlerContext;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::protocol::analysis_result::{AnalysisResult, AnalysisScope, Finding};
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, info};

/// Helper to downcast AnalysisConfigTrait to concrete AnalysisConfig
fn get_analysis_config(context: &ToolHandlerContext) -> ServerResult<&super::config::AnalysisConfig> {
    context.analysis_config
        .as_any()
        .downcast_ref::<super::config::AnalysisConfig>()
        .ok_or_else(|| ServerError::internal("Failed to downcast AnalysisConfigTrait to AnalysisConfig"))
}

/// Analysis function signature - takes parsed data and returns findings
///
/// This function receives all the parsed and analyzed data needed to perform
/// custom analysis logic and generate findings.
///
/// # Parameters
/// - `complexity_report`: The complexity analysis report for the file
/// - `content`: The raw file content as a string
/// - `symbols`: The parsed symbols from the language plugin
/// - `language`: The language name (e.g., "rust", "typescript")
/// - `file_path`: The path to the file being analyzed
/// - `registry`: The language plugin registry for dynamic plugin lookup
///
/// # Returns
/// A vector of findings detected by the analysis function
use super::config::AnalysisConfig;

pub type AnalysisFn = fn(
    &mill_ast::complexity::ComplexityReport,
    &str,
    &[mill_plugin_api::Symbol],
    &str,
    &str,
    &dyn mill_handler_api::LanguagePluginRegistry,
    &AnalysisConfig,
) -> Vec<Finding>;

/// Markdown analysis function signature - simplified for non-code analysis
///
/// This function is tailored for analyses like Markdown that don't rely on
/// complexity metrics. It receives the raw content and parsed symbols directly.
pub type MarkdownAnalysisFn = fn(
    &str,
    &[mill_plugin_api::Symbol],
    &str,
    &str,
    &dyn mill_handler_api::LanguagePluginRegistry,
) -> Vec<Finding>;

/// Scope parameter structure for analysis requests
///
/// This matches the structure expected by MCP clients when specifying
/// the scope of an analysis operation.
/// Scope parameter for analysis (internal to mill-handlers)
#[derive(Deserialize, Debug)]
pub(crate) struct ScopeParam {
    /// The type of scope (e.g., "file", "workspace", "directory")
    #[serde(rename = "type")]
    pub scope_type: Option<String>,

    /// The path to analyze (file or directory path)
    #[serde(default)]
    pub path: Option<String>,

    /// Patterns to include in the analysis
    #[serde(default)]
    pub include: Vec<String>,

    /// Patterns to exclude from the analysis
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// Parse the scope parameter from tool call arguments
///
/// Extracts and deserializes the scope parameter, providing defaults
/// if the scope is not specified.
///
/// # Arguments
/// - `args`: The tool call arguments JSON value
///
/// # Returns
/// A `ServerResult` containing the parsed `ScopeParam` or an error
pub(crate) fn parse_scope_param(args: &Value) -> ServerResult<ScopeParam> {
    if let Some(scope_value) = args.get("scope") {
        serde_json::from_value(scope_value.clone())
            .map_err(|e| ServerError::invalid_request(format!("Invalid scope: {}", e)))
    } else {
        Ok(ScopeParam {
            scope_type: None,
            path: None,
            include: vec![],
            exclude: vec![],
        })
    }
}

/// Extract file path from arguments and scope parameter
///
/// Determines the file path to analyze by checking both the scope.path
/// and the file_path parameter. This provides backwards compatibility
/// while supporting the newer scope-based API.
///
/// # Arguments
/// - `args`: The tool call arguments JSON value
/// - `scope_param`: The parsed scope parameter
///
/// # Returns
/// A `ServerResult` containing the file path string or an error if no path found
pub(crate) fn extract_file_path(args: &Value, scope_param: &ScopeParam) -> ServerResult<String> {
    scope_param
        .path
        .clone()
        .or_else(|| args.get("filePath").and_then(|v| v.as_str()).map(String::from))
        .ok_or_else(|| {
            ServerError::invalid_request(
                "Missing file path. For MVP, only file-level analysis is supported via scope.path or file_path parameter",
            )
        })
}

/// Orchestrates the entire analysis workflow
///
/// This is the main entry point for all analysis operations. It handles the
/// complete workflow from parsing arguments to returning the final result.
///
/// # Workflow
/// 1. Parse and validate arguments (scope parameter)
/// 2. Extract file_path from scope.path or file_path parameter
/// 3. Read file via file_service
/// 4. Get language plugin by extension
/// 5. Parse file with plugin.parse()
/// 6. Run complexity analysis
/// 7. Build AnalysisScope struct
/// 8. Execute the custom analysis_fn
/// 9. Build AnalysisResult, add findings, finalize with timing
/// 10. Serialize to JSON and return
///
/// # Arguments
/// - `context`: The tool handler context with app state and services
/// - `tool_call`: The MCP tool call with arguments
/// - `category`: The analysis category (e.g., "quality", "security")
/// - `kind`: The analysis kind (e.g., "complexity", "smells")
/// - `analysis_fn`: The custom analysis function to execute
///
/// # Returns
/// A `ServerResult` containing the serialized AnalysisResult or an error
///
/// # Example
/// ```ignore
/// use mill_handlers::handlers::tools::analysis::engine::{run_analysis, AnalysisFn};
/// use mill_foundation::protocol::analysis_result::Finding;
/// use mill_handlers::language_plugin_registry::LanguagePluginRegistry;
///
/// fn my_analysis_fn(
///     complexity_report: &mill_ast::complexity::ComplexityReport,
///     content: &str,
///     symbols: &[mill_plugin_api::Symbol],
///     language: &str,
///     file_path: &str,
///     registry: &LanguagePluginRegistry,
/// ) -> Vec<Finding> {
///     // Custom analysis logic here
///     vec![]
/// }
/// # async fn example(context: mill_handlers::ToolHandlerContext, tool_call: mill_foundation::model::mcp::ToolCall) {
/// // In your handler:
/// run_analysis(&context, &tool_call, "quality", "smells", my_analysis_fn).await;
/// # }
/// ```
pub async fn run_analysis(
    context: &ToolHandlerContext,
    tool_call: &ToolCall,
    category: &str,
    kind: &str,
    analysis_fn: AnalysisFn,
) -> ServerResult<Value> {
    let config = get_analysis_config(context)?;
    run_analysis_with_config(
        context,
        tool_call,
        category,
        kind,
        analysis_fn,
        config,
    )
    .await
}

/// Orchestrates the entire analysis workflow with configuration
///
/// This is an enhanced version of `run_analysis` that accepts a
/// configuration to customize analysis behavior (thresholds, enabled kinds, etc.).
///
/// # Configuration Support
/// - Checks if the analysis kind is enabled in the configuration
/// - Passes threshold values to detection functions via context (future enhancement)
///
/// # Arguments
/// - `context`: The tool handler context with app state and services
/// - `tool_call`: The MCP tool call with arguments
/// - `category`: The analysis category (e.g., "quality", "security")
/// - `kind`: The analysis kind (e.g., "complexity", "smells")
/// - `analysis_fn`: The custom analysis function to execute
/// - `config`: Analysis configuration to customize behavior
///
/// # Returns
/// A `ServerResult` containing the serialized AnalysisResult or an error
///
/// # Errors
/// - Returns `ServerError::InvalidRequest` if the kind is disabled in configuration
///
/// # TODO
/// - Pass threshold values to analysis functions via extended context
/// - Support workspace-level configuration caching
/// - Add configuration validation at handler registration time
///
/// # Example
/// ```ignore
/// use mill_handlers::handlers::tools::analysis::engine::{run_analysis_with_config, AnalysisFn};
/// use mill_handlers::handlers::tools::analysis::config::AnalysisConfig;
/// use mill_foundation::protocol::analysis_result::Finding;
/// use mill_handlers::language_plugin_registry::LanguagePluginRegistry;
///
/// fn my_analysis_fn(
///     complexity_report: &mill_ast::complexity::ComplexityReport,
///     content: &str,
///     symbols: &[mill_plugin_api::Symbol],
///     language: &str,
///     file_path: &str,
///     registry: &LanguagePluginRegistry,
/// ) -> Vec<Finding> {
///     vec![]
/// }
/// # async fn example(context: mill_handlers::ToolHandlerContext, tool_call: mill_foundation::model::mcp::ToolCall) {
/// // With configuration:
/// let config = AnalysisConfig::default();
/// run_analysis_with_config(
///     &context,
///     &tool_call,
///     "quality",
///     "complexity",
///     my_analysis_fn,
///     &config
/// ).await;
/// # }
/// ```
pub async fn run_analysis_with_config(
    context: &ToolHandlerContext,
    tool_call: &ToolCall,
    category: &str,
    kind: &str,
    analysis_fn: AnalysisFn,
    config: &super::config::AnalysisConfig,
) -> ServerResult<Value> {
    let start_time = Instant::now();
    let args = tool_call.arguments.clone().unwrap_or(serde_json::json!({}));

    // Check if kind is enabled in configuration
    if !config.is_kind_enabled(category, kind) {
        return Err(ServerError::invalid_request(format!(
            "Analysis kind '{}' is disabled in configuration for category '{}'",
            kind, category
        )));
    }
    debug!(
        category = %category,
        kind = %kind,
        preset = ?config.preset,
        "Starting analysis workflow with config"
    );

    // Step 1: Parse scope parameter
    let scope_param = parse_scope_param(&args)?;

    // Step 2: Extract file path
    let file_path = extract_file_path(&args, &scope_param)?;
    let scope_type = scope_param
        .scope_type
        .clone()
        .unwrap_or_else(|| "file".to_string());

    info!(
        file_path = %file_path,
        category = %category,
        kind = %kind,
        scope_type = %scope_type,
        "Running analysis"
    );

    // Step 3: Read file
    let file_path_obj = Path::new(&file_path);
    let extension = file_path_obj
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| {
            ServerError::invalid_request(format!("File has no extension: {}", file_path))
        })?;

    let content = context
        .app_state
        .file_service
        .read_file(file_path_obj)
        .await
        .map_err(|e| ServerError::internal(format!("Failed to read file: {}", e)))?;

    // Step 4: Get language plugin
    let plugin = context
        .app_state
        .language_plugins
        .get_plugin(extension)
        .ok_or_else(|| {
            ServerError::not_supported(format!(
                "No language plugin found for extension: {}",
                extension
            ))
        })?;

    // Step 5: Parse file
    let parsed = plugin
        .parse(&content)
        .await
        .map_err(|e| ServerError::internal(format!("Failed to parse file: {}", e)))?;

    let language = plugin.metadata().name;

    debug!(
        file_path = %file_path,
        language = %language,
        symbols_count = parsed.symbols.len(),
        "File parsed successfully"
    );

    // Step 6: Analyze complexity
    let complexity_report = mill_ast::complexity::analyze_file_complexity(
        &file_path,
        &content,
        &parsed.symbols,
        language,
    );

    debug!(
        file_path = %file_path,
        total_functions = complexity_report.total_functions,
        avg_complexity = complexity_report.average_complexity,
        "Complexity analysis complete"
    );

    // Step 7: Build scope for result
    let scope = AnalysisScope {
        scope_type,
        path: file_path.clone(),
        include: scope_param.include,
        exclude: scope_param.exclude,
    };

    // Step 8: Execute the custom analysis function
    let findings = analysis_fn(
        &complexity_report,
        &content,
        &parsed.symbols,
        language,
        &file_path,
        context.app_state.language_plugins.as_ref(),
        config,
    );

    debug!(
        file_path = %file_path,
        findings_count = findings.len(),
        "Analysis function complete"
    );

    // Step 9: Build AnalysisResult
    let mut result = AnalysisResult::new(category, kind, scope);

    // Set language in metadata
    result.metadata.language = Some(language.to_string());

    // Add all findings
    for finding in findings {
        result.add_finding(finding);
    }

    // Update summary
    result.summary.files_analyzed = 1;
    result.summary.symbols_analyzed = Some(complexity_report.total_functions);

    // Finalize with timing
    result.finalize(start_time.elapsed().as_millis() as u64);

    info!(
        file_path = %file_path,
        category = %category,
        kind = %kind,
        findings_count = result.summary.total_findings,
        analysis_time_ms = result.summary.analysis_time_ms,
        "Analysis complete"
    );

    // Step 10: Serialize to JSON and return
    serde_json::to_value(result)
        .map_err(|e| ServerError::internal(format!("Failed to serialize result: {}", e)))
}

/// Orchestrates a simplified analysis workflow for Markdown files
///
/// This function is a variant of `run_analysis` specifically for Markdown.
/// It skips the code-specific complexity analysis step, making it more
/// efficient for non-code content.
///
/// # Workflow
/// 1. Parse and validate arguments
/// 2. Read file and get language plugin
/// 3. Parse file with language plugin
/// 4. Execute the custom `MarkdownAnalysisFn`
/// 5. Build and return AnalysisResult
pub async fn run_markdown_analysis(
    context: &ToolHandlerContext,
    tool_call: &ToolCall,
    category: &str,
    kind: &str,
    analysis_fn: MarkdownAnalysisFn,
) -> ServerResult<Value> {
    let start_time = Instant::now();
    let args = tool_call.arguments.clone().unwrap_or(serde_json::json!({}));

    debug!(
        category = %category,
        kind = %kind,
        "Starting markdown analysis workflow"
    );

    // Step 1: Parse scope and file path
    let scope_param = parse_scope_param(&args)?;
    let file_path = extract_file_path(&args, &scope_param)?;
    let scope_type = scope_param
        .scope_type
        .clone()
        .unwrap_or_else(|| "file".to_string());

    info!(
        file_path = %file_path,
        category = %category,
        kind = %kind,
        "Running markdown analysis"
    );

    // Step 2: Read file and get plugin
    let file_path_obj = Path::new(&file_path);
    let extension = file_path_obj
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| {
            ServerError::invalid_request(format!("File has no extension: {}", file_path))
        })?;

    let content = context
        .app_state
        .file_service
        .read_file(file_path_obj)
        .await
        .map_err(|e| ServerError::internal(format!("Failed to read file: {}", e)))?;

    let plugin = context
        .app_state
        .language_plugins
        .get_plugin(extension)
        .ok_or_else(|| {
            ServerError::not_supported(format!(
                "No language plugin found for extension: {}",
                extension
            ))
        })?;

    // Step 3: Parse file
    let parsed = plugin
        .parse(&content)
        .await
        .map_err(|e| ServerError::internal(format!("Failed to parse file: {}", e)))?;
    let language = plugin.metadata().name;

    // Step 4: Execute the custom analysis function (no complexity report)
    let findings = analysis_fn(
        &content,
        &parsed.symbols,
        language,
        &file_path,
        context.app_state.language_plugins.as_ref(),
    );

    // Step 5: Build AnalysisResult
    let scope = AnalysisScope {
        scope_type,
        path: file_path.clone(),
        include: scope_param.include,
        exclude: scope_param.exclude,
    };
    let mut result = AnalysisResult::new(category, kind, scope);
    result.metadata.language = Some(language.to_string());
    for finding in findings {
        result.add_finding(finding);
    }
    result.summary.files_analyzed = 1;
    result.summary.symbols_analyzed = Some(parsed.symbols.len());
    result.finalize(start_time.elapsed().as_millis() as u64);

    info!(
        file_path = %file_path,
        findings_count = result.summary.total_findings,
        analysis_time_ms = result.summary.analysis_time_ms,
        "Markdown analysis complete"
    );

    serde_json::to_value(result)
        .map_err(|e| ServerError::internal(format!("Failed to serialize result: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_scope_with_all_fields() {
        let args = json!({
            "scope": {
                "type": "file",
                "path": "/path/to/file.rs",
                "include": ["*.rs"],
                "exclude": ["target/*"]
            }
        });

        let scope = parse_scope_param(&args).unwrap();
        assert_eq!(scope.scope_type, Some("file".to_string()));
        assert_eq!(scope.path, Some("/path/to/file.rs".to_string()));
        assert_eq!(scope.include, vec!["*.rs"]);
        assert_eq!(scope.exclude, vec!["target/*"]);
    }

    #[test]
    fn test_parse_scope_with_defaults() {
        let args = json!({
            "scope": {
                "path": "/path/to/file.rs"
            }
        });

        let scope = parse_scope_param(&args).unwrap();
        assert_eq!(scope.scope_type, None);
        assert_eq!(scope.path, Some("/path/to/file.rs".to_string()));
        assert!(scope.include.is_empty());
        assert!(scope.exclude.is_empty());
    }

    #[test]
    fn test_parse_scope_missing() {
        let args = json!({});
        let scope = parse_scope_param(&args).unwrap();
        assert_eq!(scope.scope_type, None);
        assert_eq!(scope.path, None);
        assert!(scope.include.is_empty());
        assert!(scope.exclude.is_empty());
    }

    #[test]
    fn test_extract_file_path_from_scope() {
        let args = json!({});
        let scope = ScopeParam {
            scope_type: Some("file".to_string()),
            path: Some("/path/from/scope.rs".to_string()),
            include: vec![],
            exclude: vec![],
        };

        let file_path = extract_file_path(&args, &scope).unwrap();
        assert_eq!(file_path, "/path/from/scope.rs");
    }

    #[test]
    fn test_extract_file_path_from_file_path_param() {
        let args = json!({
            "filePath": "/path/from/param.rs"
        });
        let scope = ScopeParam {
            scope_type: None,
            path: None,
            include: vec![],
            exclude: vec![],
        };

        let file_path = extract_file_path(&args, &scope).unwrap();
        assert_eq!(file_path, "/path/from/param.rs");
    }

    #[test]
    fn test_extract_file_path_scope_takes_precedence() {
        let args = json!({
            "filePath": "/path/from/param.rs"
        });
        let scope = ScopeParam {
            scope_type: None,
            path: Some("/path/from/scope.rs".to_string()),
            include: vec![],
            exclude: vec![],
        };

        let file_path = extract_file_path(&args, &scope).unwrap();
        assert_eq!(file_path, "/path/from/scope.rs");
    }

    #[test]
    fn test_extract_file_path_missing() {
        let args = json!({});
        let scope = ScopeParam {
            scope_type: None,
            path: None,
            include: vec![],
            exclude: vec![],
        };

        let result = extract_file_path(&args, &scope);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing file path"));
    }
}
