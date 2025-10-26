#![allow(dead_code, unused_variables)]

//! Test analysis handler
//!
//! This module provides detection for test-related patterns including:
//! - Coverage: Test coverage analysis
//! - Quality: Test quality assessment
//! - Assertions: Assertion pattern analysis
//! - Organization: Test organization patterns
//!
//! Uses the shared analysis engine for orchestration and focuses only on
//! detection logic.

use super::super::{ToolHandler, ToolHandlerContext};
use async_trait::async_trait;
use mill_foundation::core::model::mcp::ToolCall;
use mill_foundation::protocol::analysis_result::{
    Finding, FindingLocation, SafetyLevel, Severity, Suggestion,
};
use mill_foundation::protocol::{ApiError as ServerError, ApiResult as ServerResult};
use mill_plugin_api::Symbol;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::debug;

/// Analyze test coverage
///
/// This function calculates test coverage by counting test functions and
/// comparing them to production functions, identifying untested code.
///
/// # Algorithm
/// 1. Count total test functions using language-specific patterns
/// 2. Count production functions from complexity report
/// 3. Calculate test coverage ratio (tests per production function)
/// 4. Identify untested modules or files (heuristic-based)
/// 5. Generate findings with coverage metrics
///
/// # Heuristics
/// - Test function detection: Language-specific patterns
///   - Rust: `#[test]`, `#[tokio::test]`, `fn test_`
///   - TypeScript/JavaScript: `it(`, `test(`, `describe(`
///   - Python: `def test_`, `class Test`, `@pytest.mark`
///   - Go: `func Test`, `func Benchmark`
/// - Coverage threshold: <0.5 High, <0.8 Medium, else Low
/// - For MVP: Regex-based test function detection
///
/// # Future Enhancements
/// TODO: Add AST-based test detection for accuracy
/// TODO: Support workspace-wide test coverage analysis
/// TODO: Detect code coverage via instrumentation
/// TODO: Generate test coverage reports (HTML/JSON)
///
/// # Parameters
/// - `complexity_report`: Used for production function count
/// - `content`: The raw file content for test detection
/// - `symbols`: The parsed symbols to analyze
/// - `language`: The language name for test patterns
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A single finding with:
/// - Metrics including total_tests, total_functions, coverage_ratio, untested_functions
/// - Severity: High if ratio < 0.5, Medium if < 0.8, Low otherwise
/// - Suggestions to add tests for untested functions
pub fn detect_coverage(
    complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    symbols: &[Symbol],
    language: &str,
    file_path: &str,
    _registry: &crate::LanguagePluginRegistry,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Count test functions
    let total_tests = count_test_functions(content, language);

    // Count production functions (from complexity report)
    let total_functions = complexity_report.total_functions;

    // Calculate coverage ratio
    let coverage_ratio = if total_functions > 0 {
        total_tests as f64 / total_functions as f64
    } else {
        0.0
    };

    // Identify untested functions (functions without corresponding tests)
    let untested_functions = identify_untested_functions(complexity_report, content, language);

    // Determine severity
    let severity = if coverage_ratio < 0.5 {
        Severity::High
    } else if coverage_ratio < 0.8 {
        Severity::Medium
    } else {
        Severity::Low
    };

    let mut metrics = HashMap::new();
    metrics.insert("total_tests".to_string(), json!(total_tests));
    metrics.insert("total_functions".to_string(), json!(total_functions));
    metrics.insert("coverage_ratio".to_string(), json!(coverage_ratio));
    metrics.insert(
        "untested_functions_count".to_string(),
        json!(untested_functions.len()),
    );
    metrics.insert("untested_functions".to_string(), json!(untested_functions));

    let message = if coverage_ratio < 0.5 {
        format!(
            "Low test coverage: {:.1}% ({} tests for {} functions), {} functions lack tests",
            coverage_ratio * 100.0,
            total_tests,
            total_functions,
            untested_functions.len()
        )
    } else if coverage_ratio < 0.8 {
        format!(
            "Moderate test coverage: {:.1}% ({} tests for {} functions), {} functions lack tests",
            coverage_ratio * 100.0,
            total_tests,
            total_functions,
            untested_functions.len()
        )
    } else {
        format!(
            "Good test coverage: {:.1}% ({} tests for {} functions)",
            coverage_ratio * 100.0,
            total_tests,
            total_functions
        )
    };

    findings.push(Finding {
        id: format!("test-coverage-{}", file_path),
        kind: "coverage".to_string(),
        severity,
        location: FindingLocation {
            file_path: file_path.to_string(),
            range: None, // File-level finding
            symbol: None,
            symbol_kind: Some("module".to_string()),
        },
        metrics: Some(metrics),
        message,
        suggestions: if !untested_functions.is_empty() {
            vec![Suggestion {
                action: "add_tests".to_string(),
                description: format!(
                    "Add tests for {} untested functions: {}",
                    untested_functions.len(),
                    untested_functions
                        .iter()
                        .take(5)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                target: None,
                estimated_impact: format!(
                    "Would increase coverage from {:.1}% to potentially 100%",
                    coverage_ratio * 100.0
                ),
                safety: SafetyLevel::Safe,
                confidence: 0.90,
                reversible: true,
                refactor_call: None,
            }]
        } else {
            vec![]
        },
    });

    findings
}

/// Assess test quality
///
/// This function analyzes the quality of existing tests by detecting test smells,
/// checking naming patterns, and identifying slow tests.
///
/// # Algorithm
/// 1. Detect test smells: empty tests, single assertion tests, try-catch all
/// 2. Check for test naming patterns (should/test/it + descriptive name)
/// 3. Detect slow tests (> 1s execution time - heuristic from comments)
/// 4. Check for test data quality (hardcoded values vs fixtures)
/// 5. Generate findings with quality metrics
///
/// # Heuristics
/// - Test smells:
///   - Empty tests: No assertions or expect statements
///   - Single assertion: Only one assert/expect
///   - Try-catch all: Broad exception handling
/// - Naming patterns: Descriptive names with context
/// - Slow tests: Heuristic based on comments or TODO markers
/// - For MVP: Pattern-based detection
///
/// # Future Enhancements
/// TODO: Add AST-based test smell detection
/// TODO: Integrate with test execution metrics for actual timing
/// TODO: Detect flaky tests (non-deterministic)
/// TODO: Analyze test isolation and independence
///
/// # Parameters
/// - `complexity_report`: Used for test complexity metrics
/// - `content`: The raw file content for test analysis
/// - `symbols`: The parsed symbols for test identification
/// - `language`: The language name for test patterns
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A vector of findings with:
/// - Metrics including test_smells_count, naming_violations, slow_tests_estimated
/// - Severity: Medium if test smells found
/// - Suggestions to refactor test smells, improve test naming
pub fn detect_quality(
    complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    symbols: &[Symbol],
    language: &str,
    file_path: &str,
    _registry: &crate::LanguagePluginRegistry,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Extract test functions
    let test_functions = extract_test_functions(content, language);

    if test_functions.is_empty() {
        return findings;
    }

    let mut test_smells_count = 0;
    let mut naming_violations = 0;
    let mut slow_tests_estimated = 0;
    let mut test_smells: Vec<TestSmell> = Vec::new();

    for test_fn in &test_functions {
        // Check for test smells
        let smells = detect_test_smells(&test_fn.body, language);
        if !smells.is_empty() {
            test_smells_count += smells.len();
            for smell in smells {
                test_smells.push(TestSmell {
                    test_name: test_fn.name.clone(),
                    smell_type: smell,
                    line: test_fn.line,
                });
            }
        }

        // Check naming pattern
        if !has_descriptive_test_name(&test_fn.name, language) {
            naming_violations += 1;
        }

        // Check for slow test indicators
        if has_slow_test_indicators(&test_fn.body) {
            slow_tests_estimated += 1;
        }
    }

    if test_smells_count == 0 && naming_violations == 0 && slow_tests_estimated == 0 {
        // No quality issues
        return findings;
    }

    let severity = Severity::Medium;

    let mut metrics = HashMap::new();
    metrics.insert("test_smells_count".to_string(), json!(test_smells_count));
    metrics.insert("naming_violations".to_string(), json!(naming_violations));
    metrics.insert(
        "slow_tests_estimated".to_string(),
        json!(slow_tests_estimated),
    );
    metrics.insert("total_tests".to_string(), json!(test_functions.len()));
    metrics.insert(
        "test_smells".to_string(),
        json!(test_smells
            .iter()
            .map(|s| json!({
                "test": s.test_name,
                "smell": s.smell_type,
                "line": s.line,
            }))
            .collect::<Vec<_>>()),
    );

    let message = format!(
        "Test quality issues: {} test smells, {} naming violations, {} estimated slow tests out of {} tests",
        test_smells_count, naming_violations, slow_tests_estimated, test_functions.len()
    );

    let mut suggestions = Vec::new();
    if test_smells_count > 0 {
        suggestions.push(Suggestion {
            action: "refactor_test_smells".to_string(),
            description: format!(
                "Refactor {} test smells to improve test reliability",
                test_smells_count
            ),
            target: None,
            estimated_impact: "Improves test maintainability and reliability".to_string(),
            safety: SafetyLevel::Safe,
            confidence: 0.85,
            reversible: true,
            refactor_call: None,
        });
    }

    if naming_violations > 0 {
        suggestions.push(Suggestion {
            action: "improve_test_naming".to_string(),
            description: format!(
                "Improve naming for {} tests to be more descriptive",
                naming_violations
            ),
            target: None,
            estimated_impact: "Improves test readability and documentation".to_string(),
            safety: SafetyLevel::Safe,
            confidence: 0.90,
            reversible: true,
            refactor_call: None,
        });
    }

    if slow_tests_estimated > 0 {
        suggestions.push(Suggestion {
            action: "optimize_slow_tests".to_string(),
            description: format!("Optimize {} potentially slow tests", slow_tests_estimated),
            target: None,
            estimated_impact: "Reduces test execution time".to_string(),
            safety: SafetyLevel::RequiresReview,
            confidence: 0.70,
            reversible: false,
            refactor_call: None,
        });
    }

    findings.push(Finding {
        id: format!("test-quality-{}", file_path),
        kind: "quality".to_string(),
        severity,
        location: FindingLocation {
            file_path: file_path.to_string(),
            range: None,
            symbol: None,
            symbol_kind: Some("module".to_string()),
        },
        metrics: Some(metrics),
        message,
        suggestions,
    });

    findings
}

/// Analyze assertion patterns
///
/// This function analyzes assertions in tests to detect missing assertions,
/// overly complex tests, and assertion pattern issues.
///
/// # Algorithm
/// 1. Count assertions per test (average, min, max)
/// 2. Detect missing assertions (tests with no assert/expect/should)
/// 3. Detect assertion types (equality, truthiness, exceptions, etc.)
/// 4. Flag overly complex tests (> 10 assertions)
/// 5. Generate findings with assertion metrics
///
/// # Heuristics
/// - Assertion detection: Language-specific patterns
///   - Rust: `assert!`, `assert_eq!`, `assert_ne!`
///   - TypeScript/JavaScript: `expect(`, `assert(`, `should.`
///   - Python: `assert `, `self.assert`
///   - Go: `t.Fatal`, `t.Error`, `assert.`
/// - Complex test threshold: > 10 assertions
/// - For MVP: Regex-based assertion counting
///
/// # Future Enhancements
/// TODO: Add AST-based assertion extraction
/// TODO: Detect assertion quality (specific vs generic)
/// TODO: Detect redundant assertions
/// TODO: Suggest assertion improvements
///
/// # Parameters
/// - `complexity_report`: Not used for assertion analysis
/// - `content`: The raw file content for assertion detection
/// - `symbols`: The parsed symbols for test identification
/// - `language`: The language name for assertion patterns
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A vector of findings with:
/// - Metrics including avg_assertions_per_test, tests_without_assertions, assertion_types
/// - Severity: Medium if tests lack assertions or have too many
/// - Suggestions to add assertions, split complex tests
pub fn detect_assertions(
    _complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    symbols: &[Symbol],
    language: &str,
    file_path: &str,
    _registry: &crate::LanguagePluginRegistry,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Extract test functions
    let test_functions = extract_test_functions(content, language);

    if test_functions.is_empty() {
        return findings;
    }

    let mut total_assertions = 0;
    let mut tests_without_assertions: Vec<String> = Vec::new();
    let mut overly_complex_tests: Vec<String> = Vec::new();
    let mut assertion_type_counts: HashMap<String, usize> = HashMap::new();
    let mut min_assertions = usize::MAX;
    let mut max_assertions = 0;

    for test_fn in &test_functions {
        let assertion_info = count_assertions(&test_fn.body, language);
        let count = assertion_info.total;

        total_assertions += count;

        if count == 0 {
            tests_without_assertions.push(test_fn.name.clone());
        } else {
            min_assertions = min_assertions.min(count);
            max_assertions = max_assertions.max(count);
        }

        if count > 10 {
            overly_complex_tests.push(test_fn.name.clone());
        }

        // Count assertion types
        for (assertion_type, type_count) in assertion_info.types {
            *assertion_type_counts.entry(assertion_type).or_insert(0) += type_count;
        }
    }

    let avg_assertions = if !test_functions.is_empty() {
        total_assertions as f64 / test_functions.len() as f64
    } else {
        0.0
    };

    if min_assertions == usize::MAX {
        min_assertions = 0;
    }

    let has_issues = !tests_without_assertions.is_empty() || !overly_complex_tests.is_empty();

    let severity = if has_issues {
        Severity::Medium
    } else {
        Severity::Low
    };

    let mut metrics = HashMap::new();
    metrics.insert("avg_assertions_per_test".to_string(), json!(avg_assertions));
    metrics.insert("min_assertions".to_string(), json!(min_assertions));
    metrics.insert("max_assertions".to_string(), json!(max_assertions));
    metrics.insert(
        "tests_without_assertions".to_string(),
        json!(tests_without_assertions),
    );
    metrics.insert(
        "overly_complex_tests".to_string(),
        json!(overly_complex_tests),
    );
    metrics.insert("assertion_types".to_string(), json!(assertion_type_counts));
    metrics.insert("total_tests".to_string(), json!(test_functions.len()));

    let message = if !tests_without_assertions.is_empty() {
        format!(
            "Assertion issues: {} tests without assertions, {} overly complex tests (avg: {:.1} assertions/test)",
            tests_without_assertions.len(),
            overly_complex_tests.len(),
            avg_assertions
        )
    } else if !overly_complex_tests.is_empty() {
        format!(
            "Assertion complexity: {} tests with > 10 assertions (avg: {:.1} assertions/test)",
            overly_complex_tests.len(),
            avg_assertions
        )
    } else {
        format!(
            "Assertion analysis: avg {:.1} assertions/test across {} tests",
            avg_assertions,
            test_functions.len()
        )
    };

    let mut suggestions = Vec::new();
    if !tests_without_assertions.is_empty() {
        suggestions.push(Suggestion {
            action: "add_assertions".to_string(),
            description: format!(
                "Add assertions to {} tests: {}",
                tests_without_assertions.len(),
                tests_without_assertions
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            target: None,
            estimated_impact: "Ensures tests actually verify behavior".to_string(),
            safety: SafetyLevel::Safe,
            confidence: 0.95,
            reversible: true,
            refactor_call: None,
        });
    }

    if !overly_complex_tests.is_empty() {
        suggestions.push(Suggestion {
            action: "split_complex_tests".to_string(),
            description: format!(
                "Split {} complex tests into smaller, focused tests",
                overly_complex_tests.len()
            ),
            target: None,
            estimated_impact: "Improves test clarity and maintainability".to_string(),
            safety: SafetyLevel::Safe,
            confidence: 0.80,
            reversible: true,
            refactor_call: None,
        });
    }

    findings.push(Finding {
        id: format!("test-assertions-{}", file_path),
        kind: "assertions".to_string(),
        severity,
        location: FindingLocation {
            file_path: file_path.to_string(),
            range: None,
            symbol: None,
            symbol_kind: Some("module".to_string()),
        },
        metrics: Some(metrics),
        message,
        suggestions,
    });

    findings
}

/// Check test organization
///
/// This function analyzes test file patterns, directory structure, and
/// organization to detect orphaned tests and missing test files.
///
/// # Algorithm
/// 1. Detect test file patterns (_test.rs, .test.ts, test_*.py)
/// 2. Check for test directory structure (tests/, __tests__, spec/)
/// 3. Identify orphaned tests (no corresponding production file)
/// 4. Detect missing test files for production code
/// 5. Check for test suites/describes organization
/// 6. Generate findings with organization metrics
///
/// # Heuristics
/// - Test file patterns:
///   - Rust: `*_test.rs`, `tests/*.rs`
///   - TypeScript/JavaScript: `*.test.ts`, `*.spec.ts`, `__tests__/*.ts`
///   - Python: `test_*.py`, `*_test.py`
///   - Go: `*_test.go`
/// - Organization score based on structure adherence
/// - For MVP: File path and pattern matching
///
/// # Future Enhancements
/// TODO: Add workspace-wide test organization analysis
/// TODO: Detect test suite patterns and hierarchy
/// TODO: Calculate test organization complexity
/// TODO: Suggest test file restructuring
///
/// # Parameters
/// - `complexity_report`: Used for production code metrics
/// - `content`: The raw file content for organization analysis
/// - `symbols`: The parsed symbols for test suite detection
/// - `language`: The language name for file patterns
/// - `file_path`: The path to the file being analyzed
///
/// # Returns
/// A vector of findings with:
/// - Metrics including test_files_count, orphaned_tests, missing_test_files, organization_score
/// - Severity: Medium if poor organization
/// - Suggestions to organize tests, add missing test files
pub fn detect_organization(
    complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    symbols: &[Symbol],
    language: &str,
    file_path: &str,
    _registry: &crate::LanguagePluginRegistry,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Check if this is a test file
    let is_test_file = is_test_file_pattern(file_path, language);

    // Check for test suites/describes
    let test_suites = detect_test_suites(content, language);

    // Calculate organization score
    let organization_score = calculate_organization_score(
        file_path,
        language,
        is_test_file,
        test_suites.len(),
        complexity_report.total_functions,
    );

    // Determine if there are organization issues
    let has_issues = organization_score < 0.7;

    let severity = if has_issues {
        Severity::Medium
    } else {
        Severity::Low
    };

    // Determine production file correspondence
    let (has_corresponding_file, corresponding_file) =
        find_corresponding_file(file_path, language, is_test_file);

    let mut metrics = HashMap::new();
    metrics.insert("is_test_file".to_string(), json!(is_test_file));
    metrics.insert("test_suites_count".to_string(), json!(test_suites.len()));
    metrics.insert("test_suites".to_string(), json!(test_suites));
    metrics.insert("organization_score".to_string(), json!(organization_score));
    metrics.insert(
        "has_corresponding_file".to_string(),
        json!(has_corresponding_file),
    );
    if let Some(ref file) = corresponding_file {
        metrics.insert("corresponding_file".to_string(), json!(file));
    }
    metrics.insert(
        "total_functions".to_string(),
        json!(complexity_report.total_functions),
    );

    let message = if is_test_file {
        if !has_corresponding_file {
            format!(
                "Test file organization: Orphaned test file with no corresponding production file (score: {:.1}%)",
                organization_score * 100.0
            )
        } else if test_suites.is_empty() && complexity_report.total_functions > 5 {
            format!(
                "Test file organization: {} test functions but no test suites/describes for organization (score: {:.1}%)",
                complexity_report.total_functions,
                organization_score * 100.0
            )
        } else {
            format!(
                "Test file organization: {} test suites organizing {} functions (score: {:.1}%)",
                test_suites.len(),
                complexity_report.total_functions,
                organization_score * 100.0
            )
        }
    } else if !has_corresponding_file {
        format!(
            "Test file organization: Production file with no corresponding test file (score: {:.1}%)",
            organization_score * 100.0
        )
    } else {
        format!(
            "Test file organization: Production file with tests (score: {:.1}%)",
            organization_score * 100.0
        )
    };

    let mut suggestions = Vec::new();
    if is_test_file && !has_corresponding_file {
        suggestions.push(Suggestion {
            action: "link_or_remove_orphaned_test".to_string(),
            description:
                "Either create the corresponding production file or remove this orphaned test"
                    .to_string(),
            target: None,
            estimated_impact: "Improves test organization and maintainability".to_string(),
            safety: SafetyLevel::RequiresReview,
            confidence: 0.75,
            reversible: true,
            refactor_call: None,
        });
    } else if !is_test_file && !has_corresponding_file {
        suggestions.push(Suggestion {
            action: "create_test_file".to_string(),
            description: format!(
                "Create test file {} for this production code",
                corresponding_file.unwrap_or_else(|| "test file".to_string())
            ),
            target: None,
            estimated_impact: "Increases test coverage and code quality".to_string(),
            safety: SafetyLevel::Safe,
            confidence: 0.85,
            reversible: true,
            refactor_call: None,
        });
    }

    if is_test_file && test_suites.is_empty() && complexity_report.total_functions > 5 {
        suggestions.push(Suggestion {
            action: "organize_with_test_suites".to_string(),
            description: "Organize tests using describe/test suites for better structure"
                .to_string(),
            target: None,
            estimated_impact: "Improves test readability and organization".to_string(),
            safety: SafetyLevel::Safe,
            confidence: 0.80,
            reversible: true,
            refactor_call: None,
        });
    }

    findings.push(Finding {
        id: format!("test-organization-{}", file_path),
        kind: "organization".to_string(),
        severity,
        location: FindingLocation {
            file_path: file_path.to_string(),
            range: None,
            symbol: None,
            symbol_kind: Some("module".to_string()),
        },
        metrics: Some(metrics),
        message,
        suggestions,
    });

    findings
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Helper struct for test function information
#[derive(Debug, Clone)]
struct TestFunction {
    name: String,
    line: usize,
    body: String,
}

/// Helper struct for test smell tracking
#[derive(Debug, Clone)]
struct TestSmell {
    test_name: String,
    smell_type: String,
    line: usize,
}

/// Helper struct for assertion information
#[derive(Debug)]
struct AssertionInfo {
    total: usize,
    types: HashMap<String, usize>,
}

/// Count test functions in content
///
/// # Parameters
/// - `content`: The file content
/// - `language`: The language name
///
/// # Returns
/// The number of test functions found
fn count_test_functions(content: &str, language: &str) -> usize {
    let patterns = get_test_patterns(language);
    let mut count = 0;

    for line in content.lines() {
        for pattern in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                if re.is_match(line) {
                    count += 1;
                    break; // Count each line once
                }
            }
        }
    }

    count
}

/// Get language-specific test function patterns
///
/// # Parameters
/// - `language`: The language name
///
/// # Returns
/// A vector of regex patterns for test detection
fn get_test_patterns(language: &str) -> Vec<&'static str> {
    match language.to_lowercase().as_str() {
        "rust" => vec![r"#\[test\]", r"#\[tokio::test\]", r"fn test_"],
        "typescript" | "javascript" => vec![r"\bit\(", r"\btest\(", r"\bdescribe\("],
        "python" => vec![r"def test_", r"class Test", r"@pytest\.mark"],
        "go" => vec![r"func Test", r"func Benchmark"],
        _ => vec![r"fn test_", r"\btest\("],
    }
}

