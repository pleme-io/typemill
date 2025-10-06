//! Code Complexity Analysis
//!
//! This module provides language-agnostic complexity and quality metrics
//! for functions and methods.
//!
//! # Cyclomatic Complexity
//!
//! CC = E - N + 2P (simplified: decision points + 1)
//! - Measures number of linearly independent paths
//! - Language-agnostic decision point counting
//!
//! # Cognitive Complexity
//!
//! More accurate measure of code understandability:
//! - Adds nesting penalties (nested if = harder to understand)
//! - Ignores "shortcut" structures (early returns are good)
//! - Better predicts actual maintenance difficulty
//!
//! Example:
//! ```ignore
//! // Cyclomatic: 4, Cognitive: 7 (deeply nested)
//! if (a) {
//!     if (b) {
//!         if (c) {
//!             doSomething();
//!         }
//!     }
//! }
//!
//! // Cyclomatic: 4, Cognitive: 3 (flat structure)
//! if (!a) return;
//! if (!b) return;
//! if (!c) return;
//! doSomething();
//! ```
//!
//! # Code Metrics
//!
//! - SLOC (Source Lines of Code)
//! - Parameter count
//! - Comment ratio
//! - Maximum nesting depth
//!
//! # Complexity Ratings
//!
//! - 1-5: Simple (low risk, easy to test)
//! - 6-10: Moderate (manageable complexity)
//! - 11-20: Complex (needs attention, harder to test)
//! - 21+: Very complex (high risk, should be refactored)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// Language-specific decision point patterns
struct LanguagePatterns {
    decision_keywords: Vec<&'static str>,
    logical_operators: Vec<&'static str>,
}

impl LanguagePatterns {
    /// Get patterns for a specific language
    fn for_language(language: &str) -> Self {
        match language.to_lowercase().as_str() {
            "rust" | "go" | "java" => Self {
                decision_keywords: vec![
                    "if", "else if", "for", "while", "match", "case", "catch",
                ],
                logical_operators: vec!["&&", "||"],
            },
            "typescript" | "javascript" => Self {
                decision_keywords: vec![
                    "if", "else if", "for", "while", "do", "switch", "case", "catch",
                ],
                logical_operators: vec!["&&", "||", "?"],
            },
            "python" => Self {
                decision_keywords: vec![
                    "if", "elif", "for", "while", "except", "case", // case for Python 3.10+
                ],
                logical_operators: vec!["and", "or"],
            },
            _ => {
                // Fallback for unknown languages
                Self {
                    decision_keywords: vec!["if", "for", "while", "case", "catch"],
                    logical_operators: vec!["&&", "||"],
                }
            }
        }
    }
}

/// Calculate cyclomatic complexity for a function body
///
/// Uses a simplified algorithm that counts decision points in the source code.
/// This is not as accurate as building a full control flow graph, but it's fast
/// and works across all languages.
pub fn calculate_complexity(function_body: &str, language: &str) -> u32 {
    let patterns = LanguagePatterns::for_language(language);
    let mut complexity = 1; // Base complexity

    // Count decision keywords
    for keyword in &patterns.decision_keywords {
        complexity += count_keyword_occurrences(function_body, keyword);
    }

    // Count logical operators (each adds a branch)
    for operator in &patterns.logical_operators {
        complexity += count_operator_occurrences(function_body, operator);
    }

    complexity
}

/// Calculate comprehensive complexity metrics (cyclomatic + cognitive + nesting)
///
/// Cognitive complexity adds nesting penalties to better reflect human comprehension difficulty.
/// Unlike cyclomatic complexity, cognitive complexity penalizes nested structures more heavily.
pub fn calculate_complexity_metrics(function_body: &str, language: &str) -> ComplexityMetrics {
    let patterns = LanguagePatterns::for_language(language);

    let mut cyclomatic: u32 = 1; // Base complexity
    let mut cognitive: u32 = 0;
    let mut nesting_level: u32 = 0;
    let mut max_nesting: u32 = 0;

    let lines: Vec<&str> = function_body.lines().collect();

    for line in lines {
        let trimmed = line.trim();

        // Track nesting level by counting braces
        for ch in line.chars() {
            if ch == '{' {
                nesting_level += 1;
                max_nesting = max_nesting.max(nesting_level);
            } else if ch == '}' {
                nesting_level = nesting_level.saturating_sub(1);
            }
        }

        // Count decision keywords
        for keyword in &patterns.decision_keywords {
            let occurrences = count_keyword_occurrences(trimmed, keyword);
            if occurrences > 0 {
                // Cyclomatic: simple count
                cyclomatic += occurrences;

                // Cognitive: base increment + nesting penalty
                // Each decision point gets +1, plus +1 for each nesting level
                cognitive += occurrences + (occurrences * nesting_level);
            }
        }

        // Count logical operators
        for operator in &patterns.logical_operators {
            let occurrences = count_operator_occurrences(trimmed, operator);
            if occurrences > 0 {
                cyclomatic += occurrences;
                cognitive += occurrences + (occurrences * nesting_level);
            }
        }

        // Detect early returns (reduce cognitive complexity)
        if is_early_return(trimmed, language) && nesting_level == 0 {
            // Early returns at function level don't add cognitive complexity
            // (they're actually good for readability)
            cognitive = cognitive.saturating_sub(1);
        }
    }

    ComplexityMetrics {
        cyclomatic,
        cognitive,
        max_nesting_depth: max_nesting,
    }
}

