use super::super::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use cb_core::model::mcp::ToolCall;
use cb_protocol::analysis_result::{
    AnalysisResult, AnalysisScope, Finding, FindingLocation, Position, Range, SafetyLevel,
    Severity, Suggestion,
};
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, info};

#[derive(Deserialize, Debug)]
struct QualityOptions {
    #[serde(default)]
    thresholds: Option<QualityThresholds>,
    #[serde(default)]
    severity_filter: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    offset: usize,
    #[serde(default = "default_format")]
    format: String,
    #[serde(default = "default_include_suggestions")]
    include_suggestions: bool,
}

fn default_limit() -> usize {
    1000
}

fn default_format() -> String {
    "detailed".to_string()
}

fn default_include_suggestions() -> bool {
    true
}

#[derive(Deserialize, Debug)]
struct QualityThresholds {
    #[serde(default = "default_cyclomatic")]
    cyclomatic_complexity: u32,
    #[serde(default = "default_cognitive")]
    cognitive_complexity: u32,
    #[serde(default = "default_nesting")]
    nesting_depth: u32,
    #[serde(default = "default_params")]
    parameter_count: u32,
    #[serde(default = "default_function_length")]
    function_length: u32,
}

fn default_cyclomatic() -> u32 {
    15
}
fn default_cognitive() -> u32 {
    10
}
fn default_nesting() -> u32 {
    4
}
fn default_params() -> u32 {
    5
}
fn default_function_length() -> u32 {
    50
}

impl Default for QualityThresholds {
    fn default() -> Self {
        Self {
            cyclomatic_complexity: default_cyclomatic(),
            cognitive_complexity: default_cognitive(),
            nesting_depth: default_nesting(),
            parameter_count: default_params(),
            function_length: default_function_length(),
        }
    }
}

#[derive(Deserialize, Debug)]
struct QualityScopeParam {
    #[serde(rename = "type")]
    scope_type: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    include: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
}

pub struct QualityHandler;

impl QualityHandler {
    pub fn new() -> Self {
        Self
    }

    /// Transform ComplexityReport into AnalysisResult
    fn transform_complexity_report(
        &self,
        report: cb_ast::complexity::ComplexityReport,
        thresholds: &QualityThresholds,
        include_suggestions: bool,
        scope: AnalysisScope,
        analysis_time_ms: u64,
    ) -> AnalysisResult {
        let mut result = AnalysisResult::new("quality", "complexity", scope);

        // Set language if available
        result.metadata.language = Some("unknown".to_string());

        // Add thresholds to metadata
        let mut threshold_map = HashMap::new();
        threshold_map.insert(
            "cyclomatic_complexity".to_string(),
            json!(thresholds.cyclomatic_complexity),
        );
        threshold_map.insert(
            "cognitive_complexity".to_string(),
            json!(thresholds.cognitive_complexity),
        );
        threshold_map.insert(
            "nesting_depth".to_string(),
            json!(thresholds.nesting_depth),
        );
        threshold_map.insert(
            "parameter_count".to_string(),
            json!(thresholds.parameter_count),
        );
        threshold_map.insert(
            "function_length".to_string(),
            json!(thresholds.function_length),
        );
        result.metadata.thresholds = Some(threshold_map);

        // Transform each function into a finding
        for func in &report.functions {
            // Only include functions that exceed thresholds
            if func.complexity.cognitive < thresholds.cognitive_complexity
                && func.complexity.cyclomatic < thresholds.cyclomatic_complexity
            {
                continue;
            }

            // Determine severity based on rating
            let severity = match func.rating {
                cb_ast::complexity::ComplexityRating::VeryComplex => Severity::High,
                cb_ast::complexity::ComplexityRating::Complex => Severity::Medium,
                _ => Severity::Low,
            };

            // Build metrics
            let mut metrics = HashMap::new();
            metrics.insert(
                "cyclomatic_complexity".to_string(),
                json!(func.complexity.cyclomatic),
            );
            metrics.insert(
                "cognitive_complexity".to_string(),
                json!(func.complexity.cognitive),
            );
            metrics.insert("nesting_depth".to_string(), json!(func.complexity.max_nesting_depth));
            metrics.insert(
                "parameter_count".to_string(),
                json!(func.metrics.parameters),
            );
            metrics.insert("line_count".to_string(), json!(func.metrics.sloc));

            // Build message
            let message = format!(
                "Function '{}' has high complexity (cyclomatic: {}, cognitive: {}, rating: {})",
                func.name,
                func.complexity.cyclomatic,
                func.complexity.cognitive,
                func.rating.description()
            );

            // Build location
            let location = FindingLocation {
                file_path: report.file_path.clone(),
                range: Some(Range {
                    start: Position {
                        line: func.line as u32,
                        character: 0,
                    },
                    end: Position {
                        line: (func.line + func.metrics.sloc as usize) as u32,
                        character: 0,
                    },
                }),
                symbol: Some(func.name.clone()),
                symbol_kind: Some("function".to_string()),
            };

            // Build suggestions if requested
            let mut suggestions = Vec::new();
            if include_suggestions {
                // Suggest extract function for high complexity
                if func.complexity.cognitive > thresholds.cognitive_complexity {
                    suggestions.push(Suggestion {
                        action: "extract_function".to_string(),
                        description: "Extract nested blocks to separate functions to reduce cognitive complexity".to_string(),
                        target: None,
                        estimated_impact: format!(
                            "Could reduce complexity from {} to ~{}",
                            func.complexity.cognitive,
                            func.complexity.cognitive * 2 / 3
                        ),
                        safety: SafetyLevel::RequiresReview,
                        confidence: 0.75,
                        reversible: true,
                        refactor_call: Some(cb_protocol::analysis_result::RefactorCall {
                            command: "extract.plan".to_string(),
                            arguments: json!({
                                "kind": "function",
                                "source": {
                                    "file_path": report.file_path,
                                    "range": {
                                        "start": { "line": func.line, "character": 0 },
                                        "end": { "line": func.line + func.metrics.sloc as usize, "character": 0 }
                                    }
                                }
                            }),
                        }),
                    });
                }

                // Suggest reduce nesting
                if func.complexity.max_nesting_depth > thresholds.nesting_depth {
                    suggestions.push(Suggestion {
                        action: "reduce_nesting".to_string(),
                        description: "Use early returns or guard clauses to reduce nesting depth".to_string(),
                        target: None,
                        estimated_impact: "Improves readability significantly".to_string(),
                        safety: SafetyLevel::RequiresReview,
                        confidence: 0.80,
                        reversible: true,
                        refactor_call: None,
                    });
                }

                // Suggest consolidate parameters
                if func.metrics.parameters > thresholds.parameter_count {
                    suggestions.push(Suggestion {
                        action: "consolidate_parameters".to_string(),
                        description: "Group related parameters into a configuration struct/object".to_string(),
                        target: None,
                        estimated_impact: "Reduces parameter count, improves maintainability".to_string(),
                        safety: SafetyLevel::RequiresReview,
                        confidence: 0.70,
                        reversible: false,
                        refactor_call: None,
                    });
                }
            }

            // Create finding
            let finding = Finding {
                id: format!("complexity-{}-{}", report.file_path, func.line),
                kind: "complexity_hotspot".to_string(),
                severity,
                location,
                metrics: Some(metrics),
                message,
                suggestions,
            };

            result.add_finding(finding);
        }

        // Update summary
        result.summary.files_analyzed = 1;
        result.summary.symbols_analyzed = Some(report.total_functions);
        result.finalize(analysis_time_ms);

        result
    }
}

