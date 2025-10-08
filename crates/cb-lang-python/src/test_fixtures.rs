//! Python test fixtures for integration testing
//!
//! This module defines Python-equivalent code samples for cross-language
//! testing. Each scenario represents a specific test case (simple function,
//! complex nested logic, etc.) with expected complexity metrics.

use cb_plugin_api::test_fixtures::*;

/// Get all Python test fixtures
pub fn python_test_fixtures() -> LanguageTestFixtures {
    LanguageTestFixtures {
        complexity_scenarios: vec![
            // Scenario 1: Simple function (CC=1, Cognitive=0)
            ComplexityFixture {
                scenario_name: "simple_function",
                source_code: "def simple(x):\n    return x + 1\n",
                file_name: "simple.py",
                expected_cyclomatic_min: 1,
                expected_cyclomatic_max: 1,
                expected_cognitive_min: 0,
                expected_cognitive_max: 1,
                expected_nesting_depth_min: 0,
            },

            // Scenario 2: Moderate complexity (CC=3)
            ComplexityFixture {
                scenario_name: "moderate_complexity",
                source_code: "def moderate(x):\n    if x > 0:\n        return x * 2\n    elif x < 0:\n        return x * -1\n    else:\n        return 0\n",
                file_name: "moderate.py",
                expected_cyclomatic_min: 3,
                expected_cyclomatic_max: 4,
                expected_cognitive_min: 2,
                expected_cognitive_max: 5,
                expected_nesting_depth_min: 1,
            },

            // Scenario 3: High nested complexity (CC=7+)
            ComplexityFixture {
                scenario_name: "high_nested_complexity",
                source_code: "def complex_nested(a, b, c):\n    if a > 0:\n        if b > 0:\n            if c > 0:\n                return a + b + c\n            else:\n                return a + b\n        elif c > 0:\n            return a + c\n        else:\n            return a\n    elif b > 0:\n        if c > 0:\n            return b + c\n        else:\n            return b\n    else:\n        return c if c else 0\n",
                file_name: "complex.py",
                expected_cyclomatic_min: 7,
                expected_cyclomatic_max: 10,
                expected_cognitive_min: 10,
                expected_cognitive_max: 20,
                expected_nesting_depth_min: 3,
            },

            // Scenario 4: Flat with early returns
            ComplexityFixture {
                scenario_name: "flat_early_returns",
                source_code: "def flat_guards(a, b, c):\n    if not a:\n        return False\n    if not b:\n        return False\n    if not c:\n        return False\n    return True\n",
                file_name: "flat.py",
                expected_cyclomatic_min: 4,
                expected_cyclomatic_max: 5,
                expected_cognitive_min: 3,
                expected_cognitive_max: 8,
                expected_nesting_depth_min: 1,
            },
        ],

        refactoring_scenarios: vec![
            // Extract variable
            RefactoringFixture {
                scenario_name: "extract_simple_expression",
                source_code: "def calculate():\n    result = 10 + 20\n    return result\n",
                file_name: "extract_var.py",
                operation: RefactoringOperation::ExtractVariable {
                    variable_name: "sum".to_string(),
                    start_line: 1,
                    start_char: 13,
                    end_line: 1,
                    end_char: 19,
                },
            },

            // Extract function
            RefactoringFixture {
                scenario_name: "extract_multiline_function",
                source_code: "def main():\n    x = 1\n    y = 2\n    result = x + y\n    print(result)\n",
                file_name: "extract_func.py",
                operation: RefactoringOperation::ExtractFunction {
                    new_name: "add_numbers".to_string(),
                    start_line: 1,
                    start_char: 4,
                    end_line: 3,
                    end_char: 18,
                },
            },

            // Inline variable
            RefactoringFixture {
                scenario_name: "inline_simple_variable",
                source_code: "def process():\n    multiplier = 2\n    result = 10 * multiplier\n    return result\n",
                file_name: "inline_var.py",
                operation: RefactoringOperation::InlineVariable {
                    line: 1,
                    character: 4,
                },
            },
        ],
    }
}