/// Identify untested functions
///
/// # Parameters
/// - `complexity_report`: The complexity report with function names
/// - `content`: The file content to search for tests
/// - `language`: The language name
///
/// # Returns
/// A vector of function names that lack tests
fn identify_untested_functions(
    complexity_report: &mill_ast::complexity::ComplexityReport,
    content: &str,
    language: &str,
) -> Vec<String> {
    let mut untested = Vec::new();

    for func in &complexity_report.functions {
        // Heuristic: Check if there's a test function with this name
        if !has_test_for_function(&func.name, content, language) {
            untested.push(func.name.clone());
        }
    }

    untested
}

/// Check if a function has a corresponding test
///
/// # Parameters
/// - `function_name`: The name of the function
/// - `content`: The file content
/// - `language`: The language name
///
/// # Returns
/// True if a test is found for the function
fn has_test_for_function(function_name: &str, content: &str, language: &str) -> bool {
    let test_name_patterns = match language.to_lowercase().as_str() {
        "rust" => vec![
            format!("test_{}", function_name),
            format!("{}_test", function_name),
        ],
        "typescript" | "javascript" => vec![
            format!("'{}'", function_name),
            format!("\"{}\"", function_name),
            function_name.to_string(),
        ],
        "python" => vec![
            format!("test_{}", function_name),
            format!("{}_test", function_name),
        ],
        "go" => vec![
            format!("Test{}", function_name),
            format!("TestNew{}", function_name),
        ],
        _ => vec![format!("test_{}", function_name)],
    };

    for pattern in test_name_patterns {
        if content.contains(&pattern) {
            return true;
        }
    }

    false
}

