#![allow(dead_code, unused_variables)]

//! Dead code analysis handler
//!
//! This module provides detection for unused code patterns including:
//! - Unused imports: Imports that are declared but never referenced
//! - Unused symbols: Functions, classes, and variables that are defined but never used
//!
//! Uses the shared analysis engine for orchestration and focuses only on
//! detection logic.

pub(crate) mod imports;
pub(crate) mod symbols;
#[cfg(test)]
mod tests;
pub(crate) mod types;
pub(crate) mod unreachable;
pub(crate) mod utils;
pub(crate) mod variables;

use crate::suggestions::{AnalysisContext, SuggestionGenerator};
use crate::{AnalysisConfig, ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
pub(crate) use imports::detect_unused_imports;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_foundation::protocol::analysis_result::{
    Finding, FindingLocation, Position, Range, SafetyLevel, Severity, Suggestion,
};
use serde_json::{json, Value};
pub(crate) use symbols::detect_unused_symbols;
use tracing::debug;
pub(crate) use types::detect_unused_types;
pub(crate) use unreachable::detect_unreachable_code;
use utils::{generate_dead_code_refactoring_candidates, to_protocol_safety_level};
pub(crate) use variables::{detect_unused_parameters, detect_unused_variables};

// Conditional imports for feature-gated analysis
#[cfg(any(feature = "analysis-dead-code", feature = "analysis-deep-dead-code"))]
use mill_foundation::protocol::{AnalysisMetadata, AnalysisSummary};
#[cfg(any(feature = "analysis-dead-code", feature = "analysis-deep-dead-code"))]
use uuid::Uuid;

/// Helper to downcast AnalysisConfigTrait to concrete AnalysisConfig
fn get_analysis_config(context: &ToolHandlerContext) -> ServerResult<&AnalysisConfig> {
    context
        .analysis_config
        .as_any()
        .downcast_ref::<AnalysisConfig>()
        .ok_or_else(|| {
            ServerError::internal("Failed to downcast AnalysisConfigTrait to AnalysisConfig")
        })
}

pub struct DeadCodeHandler;

impl Default for DeadCodeHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl DeadCodeHandler {
    pub fn new() -> Self {
        Self
    }

    /// Handle workspace-scoped dead code analysis using LSP
    ///
    /// This function uses the LSP-based dead code analyzer for accurate
    /// cross-file analysis when workspace scope is requested.
    ///
    /// # Feature-gated
    /// This function is only available when the `analysis-dead-code` feature is enabled,
    /// as it requires LSP integration for accurate workspace-wide analysis.
    #[cfg(feature = "analysis-dead-code")]
    async fn handle_workspace_dead_code(
        &self,
        context: &ToolHandlerContext,
        args: &Value,
        scope_param: &super::engine::ScopeParam,
        kind: &str,
    ) -> ServerResult<Value> {
        use crate::lsp_provider_adapter::LspProviderAdapter;
        use mill_analysis_common::AnalysisEngine;
        use mill_analysis_dead_code::{DeadCodeAnalyzer, DeadCodeConfig};
        use mill_foundation::protocol::analysis_result::AnalysisResult;
        use std::path::Path;
        use std::sync::Arc;
        use std::time::Instant;
        use tracing::info;

        let start_time = Instant::now();
        let path_str = scope_param.path.as_deref().ok_or_else(|| {
            ServerError::invalid_request("Missing 'path' in scope parameter".to_string())
        })?;
        let workspace_path = Path::new(path_str);

        // Determine file extension for LSP client (default to Rust)
        let file_extension = args
            .get("file_extension")
            .and_then(|v| v.as_str())
            .unwrap_or("rs")
            .to_string();

        info!(
            workspace_path = %workspace_path.display(),
            file_extension = %file_extension,
            kind = %kind,
            "Starting workspace-scoped dead code analysis"
        );

        // Create LSP provider adapter
        let lsp_adapter =
            LspProviderAdapter::new(context.lsp_adapter.clone(), file_extension.clone());

        // Configure dead code analysis
        let mut config = DeadCodeConfig::default();

        // Apply configuration from args
        if let Some(file_types) = args.get("file_types").and_then(|v| v.as_array()) {
            config.file_types = Some(
                file_types
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect(),
            );
        }

        if let Some(min_refs) = args.get("min_reference_threshold").and_then(|v| v.as_u64()) {
            config.min_reference_threshold = min_refs as usize;
        }

        // Run analysis
        let analyzer = DeadCodeAnalyzer;
        let report = analyzer
            .analyze(Arc::new(lsp_adapter), workspace_path, config)
            .await
            .map_err(|e| ServerError::analysis(format!("Dead code analysis failed: {}", e)))?;

        info!(
            dead_symbols_found = report.dead_symbols.len(),
            files_analyzed = report.stats.files_analyzed,
            duration_ms = report.stats.duration_ms,
            "Workspace dead code analysis completed"
        );

        // Convert to AnalysisResult format
        use mill_foundation::protocol::analysis_result::{AnalysisScope, SeverityBreakdown};
        use uuid::Uuid;

        let findings: Vec<Finding> = report
            .dead_symbols
            .iter()
            .map(|symbol| Finding {
                id: Uuid::new_v4().to_string(),
                kind: match symbol.kind.as_str() {
                    "Function" => "unused_function",
                    "Class" => "unused_class",
                    "Variable" => "unused_variable",
                    "Constant" => "unused_constant",
                    _ => "unused_symbol",
                }
                .to_string(),
                severity: Severity::Medium,
                location: FindingLocation {
                    file_path: symbol.file_path.clone(),
                    range: Some(Range {
                        start: Position {
                            line: symbol.line,
                            character: symbol.column,
                        },
                        end: Position {
                            line: symbol.line,
                            character: symbol.column + symbol.name.len() as u32,
                        },
                    }),
                    symbol: Some(symbol.name.clone()),
                    symbol_kind: Some(symbol.kind.clone()),
                },
                message: format!("{} '{}' is never used", symbol.kind, symbol.name),
                suggestions: vec![Suggestion {
                    action: "remove_symbol".to_string(),
                    description: format!(
                        "Remove unused {} '{}'",
                        symbol.kind.to_lowercase(),
                        symbol.name
                    ),
                    target: None,
                    estimated_impact: "low".to_string(),
                    safety: SafetyLevel::Safe,
                    confidence: 0.9,
                    reversible: true,
                    refactor_call: None,
                }],
                metrics: {
                    let mut map = std::collections::HashMap::new();
                    map.insert("symbol_kind".to_string(), serde_json::json!(symbol.kind));
                    map.insert(
                        "reference_count".to_string(),
                        serde_json::json!(symbol.reference_count),
                    );
                    Some(map)
                },
            })
            .collect();

        // Count findings by severity
        let medium_count = findings.len(); // All are Medium severity
        let by_severity = SeverityBreakdown {
            high: 0,
            medium: medium_count,
            low: 0,
        };

        let result = AnalysisResult {
            metadata: mill_foundation::protocol::analysis_result::AnalysisMetadata {
                category: "dead_code".to_string(),
                kind: kind.to_string(),
                scope: AnalysisScope {
                    scope_type: "workspace".to_string(),
                    path: workspace_path.to_string_lossy().to_string(),
                    include: vec![],
                    exclude: vec![],
                },
                language: Some(file_extension.clone()),
                timestamp: chrono::Utc::now().to_rfc3339(),
                thresholds: None,
            },
            summary: mill_foundation::protocol::analysis_result::AnalysisSummary {
                total_findings: findings.len(),
                returned_findings: findings.len(),
                has_more: false,
                by_severity,
                files_analyzed: report.stats.files_analyzed,
                symbols_analyzed: Some(report.stats.symbols_analyzed),
                analysis_time_ms: report.stats.duration_ms as u64,
                fix_actions: None,
            },
            findings,
        };

        Ok(serde_json::to_value(result).unwrap())
    }

    /// Fallback handler for when LSP feature is not enabled
    #[cfg(not(feature = "analysis-dead-code"))]
    async fn handle_workspace_dead_code(
        &self,
        _context: &ToolHandlerContext,
        _args: &Value,
        _scope_param: &super::engine::ScopeParam,
        _kind: &str,
    ) -> ServerResult<Value> {
        Err(ServerError::not_supported(
            "Workspace scope for dead code analysis requires the 'analysis-dead-code' feature to be enabled. \
             File-level analysis is available without this feature.".to_string(),
        ))
    }

    async fn run_analysis_and_suggest<F>(
        &self,
        context: &ToolHandlerContext,
        args: &Value,
        scope_param: &super::engine::ScopeParam,
        kind: &str,
        analysis_fn: F,
    ) -> ServerResult<Value>
    where
        F: Fn(
                &mill_ast::complexity::ComplexityReport,
                &str,
                &[mill_plugin_api::Symbol],
                &str,
                &str,
                &dyn mill_handler_api::LanguagePluginRegistry,
                &AnalysisConfig,
            ) -> Vec<Finding>
            + Send
            + Sync,
    {
        use mill_foundation::protocol::analysis_result::AnalysisResult;
        use std::path::Path;
        use std::time::Instant;
        use tracing::info;

        let start_time = Instant::now();

        // Replicate logic from engine::run_analysis to get access to parsed_source
        let file_path = super::engine::extract_file_path(args, scope_param)?;
        info!(file_path = %file_path, kind = %kind, "Running dead code analysis with suggestions");

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
        let parsed_source = plugin
            .parse(&content)
            .await
            .map_err(|e| ServerError::internal(format!("Failed to parse file: {}", e)))?;
        let language = plugin.metadata().name;
        let complexity_report = mill_ast::complexity::analyze_file_complexity(
            &file_path,
            &content,
            &parsed_source.symbols,
            language,
        );

        let mut findings = analysis_fn(
            &complexity_report,
            &content,
            &parsed_source.symbols,
            language,
            &file_path,
            context.app_state.language_plugins.as_ref(),
            get_analysis_config(context)?,
        );

        // Initialize suggestion generator
        let suggestion_generator = SuggestionGenerator::new();

        // Enhance findings with actionable suggestions
        for finding in &mut findings {
            let candidates = generate_dead_code_refactoring_candidates(finding, &parsed_source);

            let context = AnalysisContext {
                file_path: file_path.clone(),
                has_full_type_info: false, // File-scope analysis doesn't have LSP
                has_partial_type_info: false, // ParsedSource doesn't have this
                ast_parse_errors: 0,       // ParsedSource doesn't have this
            };

            let mut suggestions = Vec::new();
            for candidate in candidates {
                match suggestion_generator.generate_from_candidate(candidate, &context) {
                    Ok(actionable) => {
                        // Convert ActionableSuggestion to protocol::Suggestion
                        let suggestion = Suggestion {
                            action: actionable
                                .refactor_call
                                .as_ref()
                                .map(|rc| rc.tool.clone())
                                .unwrap_or_else(|| "manual_fix".to_string()),
                            description: actionable.message,
                            target: None,
                            estimated_impact: format!("{:?}", actionable.estimated_impact),
                            safety: to_protocol_safety_level(actionable.safety),
                            confidence: actionable.confidence,
                            reversible: actionable.reversible,
                            refactor_call: actionable.refactor_call.map(|rc| {
                                mill_foundation::protocol::analysis_result::RefactorCall {
                                    command: rc.tool,
                                    arguments: rc.arguments,
                                }
                            }),
                        };
                        suggestions.push(suggestion);
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            finding_kind = %finding.kind,
                            "Failed to generate suggestion"
                        );
                    }
                }
            }

            if !suggestions.is_empty() {
                finding.suggestions = suggestions;
            }
        }

        let scope = mill_foundation::protocol::analysis_result::AnalysisScope {
            scope_type: scope_param
                .scope_type
                .clone()
                .unwrap_or_else(|| "file".to_string()),
            path: file_path.clone(),
            include: scope_param.include.clone(),
            exclude: scope_param.exclude.clone(),
        };
        let mut result = AnalysisResult::new("dead_code", kind, scope);
        result.metadata.language = Some(language.to_string());
        for finding in findings {
            result.add_finding(finding);
        }
        result.summary.files_analyzed = 1;
        result.summary.symbols_analyzed = Some(complexity_report.total_functions);
        result.finalize(start_time.elapsed().as_millis() as u64);

        serde_json::to_value(result)
            .map_err(|e| ServerError::internal(format!("Failed to serialize result: {}", e)))
    }

    #[cfg(feature = "analysis-deep-dead-code")]
    async fn handle_workspace_deep_dead_code(
        &self,
        context: &ToolHandlerContext,
        args: &Value,
        scope_param: &super::engine::ScopeParam,
        kind: &str,
    ) -> ServerResult<Value> {
        use crate::lsp_provider_adapter::LspProviderAdapter;
        use mill_analysis_common::AnalysisEngine;
        use mill_analysis_deep_dead_code::{DeepDeadCodeAnalyzer, DeepDeadCodeConfig};
        use mill_foundation::protocol::analysis_result::{
            AnalysisResult, AnalysisScope, SeverityBreakdown,
        };
        use std::path::Path;
        use std::sync::Arc;
        use std::time::Instant;

        // Extract path from Option<String>
        let path_str = scope_param.path.as_deref().ok_or_else(|| {
            ServerError::invalid_request("Missing 'path' in scope parameter".to_string())
        })?;
        let workspace_path = Path::new(path_str);

        // Get file extension for LSP client (default "rs")
        let file_extension = args
            .get("file_extension")
            .and_then(|v| v.as_str())
            .unwrap_or("rs")
            .to_string();

        // Create LSP provider adapter
        let lsp_adapter =
            LspProviderAdapter::new(context.lsp_adapter.clone(), file_extension.clone());

        // Configure analysis
        let config = DeepDeadCodeConfig {
            check_public_exports: args
                .get("check_public_exports")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            exclude_patterns: args
                .get("exclude_patterns")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                }),
        };

        // Run analysis
        let start = Instant::now();
        let analyzer = DeepDeadCodeAnalyzer;
        let report = analyzer
            .analyze(Arc::new(lsp_adapter), workspace_path, config)
            .await
            .map_err(|e| ServerError::analysis(format!("Deep dead code analysis failed: {}", e)))?;
        let duration_ms = start.elapsed().as_millis() as u64;

        // Convert DeepDeadCodeReport to AnalysisResult
        let findings: Vec<Finding> = report
            .dead_symbols
            .iter()
            .map(|symbol| {
                // Convert SymbolKind to string
                let symbol_kind = format!("{:?}", symbol.kind);
                let severity = if symbol.is_public {
                    Severity::Low // Public unused exports are lower priority
                } else {
                    Severity::Medium
                };

                Finding {
                    id: Uuid::new_v4().to_string(),
                    kind: "unused_symbol".to_string(),
                    severity,
                    location: FindingLocation {
                        file_path: symbol.file_path.clone(),
                        range: Some(Range {
                            start: Position {
                                line: symbol.range.start.line,
                                character: symbol.range.start.character,
                            },
                            end: Position {
                                line: symbol.range.end.line,
                                character: symbol.range.end.character,
                            },
                        }),
                        symbol: Some(symbol.name.clone()),
                        symbol_kind: Some(symbol_kind.clone()),
                    },
                    message: format!(
                        "{} '{}' is never used",
                        if symbol.is_public {
                            "Public"
                        } else {
                            "Private"
                        },
                        symbol.name
                    ),
                    suggestions: vec![Suggestion {
                        action: "remove_symbol".to_string(),
                        description: format!(
                            "Remove unused {} '{}'",
                            symbol_kind.to_lowercase(),
                            symbol.name
                        ),
                        target: None,
                        estimated_impact: "low".to_string(),
                        safety: SafetyLevel::Safe,
                        confidence: if symbol.is_public { 0.7 } else { 0.9 },
                        reversible: true,
                        refactor_call: None,
                    }],
                    metrics: {
                        let mut map = std::collections::HashMap::new();
                        map.insert("symbol_kind".to_string(), serde_json::json!(symbol_kind));
                        map.insert("is_public".to_string(), serde_json::json!(symbol.is_public));
                        map.insert("symbol_id".to_string(), serde_json::json!(symbol.id));
                        Some(map)
                    },
                }
            })
            .collect();

        let high_count = findings
            .iter()
            .filter(|f| matches!(f.severity, Severity::High))
            .count();
        let medium_count = findings
            .iter()
            .filter(|f| matches!(f.severity, Severity::Medium))
            .count();
        let low_count = findings
            .iter()
            .filter(|f| matches!(f.severity, Severity::Low))
            .count();

        let result = AnalysisResult {
            metadata: AnalysisMetadata {
                category: "dead_code".to_string(),
                kind: kind.to_string(),
                scope: AnalysisScope {
                    scope_type: "workspace".to_string(),
                    path: workspace_path.to_string_lossy().to_string(),
                    include: vec![],
                    exclude: vec![],
                },
                language: Some(file_extension.clone()),
                timestamp: chrono::Utc::now().to_rfc3339(),
                thresholds: None,
            },
            summary: AnalysisSummary {
                total_findings: findings.len(),
                returned_findings: findings.len(),
                has_more: false,
                by_severity: SeverityBreakdown {
                    high: high_count,
                    medium: medium_count,
                    low: low_count,
                },
                files_analyzed: 0, // DeepDeadCodeReport doesn't track this
                symbols_analyzed: Some(report.dead_symbols.len()),
                analysis_time_ms: duration_ms,
                fix_actions: None,
            },
            findings,
        };

        Ok(serde_json::to_value(result).unwrap())
    }
}

