//! Cross-language complexity analysis test harness
//!
//! Provides language-equivalent fixtures for complexity testing across multiple languages.
//! Follows the same pattern as refactoring_harness.rs for consistency.

use std::collections::HashMap;

// Re-use Language enum from refactoring harness
pub use super::refactoring_harness::Language;

/// Expected complexity metrics for a test scenario
#[derive(Debug, Clone)]
pub struct ComplexityExpectation {
    pub min_cyclomatic: u32,
    pub max_cyclomatic: u32,
    pub min_cognitive: u32,
    pub max_cognitive: u32,
    pub min_nesting_depth: u32,
}

/// Language-specific code fixture for complexity testing
#[derive(Debug, Clone)]
pub struct ComplexityFixture {
    pub language: Language,
    pub source_code: &'static str,
    pub file_name: &'static str,
}

/// Complete test case for cross-language complexity analysis
pub struct ComplexityTestCase {
    pub scenario_name: &'static str,
    pub fixtures: Vec<ComplexityFixture>,
    pub expectations: HashMap<Language, ComplexityExpectation>,
}

impl ComplexityTestCase {
    pub fn new(scenario_name: &'static str) -> Self {
        Self {
            scenario_name,
            fixtures: Vec::new(),
            expectations: HashMap::new(),
        }
    }

    pub fn with_all_languages<F>(mut self, generator: F) -> Self
    where
        F: Fn(Language) -> (ComplexityFixture, ComplexityExpectation),
    {
        for lang in Language::all() {
            let (fixture, expectation) = generator(lang);
            self.expectations.insert(lang, expectation);
            self.fixtures.push(fixture);
        }
        self
    }
}

/// Predefined complexity analysis scenarios with language-equivalent fixtures
pub struct ComplexityScenarios;

impl ComplexityScenarios {
    /// Simple function with minimal complexity (CC=1, no decisions)
    pub fn simple_function() -> ComplexityTestCase {
        ComplexityTestCase::new("simple_function").with_all_languages(|lang| {
            let (source, file_name, expectation) = match lang {
                Language::Python => (
                    "def simple(x):\n    return x + 1\n",
                    "simple.py",
                    ComplexityExpectation {
                        min_cyclomatic: 1,
                        max_cyclomatic: 1,
                        min_cognitive: 0,
                        max_cognitive: 1,
                        min_nesting_depth: 0,
                    },
                ),
                Language::TypeScript => (
                    "function simple(x: number): number {\n    return x + 1;\n}\n",
                    "simple.ts",
                    ComplexityExpectation {
                        min_cyclomatic: 1,
                        max_cyclomatic: 1,
                        min_cognitive: 0,
                        max_cognitive: 1,
                        min_nesting_depth: 0,
                    },
                ),
                Language::Rust => (
                    "fn simple(x: i32) -> i32 {\n    x + 1\n}\n",
                    "simple.rs",
                    ComplexityExpectation {
                        min_cyclomatic: 1,
                        max_cyclomatic: 1,
                        min_cognitive: 0,
                        max_cognitive: 1,
                        min_nesting_depth: 0,
                    },
                ),
                Language::Go => (
                    "func simple(x int) int {\n    return x + 1\n}\n",
                    "simple.go",
                    ComplexityExpectation {
                        min_cyclomatic: 1,
                        max_cyclomatic: 1,
                        min_cognitive: 0,
                        max_cognitive: 1,
                        min_nesting_depth: 0,
                    },
                ),
            };

            (
                ComplexityFixture {
                    language: lang,
                    source_code: source,
                    file_name,
                },
                expectation,
            )
        })
    }