/// Extract test functions from content
///
/// # Parameters
/// - `content`: The file content
/// - `language`: The language name
///
/// # Returns
/// A vector of TestFunction structs
fn extract_test_functions(content: &str, language: &str) -> Vec<TestFunction> {
    let mut test_functions = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    let patterns = get_test_patterns(language);

    for (idx, line) in lines.iter().enumerate() {
        for pattern in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                if re.is_match(line) {
                    // Extract function name
                    let name = extract_test_name(line, language);

                    // Extract function body (simplified: next 20 lines)
                    let body_end = (idx + 20).min(lines.len());
                    let body = lines[idx..body_end].join("\n");

                    test_functions.push(TestFunction {
                        name,
                        line: idx + 1,
                        body,
                    });
                    break;
                }
            }
        }
    }

    test_functions
}

/// Extract test name from a line
///
/// # Parameters
/// - `line`: The line containing the test definition
/// - `language`: The language name
///
/// # Returns
/// The extracted test name
fn extract_test_name(line: &str, language: &str) -> String {
    match language.to_lowercase().as_str() {
        "rust" => {
            // Extract from: fn test_name() or #[test]
            if let Some(start) = line.find("fn ") {
                if let Some(end) = line[start + 3..].find('(') {
                    return line[start + 3..start + 3 + end].trim().to_string();
                }
            }
        }
        "typescript" | "javascript" => {
            // Extract from: it("test name", ...) or test("test name", ...)
            if let Some(start) = line.find('"') {
                if let Some(end) = line[start + 1..].find('"') {
                    return line[start + 1..start + 1 + end].to_string();
                }
            }
            if let Some(start) = line.find('\'') {
                if let Some(end) = line[start + 1..].find('\'') {
                    return line[start + 1..start + 1 + end].to_string();
                }
            }
        }
        "python" => {
            // Extract from: def test_name(...):
            if let Some(start) = line.find("def ") {
                if let Some(end) = line[start + 4..].find('(') {
                    return line[start + 4..start + 4 + end].trim().to_string();
                }
            }
        }
        _ => {}
    }

    "unnamed_test".to_string()
}

