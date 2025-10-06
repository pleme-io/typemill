//! Cross-language refactoring test harness
//!
//! This module provides a parameterized testing framework for refactoring operations
//! across multiple programming languages. It enables writing a single test that runs
//! against equivalent code in Python, TypeScript, Rust, and Go.
//!
//! ## Design Philosophy
//!
//! - **DRY**: One test covers all languages (no duplication)
//! - **Consistency**: All languages tested identically
//! - **Extensibility**: Easy to add new languages or operations
//! - **Clarity**: Clear feature matrix showing which operations are supported per language
//!
//! ## Example Usage
//!
//! ```rust
//! #[tokio::test]
//! async fn test_extract_simple_expression() {
//!     test_refactoring_across_languages(
//!         RefactoringScenario::ExtractVariable("extracted"),
//!         |result| assert!(result.edits.len() >= 2)
//!     ).await;
//! }
//! ```

use serde_json::json;
use std::collections::HashMap;

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Python,
    TypeScript,
    Rust,
    Go,
}

impl Language {
    pub fn all() -> Vec<Language> {
        vec![
            Language::Python,
            Language::TypeScript,
            Language::Rust,
            Language::Go,
        ]
    }

    pub fn file_extension(&self) -> &'static str {
        match self {
            Language::Python => "py",
            Language::TypeScript => "ts",
            Language::Rust => "rs",
            Language::Go => "go",
        }
    }

    pub fn supports_refactoring(&self) -> bool {
        // All languages now have AST-based stub implementations
        // LSP is tried first, with AST fallback for all languages
        true
    }
}

/// Refactoring operations that can be tested
#[derive(Debug, Clone)]
pub enum RefactoringOperation {
    ExtractFunction {
        new_name: String,
        start_line: u32,
        start_char: u32,
        end_line: u32,
        end_char: u32,
    },
    InlineVariable {
        line: u32,
        character: u32,
    },
    ExtractVariable {
        variable_name: String,
        start_line: u32,
        start_char: u32,
        end_line: u32,
        end_char: u32,
    },
}

impl RefactoringOperation {
    pub fn to_mcp_tool_name(&self) -> &'static str {
        match self {
            RefactoringOperation::ExtractFunction { .. } => "extract_function",
            RefactoringOperation::InlineVariable { .. } => "inline_variable",
            RefactoringOperation::ExtractVariable { .. } => "extract_variable",
        }
    }

    pub fn to_json(&self, file_path: &str) -> serde_json::Value {
        match self {
            RefactoringOperation::ExtractFunction {
                new_name,
                start_line,
                start_char,
                end_line,
                end_char,
            } => json!({
                "file_path": file_path,
                "start_line": start_line,
                "end_line": end_line,
                "function_name": new_name
            }),
            RefactoringOperation::InlineVariable { line, character } => json!({
                "file_path": file_path,
                "line": line,
                "character": character
            }),
            RefactoringOperation::ExtractVariable {
                variable_name,
                start_line,
                start_char,
                end_line,
                end_char,
            } => json!({
                "file_path": file_path,
                "start_line": start_line,
                "start_character": start_char,
                "end_line": end_line,
                "end_character": end_char,
                "variable_name": variable_name
            }),
        }
    }
}

/// Language-specific code fixture for a refactoring scenario
#[derive(Debug, Clone)]
pub struct LanguageFixture {
    pub language: Language,
    pub source_code: &'static str,
    pub operation: RefactoringOperation,
}

/// Expected behavior for a refactoring test
#[derive(Debug)]
pub enum ExpectedBehavior {
    /// Operation should succeed
    Success,
    /// Operation not supported for this language
    NotSupported,
    /// Operation supported but expected to fail (e.g., invalid code)
    ExpectedError { message_contains: Option<String> },
}

/// Complete test case for cross-language refactoring
pub struct RefactoringTestCase {
    pub scenario_name: &'static str,
    pub fixtures: Vec<LanguageFixture>,
    pub expected: HashMap<Language, ExpectedBehavior>,
}

impl RefactoringTestCase {
    pub fn new(scenario_name: &'static str) -> Self {
        Self {
            scenario_name,
            fixtures: Vec::new(),
            expected: HashMap::new(),
        }
    }