/// Check if a line contains an early return/continue/break
fn is_early_return(line: &str, language: &str) -> bool {
    let line = line.trim();
    match language.to_lowercase().as_str() {
        "rust" | "go" | "java" | "typescript" | "javascript" => {
            line.starts_with("return") || line.starts_with("continue") || line.starts_with("break")
        }
        "python" => {
            line.starts_with("return") || line.starts_with("continue") || line.starts_with("break")
        }
        _ => line.starts_with("return"),
    }
}

/// Count occurrences of a keyword as a whole word (not part of another identifier)
fn count_keyword_occurrences(code: &str, keyword: &str) -> u32 {
    let mut count = 0;
    let keyword_bytes = keyword.as_bytes();
    let code_bytes = code.as_bytes();

    for (i, window) in code_bytes.windows(keyword.len()).enumerate() {
        if window == keyword_bytes {
            // Check if it's a word boundary before
            let before_ok = i == 0
                || !code_bytes[i - 1].is_ascii_alphanumeric()
                    && code_bytes[i - 1] != b'_';

            // Check if it's a word boundary after
            let after_index = i + keyword.len();
            let after_ok = after_index >= code_bytes.len()
                || !code_bytes[after_index].is_ascii_alphanumeric()
                    && code_bytes[after_index] != b'_';

            if before_ok && after_ok {
                count += 1;
            }
        }
    }

    count
}

/// Count occurrences of an operator
fn count_operator_occurrences(code: &str, operator: &str) -> u32 {
    // Simple substring count - operators don't need word boundaries
    let mut count = 0;
    let mut start = 0;

    while let Some(pos) = code[start..].find(operator) {
        count += 1;
        start += pos + operator.len();
    }

    count
}

/// Calculate code metrics for a function body
///
/// Analyzes SLOC, comment ratio, and other code quality metrics.
pub fn calculate_code_metrics(function_body: &str, language: &str) -> CodeMetrics {
    let lines: Vec<&str> = function_body.lines().collect();
    let total_lines = lines.len() as u32;

    let mut sloc = 0;
    let mut comment_lines = 0;

    // Language-specific comment patterns
    let (single_line_comment, multi_line_start, multi_line_end) = match language.to_lowercase().as_str() {
        "rust" | "go" | "java" | "typescript" | "javascript" => ("//", "/*", "*/"),
        "python" => ("#", "\"\"\"", "\"\"\""),
        _ => ("//", "/*", "*/"),
    };

    let mut in_multiline_comment = false;

    for line in &lines {
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Check for multi-line comment boundaries
        if trimmed.contains(multi_line_start) {
            in_multiline_comment = true;
        }

        // Check if this is a comment line
        let is_comment = in_multiline_comment
            || trimmed.starts_with(single_line_comment)
            || trimmed.starts_with("*"); // Continuation of /** */ style

        if is_comment {
            comment_lines += 1;
        } else {
            sloc += 1;
        }

        if trimmed.contains(multi_line_end) {
            in_multiline_comment = false;
        }
    }

    let comment_ratio = if sloc > 0 {
        comment_lines as f64 / sloc as f64
    } else {
        0.0
    };

    CodeMetrics {
        sloc,
        total_lines,
        comment_lines,
        comment_ratio,
        parameters: 0, // Will be filled in by analyze_file_complexity
    }
}