/// Detect test smells in test body
///
/// # Parameters
/// - `body`: The test function body
/// - `language`: The language name
///
/// # Returns
/// A vector of detected test smell types
fn detect_test_smells(body: &str, language: &str) -> Vec<String> {
    let mut smells = Vec::new();

    // Check for empty test
    let has_assertions = has_any_assertion(body, language);
    if !has_assertions {
        smells.push("empty_test".to_string());
    }

    // Check for try-catch all
    if has_broad_exception_handling(body, language) {
        smells.push("try_catch_all".to_string());
    }

    // Check for single assertion (too trivial)
    let assertion_count = count_assertions(body, language).total;
    if assertion_count == 1 {
        smells.push("single_assertion".to_string());
    }

    // Check for hardcoded data
    if has_excessive_hardcoded_values(body) {
        smells.push("hardcoded_test_data".to_string());
    }

    smells
}

/// Check if test has any assertions
///
/// # Parameters
/// - `body`: The test body
/// - `language`: The language name
///
/// # Returns
/// True if assertions are found
fn has_any_assertion(body: &str, language: &str) -> bool {
    let patterns = get_assertion_patterns(language);

    for pattern in patterns {
        if body.contains(pattern) {
            return true;
        }
    }

    false
}

/// Get language-specific assertion patterns
///
/// # Parameters
/// - `language`: The language name
///
/// # Returns
/// A vector of assertion string patterns
fn get_assertion_patterns(language: &str) -> Vec<&'static str> {
    match language.to_lowercase().as_str() {
        "rust" => vec!["assert!", "assert_eq!", "assert_ne!", "panic!"],
        "typescript" | "javascript" => vec!["expect(", "assert(", "should.", "toBe"],
        "python" => vec!["assert ", "self.assert", "assertEqual"],
        "go" => vec!["t.Fatal", "t.Error", "assert.", "require."],
        _ => vec!["assert", "expect"],
    }
}

