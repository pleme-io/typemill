//! Unified analysis result structures for all analysis operations
//!
//! This module defines the common result format used across all `analyze.*` commands.
//! Every analysis operation returns an `AnalysisResult` with findings, summary, and metadata.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unified result structure returned by all analysis commands
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisResult {
    /// List of findings from the analysis
    pub findings: Vec<Finding>,
    /// Summary statistics about the analysis
    pub summary: AnalysisSummary,
    /// Metadata about the analysis execution
    pub metadata: AnalysisMetadata,
}

/// A single finding from an analysis operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    /// Unique identifier for this finding
    pub id: String,
    /// Kind of finding (e.g., "complexity_hotspot", "unused_import")
    pub kind: String,
    /// Severity level
    pub severity: Severity,
    /// Location of the finding in the codebase
    pub location: FindingLocation,
    /// Metrics associated with this finding
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<HashMap<String, serde_json::Value>>,
    /// Human-readable message describing the finding
    pub message: String,
    /// Actionable suggestions for addressing this finding
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<Suggestion>,
}

/// Severity level for a finding
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    High,
    Medium,
    Low,
}

/// Location information for a finding
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindingLocation {
    /// File path where the finding was detected
    pub file_path: String,
    /// Range in the file (optional for file-level findings)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<Range>,
    /// Symbol name (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    /// Symbol kind (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_kind: Option<String>,
}

/// Position range in a file
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// Position in a file (1-indexed line, 0-indexed character)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    /// Line number (1-indexed)
    pub line: u32,
    /// Character offset (0-indexed)
    pub character: u32,
}

/// Actionable suggestion for addressing a finding
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    /// Action type (e.g., "extract_function", "inline_variable")
    pub action: String,
    /// Human-readable description of the suggestion
    pub description: String,
    /// Target location for the suggestion (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<SuggestionTarget>,
    /// Estimated impact of applying this suggestion
    pub estimated_impact: String,
    /// Safety level for applying this suggestion
    pub safety: SafetyLevel,
    /// Algorithm confidence in this suggestion (0.0 to 1.0)
    pub confidence: f64,
    /// Whether this refactoring can be undone without information loss
    pub reversible: bool,
    /// Reference to a refactoring command that can apply this suggestion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refactor_call: Option<RefactorCall>,
}

/// Safety level for applying a suggestion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyLevel {
    /// Safe - No logic changes, preserves semantics exactly
    Safe,
    /// Requires review - Logic changes, preserves intent but needs verification
    RequiresReview,
    /// Experimental - Significant changes, requires thorough testing
    Experimental,
}

/// Target for a suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionTarget {
    /// Range to apply the suggestion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<Range>,
}

/// Reference to a refactoring command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefactorCall {
    /// Command name (e.g., "extract.plan", "inline.plan")
    pub command: String,
    /// Arguments to pass to the command
    pub arguments: serde_json::Value,
}

/// Summary statistics for an analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisSummary {
    /// Total findings available (may be more than returned)
    pub total_findings: usize,
    /// Number of findings returned in this response
    pub returned_findings: usize,
    /// Whether there are more findings available via pagination
    pub has_more: bool,
    /// Breakdown of findings by severity
    pub by_severity: SeverityBreakdown,
    /// Number of files analyzed
    pub files_analyzed: usize,
    /// Number of symbols analyzed (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbols_analyzed: Option<usize>,
    /// Analysis execution time in milliseconds
    pub analysis_time_ms: u64,
}

/// Breakdown of findings by severity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeverityBreakdown {
    pub high: usize,
    pub medium: usize,
    pub low: usize,
}

/// Metadata about the analysis execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisMetadata {
    /// Analysis category (e.g., "quality", "dead_code")
    pub category: String,
    /// Analysis kind (e.g., "complexity", "smells")
    pub kind: String,
    /// Scope of the analysis
    pub scope: AnalysisScope,
    /// Language analyzed (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Timestamp when analysis was performed
    pub timestamp: String,
    /// Thresholds applied during analysis (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thresholds: Option<HashMap<String, serde_json::Value>>,
}

/// Scope specification for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisScope {
    /// Scope type (workspace, directory, file, symbol)
    #[serde(rename = "type")]
    pub scope_type: String,
    /// Path for the scope
    pub path: String,
    /// Include patterns (optional)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub include: Vec<String>,
    /// Exclude patterns (optional)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<String>,
}

impl AnalysisResult {
    /// Create a new empty analysis result
    pub fn new(category: &str, kind: &str, scope: AnalysisScope) -> Self {
        Self {
            findings: Vec::new(),
            summary: AnalysisSummary {
                total_findings: 0,
                returned_findings: 0,
                has_more: false,
                by_severity: SeverityBreakdown {
                    high: 0,
                    medium: 0,
                    low: 0,
                },
                files_analyzed: 0,
                symbols_analyzed: None,
                analysis_time_ms: 0,
            },
            metadata: AnalysisMetadata {
                category: category.to_string(),
                kind: kind.to_string(),
                scope,
                language: None,
                timestamp: chrono::Utc::now().to_rfc3339(),
                thresholds: None,
            },
        }
    }

    /// Add a finding to the result
    pub fn add_finding(&mut self, finding: Finding) {
        // Update severity breakdown
        match finding.severity {
            Severity::High => self.summary.by_severity.high += 1,
            Severity::Medium => self.summary.by_severity.medium += 1,
            Severity::Low => self.summary.by_severity.low += 1,
        }

        self.findings.push(finding);
        self.summary.total_findings = self.findings.len();
        self.summary.returned_findings = self.findings.len();
    }

    /// Finalize the result with execution time
    pub fn finalize(&mut self, analysis_time_ms: u64) {
        self.summary.analysis_time_ms = analysis_time_ms;
    }
}
