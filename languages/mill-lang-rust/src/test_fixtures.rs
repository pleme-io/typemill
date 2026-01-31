//! Rust test fixtures for integration testing
//!
//! This module defines Rust-equivalent code samples for cross-language
//! testing. Each scenario represents a specific test case (simple function,
//! complex nested logic, lifetimes, generics, macros, traits, etc.) with
//! expected complexity metrics.

use mill_plugin_api::test_fixtures::*;

/// Get all Rust test fixtures
pub fn rust_test_fixtures() -> LanguageTestFixtures {
    LanguageTestFixtures {
        complexity_scenarios: vec![
            // Scenario 1: Simple function (CC=1, Cognitive=0)
            ComplexityFixture {
                scenario_name: "simple_function",
                source_code: "fn simple(x: i32) -> i32 {\n    x + 1\n}\n",
                file_name: "simple.rs",
                expected_cyclomatic_min: 1,
                expected_cyclomatic_max: 1,
                expected_cognitive_min: 0,
                expected_cognitive_max: 1,
                expected_nesting_depth_min: 0,
            },

            // Scenario 2: Moderate complexity (CC=3)
            ComplexityFixture {
                scenario_name: "moderate_complexity",
                source_code: "fn moderate(x: i32) -> i32 {\n    if x > 0 {\n        x * 2\n    } else if x < 0 {\n        x * -1\n    } else {\n        0\n    }\n}\n",
                file_name: "moderate.rs",
                expected_cyclomatic_min: 3,
                expected_cyclomatic_max: 4,
                expected_cognitive_min: 2,
                expected_cognitive_max: 5,
                expected_nesting_depth_min: 1,
            },

            // Scenario 3: High nested complexity (CC=7+)
            ComplexityFixture {
                scenario_name: "high_nested_complexity",
                source_code: r#"fn complex_nested(a: i32, b: i32, c: i32) -> i32 {
    if a > 0 {
        if b > 0 {
            if c > 0 {
                a + b + c
            } else {
                a + b
            }
        } else if c > 0 {
            a + c
        } else {
            a
        }
    } else if b > 0 {
        if c > 0 {
            b + c
        } else {
            b
        }
    } else if c > 0 {
        c
    } else {
        0
    }
}
"#,
                file_name: "complex.rs",
                expected_cyclomatic_min: 7,
                expected_cyclomatic_max: 10,
                expected_cognitive_min: 10,
                expected_cognitive_max: 20,
                expected_nesting_depth_min: 3,
            },

            // Scenario 4: Flat with early returns (guard clauses)
            ComplexityFixture {
                scenario_name: "flat_early_returns",
                source_code: r#"fn flat_guards(a: Option<i32>, b: Option<i32>, c: Option<i32>) -> bool {
    if a.is_none() {
        return false;
    }
    if b.is_none() {
        return false;
    }
    if c.is_none() {
        return false;
    }
    true
}
"#,
                file_name: "flat.rs",
                expected_cyclomatic_min: 4,
                expected_cyclomatic_max: 5,
                expected_cognitive_min: 3,
                expected_cognitive_max: 8,
                expected_nesting_depth_min: 1,
            },

            // Scenario 5: Lifetimes - Rust-specific complexity
            ComplexityFixture {
                scenario_name: "lifetime_complexity",
                source_code: r#"fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() {
        x
    } else {
        y
    }
}
"#,
                file_name: "lifetimes.rs",
                expected_cyclomatic_min: 2,
                expected_cyclomatic_max: 2,
                expected_cognitive_min: 1,
                expected_cognitive_max: 3,
                expected_nesting_depth_min: 1,
            },

            // Scenario 6: Generic function with trait bounds
            ComplexityFixture {
                scenario_name: "generic_with_trait_bounds",
                source_code: r#"fn process_items<T, U>(items: &[T], transformer: impl Fn(&T) -> U) -> Vec<U>
where
    T: Clone + std::fmt::Debug,
    U: Default,
{
    let mut results = Vec::new();
    for item in items {
        if item.clone().to_string().is_empty() {
            results.push(U::default());
        } else {
            results.push(transformer(item));
        }
    }
    results
}
"#,
                file_name: "generics.rs",
                expected_cyclomatic_min: 2,
                expected_cyclomatic_max: 3,
                expected_cognitive_min: 2,
                expected_cognitive_max: 5,
                expected_nesting_depth_min: 2,
            },

            // Scenario 7: Match expression with multiple arms
            ComplexityFixture {
                scenario_name: "match_expression",
                source_code: r#"enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(String),
    ChangeColor(i32, i32, i32),
}