/// Check for broad exception handling
///
/// # Parameters
/// - `body`: The test body
/// - `language`: The language name
///
/// # Returns
/// True if broad exception handling detected
fn has_broad_exception_handling(body: &str, language: &str) -> bool {
    match language.to_lowercase().as_str() {
        "rust" => body.contains("catch_unwind"),
        "typescript" | "javascript" => body.contains("catch (") && body.contains("catch (e)"),
        "python" => body.contains("except:") || body.contains("except Exception:"),
        "go" => body.contains("recover()"),
        _ => false,
    }
}

/// Check for excessive hardcoded values
///
/// # Parameters
/// - `body`: The test body
///
/// # Returns
/// True if excessive hardcoded values found
fn has_excessive_hardcoded_values(body: &str) -> bool {
    // Simple heuristic: count numeric and string literals
    let mut literal_count = 0;

    // Count numeric literals
    if let Ok(re) = Regex::new(r"\b\d+\b") {
        literal_count += re.find_iter(body).count();
    }

    // Count string literals
    literal_count += body.matches('"').count() / 2;
    literal_count += body.matches('\'').count() / 2;

    literal_count > 10
}

/// Check if test name is descriptive
///
/// # Parameters
/// - `name`: The test name
/// - `language`: The language name
///
/// # Returns
/// True if name is descriptive
fn has_descriptive_test_name(name: &str, language: &str) -> bool {
    // Check length
    if name.len() < 10 {
        return false;
    }

    // Check for common patterns
    match language.to_lowercase().as_str() {
        "rust" => name.starts_with("test_") && name.len() > 15,
        "typescript" | "javascript" => name.contains(" ") && name.len() > 15,
        "python" => name.starts_with("test_") && name.len() > 15,
        "go" => name.starts_with("Test") && name.len() > 10,
        _ => name.len() > 10,
    }
}