/// Count function parameters from symbol or function signature
///
/// This is a heuristic that counts commas in the parameter list.
fn count_parameters(function_body: &str, language: &str) -> u32 {
    // Find the function signature (first line typically)
    let first_line = function_body.lines().next().unwrap_or("");

    // Find parameter list between ( and )
    if let Some(paren_start) = first_line.find('(') {
        if let Some(paren_end) = first_line[paren_start..].find(')') {
            let params_str = &first_line[paren_start + 1..paren_start + paren_end];

            // Empty parameter list
            if params_str.trim().is_empty() {
                return 0;
            }

            // Count parameters by counting commas + 1
            // Handle special cases like "self" in Python/Rust
            let param_count = params_str.matches(',').count() as u32 + 1;

            // Adjust for languages with implicit self/this
            match language.to_lowercase().as_str() {
                "python" => {
                    // Python methods have "self" as first param
                    if params_str.trim().starts_with("self,") || params_str.trim() == "self" {
                        param_count.saturating_sub(1)
                    } else {
                        param_count
                    }
                }
                "rust" => {
                    // Rust methods might have &self, &mut self, self
                    if params_str.trim().starts_with("&self")
                        || params_str.trim().starts_with("self")
                        || params_str.trim().starts_with("&mut self") {
                        param_count.saturating_sub(1)
                    } else {
                        param_count
                    }
                }
                _ => param_count,
            }
        } else {
            0
        }
    } else {
        0
    }
}

/// Analyze complexity for all functions in a file
///
/// This requires the parsed source with function/method symbols.
pub fn analyze_file_complexity(
    file_path: &str,
    content: &str,
    symbols: &[cb_plugin_api::Symbol],
    language: &str,
) -> ComplexityReport {
    let mut functions = Vec::new();

    for symbol in symbols {
        // Only analyze functions and methods
        if !matches!(
            symbol.kind,
            cb_plugin_api::SymbolKind::Function | cb_plugin_api::SymbolKind::Method
        ) {
            continue;
        }

        // Extract function body
        let function_body = extract_function_body(content, &symbol.location);

        // Calculate comprehensive metrics
        let complexity = calculate_complexity_metrics(&function_body, language);
        let mut code_metrics = calculate_code_metrics(&function_body, language);

        // Count parameters
        code_metrics.parameters = count_parameters(&function_body, language);

        // Determine rating based on cognitive complexity (more accurate)
        let rating = ComplexityRating::from_score(complexity.cognitive);

        // Identify issues
        let mut issues = Vec::new();

        if complexity.cognitive > 15 {
            issues.push(format!(
                "High cognitive complexity ({}) due to nesting depth ({})",
                complexity.cognitive, complexity.max_nesting_depth
            ));
        }

        if code_metrics.parameters > 5 {
            issues.push(format!(
                "Too many parameters ({} > 5 recommended)",
                code_metrics.parameters
            ));
        }

        if complexity.max_nesting_depth > 4 {
            issues.push(format!(
                "Deep nesting ({} levels) reduces readability",
                complexity.max_nesting_depth
            ));
        }

        if code_metrics.comment_ratio < 0.1 && code_metrics.sloc > 20 {
            issues.push(format!(
                "Low comment ratio ({:.2}) for {} lines of code",
                code_metrics.comment_ratio, code_metrics.sloc
            ));
        }

        functions.push(FunctionComplexity {
            name: symbol.name.clone(),
            line: symbol.location.line,
            complexity,
            metrics: code_metrics,
            rating,
            issues,
            recommendation: rating.recommendation().map(|s| s.to_string()),
        });
    }

    // Calculate statistics
    let total_functions = functions.len();
    let total_complexity: u32 = functions.iter().map(|f| f.complexity.cyclomatic).sum();
    let total_cognitive: u32 = functions.iter().map(|f| f.complexity.cognitive).sum();
    let total_sloc: u32 = functions.iter().map(|f| f.metrics.sloc).sum();
    let total_issues: usize = functions.iter().map(|f| f.issues.len()).sum();

    let average_complexity = if total_functions > 0 {
        total_complexity as f64 / total_functions as f64
    } else {
        0.0
    };

    let average_cognitive_complexity = if total_functions > 0 {
        total_cognitive as f64 / total_functions as f64
    } else {
        0.0
    };

    let average_sloc = if total_functions > 0 {
        total_sloc as f64 / total_functions as f64
    } else {
        0.0
    };

    let max_complexity = functions
        .iter()
        .map(|f| f.complexity.cyclomatic)
        .max()
        .unwrap_or(0);

    let max_cognitive_complexity = functions
        .iter()
        .map(|f| f.complexity.cognitive)
        .max()
        .unwrap_or(0);

    // Count functions by rating
    let mut rating_counts: HashMap<ComplexityRating, usize> = HashMap::new();
    for func in &functions {
        *rating_counts.entry(func.rating).or_insert(0) += 1;
    }

    // Generate summary
    let needs_attention = rating_counts
        .get(&ComplexityRating::Complex)
        .unwrap_or(&0)
        + rating_counts
            .get(&ComplexityRating::VeryComplex)
            .unwrap_or(&0);

    let summary = if total_functions == 0 {
        "No functions found to analyze".to_string()
    } else if needs_attention == 0 {
        format!(
            "{} functions analyzed. All functions have acceptable complexity.",
            total_functions
        )
    } else {
        format!(
            "{} functions analyzed. {} function{} need{} attention (complexity > 10).",
            total_functions,
            needs_attention,
            if needs_attention == 1 { "" } else { "s" },
            if needs_attention == 1 { "s" } else { "" }
        )
    };

    ComplexityReport {
        file_path: file_path.to_string(),
        functions,
        average_complexity,
        average_cognitive_complexity,
        max_complexity,
        max_cognitive_complexity,
        total_functions,
        total_sloc,
        average_sloc,
        total_issues,
        summary,
    }
}

