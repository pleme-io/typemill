use super::models::{ClassComplexity, ComplexityRating, FunctionComplexity};

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
            function_name
                .rfind('.')
                .map(|dot_pos| function_name[..dot_pos].to_string())
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
        let class_name =
            extract_class_name(&func.name, language).unwrap_or_else(|| "<module>".to_string());
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
        let total_cognitive: u32 = class_functions.iter().map(|f| f.complexity.cognitive).sum();
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
        let line = class_functions.iter().map(|f| f.line).min().unwrap_or(0);

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