/// Check for slow test indicators
///
/// # Parameters
/// - `body`: The test body
///
/// # Returns
/// True if slow test indicators found
fn has_slow_test_indicators(body: &str) -> bool {
    // Look for sleep/delay/timeout patterns
    let slow_patterns = vec![
        "sleep",
        "delay",
        "setTimeout",
        "time.sleep",
        "Thread.sleep",
        "await new Promise",
    ];

    for pattern in slow_patterns {
        if body.contains(pattern) {
            return true;
        }
    }

    false
}

/// Count assertions in test body
///
/// # Parameters
/// - `body`: The test body
/// - `language`: The language name
///
/// # Returns
/// AssertionInfo with total count and types
fn count_assertions(body: &str, language: &str) -> AssertionInfo {
    let mut total = 0;
    let mut types: HashMap<String, usize> = HashMap::new();

    match language.to_lowercase().as_str() {
        "rust" => {
            total += count_pattern(body, "assert!");
            *types.entry("equality".to_string()).or_insert(0) += count_pattern(body, "assert_eq!");
            *types.entry("inequality".to_string()).or_insert(0) +=
                count_pattern(body, "assert_ne!");
            *types.entry("boolean".to_string()).or_insert(0) += count_pattern(body, "assert!");
        }
        "typescript" | "javascript" => {
            total += count_pattern(body, "expect(");
            *types.entry("equality".to_string()).or_insert(0) += count_pattern(body, "toBe(");
            *types.entry("equality".to_string()).or_insert(0) += count_pattern(body, "toEqual(");
            *types.entry("truthiness".to_string()).or_insert(0) +=
                count_pattern(body, "toBeTruthy(");
        }
        "python" => {
            total += count_pattern(body, "assert ");
            *types.entry("equality".to_string()).or_insert(0) += count_pattern(body, "assertEqual");
            *types.entry("truthiness".to_string()).or_insert(0) +=
                count_pattern(body, "assertTrue");
        }
        "go" => {
            total += count_pattern(body, "assert.");
            *types.entry("equality".to_string()).or_insert(0) +=
                count_pattern(body, "assert.Equal");
            *types.entry("error".to_string()).or_insert(0) += count_pattern(body, "t.Error");
        }
        _ => {
            total += count_pattern(body, "assert");
        }
    }

    AssertionInfo { total, types }
}

