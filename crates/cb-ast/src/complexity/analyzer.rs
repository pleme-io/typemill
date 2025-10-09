use super::metrics::{calculate_code_metrics, calculate_complexity_metrics, count_parameters};
use super::models::{ComplexityRating, FunctionComplexity, ComplexityReport};
use std::collections::HashMap;

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
    let needs_attention = rating_counts.get(&ComplexityRating::Complex).unwrap_or(&0)
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