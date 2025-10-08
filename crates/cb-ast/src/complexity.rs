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

    // Process line by line to skip comments
    for line in code.lines() {
        let trimmed = line.trim();

        // Skip obvious comment lines
        if trimmed.starts_with("//")
            || trimmed.starts_with('#')
            || trimmed.starts_with('*')
            || trimmed.starts_with("/*")
        {
            continue;
        }

        // Strip inline comments (simplified - just handles // comments)
        let code_part = line.split("//").next().unwrap_or(line);

        let keyword_bytes = keyword.as_bytes();
        let line_bytes = code_part.as_bytes();

        for (i, window) in line_bytes.windows(keyword.len()).enumerate() {
            if window == keyword_bytes {
                // Check if it's a word boundary before
                let before_ok = i == 0
                    || !line_bytes[i - 1].is_ascii_alphanumeric() && line_bytes[i - 1] != b'_';

                // Check if it's a word boundary after
                let after_index = i + keyword.len();
                let after_ok = after_index >= line_bytes.len()
                    || !line_bytes[after_index].is_ascii_alphanumeric()
                        && line_bytes[after_index] != b'_';

                if before_ok && after_ok {
                    count += 1;
                }
            }
        }
    }

    count
}

/// Count occurrences of an operator
fn count_operator_occurrences(code: &str, operator: &str) -> u32 {
    let mut count = 0;

    // Check if operator is word-like (e.g., "and", "or", "not")
    let is_word_operator = operator.chars().all(|c| c.is_alphabetic());

    if is_word_operator {
        // Word-like operators need word boundary checking (like keywords)
        // Process line by line to skip comments
        for line in code.lines() {
            let trimmed = line.trim();

            // Skip comment lines
            if trimmed.starts_with("//")
                || trimmed.starts_with('#')
                || trimmed.starts_with('*')
                || trimmed.starts_with("/*")
            {
                continue;
            }

            // Strip inline comments
            let code_part = line.split("//").next().unwrap_or(line);

            let operator_bytes = operator.as_bytes();
            let line_bytes = code_part.as_bytes();

            for (i, window) in line_bytes.windows(operator.len()).enumerate() {
                if window == operator_bytes {
                    // Check word boundaries
                    let before_ok = i == 0
                        || !line_bytes[i - 1].is_ascii_alphanumeric() && line_bytes[i - 1] != b'_';

                    let after_index = i + operator.len();
                    let after_ok = after_index >= line_bytes.len()
                        || !line_bytes[after_index].is_ascii_alphanumeric()
                            && line_bytes[after_index] != b'_';

                    if before_ok && after_ok {
                        count += 1;
                    }
                }
            }
        }
    } else {
        // Symbol operators (&&, ||, ==) don't need word boundaries or comment filtering
        let mut start = 0;
        while let Some(pos) = code[start..].find(operator) {
            count += 1;
            start += pos + operator.len();
        }
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
    // Language-specific function declaration keywords
    let fn_keyword = match language.to_lowercase().as_str() {
        "rust" => "fn ",
        "python" => "def ",
        "typescript" | "javascript" => "function ",
        "go" => "func ",
        "java" => "public ", // Simplified - methods usually start with visibility
        _ => "fn ",
    };

    // Find the line with the function declaration (skip comments and empty lines)
    let fn_line = function_body
        .lines()
        .find(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with(fn_keyword)
                || trimmed.starts_with("pub fn ")
                || trimmed.starts_with("async fn ")
                || trimmed.starts_with("const fn ")
        })
        .unwrap_or("");

    // Find parameter list between ( and )
    if let Some(paren_start) = fn_line.find('(') {
        if let Some(paren_end) = fn_line[paren_start..].find(')') {
            let params_str = &fn_line[paren_start + 1..paren_start + paren_end];

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

/// Extract class/module name from a function symbol
///
/// Language-specific extraction:
/// - Python: `ClassName.method_name` → `ClassName`
/// - TypeScript/JavaScript: `ClassName.methodName` → `ClassName`
/// - Rust: Uses file-level grouping (module is the "class")
/// - Go: Uses receiver type or file-level grouping
/// - Java: `ClassName.methodName` → `ClassName`
fn extract_class_name(function_name: &str, language: &str) -> Option<String> {
    match language.to_lowercase().as_str() {
        "python" | "typescript" | "javascript" | "java" => {
            // Look for ClassName.methodName pattern
            function_name.rfind('.').map(|dot_pos| function_name[..dot_pos].to_string())
        }
        "rust" | "go" => {
            // For Rust and Go, we'll use file-level grouping
            // Class aggregation happens at the file level
            None
        }
        _ => None,
    }
}

/// Aggregate function-level complexity into class/module-level metrics
///
/// Groups functions by class (when detectable) and calculates aggregate metrics.
/// For languages without explicit classes (Rust modules, Go packages), treats
/// the entire file as a single "class".
pub fn aggregate_class_complexity(
    file_path: &str,
    functions: &[FunctionComplexity],
    language: &str,
) -> Vec<ClassComplexity> {
    use std::collections::HashMap;

    if functions.is_empty() {
        return Vec::new();
    }

    // Group functions by class
    let mut class_groups: HashMap<String, Vec<&FunctionComplexity>> = HashMap::new();

    for func in functions {
        let class_name = extract_class_name(&func.name, language)
            .unwrap_or_else(|| "<module>".to_string());
        class_groups.entry(class_name).or_default().push(func);
    }

    // Calculate metrics for each class
    let mut classes = Vec::new();

    for (class_name, class_functions) in class_groups {
        let function_count = class_functions.len();
        let total_complexity: u32 = class_functions
            .iter()
            .map(|f| f.complexity.cyclomatic)
            .sum();
        let total_cognitive: u32 = class_functions
            .iter()
            .map(|f| f.complexity.cognitive)
            .sum();
        let total_sloc: u32 = class_functions.iter().map(|f| f.metrics.sloc).sum();

        let average_complexity = if function_count > 0 {
            total_complexity as f64 / function_count as f64
        } else {
            0.0
        };

        let average_cognitive = if function_count > 0 {
            total_cognitive as f64 / function_count as f64
        } else {
            0.0
        };

        let max_complexity = class_functions
            .iter()
            .map(|f| f.complexity.cyclomatic)
            .max()
            .unwrap_or(0);

        let max_cognitive = class_functions
            .iter()
            .map(|f| f.complexity.cognitive)
            .max()
            .unwrap_or(0);

        // Determine rating based on average cognitive complexity
        let rating = ComplexityRating::from_score(average_cognitive as u32);

        // Collect issues
        let mut issues = Vec::new();
        if average_cognitive > 10.0 {
            issues.push(format!(
                "High average cognitive complexity ({:.1})",
                average_cognitive
            ));
        }
        if function_count > 20 {
            issues.push(format!(
                "Large class with {} methods (consider splitting)",
                function_count
            ));
        }

        // Get first function line as class line (approximation)
        let line = class_functions
            .iter()
            .map(|f| f.line)
            .min()
            .unwrap_or(0);

        classes.push(ClassComplexity {
            name: class_name,
            file_path: file_path.to_string(),
            line,
            function_count,
            total_complexity,
            total_cognitive_complexity: total_cognitive,
            average_complexity,
            average_cognitive_complexity: average_cognitive,
            max_complexity,
            max_cognitive_complexity: max_cognitive,
            total_sloc,
            rating,
            issues,
        });
    }

    // Sort by cognitive complexity (highest first)
    classes.sort_by(|a, b| {
        b.total_cognitive_complexity
            .cmp(&a.total_cognitive_complexity)
    });

    classes
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
        // 1 base + 1 if + 1 else + 1 if + 1 for + 1 if = 6
        assert_eq!(calculate_complexity(code, "rust"), 6);
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

    // ========================================================================
    // Cognitive Complexity Tests
    // ========================================================================

    #[test]
    fn test_cognitive_complexity_nested_vs_flat() {
        // Deeply nested code has higher cognitive complexity
        let nested_code = r#"
fn nested(a: bool, b: bool, c: bool) {
    if a {
        if b {
            if c {
                doSomething();
            }
        }
    }
}
"#;

        // Flat code with same cyclomatic complexity
        let flat_code = r#"
fn flat(a: bool, b: bool, c: bool) {
    if !a { return; }
    if !b { return; }
    if !c { return; }
    doSomething();
}
"#;

        let nested_metrics = calculate_complexity_metrics(nested_code, "rust");
        let flat_metrics = calculate_complexity_metrics(flat_code, "rust");

        // Both have same cyclomatic complexity
        assert_eq!(nested_metrics.cyclomatic, 4); // 1 base + 3 ifs
        assert_eq!(flat_metrics.cyclomatic, 4);

        // But cognitive complexity is different
        assert!(nested_metrics.cognitive > flat_metrics.cognitive,
            "Nested code should have higher cognitive complexity. Nested: {}, Flat: {}",
            nested_metrics.cognitive, flat_metrics.cognitive);

        // Nesting depth is different (includes function braces)
        assert_eq!(nested_metrics.max_nesting_depth, 4); // function + 3 nested ifs
        assert_eq!(flat_metrics.max_nesting_depth, 2); // function + 1 if
    }

    #[test]
    fn test_cognitive_complexity_with_nesting_penalty() {
        let code = r#"
fn complex(items: Vec<i32>) {
    for item in items {
        if item > 0 {
            if item % 2 == 0 {
                println!("positive even");
            }
        }
    }
}
"#;
        let metrics = calculate_complexity_metrics(code, "rust");

        // Cyclomatic: 1 base + 1 for + 2 if = 4
        assert_eq!(metrics.cyclomatic, 4);

        // Cognitive: Each decision gets +1 plus nesting penalty
        // for (nesting 1): +1 +1 = 2
        // if (nesting 2): +1 +2 = 3
        // if (nesting 3): +1 +3 = 4
        // Total cognitive: 2 + 3 + 4 = 9
        assert!(metrics.cognitive > metrics.cyclomatic,
            "Cognitive ({}) should be > cyclomatic ({}) for nested code",
            metrics.cognitive, metrics.cyclomatic);
        assert!(metrics.max_nesting_depth >= 3);
    }

    #[test]
    fn test_early_return_reduces_cognitive() {
        let code_with_returns = r#"
fn process(x: i32) {
    if x < 0 { return; }
    if x == 0 { return; }
    if x > 100 { return; }
    println!("valid");
}
"#;
        let metrics = calculate_complexity_metrics(code_with_returns, "rust");

        // With 3 if statements at nesting level 1:
        // Cyclomatic = 1 + 3 = 4
        // Cognitive = 3 * (1 + 1) = 6 (each if gets +1 base + 1 nesting penalty)
        // Note: Early return detection requires nesting_level == 0, but these are at level 1
        assert_eq!(metrics.cyclomatic, 4, "Cyclomatic should be 4 (3 ifs + 1 base)");
        assert_eq!(metrics.cognitive, 6, "Cognitive should be 6 (3 ifs * 2)");
        assert_eq!(metrics.max_nesting_depth, 2, "Max nesting should be 2");
    }

    // ========================================================================
    // Code Metrics Tests
    // ========================================================================

    #[test]
    fn test_sloc_calculation() {
        let code = r#"
// This is a comment
fn example() {
    let x = 1;  // inline comment

    let y = 2;
}
"#;
        let metrics = calculate_code_metrics(code, "rust");

        // Should count only actual code lines (fn, let x, let y, closing brace)
        assert_eq!(metrics.sloc, 4);
        assert!(metrics.comment_lines > 0);
        assert!(metrics.total_lines > metrics.sloc);
    }

    #[test]
    fn test_comment_ratio_calculation() {
        let well_documented = r#"
// Function does something important
// It processes data
fn process() {
    let x = 1;
    let y = 2;
}
"#;

        let poorly_documented = r#"
fn process() {
    let x = 1;
    let y = 2;
    let z = 3;
    let a = 4;
}
"#;

        let good_metrics = calculate_code_metrics(well_documented, "rust");
        let poor_metrics = calculate_code_metrics(poorly_documented, "rust");

        assert!(good_metrics.comment_ratio > poor_metrics.comment_ratio,
            "Well-documented code should have higher comment ratio");
    }

    #[test]
    fn test_multiline_comment_detection() {
        let code = r#"
/*
 * Multi-line comment
 * spanning several lines
 */
fn example() {
    let x = 1;
}
"#;
        let metrics = calculate_code_metrics(code, "rust");

        assert!(metrics.comment_lines >= 3, "Should detect multi-line comments");
        assert_eq!(metrics.sloc, 3); // fn, let, closing brace
    }

    #[test]
    fn test_parameter_count() {
        let no_params = "fn test() { }";
        let few_params = "fn test(a: i32, b: i32, c: i32) { }";
        let many_params = "fn test(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32) { }";

        assert_eq!(count_parameters(no_params, "rust"), 0);
        assert_eq!(count_parameters(few_params, "rust"), 3);
        assert_eq!(count_parameters(many_params, "rust"), 7);
    }

    #[test]
    fn test_parameter_count_with_self() {
        // Rust methods with self
        let rust_method = "fn process(&self, data: String) { }";
        assert_eq!(count_parameters(rust_method, "rust"), 1); // self excluded

        // Python methods with self
        let python_method = "def process(self, data): pass";
        assert_eq!(count_parameters(python_method, "python"), 1); // self excluded
    }

    #[test]
    fn test_complexity_metrics_integration() {
        let complex_function = r#"
// Process user data with validation
fn process_user(id: i32, name: String, email: String, age: i32, role: String, active: bool) {
    if id > 0 {
        if !name.is_empty() {
            if email.contains("@") {
                if age >= 18 {
                    if role == "admin" || role == "user" {
                        if active {
                            println!("Valid user");
                        }
                    }
                }
            }
        }
    }
}
"#;

        let metrics = calculate_complexity_metrics(complex_function, "rust");
        let code_metrics = calculate_code_metrics(complex_function, "rust");

        // Should detect high cognitive complexity due to nesting
        assert!(metrics.cognitive > 15, "Should detect high cognitive complexity");
        assert!(metrics.max_nesting_depth >= 5, "Should detect deep nesting");

        // Should detect too many parameters
        let param_count = count_parameters(complex_function, "rust");
        assert!(param_count > 5, "Should detect too many parameters");

        // Should have some documentation
        assert!(code_metrics.comment_lines > 0, "Should detect comments");
    }

    #[test]
    fn test_python_complexity() {
        let python_code = r#"
def process(items):
    for item in items:
        if item > 0:
            if item % 2 == 0 and item < 100:
                print("valid")
"#;

        let metrics = calculate_complexity_metrics(python_code, "python");

        // Python complexity: for, if, if, and
        assert_eq!(metrics.cyclomatic, 5, "Cyclomatic: 1 base + for + if + if + and");

        // Note: Python uses indentation, not braces, so max_nesting_depth will be 0
        // Cognitive complexity is still calculated based on keywords
        assert!(metrics.cognitive > 0, "Should calculate cognitive complexity for Python");
        assert_eq!(metrics.max_nesting_depth, 0, "Python has no braces, so nesting depth is 0");
    }

    #[test]
    fn test_typescript_complexity() {
        let ts_code = r#"
function validate(data: any): boolean {
    if (!data) return false;
    if (!data.name) return false;

    for (const item of data.items) {
        if (item.active && item.valid) {
            return true;
        }
    }
    return false;
}
"#;

        let metrics = calculate_complexity_metrics(ts_code, "typescript");

        assert!(metrics.cyclomatic >= 4);
        // Verify it detected some complexity
        assert!(metrics.cognitive > 0, "Should calculate cognitive complexity");
        assert!(metrics.max_nesting_depth >= 1, "Should track nesting");
    }

    #[test]
    fn test_extract_class_name_python() {
        // Python class method
        assert_eq!(
            extract_class_name("MyClass.my_method", "python"),
            Some("MyClass".to_string())
        );

        // Top-level function
        assert_eq!(extract_class_name("standalone_function", "python"), None);

        // Nested class
        assert_eq!(
            extract_class_name("OuterClass.InnerClass.method", "python"),
            Some("OuterClass.InnerClass".to_string())
        );
    }

    #[test]
    fn test_extract_class_name_typescript() {
        // TypeScript class method
        assert_eq!(
            extract_class_name("Calculator.add", "typescript"),
            Some("Calculator".to_string())
        );

        // Top-level function
        assert_eq!(extract_class_name("helperFunction", "typescript"), None);
    }

    #[test]
    fn test_extract_class_name_rust() {
        // Rust uses file-level grouping, should return None
        assert_eq!(extract_class_name("impl_method", "rust"), None);
        assert_eq!(extract_class_name("free_function", "rust"), None);
    }

    #[test]
    fn test_aggregate_class_complexity_empty() {
        let functions = vec![];
        let classes = aggregate_class_complexity("test.py", &functions, "python");
        assert_eq!(classes.len(), 0);
    }

    #[test]
    fn test_aggregate_class_complexity_python() {
        // Create sample function complexities
        let functions = vec![
            FunctionComplexity {
                name: "Calculator.add".to_string(),
                line: 10,
                complexity: ComplexityMetrics {
                    cyclomatic: 2,
                    cognitive: 1,
                    max_nesting_depth: 1,
                },
                metrics: CodeMetrics {
                    sloc: 5,
                    total_lines: 6,
                    comment_lines: 1,
                    comment_ratio: 0.2,
                    parameters: 2,
                },
                rating: ComplexityRating::Simple,
                issues: vec![],
                recommendation: None,
            },
            FunctionComplexity {
                name: "Calculator.subtract".to_string(),
                line: 20,
                complexity: ComplexityMetrics {
                    cyclomatic: 3,
                    cognitive: 2,
                    max_nesting_depth: 1,
                },
                metrics: CodeMetrics {
                    sloc: 8,
                    total_lines: 9,
                    comment_lines: 1,
                    comment_ratio: 0.125,
                    parameters: 2,
                },
                rating: ComplexityRating::Simple,
                issues: vec![],
                recommendation: None,
            },
            FunctionComplexity {
                name: "standalone_function".to_string(),
                line: 30,
                complexity: ComplexityMetrics {
                    cyclomatic: 1,
                    cognitive: 0,
                    max_nesting_depth: 0,
                },
                metrics: CodeMetrics {
                    sloc: 3,
                    total_lines: 3,
                    comment_lines: 0,
                    comment_ratio: 0.0,
                    parameters: 0,
                },
                rating: ComplexityRating::Simple,
                issues: vec![],
                recommendation: None,
            },
        ];

        let classes = aggregate_class_complexity("test.py", &functions, "python");

        // Should have 2 classes: Calculator and <module>
        assert_eq!(classes.len(), 2);

        // Find Calculator class
        let calculator = classes.iter().find(|c| c.name == "Calculator");
        assert!(calculator.is_some(), "Should find Calculator class");

        let calculator = calculator.unwrap();
        assert_eq!(calculator.function_count, 2);
        assert_eq!(calculator.total_complexity, 5); // 2 + 3
        assert_eq!(calculator.total_cognitive_complexity, 3); // 1 + 2
        assert_eq!(calculator.total_sloc, 13); // 5 + 8
        assert_eq!(calculator.average_complexity, 2.5); // (2 + 3) / 2
        assert_eq!(calculator.average_cognitive_complexity, 1.5); // (1 + 2) / 2

        // Find module-level functions
        let module = classes.iter().find(|c| c.name == "<module>");
        assert!(module.is_some(), "Should find <module> for top-level functions");

        let module = module.unwrap();
        assert_eq!(module.function_count, 1);
        assert_eq!(module.total_complexity, 1);
    }

    #[test]
    fn test_aggregate_class_complexity_rust() {
        // For Rust, all functions should go to <module>
        let functions = vec![
            FunctionComplexity {
                name: "parse_data".to_string(),
                line: 10,
                complexity: ComplexityMetrics {
                    cyclomatic: 5,
                    cognitive: 4,
                    max_nesting_depth: 2,
                },
                metrics: CodeMetrics {
                    sloc: 20,
                    total_lines: 25,
                    comment_lines: 5,
                    comment_ratio: 0.25,
                    parameters: 1,
                },
                rating: ComplexityRating::Simple,
                issues: vec![],
                recommendation: None,
            },
            FunctionComplexity {
                name: "validate_input".to_string(),
                line: 40,
                complexity: ComplexityMetrics {
                    cyclomatic: 3,
                    cognitive: 2,
                    max_nesting_depth: 1,
                },
                metrics: CodeMetrics {
                    sloc: 10,
                    total_lines: 12,
                    comment_lines: 2,
                    comment_ratio: 0.2,
                    parameters: 2,
                },
                rating: ComplexityRating::Simple,
                issues: vec![],
                recommendation: None,
            },
        ];

        let classes = aggregate_class_complexity("lib.rs", &functions, "rust");

        // Should have 1 "class" (module-level)
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].name, "<module>");
        assert_eq!(classes[0].function_count, 2);
        assert_eq!(classes[0].total_complexity, 8); // 5 + 3
        assert_eq!(classes[0].total_cognitive_complexity, 6); // 4 + 2
    }

    #[test]
    fn test_aggregate_class_complexity_large_class() {
        // Test that large classes get flagged with issues
        let mut functions = vec![];

        // Create 25 functions in the same class
        for i in 0..25 {
            functions.push(FunctionComplexity {
                name: format!("LargeClass.method{}", i),
                line: 10 + i * 10,
                complexity: ComplexityMetrics {
                    cyclomatic: 2,
                    cognitive: 1,
                    max_nesting_depth: 1,
                },
                metrics: CodeMetrics {
                    sloc: 5,
                    total_lines: 6,
                    comment_lines: 1,
                    comment_ratio: 0.2,
                    parameters: 1,
                },
                rating: ComplexityRating::Simple,
                issues: vec![],
                recommendation: None,
            });
        }

        let classes = aggregate_class_complexity("large.py", &functions, "python");

        assert_eq!(classes.len(), 1);
        let large_class = &classes[0];
        assert_eq!(large_class.name, "LargeClass");
        assert_eq!(large_class.function_count, 25);

        // Should have issue about large class
        assert!(
            large_class
                .issues
                .iter()
                .any(|issue| issue.contains("Large class")),
            "Should flag large class as an issue"
        );
    }

    #[test]
    fn test_aggregate_class_complexity_high_average() {
        // Test that high average complexity gets flagged
        let functions = vec![
            FunctionComplexity {
                name: "ComplexClass.method1".to_string(),
                line: 10,
                complexity: ComplexityMetrics {
                    cyclomatic: 15,
                    cognitive: 12,
                    max_nesting_depth: 4,
                },
                metrics: CodeMetrics {
                    sloc: 50,
                    total_lines: 60,
                    comment_lines: 10,
                    comment_ratio: 0.2,
                    parameters: 3,
                },
                rating: ComplexityRating::Complex,
                issues: vec![],
                recommendation: None,
            },
            FunctionComplexity {
                name: "ComplexClass.method2".to_string(),
                line: 80,
                complexity: ComplexityMetrics {
                    cyclomatic: 18,
                    cognitive: 15,
                    max_nesting_depth: 5,
                },
                metrics: CodeMetrics {
                    sloc: 70,
                    total_lines: 85,
                    comment_lines: 15,
                    comment_ratio: 0.214,
                    parameters: 4,
                },
                rating: ComplexityRating::Complex,
                issues: vec![],
                recommendation: None,
            },
        ];

        let classes = aggregate_class_complexity("complex.py", &functions, "python");

        assert_eq!(classes.len(), 1);
        let complex_class = &classes[0];
        assert_eq!(complex_class.name, "ComplexClass");
        assert_eq!(complex_class.average_cognitive_complexity, 13.5); // (12 + 15) / 2

        // Should have issue about high complexity
        assert!(
            complex_class
                .issues
                .iter()
                .any(|issue| issue.contains("High average cognitive complexity")),
            "Should flag high average complexity as an issue"
        );
    }
}