/// Count pattern occurrences
///
/// # Parameters
/// - `text`: The text to search
/// - `pattern`: The pattern to count
///
/// # Returns
/// The number of occurrences
fn count_pattern(text: &str, pattern: &str) -> usize {
    text.matches(pattern).count()
}

/// Check if file path matches test file patterns
///
/// # Parameters
/// - `file_path`: The file path
/// - `language`: The language name
///
/// # Returns
/// True if this is a test file
fn is_test_file_pattern(file_path: &str, language: &str) -> bool {
    match language.to_lowercase().as_str() {
        "rust" => file_path.ends_with("_test.rs") || file_path.contains("/tests/"),
        "typescript" | "javascript" => {
            file_path.ends_with(".test.ts")
                || file_path.ends_with(".spec.ts")
                || file_path.ends_with(".test.js")
                || file_path.ends_with(".spec.js")
                || file_path.contains("/__tests__/")
        }
        "python" => file_path.starts_with("test_") || file_path.ends_with("_test.py"),
        "go" => file_path.ends_with("_test.go"),
        _ => file_path.contains("test"),
    }
}

/// Detect test suites in content
///
/// # Parameters
/// - `content`: The file content
/// - `language`: The language name
///
/// # Returns
/// A vector of test suite names
fn detect_test_suites(content: &str, language: &str) -> Vec<String> {
    let mut suites = Vec::new();

    match language.to_lowercase().as_str() {
        "typescript" | "javascript" => {
            // Look for describe() calls
            if let Ok(re) = Regex::new(r#"describe\s*\(\s*["']([^"']+)["']"#) {
                for cap in re.captures_iter(content) {
                    if let Some(name) = cap.get(1) {
                        suites.push(name.as_str().to_string());
                    }
                }
            }
        }
        "python" => {
            // Look for test classes
            if let Ok(re) = Regex::new(r"class (Test\w+)") {
                for cap in re.captures_iter(content) {
                    if let Some(name) = cap.get(1) {
                        suites.push(name.as_str().to_string());
                    }
                }
            }
        }
        "rust" => {
            // Look for mod tests
            if let Ok(re) = Regex::new(r"mod (\w+_tests?)") {
                for cap in re.captures_iter(content) {
                    if let Some(name) = cap.get(1) {
                        suites.push(name.as_str().to_string());
                    }
                }
            }
        }
        _ => {}
    }

    suites
}

