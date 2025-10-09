use serde::{Deserialize, Serialize};

/// Complexity rating for a function
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComplexityRating {
    Simple,
    Moderate,
    Complex,
    VeryComplex,
}

impl ComplexityRating {
    /// Get rating from complexity score
    pub fn from_score(score: u32) -> Self {
        match score {
            1..=5 => Self::Simple,
            6..=10 => Self::Moderate,
            11..=20 => Self::Complex,
            _ => Self::VeryComplex,
        }
    }

    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Simple => "Low risk, easy to test",
            Self::Moderate => "Manageable complexity",
            Self::Complex => "Needs attention, harder to test",
            Self::VeryComplex => "High risk, should be refactored",
        }
    }

    /// Get recommendation text
    pub fn recommendation(&self) -> Option<&'static str> {
        match self {
            Self::Simple | Self::Moderate => None,
            Self::Complex => Some("Consider refactoring to reduce complexity"),
            Self::VeryComplex => Some("Strongly recommended to refactor into smaller functions"),
        }
    }
}

/// Comprehensive complexity metrics (cyclomatic + cognitive)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    /// Cyclomatic complexity (decision points + 1)
    pub cyclomatic: u32,
    /// Cognitive complexity (with nesting penalties)
    pub cognitive: u32,
    /// Maximum nesting depth in the function
    pub max_nesting_depth: u32,
}

/// Code quality metrics for a function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeMetrics {
    /// Source Lines of Code (excluding blanks and comments)
    pub sloc: u32,
    /// Total lines including blanks and comments
    pub total_lines: u32,
    /// Number of comment lines
    pub comment_lines: u32,
    /// Comment ratio (comment_lines / sloc)
    pub comment_ratio: f64,
    /// Number of function parameters
    pub parameters: u32,
}

/// Complexity metrics for a single function (enhanced version)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionComplexity {
    pub name: String,
    pub line: usize,
    #[serde(flatten)]
    pub complexity: ComplexityMetrics,
    #[serde(flatten)]
    pub metrics: CodeMetrics,
    pub rating: ComplexityRating,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub issues: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<String>,
}

/// Complexity report for an entire file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityReport {
    pub file_path: String,
    pub functions: Vec<FunctionComplexity>,
    pub average_complexity: f64,
    pub average_cognitive_complexity: f64,
    pub max_complexity: u32,
    pub max_cognitive_complexity: u32,
    pub total_functions: usize,
    pub total_sloc: u32,
    pub average_sloc: f64,
    pub total_issues: usize,
    pub summary: String,
}

/// Class/module-level complexity aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassComplexity {
    pub name: String,
    pub file_path: String,
    pub line: usize,
    pub function_count: usize,
    pub total_complexity: u32,
    pub total_cognitive_complexity: u32,
    pub average_complexity: f64,
    pub average_cognitive_complexity: f64,
    pub max_complexity: u32,
    pub max_cognitive_complexity: u32,
    pub total_sloc: u32,
    pub rating: ComplexityRating,
    pub issues: Vec<String>,
}

/// Summary for a single file in project analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileComplexitySummary {
    pub file_path: String,
    pub function_count: usize,
    pub class_count: usize,
    pub average_complexity: f64,
    pub average_cognitive_complexity: f64,
    pub max_complexity: u32,
    pub total_issues: usize,
}

/// Project-wide complexity report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectComplexityReport {
    pub directory: String,
    pub total_files: usize,
    pub total_functions: usize,
    pub total_classes: usize,
    pub files: Vec<FileComplexitySummary>,
    pub classes: Vec<ClassComplexity>,
    pub average_complexity: f64,
    pub average_cognitive_complexity: f64,
    pub max_complexity: u32,
    pub max_cognitive_complexity: u32,
    pub total_sloc: u32,
    pub hotspots_summary: String,
}

/// Function hotspot with file context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionHotspot {
    pub name: String,
    pub file_path: String,
    pub line: usize,
    pub complexity: u32,
    pub cognitive_complexity: u32,
    pub rating: ComplexityRating,
    pub sloc: u32,
}

/// Hotspots report for top N complex functions/classes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityHotspotsReport {
    pub directory: String,
    pub metric: String,
    pub top_functions: Vec<FunctionHotspot>,
    pub top_classes: Vec<ClassComplexity>,
    pub summary: String,
}