fn handle_message(msg: Message) -> String {
    match msg {
        Message::Quit => "Quit".to_string(),
        Message::Move { x, y } => format!("Move to ({}, {})", x, y),
        Message::Write(text) => text,
        Message::ChangeColor(r, g, b) => format!("Color: {}, {}, {}", r, g, b),
    }
}
"#,
                file_name: "match_expr.rs",
                expected_cyclomatic_min: 4,
                expected_cyclomatic_max: 5,
                expected_cognitive_min: 1,
                expected_cognitive_max: 5,
                expected_nesting_depth_min: 1,
            },

            // Scenario 8: Nested match with guards
            ComplexityFixture {
                scenario_name: "nested_match_with_guards",
                source_code: r#"fn complex_match(value: Option<Result<i32, &str>>) -> i32 {
    match value {
        Some(Ok(n)) if n > 0 => n * 2,
        Some(Ok(n)) if n < 0 => n * -1,
        Some(Ok(_)) => 0,
        Some(Err(_)) => -1,
        None => -2,
    }
}
"#,
                file_name: "nested_match.rs",
                expected_cyclomatic_min: 5,
                expected_cyclomatic_max: 7,
                expected_cognitive_min: 3,
                expected_cognitive_max: 8,
                expected_nesting_depth_min: 1,
            },

            // Scenario 9: Trait implementation with multiple methods
            ComplexityFixture {
                scenario_name: "trait_implementation",
                source_code: r#"trait Processor {
    fn process(&self, input: i32) -> i32;
    fn validate(&self, input: i32) -> bool;
}

struct Calculator {
    multiplier: i32,
}

impl Processor for Calculator {
    fn process(&self, input: i32) -> i32 {
        if input > 0 {
            input * self.multiplier
        } else {
            0
        }
    }

    fn validate(&self, input: i32) -> bool {
        input >= 0 && input <= 100
    }
}
"#,
                file_name: "trait_impl.rs",
                expected_cyclomatic_min: 3,
                expected_cyclomatic_max: 5,
                expected_cognitive_min: 2,
                expected_cognitive_max: 6,
                expected_nesting_depth_min: 1,
            },

            // Scenario 10: Macro-heavy code (declarative macro)
            ComplexityFixture {
                scenario_name: "macro_usage",
                source_code: r#"macro_rules! create_function {
    ($func_name:ident) => {
        fn $func_name() {
            println!("Function {:?} was called", stringify!($func_name));
        }
    };
}

create_function!(foo);
create_function!(bar);

fn use_macros() {
    foo();
    bar();
}
"#,
                file_name: "macros.rs",
                expected_cyclomatic_min: 1,
                expected_cyclomatic_max: 3,
                expected_cognitive_min: 0,
                expected_cognitive_max: 3,
                expected_nesting_depth_min: 0,
            },

            // Scenario 11: Iterator chains with closures
            ComplexityFixture {
                scenario_name: "iterator_chain",
                source_code: r#"fn process_numbers(numbers: Vec<i32>) -> Vec<i32> {
    numbers
        .into_iter()
        .filter(|n| *n > 0)
        .map(|n| n * 2)
        .filter(|n| *n < 100)
        .collect()
}
"#,
                file_name: "iterators.rs",
                expected_cyclomatic_min: 1,
                expected_cyclomatic_max: 4,
                expected_cognitive_min: 0,
                expected_cognitive_max: 4,
                expected_nesting_depth_min: 0,
            },

            // Scenario 12: Result/Option chaining with ? operator
            ComplexityFixture {
                scenario_name: "error_handling_chain",
                source_code: r#"use std::fs::File;
use std::io::{self, Read};

fn read_config(path: &str) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}
"#,
                file_name: "error_chain.rs",
                expected_cyclomatic_min: 1,
                expected_cyclomatic_max: 3,
                expected_cognitive_min: 0,
                expected_cognitive_max: 3,
                expected_nesting_depth_min: 0,
            },

            // Scenario 13: Async function with complex control flow
            ComplexityFixture {
                scenario_name: "async_complexity",
                source_code: r#"async fn fetch_data(url: &str, retries: u32) -> Result<String, String> {
    let mut attempts = 0;
    loop {
        attempts += 1;
        if attempts > retries {
            return Err("Max retries exceeded".to_string());
        }
        // Simulated async fetch
        if url.starts_with("https") {
            return Ok(format!("Data from {}", url));
        } else if url.starts_with("http") {
            continue;
        } else {
            return Err("Invalid URL scheme".to_string());
        }
    }
}
"#,
                file_name: "async_fn.rs",
                expected_cyclomatic_min: 4,
                expected_cyclomatic_max: 6,
                expected_cognitive_min: 5,
                expected_cognitive_max: 10,
                expected_nesting_depth_min: 2,
            },

            // Scenario 14: Multiple lifetime parameters
            ComplexityFixture {
                scenario_name: "multiple_lifetimes",
                source_code: r#"struct Context<'a, 'b> {
    first: &'a str,
    second: &'b str,
}

