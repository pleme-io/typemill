use serde::{Deserialize, Serialize};

/// Enhanced suggestion with safety metadata and actionable refactor call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionableSuggestion {
    /// Human-readable suggestion message
    pub message: String,

    /// Safety classification
    pub safety: SafetyLevel,

    /// Confidence score (0.0 to 1.0)
    #[serde(serialize_with = "serialize_confidence")]
    pub confidence: f64,

    /// Can this refactoring be undone?
    pub reversible: bool,

    /// Estimated impact of applying this suggestion
    pub estimated_impact: ImpactLevel,

    /// Exact refactoring command to execute
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refactor_call: Option<RefactorCall>,

    /// Additional context for decision-making
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<SuggestionMetadata>,
}

/// Safety level classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyLevel {
    /// Safe to auto-apply without human review
    Safe,
    /// Requires human review before applying
    RequiresReview,
    /// Experimental - may not work in all cases
    Experimental,
}

/// Impact level of suggested change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ImpactLevel {
    Low,    // Single line, local scope
    Medium, // Multiple lines, function scope
    High,   // Cross-function, file scope
    Critical, // Cross-file, module scope
}

/// Refactoring command reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactorCall {
    /// Tool name (e.g., "extract.plan", "inline.plan")
    pub tool: String,

    /// Arguments to pass to the tool
    pub arguments: serde_json::Value,
}

/// Additional metadata for suggestion evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionMetadata {
    /// Why this suggestion was made
    pub rationale: String,

    /// Potential risks or edge cases
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub risks: Vec<String>,

    /// Expected benefits
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub benefits: Vec<String>,
}

/// Serialize confidence with 2 decimal places
fn serialize_confidence<S>(confidence: &f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_f64((*confidence * 100.0).round() / 100.0)
}

/// Internal structure for refactoring candidates (used during generation)
#[derive(Debug, Clone)]
pub struct RefactoringCandidate {
    pub refactor_type: RefactorType,
    pub message: String,
    pub scope: Scope,
    pub has_side_effects: bool,
    pub reference_count: Option<usize>,
    pub is_unreachable: bool,
    pub is_recursive: bool,
    pub involves_generics: bool,
    pub involves_macros: bool,
    pub evidence_strength: EvidenceStrength,
    pub location: Location,
    pub refactor_call_args: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefactorType {
    RemoveUnusedImport,
    RemoveUnusedVariable,
    RemoveDeadCode,
    SimplifyBooleanExpression,
    ExtractMethod,
    Inline,
    Move,
    Rename,
    Transform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Local,
    Function,
    File,
    CrossFile,
    CrossCrate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceStrength {
    Weak,   // Pattern matching only
    Medium, // AST shows no references
    Strong, // LSP confirms unused
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub file: String,
    pub line: usize,
    pub character: usize,
}

/// Analysis context for suggestion generation
#[derive(Debug, Clone)]
pub struct AnalysisContext {
    pub file_path: String,
    pub has_full_type_info: bool,
    pub has_partial_type_info: bool,
    pub ast_parse_errors: usize,
}