#[async_trait]
impl ToolHandler for DeadCodeHandler {
    fn tool_names(&self) -> &[&str] {
        &["analyze.dead_code"]
    }

    fn is_internal(&self) -> bool {
        false // PUBLIC tool
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        let args = tool_call.arguments.clone().unwrap_or(json!({}));

        // Parse kind (required)
        let kind = args
            .get("kind")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::invalid_request("Missing 'kind' parameter"))?;

        // Validate kind
        let is_valid = match kind {
            "unused_imports" | "unused_symbols" | "unreachable_code" | "unused_parameters"
            | "unused_types" | "unused_variables" => true,
            #[cfg(feature = "analysis-deep-dead-code")]
            "deep" => true,
            _ => false,
        };

        if !is_valid {
            #[cfg(feature = "analysis-deep-dead-code")]
            let supported = "'unused_imports', 'unused_symbols', 'unreachable_code', 'unused_parameters', 'unused_types', 'unused_variables', 'deep'".to_string();
            #[cfg(not(feature = "analysis-deep-dead-code"))]
            let supported = "'unused_imports', 'unused_symbols', 'unreachable_code', 'unused_parameters', 'unused_types', 'unused_variables'".to_string();
            return Err(ServerError::invalid_request(format!(
                "Unsupported kind '{}'. Supported: {}",
                kind, supported
            )));
        }

