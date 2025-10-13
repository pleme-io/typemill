
use super::super::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use cb_analysis_circular_deps::{builder::DependencyGraphBuilder, find_circular_dependencies, Cycle};
use cb_core::model::mcp::ToolCall;
use cb_protocol::analysis_result::{
    AnalysisResult, Finding, FindingLocation, SafetyLevel, Severity, Suggestion,
};
use cb_protocol::{ApiError as ServerError, ApiResult as ServerResult};
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::debug;

pub struct CircularDependenciesHandler;

impl CircularDependenciesHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ToolHandler for CircularDependenciesHandler {
    fn tool_names(&self) -> &[&str] {
        &["analyze.circular_dependencies"]
    }

    fn is_internal(&self) -> bool {
        false
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        let args = tool_call.arguments.clone().unwrap_or(json!({}));

        debug!("Handling analyze.circular_dependencies request");

        #[cfg(feature = "analysis-circular-deps")]
            {
                let project_root = &context.app_state.project_root;
                let path = args
                    .get("scope")
                    .and_then(|s| s.get("path"))
                    .and_then(|p| p.as_str())
                    .map(|p| project_root.join(p))
                    .unwrap_or_else(|| project_root.clone());

                let builder = DependencyGraphBuilder::new(&context.app_state.language_plugins.inner);
                let graph = builder
                    .build(&path)
                    .map_err(|e| ServerError::Internal(e))?;
                let min_size = args
                    .get("min_size")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                let result = find_circular_dependencies(&graph, min_size)
                    .map_err(|e| ServerError::Internal(e.to_string()))?;

                let findings = result.cycles.into_iter().map(|cycle| {
                    let mut metrics = HashMap::new();
                    metrics.insert("cycle_length".to_string(), json!(cycle.modules.len()));
                    metrics.insert("cycle_path".to_string(), json!(cycle.modules));

                    // Add import chain to metrics for detailed analysis
                    let import_chain_json: Vec<_> = cycle.import_chain.iter().map(|link| {
                        json!({
                            "from": link.from,
                            "to": link.to,
                            "symbols": link.symbols
                        })
                    }).collect();
                    metrics.insert("import_chain".to_string(), json!(import_chain_json));

                    // Generate actionable suggestions based on cycle characteristics
                    let suggestions = generate_cycle_break_suggestions(&cycle);

                    Finding {
                        id: format!("circular-dependency-{}", cycle.id),
                        kind: "circular_dependency".to_string(),
                        severity: Severity::High,
                        location: FindingLocation {
                            file_path: cycle.modules.get(0).cloned().unwrap_or_default(),
                            range: None,
                            symbol: None,
                            symbol_kind: Some("module".to_string()),
                        },
                        metrics: Some(metrics),
                        message: format!(
                            "Circular dependency detected: {} modules form a cycle ({})",
                            cycle.modules.len(),
                            cycle.modules.join(" → ")
                        ),

                        suggestions,
                    }
                }).collect();

                let analysis_result = AnalysisResult {
                    findings,
                    summary: cb_protocol::analysis_result::AnalysisSummary {
                        total_findings: result.summary.total_cycles,
                        returned_findings: result.summary.total_cycles,
                        has_more: false,
                        by_severity: cb_protocol::analysis_result::SeverityBreakdown {
                            high: result.summary.total_cycles,
                            medium: 0,
                            low: 0,
                        },
                        files_analyzed: result.summary.files_analyzed,
                        symbols_analyzed: Some(result.summary.total_modules_in_cycles),
                        analysis_time_ms: result.summary.analysis_time_ms,
                    },
                    metadata: cb_protocol::analysis_result::AnalysisMetadata {
                        category: "dependencies".to_string(),
                        kind: "circular".to_string(),
                        scope: cb_protocol::analysis_result::AnalysisScope {
                            scope_type: "workspace".to_string(),
                            path: project_root.to_string_lossy().to_string(),
                            include: vec![],
                            exclude: vec![],
                        },
                        language: None,
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        thresholds: None,
                    },
                };

                return Ok(serde_json::to_value(analysis_result)?);
            }
    }
}
#[cfg(feature = "analysis-circular-deps")]
/// Generate actionable suggestions for breaking circular dependencies
fn generate_cycle_break_suggestions(cycle: &Cycle) -> Vec<Suggestion> {
    let mut suggestions = Vec::new();

    // Suggestion 1: Extract interface/trait
    if cycle.modules.len() == 2 {
        suggestions.push(Suggestion {
            action: "extract_interface".to_string(),
            description: format!(
                "Extract a shared interface or trait between '{}' and '{}'. Move common dependencies to the interface to break the cycle.",
                cycle.modules.get(0).map(|s| s.as_str()).unwrap_or("module A"),
                cycle.modules.get(1).map(|s| s.as_str()).unwrap_or("module B")
            ),
            target: None,
            estimated_impact: "Eliminates circular dependency, improves testability and modularity".to_string(),
            safety: SafetyLevel::Safe,
            confidence: 0.85,
            reversible: true,
            refactor_call: None,
        });
    }

    // Suggestion 2: Dependency injection
    suggestions.push(Suggestion {
        action: "dependency_injection".to_string(),
        description: "Use dependency injection to invert the dependency direction. Pass dependencies as parameters instead of importing directly.".to_string(),
        target: None,
        estimated_impact: "Breaks cycle by inverting control, improves testability".to_string(),
        safety: SafetyLevel::RequiresReview,
        confidence: 0.80,
        reversible: true,
        refactor_call: None,
    });

    // Suggestion 3: Extract shared module
    if cycle.modules.len() > 2 {
        suggestions.push(Suggestion {
            action: "extract_shared_module".to_string(),
            description: format!(
                "Extract shared code from the {} modules into a new common module. This breaks the cycle by creating a dependency tree instead of a cycle.",
                cycle.modules.len()
            ),
            target: None,
            estimated_impact: "Eliminates circular dependency, reduces coupling".to_string(),
            safety: SafetyLevel::RequiresReview,
            confidence: 0.75,
            reversible: true,
            refactor_call: None,
        });
    }

    // Suggestion 4: Merge modules (for small cycles)
    if cycle.modules.len() == 2 {
        suggestions.push(Suggestion {
            action: "merge_modules".to_string(),
            description: "If the modules are tightly coupled and small, consider merging them into a single module.".to_string(),
            target: None,
            estimated_impact: "Simplifies architecture by removing artificial separation".to_string(),
            safety: SafetyLevel::RequiresReview,
            confidence: 0.70,
            reversible: true,
            refactor_call: None,
        });
    }

    suggestions
}