#[async_trait]
impl ToolHandler for QualityHandler {
    fn tool_names(&self) -> &[&str] {
        &["analyze.quality"]
    }

    fn is_internal(&self) -> bool {
        false // PUBLIC tool
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        let start_time = Instant::now();
        let args = tool_call.arguments.clone().unwrap_or(json!({}));

        // Parse kind (required)
        let kind = args
            .get("kind")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'kind' parameter".into()))?;

        // Only support "complexity" for MVP
        if kind != "complexity" {
            return Err(ServerError::InvalidRequest(format!(
                "Unsupported kind '{}'. Only 'complexity' is currently supported. Coming soon: 'smells', 'maintainability', 'readability'",
                kind
            )));
        }

        debug!(kind = %kind, "Handling analyze.quality request");

        // Parse scope
        let scope_param: QualityScopeParam = if let Some(scope_value) = args.get("scope") {
            serde_json::from_value(scope_value.clone())
                .map_err(|e| ServerError::InvalidRequest(format!("Invalid scope: {}", e)))?
        } else {
            QualityScopeParam {
                scope_type: None,
                path: None,
                include: vec![],
                exclude: vec![],
            }
        };

        // Determine file path (for MVP, only support file scope)
        let file_path = scope_param
            .path
            .or_else(|| args.get("file_path").and_then(|v| v.as_str()).map(String::from))
            .ok_or_else(|| {
                ServerError::InvalidRequest(
                    "Missing file path. For MVP, only file-level analysis is supported via scope.path or file_path parameter".into(),
                )
            })?;

        let scope_type = scope_param.scope_type.unwrap_or_else(|| "file".to_string());

        // Parse options
        let options: QualityOptions = args
            .get("options")
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()
            .map_err(|e| ServerError::InvalidRequest(format!("Invalid options: {}", e)))?
            .unwrap_or_else(|| QualityOptions {
                thresholds: None,
                severity_filter: None,
                limit: default_limit(),
                offset: 0,
                format: default_format(),
                include_suggestions: default_include_suggestions(),
            });

        let thresholds = options.thresholds.unwrap_or_default();

        info!(
            file_path = %file_path,
            kind = %kind,
            scope_type = %scope_type,
            "Analyzing code quality"
        );

        // Read file
        let file_path_obj = Path::new(&file_path);
        let extension = file_path_obj
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| {
                ServerError::InvalidRequest(format!("File has no extension: {}", file_path))
            })?;

        let content = context
            .app_state
            .file_service
            .read_file(file_path_obj)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to read file: {}", e)))?;

        // Get language plugin
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

        // Parse file
        let parsed = plugin
            .parse(&content)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to parse file: {}", e)))?;

        let language = plugin.metadata().name;

        // Analyze complexity
        let complexity_report = cb_ast::complexity::analyze_file_complexity(
            &file_path,
            &content,
            &parsed.symbols,
            &language,
        );

        // Build scope for result
        let scope = AnalysisScope {
            scope_type,
            path: file_path.clone(),
            include: scope_param.include,
            exclude: scope_param.exclude,
        };

        // Transform to AnalysisResult
        let mut result = self.transform_complexity_report(
            complexity_report,
            &thresholds,
            options.include_suggestions,
            scope,
            start_time.elapsed().as_millis() as u64,
        );

        // Set language in metadata
        result.metadata.language = Some(language.to_string());

        info!(
            file_path = %file_path,
            findings_count = result.summary.total_findings,
            analysis_time_ms = result.summary.analysis_time_ms,
            "Quality analysis complete"
        );

        // Serialize to JSON
        serde_json::to_value(result)
            .map_err(|e| ServerError::Internal(format!("Failed to serialize result: {}", e)))
    }
}