        debug!(kind = %kind, "Handling analyze.dead_code request");

        // Check if workspace scope is requested
        let scope_param = super::engine::parse_scope_param(&args)?;
        let scope_type = scope_param.scope_type.as_deref().unwrap_or("file");

        if scope_type == "workspace" {
            // Use LSP-based workspace analysis
            #[cfg(feature = "analysis-deep-dead-code")]
            if kind == "deep" {
                return self
                    .handle_workspace_deep_dead_code(context, &args, &scope_param, kind)
                    .await;
            }

            self.handle_workspace_dead_code(context, &args, &scope_param, kind)
                .await
        } else {
            // For file-scope, we can choose to use the suggestion generator
            match kind {
                "unused_imports" | "unused_symbols" => {
                    let analysis_fn = if kind == "unused_imports" {
                        detect_unused_imports
                    } else {
                        detect_unused_symbols
                    };
                    self.run_analysis_and_suggest(context, &args, &scope_param, kind, analysis_fn)
                        .await
                }
                "unreachable_code" | "unused_parameters" | "unused_types" | "unused_variables" => {
                    let analysis_fn = match kind {
                        "unreachable_code" => detect_unreachable_code,
                        "unused_parameters" => detect_unused_parameters,
                        "unused_types" => detect_unused_types,
                        "unused_variables" => detect_unused_variables,
                        _ => unreachable!(),
                    };
                    self.run_analysis_and_suggest(context, &args, &scope_param, kind, analysis_fn)
                        .await
                }
                _ => {
                    return Err(ServerError::invalid_request(format!(
                        "Kind '{}' is not supported for file-scope analysis. Use scope_type='workspace' or choose a different kind.",
                        kind
                    )));
                }
            }
        }
    }
}