/// Extract function body from source code using location information
///
/// This is a heuristic extraction since Symbol only has a start location.
/// We extract from the symbol line to the end of the logical block.
fn extract_function_body(content: &str, location: &cb_plugin_api::SourceLocation) -> String {
    let lines: Vec<&str> = content.lines().collect();

    let start_line = location.line.saturating_sub(1);

    if start_line >= lines.len() {
        return String::new();
    }

    // Heuristic: Extract until we find matching braces or reach end
    // For complexity analysis, we need to capture the entire function body
    let mut body_lines = Vec::new();
    let mut brace_count = 0;
    let mut started = false;

    for (idx, line) in lines.iter().enumerate().skip(start_line) {
        body_lines.push(*line);

        // Track braces to find function end
        for ch in line.chars() {
            match ch {
                '{' => {
                    brace_count += 1;
                    started = true;
                }
                '}' => {
                    brace_count -= 1;
                    if started && brace_count == 0 {
                        return body_lines.join("\n");
                    }
                }
                _ => {}
            }
        }

        // Safety limit: don't extract more than 500 lines
        if idx - start_line > 500 {
            break;
        }
    }

    body_lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_function_complexity() {
        let code = r#"
fn simple() {
    let x = 1;
    return x;
}
"#;
        assert_eq!(calculate_complexity(code, "rust"), 1);
    }

    #[test]
    fn test_function_with_if() {
        let code = r#"
fn with_if(x: i32) {
    if x > 0 {
        println!("positive");
    }
}
"#;
        assert_eq!(calculate_complexity(code, "rust"), 2); // 1 base + 1 if
    }

    #[test]
    fn test_function_with_multiple_branches() {
        let code = r#"
fn complex(x: i32) {
    if x > 0 {
        println!("positive");
    } else if x < 0 {
        println!("negative");
    }

    for i in 0..10 {
        if i % 2 == 0 {
            continue;
        }
    }
}
"#;
        // 1 base + 1 if + 1 else if + 1 for + 1 if = 5
        assert_eq!(calculate_complexity(code, "rust"), 5);
    }

    #[test]
    fn test_function_with_logical_operators() {
        let code = r#"
fn with_logic(x: i32, y: i32) {
    if x > 0 && y > 0 {
        println!("both positive");
    }

    if x < 0 || y < 0 {
        println!("at least one negative");
    }
}
"#;
        // 1 base + 2 if + 1 && + 1 || = 5
        assert_eq!(calculate_complexity(code, "rust"), 5);
    }

    #[test]
    fn test_complexity_rating() {
        assert_eq!(ComplexityRating::from_score(3), ComplexityRating::Simple);
        assert_eq!(
            ComplexityRating::from_score(8),
            ComplexityRating::Moderate
        );
        assert_eq!(
            ComplexityRating::from_score(15),
            ComplexityRating::Complex
        );
        assert_eq!(
            ComplexityRating::from_score(25),
            ComplexityRating::VeryComplex
        );
    }

    #[test]
    fn test_keyword_not_in_identifier() {
        let code = r#"
fn test() {
    let iffy = 5;  // Should not count 'if' in 'iffy'
    if iffy > 0 {  // Should count this 'if'
        println!("positive");
    }
}
"#;
        assert_eq!(calculate_complexity(code, "rust"), 2); // 1 base + 1 if (not counting 'iffy')
    }

    #[test]
    fn test_python_keywords() {
        let code = r#"
def test(x):
    if x > 0:
        print("positive")
    elif x < 0:
        print("negative")

    for i in range(10):
        if i % 2 == 0 and i > 5:
            continue
"#;
        // 1 base + 1 if + 1 elif + 1 for + 1 if + 1 and = 6
        assert_eq!(calculate_complexity(code, "python"), 6);
    }

    #[test]
    fn test_typescript_keywords() {
        let code = r#"
function test(x: number) {
    if (x > 0) {
        console.log("positive");
    }

    const result = x > 10 ? "big" : "small";  // ternary operator

    for (let i = 0; i < 10; i++) {
        if (i % 2 === 0 || i > 7) {
            continue;
        }
    }
}
"#;
        // 1 base + 1 if + 1 ? + 1 for + 1 if + 1 || = 6
        assert_eq!(calculate_complexity(code, "typescript"), 6);
    }
}