    /// Function with if/elif/else logic (CC=3)
    pub fn moderate_complexity() -> ComplexityTestCase {
        ComplexityTestCase::new("moderate_complexity").with_all_languages(|lang| {
            let (source, file_name, expectation) = match lang {
                Language::Python => (
                    "def moderate(x):\n    if x > 0:\n        return x * 2\n    elif x < 0:\n        return x * -1\n    else:\n        return 0\n",
                    "moderate.py",
                    ComplexityExpectation {
                        min_cyclomatic: 3,
                        max_cyclomatic: 4,
                        min_cognitive: 2,
                        max_cognitive: 5,
                        min_nesting_depth: 1,
                    },
                ),
                Language::TypeScript => (
                    "function moderate(x: number): number {\n    if (x > 0) {\n        return x * 2;\n    } else if (x < 0) {\n        return x * -1;\n    } else {\n        return 0;\n    }\n}\n",
                    "moderate.ts",
                    ComplexityExpectation {
                        min_cyclomatic: 3,
                        max_cyclomatic: 4,
                        min_cognitive: 2,
                        max_cognitive: 5,
                        min_nesting_depth: 1,
                    },
                ),
                Language::Rust => (
                    "fn moderate(x: i32) -> i32 {\n    if x > 0 {\n        x * 2\n    } else if x < 0 {\n        x * -1\n    } else {\n        0\n    }\n}\n",
                    "moderate.rs",
                    ComplexityExpectation {
                        min_cyclomatic: 3,
                        max_cyclomatic: 4,
                        min_cognitive: 2,
                        max_cognitive: 5,
                        min_nesting_depth: 1,
                    },
                ),
                Language::Go => (
                    "func moderate(x int) int {\n    if x > 0 {\n        return x * 2\n    } else if x < 0 {\n        return x * -1\n    } else {\n        return 0\n    }\n}\n",
                    "moderate.go",
                    ComplexityExpectation {
                        min_cyclomatic: 3,
                        max_cyclomatic: 4,
                        min_cognitive: 2,
                        max_cognitive: 5,
                        min_nesting_depth: 1,
                    },
                ),
            };

            (
                ComplexityFixture {
                    language: lang,
                    source_code: source,
                    file_name,
                },
                expectation,
            )
        })
    }

    /// Deeply nested if statements (CC=7+, high cognitive load)
    pub fn high_nested_complexity() -> ComplexityTestCase {
        ComplexityTestCase::new("high_nested_complexity").with_all_languages(|lang| {
            let (source, file_name, expectation) = match lang {
                Language::Python => (
                    "def complex_nested(a, b, c):\n    if a > 0:\n        if b > 0:\n            if c > 0:\n                return a + b + c\n            else:\n                return a + b\n        elif c > 0:\n            return a + c\n        else:\n            return a\n    elif b > 0:\n        if c > 0:\n            return b + c\n        else:\n            return b\n    else:\n        return c if c else 0\n",
                    "complex_nested.py",
                    ComplexityExpectation {
                        min_cyclomatic: 7,
                        max_cyclomatic: 10,
                        min_cognitive: 10,
                        max_cognitive: 20,
                        min_nesting_depth: 3,
                    },
                ),
                Language::TypeScript => (
                    "function complexNested(a: number, b: number, c: number): number {\n    if (a > 0) {\n        if (b > 0) {\n            if (c > 0) {\n                return a + b + c;\n            } else {\n                return a + b;\n            }\n        } else if (c > 0) {\n            return a + c;\n        } else {\n            return a;\n        }\n    } else if (b > 0) {\n        if (c > 0) {\n            return b + c;\n        } else {\n            return b;\n        }\n    } else {\n        return c || 0;\n    }\n}\n",
                    "complex_nested.ts",
                    ComplexityExpectation {
                        min_cyclomatic: 7,
                        max_cyclomatic: 10,
                        min_cognitive: 10,
                        max_cognitive: 20,
                        min_nesting_depth: 3,
                    },
                ),
                Language::Rust => (
                    "fn complex_nested(a: i32, b: i32, c: i32) -> i32 {\n    if a > 0 {\n        if b > 0 {\n            if c > 0 {\n                a + b + c\n            } else {\n                a + b\n            }\n        } else if c > 0 {\n            a + c\n        } else {\n            a\n        }\n    } else if b > 0 {\n        if c > 0 {\n            b + c\n        } else {\n            b\n        }\n    } else {\n        if c != 0 { c } else { 0 }\n    }\n}\n",
                    "complex_nested.rs",
                    ComplexityExpectation {
                        min_cyclomatic: 7,
                        max_cyclomatic: 10,
                        min_cognitive: 10,
                        max_cognitive: 20,
                        min_nesting_depth: 3,
                    },
                ),
                Language::Go => (
                    "func complexNested(a int, b int, c int) int {\n    if a > 0 {\n        if b > 0 {\n            if c > 0 {\n                return a + b + c\n            } else {\n                return a + b\n            }\n        } else if c > 0 {\n            return a + c\n        } else {\n            return a\n        }\n    } else if b > 0 {\n        if c > 0 {\n            return b + c\n        } else {\n            return b\n        }\n    } else {\n        if c != 0 {\n            return c\n        }\n        return 0\n    }\n}\n",
                    "complex_nested.go",
                    ComplexityExpectation {
                        min_cyclomatic: 7,
                        max_cyclomatic: 10,
                        min_cognitive: 10,
                        max_cognitive: 20,
                        min_nesting_depth: 3,
                    },
                ),
            };

            (
                ComplexityFixture {
                    language: lang,
                    source_code: source,
                    file_name,
                },
                expectation,
            )
        })
    }