fn complex_lifetimes<'a, 'b>(ctx: &Context<'a, 'b>) -> &'a str
where
    'b: 'a,
{
    if ctx.first.len() > ctx.second.len() {
        ctx.first
    } else {
        ctx.first // Still returns 'a lifetime
    }
}
"#,
                file_name: "multi_lifetime.rs",
                expected_cyclomatic_min: 2,
                expected_cyclomatic_max: 2,
                expected_cognitive_min: 1,
                expected_cognitive_max: 3,
                expected_nesting_depth_min: 1,
            },
        ],

        refactoring_scenarios: vec![
            // Extract variable
            RefactoringFixture {
                scenario_name: "extract_simple_expression",
                source_code: "fn calculate() -> i32 {\n    let result = 10 + 20;\n    result\n}\n",
                file_name: "extract_var.rs",
                operation: RefactoringOperation::ExtractVariable {
                    variable_name: "sum".to_string(),
                    start_line: 1,
                    start_char: 17,
                    end_line: 1,
                    end_char: 23,
                },
            },

            // Extract function - multiline
            RefactoringFixture {
                scenario_name: "extract_multiline_function",
                source_code: r#"fn main() {
    let x = 1;
    let y = 2;
    let result = x + y;
    println!("{}", result);
}
"#,
                file_name: "extract_func.rs",
                operation: RefactoringOperation::ExtractFunction {
                    new_name: "add_numbers".to_string(),
                    start_line: 1,
                    start_char: 4,
                    end_line: 3,
                    end_char: 22,
                },
            },

            // Inline variable
            RefactoringFixture {
                scenario_name: "inline_simple_variable",
                source_code: r#"fn process() -> i32 {
    let multiplier = 2;
    let result = 10 * multiplier;
    result
}
"#,
                file_name: "inline_var.rs",
                operation: RefactoringOperation::InlineVariable {
                    line: 1,
                    character: 8,
                },
            },

            // Extract function with generics
            RefactoringFixture {
                scenario_name: "extract_generic_function",
                source_code: r#"fn process<T: Clone>(items: &[T]) -> Vec<T> {
    let first = items[0].clone();
    let second = items[1].clone();
    vec![first, second]
}
"#,
                file_name: "extract_generic.rs",
                operation: RefactoringOperation::ExtractFunction {
                    new_name: "get_first_two".to_string(),
                    start_line: 1,
                    start_char: 4,
                    end_line: 2,
                    end_char: 34,
                },
            },

            // Extract variable from match arm
            RefactoringFixture {
                scenario_name: "extract_match_arm_expression",
                source_code: r#"fn handle(value: Option<i32>) -> i32 {
    match value {
        Some(n) => n * 2 + 10,
        None => 0,
    }
}
"#,
                file_name: "extract_match.rs",
                operation: RefactoringOperation::ExtractVariable {
                    variable_name: "computed".to_string(),
                    start_line: 2,
                    start_char: 19,
                    end_line: 2,
                    end_char: 29,
                },
            },

            // Inline variable in closure
            RefactoringFixture {
                scenario_name: "inline_variable_in_closure",
                source_code: r#"fn map_values(items: Vec<i32>) -> Vec<i32> {
    let multiplier = 3;
    items.into_iter().map(|x| x * multiplier).collect()
}
"#,
                file_name: "inline_closure.rs",
                operation: RefactoringOperation::InlineVariable {
                    line: 1,
                    character: 8,
                },
            },

            // Extract function from impl block
            RefactoringFixture {
                scenario_name: "extract_from_impl",
                source_code: r#"struct Calculator {
    value: i32,
}