/// Calculate organization score
///
/// # Parameters
/// - `file_path`: The file path
/// - `language`: The language name
/// - `is_test_file`: Whether this is a test file
/// - `test_suite_count`: Number of test suites
/// - `function_count`: Total function count
///
/// # Returns
/// An organization score from 0.0 to 1.0
fn calculate_organization_score(
    file_path: &str,
    language: &str,
    is_test_file: bool,
    test_suite_count: usize,
    function_count: usize,
) -> f64 {
    let mut score: f64 = 0.5; // Base score

    // Good: test file follows naming convention
    if is_test_file && is_test_file_pattern(file_path, language) {
        score += 0.2;
    }

    // Good: has test suites for organization
    if test_suite_count > 0 && function_count > 5 {
        score += 0.2;
    }

    // Good: appropriate test suite to function ratio
    if test_suite_count > 0 && function_count > 0 {
        let ratio = test_suite_count as f64 / function_count as f64;
        if ratio > 0.1 && ratio < 0.5 {
            score += 0.1;
        }
    }

    score.min(1.0)
}

/// Find corresponding production or test file
///
/// # Parameters
/// - `file_path`: The current file path
/// - `language`: The language name
/// - `is_test_file`: Whether this is a test file
///
/// # Returns
/// A tuple of (exists, path_suggestion)
fn find_corresponding_file(
    file_path: &str,
    language: &str,
    is_test_file: bool,
) -> (bool, Option<String>) {
    // For MVP, we just suggest the path without checking existence
    let corresponding_path = if is_test_file {
        // Suggest production file path
        match language.to_lowercase().as_str() {
            "rust" => Some(file_path.replace("_test.rs", ".rs")),
            "typescript" | "javascript" => Some(
                file_path
                    .replace(".test.ts", ".ts")
                    .replace(".spec.ts", ".ts")
                    .replace(".test.js", ".js")
                    .replace(".spec.js", ".js"),
            ),
            "python" => Some(file_path.replace("test_", "").replace("_test.py", ".py")),
            "go" => Some(file_path.replace("_test.go", ".go")),
            _ => None,
        }
    } else {
        // Suggest test file path
        match language.to_lowercase().as_str() {
            "rust" => Some(file_path.replace(".rs", "_test.rs")),
            "typescript" | "javascript" => Some(file_path.replace(".ts", ".test.ts")),
            "python" => Some(format!("test_{}", file_path)),
            "go" => Some(file_path.replace(".go", "_test.go")),
            _ => None,
        }
    };

    // For MVP, we always return false for exists since we don't check filesystem
    (false, corresponding_path)
}

// ============================================================================
// Handler Implementation
// ============================================================================

pub struct TestsHandler;

impl TestsHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ToolHandler for TestsHandler {
    fn tool_names(&self) -> &[&str] {
        &["analyze.tests"]
    }

    fn is_internal(&self) -> bool {
        false // PUBLIC tool
    }

    async fn handle_tool_call(
        &self,
        context: &ToolHandlerContext,
        tool_call: &ToolCall,
    ) -> ServerResult<Value> {
        let args = tool_call.arguments.clone().unwrap_or(json!({}));

        // Parse kind (required)
        let kind = args
            .get("kind")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::InvalidRequest("Missing 'kind' parameter".into()))?;

        // Validate kind
        if !matches!(kind, "coverage" | "quality" | "assertions" | "organization") {
            return Err(ServerError::InvalidRequest(format!(
                "Unsupported kind '{}'. Supported: 'coverage', 'quality', 'assertions', 'organization'",
                kind
            )));
        }

        debug!(kind = %kind, "Handling analyze.tests request");

        // Dispatch to appropriate analysis function
        match kind {
            "coverage" => {
                super::engine::run_analysis(context, tool_call, "tests", kind, detect_coverage)
                    .await
            }
            "quality" => {
                super::engine::run_analysis(context, tool_call, "tests", kind, detect_quality).await
            }
            "assertions" => {
                super::engine::run_analysis(context, tool_call, "tests", kind, detect_assertions)
                    .await
            }
            "organization" => {
                super::engine::run_analysis(context, tool_call, "tests", kind, detect_organization)
                    .await
            }
            _ => unreachable!("Kind validated earlier"),
        }
    }
}