    pub fn with_fixture(mut self, fixture: LanguageFixture, behavior: ExpectedBehavior) -> Self {
        self.expected.insert(fixture.language, behavior);
        self.fixtures.push(fixture);
        self
    }

    pub fn with_all_languages<F>(mut self, generator: F) -> Self
    where
        F: Fn(Language) -> (LanguageFixture, ExpectedBehavior),
    {
        for lang in Language::all() {
            let (fixture, behavior) = generator(lang);
            self.expected.insert(lang, behavior);
            self.fixtures.push(fixture);
        }
        self
    }
}

/// Predefined refactoring scenarios with language-equivalent fixtures
pub struct RefactoringScenarios;

impl RefactoringScenarios {
    /// Extract a simple arithmetic expression into a variable
    pub fn extract_simple_expression() -> RefactoringTestCase {
        RefactoringTestCase::new("extract_simple_expression").with_all_languages(|lang| {
            let (source, operation, behavior) = match lang {
                Language::Python => (
                    "def calculate():\n    result = 10 + 20\n    return result\n",
                    RefactoringOperation::ExtractVariable {
                        variable_name: "sum".to_string(),
                        start_line: 1,
                        start_char: 13,
                        end_line: 1,
                        end_char: 19,
                    },
                    ExpectedBehavior::Success,
                ),
                Language::TypeScript => (
                    "function calculate() {\n    const result = 10 + 20;\n    return result;\n}\n",
                    RefactoringOperation::ExtractVariable {
                        variable_name: "sum".to_string(),
                        start_line: 1,
                        start_char: 19,
                        end_line: 1,
                        end_char: 25,
                    },
                    ExpectedBehavior::Success,
                ),
                Language::Rust => (
                    "fn calculate() -> i32 {\n    let result = 10 + 20;\n    result\n}\n",
                    RefactoringOperation::ExtractVariable {
                        variable_name: "sum".to_string(),
                        start_line: 1,
                        start_char: 17,
                        end_line: 1,
                        end_char: 23,
                    },
                    // AST fallback stub now exists (will be fully implemented later)
                    ExpectedBehavior::Success
                ),
                Language::Go => (
                    "func calculate() int {\n    result := 10 + 20\n    return result\n}\n",
                    RefactoringOperation::ExtractVariable {
                        variable_name: "sum".to_string(),
                        start_line: 1,
                        start_char: 14,
                        end_line: 1,
                        end_char: 20,
                    },
                    // AST fallback stub now exists (will be fully implemented later)
                    ExpectedBehavior::Success
                ),
            };

            (
                LanguageFixture {
                    language: lang,
                    source_code: source,
                    operation,
                },
                behavior,
            )
        })
    }

    /// Extract multiple lines into a function
    pub fn extract_multiline_function() -> RefactoringTestCase {
        RefactoringTestCase::new("extract_multiline_function").with_all_languages(|lang| {
            let (source, operation, behavior) = match lang {
                Language::Python => (
                    "def main():\n    x = 1\n    y = 2\n    result = x + y\n    print(result)\n",
                    RefactoringOperation::ExtractFunction {
                        new_name: "add_numbers".to_string(),
                        start_line: 1,
                        start_char: 4,
                        end_line: 3,
                        end_char: 18,
                    },
                    ExpectedBehavior::Success,
                ),
                Language::TypeScript => (
                    "function main() {\n    const x = 1;\n    const y = 2;\n    const result = x + y;\n    console.log(result);\n}\n",
                    RefactoringOperation::ExtractFunction {
                        new_name: "addNumbers".to_string(),
                        start_line: 1,
                        start_char: 4,
                        end_line: 3,
                        end_char: 24,
                    },
                    ExpectedBehavior::Success,
                ),
                Language::Rust => (
                    "fn main() {\n    let x = 1;\n    let y = 2;\n    let result = x + y;\n    println!(\"{}\", result);\n}\n",
                    RefactoringOperation::ExtractFunction {
                        new_name: "add_numbers".to_string(),
                        start_line: 1,
                        start_char: 4,
                        end_line: 3,
                        end_char: 22,
                    },
                    // AST fallback stub now exists (will be fully implemented later)
                    ExpectedBehavior::Success
                ),
                Language::Go => (
                    "func main() {\n    x := 1\n    y := 2\n    result := x + y\n    fmt.Println(result)\n}\n",
                    RefactoringOperation::ExtractFunction {
                        new_name: "addNumbers".to_string(),
                        start_line: 1,
                        start_char: 4,
                        end_line: 3,
                        end_char: 19,
                    },
                    // AST fallback stub now exists (will be fully implemented later)
                    ExpectedBehavior::Success
                ),
            };

            (
                LanguageFixture {
                    language: lang,
                    source_code: source,
                    operation,
                },
                behavior,
            )
        })
    }