    /// Flat structure with guard clauses (same CC as nested, lower cognitive)
    pub fn flat_early_returns() -> ComplexityTestCase {
        ComplexityTestCase::new("flat_early_returns").with_all_languages(|lang| {
            let (source, file_name, expectation) = match lang {
                Language::Python => (
                    "def flat_guards(a, b, c):\n    if not a:\n        return False\n    if not b:\n        return False\n    if not c:\n        return False\n    return True\n",
                    "flat_guards.py",
                    ComplexityExpectation {
                        min_cyclomatic: 4,
                        max_cyclomatic: 5,
                        min_cognitive: 3,
                        max_cognitive: 8,
                        min_nesting_depth: 1,
                    },
                ),
                Language::TypeScript => (
                    "function flatGuards(a: boolean, b: boolean, c: boolean): boolean {\n    if (!a) return false;\n    if (!b) return false;\n    if (!c) return false;\n    return true;\n}\n",
                    "flat_guards.ts",
                    ComplexityExpectation {
                        min_cyclomatic: 4,
                        max_cyclomatic: 5,
                        min_cognitive: 3,
                        max_cognitive: 8,
                        min_nesting_depth: 1,
                    },
                ),
                Language::Rust => (
                    "fn flat_guards(a: bool, b: bool, c: bool) -> bool {\n    if !a {\n        return false;\n    }\n    if !b {\n        return false;\n    }\n    if !c {\n        return false;\n    }\n    true\n}\n",
                    "flat_guards.rs",
                    ComplexityExpectation {
                        min_cyclomatic: 4,
                        max_cyclomatic: 5,
                        min_cognitive: 3,
                        max_cognitive: 8,
                        min_nesting_depth: 1,
                    },
                ),
                Language::Go => (
                    "func flatGuards(a bool, b bool, c bool) bool {\n    if !a {\n        return false\n    }\n    if !b {\n        return false\n    }\n    if !c {\n        return false\n    }\n    return true\n}\n",
                    "flat_guards.go",
                    ComplexityExpectation {
                        min_cyclomatic: 4,
                        max_cyclomatic: 5,
                        min_cognitive: 3,
                        max_cognitive: 8,
                        min_nesting_depth: 1,
                    },
                ),
            };

            (
                ComplexityFixture {
                    language: lang,
                    source_code: source,
                    file_name,
                },
                expectation,
            )
        })
    }

    /// Get all predefined scenarios
    pub fn all() -> Vec<ComplexityTestCase> {
        vec![
            Self::simple_function(),
            Self::moderate_complexity(),
            Self::high_nested_complexity(),
            Self::flat_early_returns(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_has_all_languages() {
        let scenario = ComplexityScenarios::simple_function();
        assert_eq!(scenario.fixtures.len(), 4);

        let languages: Vec<Language> = scenario.fixtures.iter().map(|f| f.language).collect();
        assert!(languages.contains(&Language::Python));
        assert!(languages.contains(&Language::TypeScript));
        assert!(languages.contains(&Language::Rust));
        assert!(languages.contains(&Language::Go));
    }

    #[test]
    fn test_all_scenarios_defined() {
        let scenarios = ComplexityScenarios::all();
        assert_eq!(scenarios.len(), 4);
        assert_eq!(scenarios[0].scenario_name, "simple_function");
        assert_eq!(scenarios[1].scenario_name, "moderate_complexity");
        assert_eq!(scenarios[2].scenario_name, "high_nested_complexity");
        assert_eq!(scenarios[3].scenario_name, "flat_early_returns");
    }

    #[test]
    fn test_expectations_are_present() {
        let scenario = ComplexityScenarios::simple_function();
        assert_eq!(scenario.expectations.len(), 4);

        for lang in Language::all() {
            assert!(scenario.expectations.contains_key(&lang));
        }
    }

    #[test]
    fn test_file_extensions_correct() {
        let scenario = ComplexityScenarios::simple_function();

        for fixture in scenario.fixtures.iter() {
            let expected_ext = fixture.language.file_extension();
            assert!(
                fixture.file_name.ends_with(&format!(".{}", expected_ext)),
                "File {} should end with .{}",
                fixture.file_name,
                expected_ext
            );
        }
    }
}
