use super::super::complexity::{
    aggregation::*,
    analyzer::*,
    metrics::*,
    models::*,
};

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
    assert_eq!(ComplexityRating::from_score(8), ComplexityRating::Moderate);
    assert_eq!(ComplexityRating::from_score(15), ComplexityRating::Complex);
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
    assert!(
        nested_metrics.cognitive > flat_metrics.cognitive,
        "Nested code should have higher cognitive complexity. Nested: {}, Flat: {}",
        nested_metrics.cognitive,
        flat_metrics.cognitive
    );

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
    assert!(
        metrics.cognitive > metrics.cyclomatic,
        "Cognitive ({}) should be > cyclomatic ({}) for nested code",
        metrics.cognitive,
        metrics.cyclomatic
    );
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
    assert_eq!(
        metrics.cyclomatic, 4,
        "Cyclomatic should be 4 (3 ifs + 1 base)"
    );
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

    assert!(
        good_metrics.comment_ratio > poor_metrics.comment_ratio,
        "Well-documented code should have higher comment ratio"
    );
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

    assert!(
        metrics.comment_lines >= 3,
        "Should detect multi-line comments"
    );
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
    assert!(
        metrics.cognitive > 15,
        "Should detect high cognitive complexity"
    );
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
    assert_eq!(
        metrics.cyclomatic, 5,
        "Cyclomatic: 1 base + for + if + if + and"
    );

    // Note: Python uses indentation, not braces, so max_nesting_depth will be 0
    // Cognitive complexity is still calculated based on keywords
    assert!(
        metrics.cognitive > 0,
        "Should calculate cognitive complexity for Python"
    );
    assert_eq!(
        metrics.max_nesting_depth, 0,
        "Python has no braces, so nesting depth is 0"
    );
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
    assert!(
        metrics.cognitive > 0,
        "Should calculate cognitive complexity"
    );
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
    assert!(
        module.is_some(),
        "Should find <module> for top-level functions"
    );

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