    /// Inline a simple variable
    pub fn inline_simple_variable() -> RefactoringTestCase {
        RefactoringTestCase::new("inline_simple_variable").with_all_languages(|lang| {
            let (source, operation, behavior) = match lang {
                Language::Python => (
                    "def process():\n    multiplier = 2\n    result = 10 * multiplier\n    return result\n",
                    RefactoringOperation::InlineVariable {
                        line: 1,
                        character: 4,
                    },
                    ExpectedBehavior::Success,
                ),
                Language::TypeScript => (
                    "function process() {\n    const multiplier = 2;\n    const result = 10 * multiplier;\n    return result;\n}\n",
                    RefactoringOperation::InlineVariable {
                        line: 1,
                        character: 10,
                    },
                    // AST fallback stub now exists (will be fully implemented later)
                    ExpectedBehavior::Success,
                ),
                Language::Rust => (
                    "fn process() -> i32 {\n    let multiplier = 2;\n    let result = 10 * multiplier;\n    result\n}\n",
                    RefactoringOperation::InlineVariable {
                        line: 1,
                        character: 8,
                    },
                    // AST fallback stub now exists (will be fully implemented later)
                    ExpectedBehavior::Success
                ),
                Language::Go => (
                    "func process() int {\n    multiplier := 2\n    result := 10 * multiplier\n    return result\n}\n",
                    RefactoringOperation::InlineVariable {
                        line: 1,
                        character: 4,
                    },
                    // AST fallback stub now exists (will be fully implemented later)
                    ExpectedBehavior::Success
                ),
            };

            (
                LanguageFixture {
                    language: lang,
                    source_code: source,
                    operation,
                },
                behavior,
            )
        })
    }

    /// Get all predefined scenarios
    pub fn all() -> Vec<RefactoringTestCase> {
        vec![
            Self::extract_simple_expression(),
            Self::extract_multiline_function(),
            Self::inline_simple_variable(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_extensions() {
        assert_eq!(Language::Python.file_extension(), "py");
        assert_eq!(Language::TypeScript.file_extension(), "ts");
        assert_eq!(Language::Rust.file_extension(), "rs");
        assert_eq!(Language::Go.file_extension(), "go");
    }

    #[test]
    fn test_refactoring_support() {
        // All languages now have AST-based stub implementations
        assert!(Language::Python.supports_refactoring());
        assert!(Language::TypeScript.supports_refactoring());
        assert!(Language::Rust.supports_refactoring());
        assert!(Language::Go.supports_refactoring());
    }

    #[test]
    fn test_scenario_has_all_languages() {
        let scenario = RefactoringScenarios::extract_simple_expression();
        assert_eq!(scenario.fixtures.len(), 4);

        let languages: Vec<Language> = scenario.fixtures.iter().map(|f| f.language).collect();
        assert!(languages.contains(&Language::Python));
        assert!(languages.contains(&Language::TypeScript));
        assert!(languages.contains(&Language::Rust));
        assert!(languages.contains(&Language::Go));
    }

    #[test]
    fn test_all_scenarios_defined() {
        let scenarios = RefactoringScenarios::all();
        assert_eq!(scenarios.len(), 3);
        assert_eq!(scenarios[0].scenario_name, "extract_simple_expression");
        assert_eq!(scenarios[1].scenario_name, "extract_multiline_function");
        assert_eq!(scenarios[2].scenario_name, "inline_simple_variable");
    }
}