impl Calculator {
    fn compute(&self) -> i32 {
        let doubled = self.value * 2;
        let tripled = doubled + self.value;
        tripled
    }
}
"#,
                file_name: "extract_impl.rs",
                operation: RefactoringOperation::ExtractFunction {
                    new_name: "triple_value".to_string(),
                    start_line: 6,
                    start_char: 8,
                    end_line: 7,
                    end_char: 41,
                },
            },

            // Extract variable from complex expression with Result
            RefactoringFixture {
                scenario_name: "extract_result_expression",
                source_code: r#"fn try_parse(input: &str) -> Result<i32, String> {
    let value = input.parse::<i32>().map_err(|e| e.to_string())?;
    Ok(value * 2)
}
"#,
                file_name: "extract_result.rs",
                operation: RefactoringOperation::ExtractVariable {
                    variable_name: "parsed".to_string(),
                    start_line: 1,
                    start_char: 16,
                    end_line: 1,
                    end_char: 59,
                },
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_fixtures_not_empty() {
        let fixtures = rust_test_fixtures();
        assert!(
            !fixtures.complexity_scenarios.is_empty(),
            "Should have complexity scenarios"
        );
        assert!(
            !fixtures.refactoring_scenarios.is_empty(),
            "Should have refactoring scenarios"
        );
    }

    #[test]
    fn test_all_complexity_fixtures_have_valid_ranges() {
        let fixtures = rust_test_fixtures();
        for scenario in &fixtures.complexity_scenarios {
            assert!(
                scenario.expected_cyclomatic_min <= scenario.expected_cyclomatic_max,
                "Scenario '{}' has invalid cyclomatic range: min={} > max={}",
                scenario.scenario_name,
                scenario.expected_cyclomatic_min,
                scenario.expected_cyclomatic_max
            );
            assert!(
                scenario.expected_cognitive_min <= scenario.expected_cognitive_max,
                "Scenario '{}' has invalid cognitive range: min={} > max={}",
                scenario.scenario_name,
                scenario.expected_cognitive_min,
                scenario.expected_cognitive_max
            );
        }
    }

    #[test]
    fn test_all_rust_files_have_rs_extension() {
        let fixtures = rust_test_fixtures();
        for scenario in &fixtures.complexity_scenarios {
            assert!(
                scenario.file_name.ends_with(".rs"),
                "Scenario '{}' has non-Rust file name: {}",
                scenario.scenario_name,
                scenario.file_name
            );
        }
        for scenario in &fixtures.refactoring_scenarios {
            assert!(
                scenario.file_name.ends_with(".rs"),
                "Scenario '{}' has non-Rust file name: {}",
                scenario.scenario_name,
                scenario.file_name
            );
        }
    }

    #[test]
    fn test_rust_specific_scenarios_exist() {
        let fixtures = rust_test_fixtures();
        let scenario_names: Vec<&str> = fixtures
            .complexity_scenarios
            .iter()
            .map(|s| s.scenario_name)
            .collect();

        // Verify Rust-specific scenarios exist
        assert!(
            scenario_names.contains(&"lifetime_complexity"),
            "Should have lifetime complexity scenario"
        );
        assert!(
            scenario_names.contains(&"generic_with_trait_bounds"),
            "Should have generic with trait bounds scenario"
        );
        assert!(
            scenario_names.contains(&"match_expression"),
            "Should have match expression scenario"
        );
        assert!(
            scenario_names.contains(&"trait_implementation"),
            "Should have trait implementation scenario"
        );
        assert!(
            scenario_names.contains(&"macro_usage"),
            "Should have macro usage scenario"
        );
        assert!(
            scenario_names.contains(&"async_complexity"),
            "Should have async complexity scenario"
        );
    }

    #[test]
    fn test_source_code_is_valid_rust() {
        let fixtures = rust_test_fixtures();
        for scenario in &fixtures.complexity_scenarios {
            // Attempt to parse each source code snippet as valid Rust
            let result = syn::parse_file(scenario.source_code);
            assert!(
                result.is_ok(),
                "Scenario '{}' has invalid Rust code: {:?}\nSource:\n{}",
                scenario.scenario_name,
                result.err(),
                scenario.source_code
            );
        }
    }

    #[test]
    fn test_refactoring_operations_have_valid_positions() {
        let fixtures = rust_test_fixtures();
        for scenario in &fixtures.refactoring_scenarios {
            match &scenario.operation {
                RefactoringOperation::ExtractFunction {
                    start_line,
                    end_line,
                    ..
                } => {
                    assert!(
                        start_line <= end_line,
                        "Scenario '{}' has invalid line range: start={} > end={}",
                        scenario.scenario_name,
                        start_line,
                        end_line
                    );
                }
                RefactoringOperation::ExtractVariable {
                    start_line,
                    end_line,
                    ..
                } => {
                    assert!(
                        start_line <= end_line,
                        "Scenario '{}' has invalid line range: start={} > end={}",
                        scenario.scenario_name,
                        start_line,
                        end_line
                    );
                }
                RefactoringOperation::InlineVariable { .. } => {
                    // Single position, no range validation needed
                }
            }
        }
    }
}
